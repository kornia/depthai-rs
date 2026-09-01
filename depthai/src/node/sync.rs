//! [`Sync`]: `dai::node::Sync`, time-aligns several streams into one
//! [`MessageGroup`] per tick.

use std::time::Duration;

use depthai_sys as sys;

use crate::error::{check, cstring, out_handle, Result};
use crate::message::MessageGroup;
use crate::node::{node_type, NodeHandle};
use crate::port::{Input, Output};

/// A `dai::node::Sync`. Link each stream into a named key of
/// [`input`](Self::input); read groups from [`out`](Self::out).
#[derive(Clone)]
pub struct Sync(pub(crate) NodeHandle);
node_type!(Sync, dai_pipeline_create_sync);

impl Sync {
    /// The `inputs[name]` map entry — created on first use, like depthai's
    /// `InputMap::operator[]`. A typo therefore yields a dangling, never-fed input
    /// (the Sync never emits) rather than an error; check
    /// [`input_names`](crate::node::Node::input_names) when debugging.
    pub fn input(&self, name: &str) -> Result<Input> {
        let c = cstring(name)?;
        let raw = out_handle(|out| unsafe { sys::dai_sync_input(self.0.raw(), c.as_ptr(), out) })?;
        // SAFETY: the port belongs to this Sync node.
        Ok(unsafe { Input::from_raw(self.0.clone(), raw) })
    }

    /// The grouped output.
    pub fn out(&self) -> Result<Output<MessageGroup>> {
        self.0.output_required("out")
    }

    /// Maximum timestamp spread inside one group.
    pub fn set_sync_threshold(&self, threshold: Duration) -> Result<()> {
        let ns = threshold.as_nanos().min(i64::MAX as u128) as i64;
        check(unsafe { sys::dai_sync_set_sync_threshold_ns(self.0.raw(), ns) })
    }

    /// How many attempts before a group is emitted unsynced (`-1` = infinite).
    pub fn set_sync_attempts(&self, attempts: i32) -> Result<()> {
        check(unsafe { sys::dai_sync_set_sync_attempts(self.0.raw(), attempts) })
    }

    pub fn set_run_on_host(&self, run_on_host: bool) -> Result<()> {
        check(unsafe { sys::dai_sync_set_run_on_host(self.0.raw(), run_on_host as i32) })
    }
}
