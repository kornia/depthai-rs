//! Safe Rust for Luxonis **depthai-core v3** (OAK cameras).
//!
//! A faithful, unopinionated wrapper shaped like depthai's own node-graph API:
//! open a [`Device`], build a [`Pipeline`] of nodes ([`Camera`](node::Camera),
//! [`Sync`](node::Sync), [`StereoDepth`](node::StereoDepth),
//! [`VideoEncoder`](node::VideoEncoder), [`Imu`](node::Imu)), link
//! [`Output`]s to [`Input`]s, create [`OutputQueue`]s, start, and pull typed
//! messages ([`ImgFrame`], [`MessageGroup`], [`ImuData`], [`EncodedFrame`]).
//!
//! ```no_run
//! use std::time::Duration;
//! use depthai::{node::{Camera, Sync}, CameraBoardSocket, Device, ImgFrame, ImgFrameType,
//!               ImgResizeMode, Message, MessageGroup, Pipeline};
//!
//! # fn main() -> depthai::Result<()> {
//! let dev = Device::open(None, None)?;
//! let pipeline = Pipeline::new(&dev)?;
//! let left = pipeline.create::<Camera>()?;
//! left.build(CameraBoardSocket::CamB)?;
//! let right = pipeline.create::<Camera>()?;
//! right.build(CameraBoardSocket::CamC)?;
//! let lo = left.request_output((640, 400), Some(ImgFrameType::Gray8), ImgResizeMode::Crop, Some(30.0), Some(false))?;
//! let ro = right.request_output((640, 400), Some(ImgFrameType::Gray8), ImgResizeMode::Crop, Some(30.0), Some(false))?;
//! let sync = pipeline.create::<Sync>()?;
//! sync.set_sync_threshold(Duration::from_millis(16))?;
//! lo.link(&sync.input("left")?)?;
//! ro.link(&sync.input("right")?)?;
//! let q = sync.out()?.create_output_queue(4, false)?;
//! pipeline.start()?;
//! while let Some(group) = q.get(Duration::from_secs(1))? {
//!     let l: ImgFrame = group.get("left")?.expect("left");
//!     println!("{}x{} at {:?}", l.width(), l.height(), l.timestamp());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! What this crate deliberately does **not** do: read environment variables,
//! convert timestamps to wall-clock, clamp rates, choose presets, repack strides,
//! or default the unit/spec-translation choices in
//! [`CalibrationHandler`]. Those are policy, and belong to the driver built on
//! top (e.g. `sensor-oak` in `kornia/sensor-rt`).
//!
//! # Threading
//!
//! Every handle is `Send + Sync`. Messages are immutable and refcounted, so
//! they can be cloned, retained past the next poll, and shared across threads
//! (zero-copy). Queues are internally synchronised. Build the graph from one
//! thread before [`Pipeline::start`]; that is depthai's own contract.
//!
//! # Building
//!
//! `depthai-sys` links against a depthai-core install prefix (`DEPTHAI_PREFIX`,
//! or a `vendor/depthai` found by walking up, or the `vendored` feature). See its
//! `build.rs`.

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]

pub mod bootloader;
pub mod calibration;
pub mod device;
pub mod enums;
pub mod error;
pub mod message;
pub mod node;
pub mod pipeline;
pub mod port;
pub mod queue;

pub use bootloader::DeviceBootloader;
pub use calibration::CalibrationHandler;
pub use device::{build_version, steady_now, Device, DeviceInfo};
pub use enums::*;
pub use error::{DepthaiError, Result};
pub use message::{
    AnyMessage, EncodedFrame, EncodedFrameInfo, ImgFrame, ImgFrameInfo, ImuData, ImuPacket,
    ImuRotationVector, ImuVecReport, Message, MessageGroup, Msg, RawTimestamp,
};
pub use node::{Node, NodeHandle, NodeType};
pub use pipeline::Pipeline;
pub use port::{Input, Output};
pub use queue::OutputQueue;

/// The depthai-core tag the constants in this crate are pinned against.
pub const DEPTHAI_CORE_TAG: &str = depthai_sys::DEPTHAI_CORE_TAG;
