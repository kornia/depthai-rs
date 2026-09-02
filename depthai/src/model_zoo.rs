//! `dai::NNModelDescription` and `dai::getModelFromZoo`: naming a model on the
//! Luxonis model zoo and fetching its NNArchive.

use std::ffi::CString;
use std::path::{Path, PathBuf};

use depthai_sys as sys;

use crate::error::{cstring, out_string, Result};

/// A `dai::NNModelDescription`: which zoo model, for which platform. Only
/// `model` is required; `platform` is filled in from the device by
/// [`NeuralNetwork::build_camera`](crate::node::NeuralNetwork::build_camera) /
/// [`DetectionNetwork::build_camera`](crate::node::DetectionNetwork::build_camera),
/// and must be given (`"RVC2"` / `"RVC4"`) for a bare
/// [`get_model_from_zoo`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NNModelDescription {
    /// Zoo slug, e.g. `"luxonis/yolov6-nano:r2-coco-512x288"`.
    pub model: String,
    pub platform: Option<String>,
    pub optimization_level: Option<String>,
    pub compression_level: Option<String>,
    pub snpe_version: Option<String>,
    pub model_precision_type: Option<String>,
}

impl NNModelDescription {
    pub fn new(model: impl Into<String>) -> Self {
        NNModelDescription {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = Some(platform.into());
        self
    }

    /// Run `f` with the C mirror; the `CString`s live for the call.
    pub(crate) fn with_raw<R>(
        &self,
        f: impl FnOnce(&sys::dai_nn_model_description) -> Result<R>,
    ) -> Result<R> {
        let model = cstring(&self.model)?;
        let opt = |o: &Option<String>| -> Result<Option<CString>> {
            o.as_deref().map(cstring).transpose()
        };
        let platform = opt(&self.platform)?;
        let optimization_level = opt(&self.optimization_level)?;
        let compression_level = opt(&self.compression_level)?;
        let snpe_version = opt(&self.snpe_version)?;
        let model_precision_type = opt(&self.model_precision_type)?;
        let ptr = |o: &Option<CString>| o.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        let raw = sys::dai_nn_model_description {
            model: model.as_ptr(),
            platform: ptr(&platform),
            optimization_level: ptr(&optimization_level),
            compression_level: ptr(&compression_level),
            snpe_version: ptr(&snpe_version),
            model_precision_type: ptr(&model_precision_type),
        };
        f(&raw)
    }
}

/// `dai::getModelFromZoo(desc, useCached, cacheDir, apiKey)`: download (or find
/// cached) the NNArchive for `desc` and return its path. `cache_dir` `None` =
/// depthai's default (`DEPTHAI_ZOO_CACHE_PATH`, else `.depthai_cached_models`
/// in the working directory). Needs `desc.platform`.
pub fn get_model_from_zoo(
    desc: &NNModelDescription,
    use_cached: bool,
    cache_dir: Option<&Path>,
    api_key: Option<&str>,
) -> Result<PathBuf> {
    let cache_dir = cache_dir
        .map(|p| cstring(&p.to_string_lossy()))
        .transpose()?;
    let api_key = api_key.map(cstring).transpose()?;
    let ptr = |o: &Option<CString>| o.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    desc.with_raw(|raw| {
        out_string(|out| unsafe {
            sys::dai_model_zoo_get(raw, use_cached as i32, ptr(&cache_dir), ptr(&api_key), out)
        })
    })
    .map(PathBuf::from)
}
