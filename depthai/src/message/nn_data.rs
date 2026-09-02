//! [`NnData`]: the raw output tensors of a `NeuralNetwork`, zero-copy.

use depthai_sys as sys;

use crate::enums::{Datatype, StorageOrder, TensorDataType};
use crate::error::{cstring, fixed_string, out_lines, take_native_error, DepthaiError, Result};
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
    /// Bytes between consecutive elements along each dim; may be empty (dense).
    pub strides: Vec<u32>,
    /// Byte offset of the first element in the message buffer.
    pub offset: u32,
    /// Quantisation: real = `(stored - qp_zp) * qp_scale` (depthai defaults 1 / 0).
    pub qp_scale: f32,
    pub qp_zp: f32,
}

impl TensorInfo {
    fn from_raw(r: &sys::dai_tensor_info) -> Self {
        let n = (r.num_dims as usize).min(8);
        TensorInfo {
            name: fixed_string(&r.name),
            datatype: TensorDataType::from_raw(r.datatype),
            order: StorageOrder::from_raw(r.order),
            dims: r.dims[..n].to_vec(),
            strides: r.strides[..n].to_vec(),
            offset: r.offset,
            qp_scale: r.qp_scale,
            qp_zp: r.qp_zp,
        }
    }

    /// Product of `dims`.
    pub fn num_elements(&self) -> usize {
        self.dims.iter().map(|&d| d as usize).product()
    }

    /// Byte offset of the element at `index` (one coordinate per dim), relative
    /// to `offset`; dense when `strides` is empty.
    fn element_offset(&self, index: &[usize], elem: usize) -> usize {
        if self.strides.len() == self.dims.len() {
            index
                .iter()
                .zip(&self.strides)
                .map(|(&i, &s)| i * s as usize)
                .sum()
        } else {
            index
                .iter()
                .zip(&self.dims)
                .fold(0, |acc, (&i, &d)| acc * d as usize + i)
                * elem
        }
    }

    /// Bytes the tensor spans in the buffer (highest extent over the dims).
    fn byte_extent(&self, elem: usize) -> usize {
        if self.dims.is_empty() {
            return 0;
        }
        let last: Vec<usize> = self
            .dims
            .iter()
            .map(|&d| d.saturating_sub(1) as usize)
            .collect();
        self.element_offset(&last, elem) + elem
    }
}

/// A `dai::NNData`: one buffer holding every output layer, described by
/// [`TensorInfo`]s. [`tensor_bytes`](Self::tensor_bytes) is zero-copy;
/// [`tensor_f32`](Self::tensor_f32) converts (f16, quantised u8/i8/u16, i32,
/// f64) into a dense `Vec<f32>` in `dims` order.
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
    /// `getAllLayerNames()`.
    pub fn layer_names(&self) -> Result<Vec<String>> {
        out_lines(|out| unsafe { sys::dai_nn_data_layer_names(self.any.raw(), out) })
    }

    /// `getTensorInfo(name)`; `None` when there is no such layer.
    pub fn tensor_info(&self, name: &str) -> Result<Option<TensorInfo>> {
        let name = cstring(name)?;
        let mut raw = sys::dai_tensor_info::default();
        match unsafe { sys::dai_nn_data_tensor_info(self.any.raw(), name.as_ptr(), &mut raw) } {
            1 => Ok(Some(TensorInfo::from_raw(&raw))),
            0 => Ok(None),
            _ => Err(take_native_error()),
        }
    }

    /// Every layer's [`TensorInfo`], in `layer_names()` order.
    pub fn tensors(&self) -> Result<Vec<TensorInfo>> {
        self.layer_names()?
            .iter()
            .filter_map(|n| self.tensor_info(n).transpose())
            .collect()
    }

    /// The bytes of `info`'s layer as stored (strided, not converted).
    pub fn tensor_bytes(&self, info: &TensorInfo) -> Result<&[u8]> {
        let elem = info.datatype.size().ok_or_else(|| {
            DepthaiError::Malformed(format!(
                "layer {:?}: unknown datatype {:?}",
                info.name, info.datatype
            ))
        })?;
        let start = info.offset as usize;
        let end = start + info.byte_extent(elem);
        self.any.data().get(start..end).ok_or_else(|| {
            DepthaiError::Malformed(format!(
                "layer {:?} spans {start}..{end} of a {}-byte buffer",
                info.name,
                self.any.data().len()
            ))
        })
    }

    /// `info`'s layer as dense `f32`, dequantised for the integer types.
    pub fn tensor_f32(&self, info: &TensorInfo) -> Result<Vec<f32>> {
        let bytes = self.tensor_bytes(info)?;
        let elem = info.datatype.size().unwrap_or(0);
        let n = info.num_elements();
        let mut out = Vec::with_capacity(n);
        let mut index = vec![0usize; info.dims.len()];
        let dequant = |v: f32| (v - info.qp_zp) * info.qp_scale;
        for _ in 0..n {
            let at = info.element_offset(&index, elem);
            let b = &bytes[at..at + elem];
            out.push(match info.datatype {
                TensorDataType::Fp16 => f16_to_f32(u16::from_le_bytes([b[0], b[1]])),
                TensorDataType::Fp32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
                TensorDataType::Fp64 => f64::from_le_bytes(b.try_into().unwrap()) as f32,
                TensorDataType::Int => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32,
                TensorDataType::U8f => dequant(b[0] as f32),
                TensorDataType::I8 => dequant(b[0] as i8 as f32),
                TensorDataType::U16f => dequant(u16::from_le_bytes([b[0], b[1]]) as f32),
                TensorDataType::Other(_) => unreachable!("tensor_bytes rejects unknown types"),
            });
            // Odometer increment, innermost dim fastest.
            for (i, d) in index.iter_mut().zip(&info.dims).rev() {
                *i += 1;
                if *i < *d as usize {
                    break;
                }
                *i = 0;
            }
        }
        Ok(out)
    }
}

/// IEEE 754 binary16 → f32 (no `half` dependency).
pub fn f16_to_f32(h: u16) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_conversion() {
        assert_eq!(f16_to_f32(0x3c00), 1.0);
        assert_eq!(f16_to_f32(0xc000), -2.0);
        assert_eq!(f16_to_f32(0x3800), 0.5);
        assert_eq!(f16_to_f32(0x0000), 0.0);
        assert_eq!(f16_to_f32(0x0001), 5.960_464_5e-8);
        assert_eq!(f16_to_f32(0x03ff), 6.097_555e-5);
        assert_eq!(f16_to_f32(0x7bff), 65504.0);
        assert!(f16_to_f32(0x7c00).is_infinite());
        assert!(f16_to_f32(0x7e00).is_nan());
    }

    fn info(dims: &[u32], strides: &[u32], datatype: TensorDataType) -> TensorInfo {
        TensorInfo {
            name: "t".into(),
            datatype,
            order: StorageOrder::Nchw,
            dims: dims.to_vec(),
            strides: strides.to_vec(),
            offset: 0,
            qp_scale: 1.0,
            qp_zp: 0.0,
        }
    }

    #[test]
    fn dense_and_strided_extents() {
        assert_eq!(
            info(&[1, 100, 4], &[], TensorDataType::Fp16).byte_extent(2),
            800
        );
        // Padded rows: 2x3 u8 with a row stride of 8.
        let t = info(&[2, 3], &[8, 1], TensorDataType::U8f);
        assert_eq!(t.byte_extent(1), 8 + 2 + 1);
        assert_eq!(t.element_offset(&[1, 2], 1), 10);
        assert_eq!(info(&[], &[], TensorDataType::Fp32).byte_extent(4), 0);
    }
}
