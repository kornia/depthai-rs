//! [`ImuData`]: one batch of IMU packets from the [`Imu`](crate::node::Imu) node.

use std::time::Duration;

use depthai_sys as sys;

use crate::enums::{Datatype, ImuAccuracy};
use crate::error::{check, Result};
use crate::message::{Message, Msg, Sealed};

/// A raw `dai::Timestamp { sec, nsec }` on the host `steady_clock`.
///
/// depthai value-initialises the reports inside an `IMUPacket`, so a report the
/// firmware did not fill in shows up with a `{0, 0}` timestamp — check
/// [`is_zero`](Self::is_zero) before trusting such a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RawTimestamp {
    pub sec: i64,
    pub nsec: i64,
}

impl RawTimestamp {
    /// The "never written" sentinel.
    pub fn is_zero(&self) -> bool {
        self.sec == 0 && self.nsec == 0
    }

    /// As nanoseconds since the clock's epoch (what `IMUReport::getTimestamp()` yields).
    pub fn as_nanos(&self) -> i64 {
        self.sec
            .saturating_mul(1_000_000_000)
            .saturating_add(self.nsec)
    }

    pub fn as_duration(&self) -> Duration {
        Duration::from_nanos(self.as_nanos().max(0) as u64)
    }
}

/// One `dai::IMUReport` with a 3-vector: accelerometer (m/s²), gyroscope (rad/s)
/// or magnetometer (µT).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuVecReport {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Host `steady_clock` timestamp (raw).
    pub timestamp: RawTimestamp,
    /// Device clock timestamp (raw).
    pub timestamp_device: RawTimestamp,
    pub sequence: i32,
    /// Note: firmware leaves this `Unreliable` for the `*_RAW` streams.
    pub accuracy: ImuAccuracy,
}

impl ImuVecReport {
    fn from_raw(r: &sys::dai_imu_vec_report) -> Self {
        ImuVecReport {
            x: r.x,
            y: r.y,
            z: r.z,
            timestamp: RawTimestamp {
                sec: r.ts_sec,
                nsec: r.ts_nsec,
            },
            timestamp_device: RawTimestamp {
                sec: r.ts_device_sec,
                nsec: r.ts_device_nsec,
            },
            sequence: r.sequence,
            accuracy: ImuAccuracy::from_raw(r.accuracy),
        }
    }

    pub fn xyz(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

/// `dai::IMUReportRotationVectorWAcc`: a unit quaternion plus accuracy estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuRotationVector {
    pub i: f32,
    pub j: f32,
    pub k: f32,
    pub real: f32,
    /// Accuracy estimate in radians (0 = no estimate).
    pub accuracy_rad: f32,
    pub timestamp: RawTimestamp,
    pub timestamp_device: RawTimestamp,
    pub sequence: i32,
    pub accuracy: ImuAccuracy,
}

impl ImuRotationVector {
    fn from_raw(r: &sys::dai_imu_rotvec_report) -> Self {
        ImuRotationVector {
            i: r.i,
            j: r.j,
            k: r.k,
            real: r.real,
            accuracy_rad: r.accuracy_rad,
            timestamp: RawTimestamp {
                sec: r.ts_sec,
                nsec: r.ts_nsec,
            },
            timestamp_device: RawTimestamp {
                sec: r.ts_device_sec,
                nsec: r.ts_device_nsec,
            },
            sequence: r.sequence,
            accuracy: ImuAccuracy::from_raw(r.accuracy),
        }
    }
}

/// One `dai::IMUPacket`. Only the reports for sensors enabled on the node carry
/// data; the others are value-initialised (zero timestamp).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuPacket {
    pub accelerometer: ImuVecReport,
    pub gyroscope: ImuVecReport,
    pub magnetic_field: ImuVecReport,
    pub rotation_vector: ImuRotationVector,
}

impl ImuPacket {
    pub(crate) fn from_raw(p: &sys::dai_imu_packet) -> Self {
        ImuPacket {
            accelerometer: ImuVecReport::from_raw(&p.accelerometer),
            gyroscope: ImuVecReport::from_raw(&p.gyroscope),
            magnetic_field: ImuVecReport::from_raw(&p.magnetic_field),
            rotation_vector: ImuRotationVector::from_raw(&p.rotation_vector),
        }
    }
}

/// A `dai::IMUData` batch (up to `setMaxBatchReports` packets).
#[derive(Clone, Debug)]
pub struct ImuData {
    msg: Msg,
}

impl Sealed for ImuData {}
impl Message for ImuData {
    const DATATYPE: Option<Datatype> = Some(Datatype::ImuData);

    unsafe fn from_msg(msg: Msg) -> Result<Self> {
        Ok(ImuData { msg })
    }

    fn as_msg(&self) -> &Msg {
        &self.msg
    }
}

impl ImuData {
    /// Copy the packets out, appending to `out`; returns how many were appended.
    pub fn packets_into(&self, out: &mut Vec<ImuPacket>) -> Result<usize> {
        let mut buf = [sys::dai_imu_packet::default(); 16];
        let mut n: usize = 0;
        check(unsafe {
            sys::dai_imu_data_packets(self.msg.raw(), buf.as_mut_ptr(), buf.len(), &mut n)
        })?;
        if n > buf.len() {
            let mut big = vec![sys::dai_imu_packet::default(); n];
            check(unsafe {
                sys::dai_imu_data_packets(self.msg.raw(), big.as_mut_ptr(), big.len(), &mut n)
            })?;
            let take = n.min(big.len());
            out.extend(big[..take].iter().map(ImuPacket::from_raw));
            return Ok(take);
        }
        out.extend(buf[..n].iter().map(ImuPacket::from_raw));
        Ok(n)
    }

    /// The packets in this batch.
    pub fn packets(&self) -> Result<Vec<ImuPacket>> {
        let mut v = Vec::new();
        self.packets_into(&mut v)?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_timestamp_math() {
        let t = RawTimestamp { sec: 3, nsec: 7 };
        assert_eq!(t.as_nanos(), 3_000_000_007);
        assert!(!t.is_zero());
        assert!(RawTimestamp::default().is_zero());
        assert_eq!(
            RawTimestamp { sec: -1, nsec: 0 }.as_duration(),
            Duration::ZERO
        );
    }

    #[test]
    fn packet_conversion_keeps_fields() {
        let mut raw = sys::dai_imu_packet::default();
        raw.accelerometer.x = 1.0;
        raw.accelerometer.ts_sec = 5;
        raw.gyroscope.accuracy = 3;
        raw.rotation_vector.real = 1.0;
        let p = ImuPacket::from_raw(&raw);
        assert_eq!(p.accelerometer.x, 1.0);
        assert_eq!(p.accelerometer.timestamp.sec, 5);
        assert_eq!(p.gyroscope.accuracy, ImuAccuracy::High);
        assert!(p.gyroscope.timestamp.is_zero());
        assert_eq!(p.rotation_vector.real, 1.0);
    }
}
