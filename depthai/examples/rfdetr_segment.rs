//! RF-DETR nano instance segmentation on an OAK 4 (RVC4), decoded on the host,
//! with the H.264 stream **gated on detections**: the encoder only runs (and the
//! link only carries video) for a burst after something is seen.
//!
//! Model: <https://models.luxonis.com/luxonis/rf-detr-nano-instance-segmentation>
//! (`luxonis/rfdetr-nano-instance-segmentation:coco-312x312`, RVC4 only). The
//! zoo ships it with a Python-only parser, so the decode lives here:
//!
//! - `boxes  [1, Q, 4]`      cx, cy, w, h normalised to the input
//! - `scores [1, Q, 91]`     class logits (sigmoid; index 0 = background)
//! - `masks  [1, Q, H, W]`   per-query mask logits (sigmoid > 0.5)
//!
//! The layers are found by shape, not name, so a re-export with renamed heads
//! still decodes. Class ids are COCO-91 indices (1 = person, 3 = car, ...).
//!
//! `cargo run --example rfdetr_segment [-- <device-id-or-ip>]`

use std::time::{Duration, Instant};

use depthai::node::{Camera, Gate, NeuralNetwork, VideoEncoder};
use depthai::{
    CameraBoardSocket, Device, GateControl, ImgFrame, ImgFrameType, ImgResizeMode, Message,
    NNModelDescription, NnData, Pipeline, TensorInfo, VideoEncoderProfile,
};

const MODEL: &str = "luxonis/rfdetr-nano-instance-segmentation:coco-312x312";
const SCORE_THRESHOLD: f32 = 0.5;
const MASK_THRESHOLD: f32 = 0.5;
/// H.264 frames to pass through the gate per detection event.
const BURST_FRAMES: u32 = 15;

fn main() -> depthai::Result<()> {
    let id = std::env::args().nth(1);
    let dev = Device::open(id.as_deref(), None)?;
    let pipeline = Pipeline::new(&dev)?;

    let cam = pipeline.create::<Camera>()?;
    cam.build(CameraBoardSocket::CamA)?;

    // Inference: the node requests its own 312x312 input from the camera.
    let nn = pipeline.create::<NeuralNetwork>()?;
    nn.build_camera(&cam, &NNModelDescription::new(MODEL), Some(10.0), None)?;
    let nn_q = nn.out()?.create_output_queue(4, false)?;

    // Video: camera NV12 -> Gate -> H.264, opened in bursts from the host.
    let nv12 = cam.request_output(
        (640, 360),
        Some(ImgFrameType::Nv12),
        ImgResizeMode::Crop,
        Some(30.0),
        Some(true),
    )?;
    let gate = pipeline.create::<Gate>()?;
    nv12.link(&gate.input()?)?;
    let gate_ctl = gate.input_control()?.create_input_queue(4, false)?;
    let enc = pipeline.create::<VideoEncoder>()?;
    enc.set_default_profile_preset(30.0, VideoEncoderProfile::H264Baseline)?;
    enc.set_keyframe_frequency(BURST_FRAMES as i32)?;
    gate.output()?.cast::<ImgFrame>().link(&enc.input()?)?;
    let video_q = enc.bitstream()?.create_output_queue(30, false)?;

    pipeline.start()?;
    gate_ctl.send(&GateControl::close()?)?;
    println!(
        "running for 20 s; video passes only in {BURST_FRAMES}-frame bursts after a detection"
    );

    let start = Instant::now();
    let (mut video_frames, mut video_bytes) = (0u32, 0usize);
    while start.elapsed() < Duration::from_secs(20) {
        while let Some(au) = video_q.try_get()? {
            video_frames += 1;
            video_bytes += au.data().len();
        }
        let Some(out) = nn_q.get(Duration::from_millis(200))? else {
            continue;
        };
        let dets = decode(&out)?;
        if dets.is_empty() {
            continue;
        }
        gate_ctl.send(&GateControl::open_for(BURST_FRAMES, None)?)?;
        println!(
            "#{:<5} {} instances -> video burst",
            out.sequence_num(),
            dets.len()
        );
        for d in &dets {
            println!(
                "    class {:<3} {:.2}  box [{:.2} {:.2} {:.2} {:.2}]  mask {} px",
                d.class, d.score, d.bbox[0], d.bbox[1], d.bbox[2], d.bbox[3], d.mask_pixels
            );
        }
    }
    println!(
        "video delivered: {video_frames} access units, {} kB",
        video_bytes / 1024
    );
    pipeline.stop()?;
    Ok(())
}

struct Instance {
    class: usize,
    score: f32,
    /// xmin, ymin, xmax, ymax normalised to the network input.
    bbox: [f32; 4],
    /// Pixels of the (H x W) mask above `MASK_THRESHOLD`.
    mask_pixels: usize,
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn logit(p: f32) -> f32 {
    (p / (1.0 - p)).ln()
}

/// Pick the three heads by shape: `[1, Q, 4]`, `[1, Q, C]`, `[1, Q, H, W]`.
fn find_heads(tensors: &[TensorInfo]) -> Option<(&TensorInfo, &TensorInfo, &TensorInfo)> {
    let boxes = tensors
        .iter()
        .find(|t| t.dims.len() == 3 && t.dims[2] == 4)?;
    let queries = boxes.dims[1];
    let scores = tensors
        .iter()
        .find(|t| t.dims.len() == 3 && t.dims[1] == queries && t.dims[2] > 4)?;
    let masks = tensors
        .iter()
        .find(|t| t.dims.len() == 4 && t.dims[1] == queries)?;
    Some((boxes, scores, masks))
}

fn decode(out: &NnData) -> depthai::Result<Vec<Instance>> {
    let tensors = out.tensors()?;
    let Some((boxes_t, scores_t, masks_t)) = find_heads(&tensors) else {
        let names: Vec<_> = tensors
            .iter()
            .map(|t| format!("{}{:?}", t.name, t.dims))
            .collect();
        eprintln!("unexpected heads: {}", names.join(" "));
        return Ok(Vec::new());
    };
    let boxes = out.tensor_f32(boxes_t, true)?;
    let scores = out.tensor_f32(scores_t, true)?;
    let classes = scores_t.dims[2] as usize;
    // sigmoid(x) > t  <=>  x > logit(t): compare logits, no exp per element.
    let score_logit = logit(SCORE_THRESHOLD);
    let mask_logit = logit(MASK_THRESHOLD);

    // Pass 1: queries whose best non-background class clears the threshold.
    let mut found: Vec<(usize, Instance)> = Vec::new();
    for (q, (bx, row)) in boxes
        .chunks_exact(4)
        .zip(scores.chunks_exact(classes))
        .enumerate()
    {
        let Some((class, &best)) = row
            .iter()
            .enumerate()
            .skip(1)
            .max_by(|a, b| a.1.total_cmp(b.1))
        else {
            continue;
        };
        if best <= score_logit {
            continue;
        }
        let [cx, cy, w, h] = [bx[0], bx[1], bx[2], bx[3]];
        let bbox = [
            (cx - w / 2.0).clamp(0.0, 1.0),
            (cy - h / 2.0).clamp(0.0, 1.0),
            (cx + w / 2.0).clamp(0.0, 1.0),
            (cy + h / 2.0).clamp(0.0, 1.0),
        ];
        found.push((
            q,
            Instance {
                class,
                score: sigmoid(best),
                bbox,
                mask_pixels: 0,
            },
        ));
    }
    // Pass 2: masks only when something was found (the big tensor).
    if !found.is_empty() {
        let masks = out.tensor_f32(masks_t, true)?;
        let mask_hw = (masks_t.dims[2] * masks_t.dims[3]) as usize;
        for (q, inst) in &mut found {
            inst.mask_pixels = masks[*q * mask_hw..(*q + 1) * mask_hw]
                .iter()
                .filter(|&&v| v > mask_logit)
                .count();
        }
    }
    let mut found: Vec<Instance> = found.into_iter().map(|(_, i)| i).collect();
    found.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(found)
}
