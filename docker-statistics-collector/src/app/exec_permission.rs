use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use rust_extensions::date_time::DateTimeAsMicroseconds;

/// In-memory gate guarding the dangerous `exec_in_container` MCP tool.
///
/// Holds the deadline (unix microseconds) until which MCP-originated exec is
/// permitted; `0` means disabled. Deliberately never persisted — a collector
/// restart always lands back in the disabled state.
///
/// The gate is enforced on the instance that actually runs the command (see
/// `peers_client::fanout_exec`), so unlocking one collector never opens exec on
/// its peers.
pub struct ExecPermission {
    enabled_until: AtomicI64,
}

impl ExecPermission {
    pub fn new() -> Self {
        Self {
            enabled_until: AtomicI64::new(0),
        }
    }

    /// Opens the window for `duration` starting now. Enabling again while a
    /// window is already open simply replaces the deadline (i.e. extends it).
    pub fn enable_for(&self, duration: Duration) -> ExecPermissionStatus {
        let mut deadline = DateTimeAsMicroseconds::now();
        deadline.add_seconds(duration.as_secs() as i64);
        self.enabled_until
            .store(deadline.unix_microseconds, Ordering::SeqCst);
        self.get_status()
    }

    /// Revokes the window immediately, before it would expire on its own.
    pub fn disable(&self) -> ExecPermissionStatus {
        self.enabled_until.store(0, Ordering::SeqCst);
        self.get_status()
    }

    pub fn is_enabled(&self) -> bool {
        let until = self.enabled_until.load(Ordering::SeqCst);
        until > 0 && DateTimeAsMicroseconds::now().unix_microseconds < until
    }

    pub fn get_status(&self) -> ExecPermissionStatus {
        let until = self.enabled_until.load(Ordering::SeqCst);
        let now = DateTimeAsMicroseconds::now().unix_microseconds;

        if until <= 0 || now >= until {
            return ExecPermissionStatus {
                enabled: false,
                seconds_left: 0,
            };
        }

        ExecPermissionStatus {
            enabled: true,
            // round up, so a freshly opened 600s window reads as 600, not 599
            seconds_left: (until - now + 999_999) / 1_000_000,
        }
    }
}

pub struct ExecPermissionStatus {
    pub enabled: bool,
    pub seconds_left: i64,
}
