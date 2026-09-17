use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use windows_sys::Win32::System::Console::{
    SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT, CTRL_LOGOFF_EVENT,
    CTRL_SHUTDOWN_EVENT,
};

static SHUTDOWN_FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Win32 console control handler callback.
///
/// SAFETY: Must remain non-blocking. Never perform filesystem, network, or complex
/// synchronization calls inside this callback, as the OS imposes strict execution limits.
unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT => {
            if let Some(flag) = SHUTDOWN_FLAG.get() {
                flag.store(true, Ordering::SeqCst);
            }
            1 // TRUE: signal handled, prevent abrupt process termination
        }
        CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT => {
            if let Some(flag) = SHUTDOWN_FLAG.get() {
                flag.store(true, Ordering::SeqCst);
            }
            0 // FALSE: notify daemon loop, then allow OS shutdown machinery to continue
        }
        _ => 0,
    }
}

/// Registers the console control handler with Windows kernel for graceful shutdown.
pub fn install_console_ctrl_handler(shutdown_flag: Arc<AtomicBool>) -> Result<(), String> {
    let _ = SHUTDOWN_FLAG.set(shutdown_flag);
    let success = unsafe { SetConsoleCtrlHandler(Some(console_ctrl_handler), 1) };
    if success != 0 {
        Ok(())
    } else {
        Err(String::from(
            "failed to register Win32 console control handler",
        ))
    }
}

pub fn register_console_shutdown_handler(shutdown_flag: Arc<AtomicBool>) -> Result<(), String> {
    install_console_ctrl_handler(shutdown_flag)
}
