//! [`ImgDetections`]: decoded boxes from a `DetectionNetwork`.

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{fill_vec, fixed_string, Result};
use crate::message::{AnyMessage, Message, Sealed};

/// A `dai::ImgDetection`. Coordinates are normalised `[0, 1]` of the network's
/// input frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ImgDetection {
    pub label: u32,
    /// From the archive's class list; empty when it has none.
    pub label_name: String,
    pub confidence: f32,
    pub xmin: f32,
    pub ymin: f32,
    pub xmax: f32,
    pub ymax: f32,
}

/// A `dai::ImgDetections`.
#[derive(Clone, Debug)]
pub struct ImgDetections {
    any: AnyMessage,
}

impl Sealed for ImgDetections {}
impl Message for ImgDetections {
    const DATATYPE: Option<Datatype> = Some(Datatype::ImgDetections);

    fn from_any(any: AnyMessage) -> Result<Self> {
        Ok(ImgDetections { any })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl ImgDetection {
    fn from_raw(d: &sys::dai_img_detection) -> Self {
        ImgDetection {
            label: d.label,
            label_name: fixed_string(&d.label_name),
            confidence: d.confidence,
            xmin: d.xmin,
            ymin: d.ymin,
            xmax: d.xmax,
            ymax: d.ymax,
        }
    }
}

impl ImgDetections {
    /// `detections`, copied out.
    pub fn detections(&self) -> Result<Vec<ImgDetection>> {
        let raw = fill_vec(16, |buf| {
            let mut n = 0usize;
            let rc = unsafe {
                sys::dai_img_detections(self.any.raw(), buf.as_mut_ptr(), buf.len(), &mut n)
            };
            crate::error::check(rc).map(|()| n)
        })?;
        Ok(raw.iter().map(ImgDetection::from_raw).collect())
    }
}
