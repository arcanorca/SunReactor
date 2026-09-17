pub mod protocol;
#[cfg(target_os = "linux")]
pub mod socket;
pub mod transport;

#[cfg(target_os = "windows")]
pub use crate::platform::windows::pipe::*;
pub use protocol::*;
#[cfg(target_os = "linux")]
pub use socket::*;
pub(crate) use transport::*;
