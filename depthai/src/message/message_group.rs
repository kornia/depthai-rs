//! [`MessageGroup`]: the [`Sync`](crate::node::Sync) node's output — several
//! messages grouped by name, time-aligned on the device.

use std::time::Duration;

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{cstring, out_bool, out_string, out_val, poll_handle, Result};
use crate::message::{Message, Msg, Sealed};

/// A `dai::MessageGroup`.
#[derive(Clone, Debug)]
pub struct MessageGroup {
    msg: Msg,
}

impl Sealed for MessageGroup {}
impl Message for MessageGroup {
    const DATATYPE: Option<Datatype> = Some(Datatype::MessageGroup);

    unsafe fn from_msg(msg: Msg) -> Result<Self> {
        Ok(MessageGroup { msg })
    }

    fn as_msg(&self) -> &Msg {
        &self.msg
    }
}

impl MessageGroup {
    /// The member named `name` (the key it was linked into the Sync node's
    /// `inputs` map under), typed as `M`. `Ok(None)` when absent.
    pub fn get<M: Message>(&self, name: &str) -> Result<Option<M>> {
        let c = cstring(name)?;
        let found =
            poll_handle(|out| unsafe { sys::dai_msg_group_get(self.msg.raw(), c.as_ptr(), out) })?;
        let Some(raw) = found else {
            return Ok(None);
        };
        // SAFETY: a fresh owned handle from the shim.
        unsafe { Msg::from_raw(raw) }.into_typed().map(Some)
    }

    /// Member names.
    pub fn names(&self) -> Result<Vec<String>> {
        let names = out_string(|p| unsafe { sys::dai_msg_group_names(self.msg.raw(), p) })?;
        Ok(names
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect())
    }

    pub fn num_messages(&self) -> Result<i64> {
        out_val(|v| unsafe { sys::dai_msg_group_num_messages(self.msg.raw(), v) })
    }

    /// `isSynced(threshold)`: whether every member's timestamp lies within
    /// `threshold` of the others.
    pub fn is_synced(&self, threshold: Duration) -> Result<bool> {
        let ns = threshold.as_nanos().min(i64::MAX as u128) as i64;
        out_bool(|v| unsafe { sys::dai_msg_group_is_synced(self.msg.raw(), ns, v) })
    }

    /// `getIntervalNs()`: the spread between the earliest and latest member.
    pub fn interval(&self) -> Result<Duration> {
        let ns = out_val(|v| unsafe { sys::dai_msg_group_interval_ns(self.msg.raw(), v) })?;
        Ok(Duration::from_nanos(ns.max(0) as u64))
    }
}
