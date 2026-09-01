//! [`MessageGroup`]: the [`Sync`](crate::node::Sync) node's output — several
//! messages grouped by name, time-aligned on the device.

use std::time::Duration;

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{
    cstring, duration_from_ns, duration_to_ns, out_bool, out_lines, out_val, poll_handle, Result,
};
use crate::message::{typed_from_raw, AnyMessage, Message, Sealed};

/// A `dai::MessageGroup`.
#[derive(Clone, Debug)]
pub struct MessageGroup {
    any: AnyMessage,
}

impl Sealed for MessageGroup {}
impl Message for MessageGroup {
    const DATATYPE: Option<Datatype> = Some(Datatype::MessageGroup);

    fn from_any(any: AnyMessage) -> Result<Self> {
        Ok(MessageGroup { any })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl MessageGroup {
    /// The member named `name` (the key it was linked into the Sync node's
    /// `inputs` map under), typed as `M`. `Ok(None)` when absent.
    pub fn get<M: Message>(&self, name: &str) -> Result<Option<M>> {
        let c = cstring(name)?;
        let found =
            poll_handle(|out| unsafe { sys::dai_msg_group_get(self.any.raw(), c.as_ptr(), out) })?;
        // SAFETY: a fresh owned handle from the shim.
        found.map(|raw| unsafe { typed_from_raw(raw) }).transpose()
    }

    /// Member names.
    pub fn names(&self) -> Result<Vec<String>> {
        out_lines(|p| unsafe { sys::dai_msg_group_names(self.any.raw(), p) })
    }

    pub fn num_messages(&self) -> Result<i64> {
        out_val(|v| unsafe { sys::dai_msg_group_num_messages(self.any.raw(), v) })
    }

    /// `isSynced(threshold)`: whether every member's timestamp lies within
    /// `threshold` of the others.
    pub fn is_synced(&self, threshold: Duration) -> Result<bool> {
        out_bool(|v| unsafe {
            sys::dai_msg_group_is_synced(self.any.raw(), duration_to_ns(threshold), v)
        })
    }

    /// `getIntervalNs()`: the spread between the earliest and latest member.
    pub fn interval(&self) -> Result<Duration> {
        out_val(|v| unsafe { sys::dai_msg_group_interval_ns(self.any.raw(), v) })
            .map(duration_from_ns)
    }
}
