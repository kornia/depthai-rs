//! [`Imu`]: `dai::node::IMU`, the on-board inertial unit.

use depthai_sys as sys;

use crate::enums::ImuSensor;
use crate::error::{check, Result};
use crate::message::ImuData;
use crate::node::node_type;
use crate::port::Output;

node_type!(
    /// A `dai::node::IMU`. Enable sensors, set batching, read
    /// [`ImuData`] batches from [`out`](Self::out).
    ///
    /// Not every OAK carries an IMU: check
    /// [`Device::connected_imu`](crate::Device::connected_imu) before creating the
    /// node, because a missing IMU fails at `Pipeline::start`, not at creation.
    Imu,
    dai_pipeline_create_imu
);

impl Imu {
    pub fn out(&self) -> Result<Output<ImuData>> {
        self.0.output_required("out")
    }

    /// Enable `sensor` at `report_rate_hz`. The BNO086 gyro tops out at 400 Hz;
    /// asking for more throws at `Pipeline::start`.
    pub fn enable_sensor(&self, sensor: ImuSensor, report_rate_hz: u32) -> Result<()> {
        check(unsafe { sys::dai_imu_enable_sensor(self.0.raw(), sensor.to_raw(), report_rate_hz) })
    }

    /// Reports to accumulate before a batch is sent.
    pub fn set_batch_report_threshold(&self, n: i32) -> Result<()> {
        check(unsafe { sys::dai_imu_set_batch_report_threshold(self.0.raw(), n) })
    }

    /// Upper bound on reports per batch (depthai caps at 5).
    pub fn set_max_batch_reports(&self, n: i32) -> Result<()> {
        check(unsafe { sys::dai_imu_set_max_batch_reports(self.0.raw(), n) })
    }
}
