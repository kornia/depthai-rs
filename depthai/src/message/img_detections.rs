//! [`ImgDetections`]: decoded boxes from a `DetectionNetwork`.

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{check, fixed_string, out_val, Result};
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

impl ImgDetections {
    pub fn len(&self) -> Result<usize> {
        out_val(|out| unsafe { sys::dai_img_detections_count(self.any.raw(), out) })
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// `detections`, copied out.
    pub fn detections(&self) -> Result<Vec<ImgDetection>> {
        let n = self.len()?;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let mut d = sys::dai_img_detection::default();
            check(unsafe { sys::dai_img_detections_get(self.any.raw(), i, &mut d) })?;
            out.push(ImgDetection {
                label: d.label,
                label_name: fixed_string(&d.label_name),
                confidence: d.confidence,
                xmin: d.xmin,
                ymin: d.ymin,
                xmax: d.xmax,
                ymax: d.ymax,
            });
        }
        Ok(out)
    }
}
