#[cfg(target_os = "linux")]
#[path = "signals/linux.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "signals/windows.rs"]
mod platform;

pub(crate) use platform::{Hangup, Terminate};
