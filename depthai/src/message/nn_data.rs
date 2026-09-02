//! [`NnData`]: the raw output tensors of a `NeuralNetwork`, zero-copy.

use depthai_sys as sys;

use crate::enums::{Datatype, StorageOrder, TensorDataType};
use crate::error::{check, cstring, fill_vec, fixed_string, poll_val, DepthaiError, Result};
use crate::message::{AnyMessage, Message, Sealed};

/// `dai::TensorInfo`: where one layer lives in the [`NnData`] buffer and how to
/// read it.
#[derive(Clone, Debug, PartialEq)]
pub struct TensorInfo {
    pub name: String,
    pub datatype: TensorDataType,
    pub order: StorageOrder,
    /// Shape, outermost first (per `order`).
    pub dims: Vec<u32>,
    /// Bytes between consecutive elements along each dim, as reported (often empty).
    pub strides: Vec<u32>,
    /// Byte offset of the first element in the message buffer.
    pub offset: u32,
    /// Whether `qp_scale`/`qp_zp` apply: real = `(stored - qp_zp) * qp_scale`.
    pub quantization: bool,
    pub qp_scale: f32,
    pub qp_zp: f32,
    /// `getDataTypeSize()`: bytes per element.
    pub elem_size: usize,
}

impl TensorInfo {
    fn from_raw(r: &sys::dai_tensor_info) -> Self {
        TensorInfo {
            name: fixed_string(&r.name),
            datatype: TensorDataType::from_raw(r.datatype),
            order: StorageOrder::from_raw(r.order),
            dims: r.dims[..(r.num_dims as usize).min(8)].to_vec(),
            strides: r.strides[..(r.num_strides as usize).min(8)].to_vec(),
            offset: r.offset,
            quantization: r.quantization != 0,
            qp_scale: r.qp_scale,
            qp_zp: r.qp_zp,
            elem_size: r.elem_size as usize,
        }
    }

    /// Product of `dims`.
    pub fn num_elements(&self) -> usize {
        self.dims.iter().map(|&d| d as usize).product()
    }
}

/// A `dai::NNData`: one buffer holding every output layer, described by
/// [`TensorInfo`]s. [`tensor_bytes`](Self::tensor_bytes) is zero-copy;
/// [`tensor_f32`](Self::tensor_f32) is `getTensor<float>`.
#[derive(Clone, Debug)]
pub struct NnData {
    any: AnyMessage,
}

impl Sealed for NnData {}
impl Message for NnData {
    const DATATYPE: Option<Datatype> = Some(Datatype::NnData);

    fn from_any(any: AnyMessage) -> Result<Self> {
        Ok(NnData { any })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl NnData {
    /// `tensors`: every layer's [`TensorInfo`].
    pub fn tensors(&self) -> Result<Vec<TensorInfo>> {
        let raw = fill_vec(8, |buf| {
            let mut n = 0usize;
            let rc = unsafe {
                sys::dai_nn_data_tensors(self.any.raw(), buf.as_mut_ptr(), buf.len(), &mut n)
            };
            check(rc).map(|()| n)
        })?;
        Ok(raw.iter().map(TensorInfo::from_raw).collect())
    }

    /// `getTensorInfo(name)`; `None` when there is no such layer.
    pub fn tensor_info(&self, name: &str) -> Result<Option<TensorInfo>> {
        let name = cstring(name)?;
        poll_val(|raw| unsafe { sys::dai_nn_data_tensor_info(self.any.raw(), name.as_ptr(), raw) })
            .map(|r| r.as_ref().map(TensorInfo::from_raw))
    }

    /// The bytes of `info`'s layer: `num_elements * elem_size` from `offset`, as
    /// stored (`getTensor` reads them contiguously too; `strides` are informational).
    pub fn tensor_bytes(&self, info: &TensorInfo) -> Result<&[u8]> {
        let start = info.offset as usize;
        let end = start + info.num_elements() * info.elem_size;
        self.any.data().get(start..end).ok_or_else(|| {
            DepthaiError::Malformed(format!(
                "layer {:?} spans {start}..{end} of a {}-byte buffer",
                info.name,
                self.any.data().len()
            ))
        })
    }

    /// `getTensor<float>(name, dequantize)`: `info`'s layer as dense `f32` in
    /// `dims` order; `dequantize` applies `qp_scale`/`qp_zp` when the layer is
    /// quantised.
    pub fn tensor_f32(&self, info: &TensorInfo, dequantize: bool) -> Result<Vec<f32>> {
        let read = reader(info.datatype).ok_or_else(|| {
            DepthaiError::Malformed(format!(
                "layer {:?}: unknown datatype {:?}",
                info.name, info.datatype
            ))
        })?;
        let bytes = self.tensor_bytes(info)?;
        let mut out: Vec<f32> = bytes.chunks_exact(info.elem_size).map(read).collect();
        if dequantize && info.quantization {
            for v in &mut out {
                *v = (*v - info.qp_zp) * info.qp_scale;
            }
        }
        Ok(out)
    }

    /// `NNData::fp16_to_fp32`.
    pub fn fp16_to_fp32(h: u16) -> f32 {
        let sign = (h as u32 & 0x8000) << 16;
        let exp = (h >> 10) as u32 & 0x1f;
        let frac = h as u32 & 0x3ff;
        let bits = match exp {
            0 if frac == 0 => sign,
            0 => {
                // Subnormal: renormalise.
                let shift = frac.leading_zeros() - 21;
                let m = (frac << shift) & 0x3ff;
                sign | ((113 - shift) << 23) | (m << 13)
            }
            31 => sign | 0x7f80_0000 | (frac << 13),
            _ => sign | ((exp + 112) << 23) | (frac << 13),
        };
        f32::from_bits(bits)
    }
}

/// One element (little-endian, `elem_size` bytes) → f32, per `getTensor<float>`.
fn reader(datatype: TensorDataType) -> Option<fn(&[u8]) -> f32> {
    Some(match datatype {
        TensorDataType::Fp16 => |b| NnData::fp16_to_fp32(u16::from_le_bytes([b[0], b[1]])),
        TensorDataType::Fp32 => |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        TensorDataType::Fp64 => |b| f64::from_le_bytes(b.try_into().unwrap()) as f32,
        TensorDataType::Int => |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32,
        TensorDataType::U8f => |b| b[0] as f32,
        TensorDataType::I8 => |b| b[0] as i8 as f32,
        TensorDataType::U16f => |b| u16::from_le_bytes([b[0], b[1]]) as f32,
        TensorDataType::Other(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_conversion() {
        let f = NnData::fp16_to_fp32;
        assert_eq!(f(0x3c00), 1.0);
        assert_eq!(f(0xc000), -2.0);
        assert_eq!(f(0x3800), 0.5);
        assert_eq!(f(0x0000), 0.0);
        assert_eq!(f(0x0001), 5.960_464_5e-8);
        assert_eq!(f(0x03ff), 6.097_555e-5);
        assert_eq!(f(0x7bff), 65504.0);
        assert!(f(0x7c00).is_infinite());
        assert!(f(0x7e00).is_nan());
    }

    #[test]
    fn element_readers() {
        assert_eq!(reader(TensorDataType::I8).unwrap()(&[0xff]), -1.0);
        assert_eq!(reader(TensorDataType::U8f).unwrap()(&[0xff]), 255.0);
        assert_eq!(
            reader(TensorDataType::Fp32).unwrap()(&1.5f32.to_le_bytes()),
            1.5
        );
        assert_eq!(
            reader(TensorDataType::Int).unwrap()(&(-7i32).to_le_bytes()),
            -7.0
        );
        assert_eq!(
            reader(TensorDataType::U16f).unwrap()(&[0x34, 0x12]),
            0x1234 as f32
        );
        assert!(reader(TensorDataType::Other(99)).is_none());
    }
}
