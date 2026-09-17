//! Linux lifecycle/display event sources.
//!
//! Event listeners only enqueue recovery hints. They never inspect hardware or
//! call an apply backend. The runtime owns debounce, observation, and writes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const LISTENER_POLL_TIMEOUT: Duration = Duration::from_millis(250);
const LISTENER_RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayRecoveryReason {
    DrmHotplug,
    DrmConnectorChange,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayRecoveryRequest {
    pub reason: DisplayRecoveryReason,
}

#[derive(Debug, Default)]
struct PendingRequest {
    request: Mutex<Option<DisplayRecoveryRequest>>,
}

impl PendingRequest {
    fn take(&self) -> Option<DisplayRecoveryRequest> {
        self.request
            .lock()
            .ok()
            .and_then(|mut pending| pending.take())
    }
}

#[derive(Debug)]
pub struct DisplayEventSources {
    shutdown: Arc<AtomicBool>,
    pending: Arc<PendingRequest>,
    handles: Vec<JoinHandle<()>>,
}

impl DisplayEventSources {
    pub fn spawn() -> Self {
        let pending = Arc::new(PendingRequest::default());
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();

        #[cfg(target_os = "linux")]
        {
            let drm_shutdown = Arc::clone(&shutdown);
            let drm_pending = Arc::clone(&pending);
            if let Ok(handle) = thread::Builder::new()
                .name(String::from("sunreactor-drm-events"))
                .spawn(move || drm_listener(drm_pending, drm_shutdown))
            {
                handles.push(handle);
                tracing::info!("drm_event_listener_started");
            } else {
                tracing::warn!("drm_event_listener_unavailable");
            }

            let logind_shutdown = Arc::clone(&shutdown);
            let logind_pending = Arc::clone(&pending);
            if let Ok(handle) = thread::Builder::new()
                .name(String::from("sunreactor-logind-events"))
                .spawn(move || logind_listener(logind_pending, logind_shutdown))
            {
                handles.push(handle);
                tracing::info!("logind_event_listener_started");
            } else {
                tracing::warn!("logind_event_listener_unavailable");
            }
        }

        Self {
            shutdown,
            pending,
            handles,
        }
    }

    pub fn drain(&self) -> Option<DisplayRecoveryRequest> {
        self.pending.take()
    }
}

impl Drop for DisplayEventSources {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

fn send_request(pending: &PendingRequest, reason: DisplayRecoveryReason) {
    let replaced = pending
        .request
        .lock()
        .ok()
        .and_then(|mut slot| slot.replace(DisplayRecoveryRequest { reason }));
    if replaced.is_some() {
        tracing::debug!(?reason, "display_recovery_request_coalesced");
    } else {
        tracing::debug!(?reason, "display_recovery_requested");
    }
}

#[cfg(target_os = "linux")]
#[allow(clippy::needless_pass_by_value)]
fn drm_listener(pending: Arc<PendingRequest>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Acquire) {
        match drm_listener_once(&pending, &shutdown) {
            Ok(()) => {}
            Err(error) => tracing::warn!(error = %error, "drm_event_listener_unavailable"),
        }
        wait_interruptibly(&shutdown, LISTENER_RETRY_DELAY);
    }
}

#[cfg(target_os = "linux")]
fn drm_listener_once(pending: &PendingRequest, shutdown: &AtomicBool) -> std::io::Result<()> {
    use std::mem::size_of;

    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
            libc::NETLINK_KOBJECT_UEVENT,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let close_fd = CloseFd(fd);

    let mut address: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    address.nl_family = libc::AF_NETLINK as u16;
    address.nl_pid = 0;
    address.nl_groups = 1;
    let bind_result = unsafe {
        libc::bind(
            close_fd.0,
            (&raw const address).cast(),
            size_of::<libc::sockaddr_nl>() as libc::socklen_t,
        )
    };
    if bind_result < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut pollfd = libc::pollfd {
        fd: close_fd.0,
        events: libc::POLLIN,
        revents: 0,
    };
    let mut buffer = [0_u8; 4096];
    while !shutdown.load(Ordering::Acquire) {
        let result =
            unsafe { libc::poll(&raw mut pollfd, 1, LISTENER_POLL_TIMEOUT.as_millis() as i32) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if result == 0 {
            continue;
        }
        let length = unsafe {
            libc::recv(
                close_fd.0,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                libc::MSG_DONTWAIT,
            )
        };
        if length < 0 {
            let error = std::io::Error::last_os_error();
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) {
                continue;
            }
            return Err(error);
        }
        if let Some(reason) = classify_uevent(&buffer[..length as usize]) {
            tracing::info!(
                reason = ?reason,
                connector = connector_id(&buffer[..length as usize]).unwrap_or_else(|| String::from("unknown")),
                "drm_event_classified"
            );
            send_request(pending, reason);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
struct CloseFd(std::os::fd::RawFd);

#[cfg(target_os = "linux")]
impl Drop for CloseFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

#[cfg(target_os = "linux")]
fn classify_uevent(payload: &[u8]) -> Option<DisplayRecoveryReason> {
    let fields = payload
        .split(|byte| *byte == 0)
        .filter_map(|field| std::str::from_utf8(field).ok());
    let mut drm_connector = false;
    let mut drm_minor_hotplug = false;
    let mut hotplug = false;
    let mut connector_id = false;
    let mut change = false;
    for field in fields {
        if field == "DEVTYPE=drm_connector" {
            drm_connector = true;
        } else if field == "DEVTYPE=drm_minor" {
            drm_minor_hotplug = true;
        } else if field == "HOTPLUG=1" {
            hotplug = true;
        } else if field
            .strip_prefix("CONNECTOR=")
            .is_some_and(|value| !value.is_empty())
        {
            connector_id = true;
        } else if field.starts_with("change@/devices/")
            && field.contains("/drm/")
            && field
                .rsplit('/')
                .next()
                .is_some_and(|name| name.contains('-'))
        {
            change = true;
        }
    }
    if (drm_connector || (drm_minor_hotplug && connector_id)) && hotplug {
        Some(DisplayRecoveryReason::DrmHotplug)
    } else if drm_connector && change {
        Some(DisplayRecoveryReason::DrmConnectorChange)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn connector_id(payload: &[u8]) -> Option<String> {
    payload
        .split(|byte| *byte == 0)
        .filter_map(|field| std::str::from_utf8(field).ok())
        .find_map(|field| {
            field
                .strip_prefix("CONNECTOR=")
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
}

#[cfg(target_os = "linux")]
#[allow(clippy::needless_pass_by_value)]
fn logind_listener(pending: Arc<PendingRequest>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Acquire) {
        match logind_listener_once(&pending, &shutdown) {
            Ok(()) => {}
            Err(error) => tracing::warn!(error = %error, "logind_event_listener_unavailable"),
        }
        wait_interruptibly(&shutdown, LISTENER_RETRY_DELAY);
    }
}

#[cfg(target_os = "linux")]
fn logind_listener_once(
    pending: &PendingRequest,
    shutdown: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    zbus::block_on(logind_listener_async(pending, shutdown))
}

#[cfg(target_os = "linux")]
async fn logind_listener_async(
    pending: &PendingRequest,
    shutdown: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_lite::{future, StreamExt};
    use zbus::{Connection, Proxy};

    let connection = bounded_setup(Connection::system())
        .await
        .ok_or("logind system-bus connection timed out")??;
    let proxy = bounded_setup(Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    ))
    .await
    .ok_or("logind proxy setup timed out")??;
    let mut signals = bounded_setup(proxy.receive_signal("PrepareForSleep"))
        .await
        .ok_or("logind signal subscription timed out")??;
    let mut sleeping = false;
    while !shutdown.load(Ordering::Acquire) {
        let message = future::or(async { signals.next().await }, async {
            async_io::Timer::after(LISTENER_POLL_TIMEOUT).await;
            None
        })
        .await;
        let Some(message) = message else { continue };
        let (start,) = message.body().deserialize::<(bool,)>()?;
        if start {
            sleeping = true;
        } else if sleeping {
            sleeping = false;
            send_request(pending, DisplayRecoveryReason::Resume);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn bounded_setup<F, T, E>(operation: F) -> Option<Result<T, E>>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    futures_lite::future::or(async { Some(operation.await) }, async {
        async_io::Timer::after(LISTENER_POLL_TIMEOUT).await;
        None
    })
    .await
}

#[cfg(target_os = "linux")]
fn wait_interruptibly(shutdown: &AtomicBool, duration: Duration) {
    let started = std::time::Instant::now();
    while !shutdown.load(Ordering::Acquire) && started.elapsed() < duration {
        thread::sleep(Duration::from_millis(50).min(duration.saturating_sub(started.elapsed())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_drain_coalesces_burst_to_newest_request() {
        let pending = PendingRequest::default();
        send_request(&pending, DisplayRecoveryReason::DrmHotplug);
        send_request(&pending, DisplayRecoveryReason::DrmConnectorChange);
        send_request(&pending, DisplayRecoveryReason::Resume);
        assert_eq!(
            pending.take().map(|request| request.reason),
            Some(DisplayRecoveryReason::Resume)
        );
    }

    #[test]
    fn queue_replaces_stale_request_without_blocking() {
        let pending = PendingRequest::default();
        send_request(&pending, DisplayRecoveryReason::DrmHotplug);
        send_request(&pending, DisplayRecoveryReason::Resume);
        assert_eq!(
            pending.take().map(|request| request.reason),
            Some(DisplayRecoveryReason::Resume)
        );
        assert_eq!(pending.take(), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn drm_filter_ignores_unrelated_uevents() {
        assert_eq!(
            classify_uevent(b"add@/devices/foo\0DEVTYPE=usb_device\0"),
            None
        );
        assert_eq!(
            classify_uevent(b"change@/devices/pci/drm/card0\0DEVTYPE=drm_connector\0"),
            None
        );
        assert_eq!(
            classify_uevent(
                b"change@/devices/pci/drm/card0-DP-1\0DEVTYPE=drm_connector\0HOTPLUG=1\0"
            ),
            Some(DisplayRecoveryReason::DrmHotplug)
        );
        assert_eq!(
            classify_uevent(
                b"change@/devices/pci/drm/card1\0DEVTYPE=drm_minor\0HOTPLUG=1\0CONNECTOR=511\0"
            ),
            Some(DisplayRecoveryReason::DrmHotplug)
        );
    }
}
