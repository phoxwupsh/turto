//! The per-track cancel signal.
//!
//! One [`Cancel`] is created with a [`YouTubeDl`](super::YouTubeDl) and lives
//! exactly as long as it, cloned into every byte producer that track ever starts.
//! Cancellation is therefore a property of the **track's lifetime**, not of any one
//! download or reader: dropping the last handle to a track -- removed from the
//! queue, replaced in the playing slot -- stops whatever it is fetching, and a
//! producer restarted after a failure inherits the same signal with nothing to
//! re-register.
//!
//! Both halves are deliberately sync and lock-free so
//! [`Drop`](super::YoutubeDlFileInner) can fire them.

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Notify, futures::Notified};

#[derive(Debug, Default)]
pub(super) struct Cancel {
    flag: AtomicBool,
    /// Wakes a producer parked at its pace gate. A producer that is actively
    /// awaiting bytes needs no wake -- it observes [`Self::is_cancelled`] at the
    /// next chunk boundary.
    signal: Notify,
}

impl Cancel {
    /// Stop whatever this track is fetching. Idempotent, and safe to call from
    /// `Drop`.
    pub(super) fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
        // `notify_one`, not `notify_waiters`: it keeps a permit for a waiter that
        // has built its future but not yet polled it, which is exactly the window
        // `PaceGate::await_room` sits in.
        self.signal.notify_one();
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// A waiter for the next [`cancel`](Self::cancel). Build it *before* checking
    /// [`is_cancelled`](Self::is_cancelled) so a cancel landing in between is still
    /// observed.
    pub(super) fn notified(&self) -> Notified<'_> {
        self.signal.notified()
    }
}
