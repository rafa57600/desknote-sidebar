#[cfg(not(target_os = "windows"))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows_impl;

#[cfg(not(target_os = "windows"))]
pub use unsupported::*;
#[cfg(target_os = "windows")]
pub use windows_impl::*;
