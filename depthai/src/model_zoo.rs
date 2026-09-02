//! `dai::NNModelDescription` and `dai::getModelFromZoo`: naming a model on the
//! Luxonis model zoo and fetching its NNArchive.

use std::path::{Path, PathBuf};

use depthai_sys as sys;

use crate::error::{cstring, opt_cstring, opt_ptr, out_string, path_cstring, Result};

/// A `dai::NNModelDescription`: which zoo model, for which platform. Only
/// `model` is required; `platform` is filled in from the device by
/// [`NeuralNetwork::build_camera`](crate::node::NeuralNetwork::build_camera) /
/// [`DetectionNetwork::build_camera`](crate::node::DetectionNetwork::build_camera),
/// and must be given (`"RVC2"` / `"RVC4"`) for a bare [`get_model_from_zoo`].
/// `None` fields take depthai's defaults.
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

    /// Run `f` with the C mirror; the `CString`s live for the call.
    pub(crate) fn with_raw<R>(
        &self,
        f: impl FnOnce(&sys::dai_nn_model_description) -> Result<R>,
    ) -> Result<R> {
        let model = cstring(&self.model)?;
        let optional = [
            &self.platform,
            &self.optimization_level,
            &self.compression_level,
            &self.snpe_version,
            &self.model_precision_type,
        ]
        .map(|o| opt_cstring(o.as_deref()));
        let [platform, optimization_level, compression_level, snpe_version, model_precision_type] =
            optional;
        let (platform, optimization_level, compression_level, snpe_version, model_precision_type) = (
            platform?,
            optimization_level?,
            compression_level?,
            snpe_version?,
            model_precision_type?,
        );
        f(&sys::dai_nn_model_description {
            model: model.as_ptr(),
            platform: opt_ptr(&platform),
            optimization_level: opt_ptr(&optimization_level),
            compression_level: opt_ptr(&compression_level),
            snpe_version: opt_ptr(&snpe_version),
            model_precision_type: opt_ptr(&model_precision_type),
        })
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
    let cache_dir = cache_dir.map(path_cstring).transpose()?;
    let api_key = opt_cstring(api_key)?;
    desc.with_raw(|raw| {
        out_string(|out| unsafe {
            sys::dai_model_zoo_get(
                raw,
                use_cached as i32,
                opt_ptr(&cache_dir),
                opt_ptr(&api_key),
                out,
            )
        })
    })
    .map(PathBuf::from)
}
