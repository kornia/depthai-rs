//! [`Gate`]: `dai::node::Gate`, a valve on a stream that the host opens and
//! closes at runtime with [`GateControl`](crate::GateControl) messages.

use depthai_sys as sys;

use crate::error::{check, Result};
use crate::message::AnyMessage;
use crate::node::node_type;
use crate::port::{Input, Output};

node_type!(
    /// A `dai::node::Gate`: forwards `input` to `output` while open, drops while
    /// closed. Drive it from the host through
    /// [`input_control`](Self::input_control) → [`Input::create_input_queue`] →
    /// [`InputQueue::send`](crate::InputQueue::send) with a
    /// [`GateControl`](crate::GateControl).
    ///
    /// Placed on the device (the default) between a camera output and a
    /// `VideoEncoder`, a closed gate means no frames are encoded and nothing
    /// crosses the link.
    Gate,
    dai_pipeline_create_gate
);

impl Gate {
    /// The gated stream in (any `Buffer`; queue size 1, non-blocking).
    pub fn input(&self) -> Result<Input> {
        self.0.input_required("input")
    }

    /// The gated stream out (same message type as what went in).
    pub fn output(&self) -> Result<Output<AnyMessage>> {
        self.0.output_required("output")
    }

    /// Where [`GateControl`](crate::GateControl) messages go.
    pub fn input_control(&self) -> Result<Input> {
        self.0.input_required("inputControl")
    }

    /// Run the valve on the host instead of the device (then frames still cross
    /// the link; only useful for host-side consumers).
    pub fn set_run_on_host(&self, run_on_host: bool) -> Result<()> {
        check(unsafe { sys::dai_gate_set_run_on_host(self.0.raw(), run_on_host as i32) })
    }
}
