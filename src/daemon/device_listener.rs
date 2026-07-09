#[cfg(feature = "remote")]
use std::path::{Path, PathBuf};

#[cfg(feature = "remote")]
use anyhow::{Context, Result};

#[cfg(feature = "remote")]
fn bind_device_listener(path: &Path) -> Result<std::os::unix::net::UnixListener> {
    use std::os::unix::net::UnixListener;

    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    UnixListener::bind(path)
        .with_context(|| format!("디바이스 소켓 바인드 실패: {}", path.display()))
}

/// RA-1 device listener 최소 결선: daemon module이 Unix listener를 바인드하고,
/// 연결 1건에 대해 Noise 승인 요청/응답을 왕복한다. 페어링, device record 영속화,
/// gate-flow 결선은 후속 RA-2/RA-3에서 붙인다.
#[cfg(feature = "remote")]
pub fn serve_device_once(
    path: &Path,
    daemon_private: &[u8],
    request: &crate::session::ApprovalRequestMsg,
) -> Result<crate::session::ApprovalResponseMsg> {
    let mut pending = Some(request.clone());
    let mut response = None;
    serve_device_loop(
        path,
        daemon_private,
        || pending.take(),
        |resp| {
            response = Some(resp);
            false
        },
    )?;
    response
        .expect("one-shot device listener must produce one response")
        .context("디바이스 승인 왕복 실패")
}

/// 백그라운드 device listener 핸들. RA-3에서 gate-flow가 `request_tx`로 승인 요청을
/// 보낸다. 응답은 요청마다 별도 channel로 되돌려 stale response가 다음 gate를
/// 오염시키지 않게 한다.
#[cfg(feature = "remote")]
pub struct DeviceListenerHandle {
    pub request_tx: std::sync::mpsc::Sender<DeviceListenerRequest>,
    pub thread: std::thread::JoinHandle<()>,
}

#[cfg(feature = "remote")]
pub struct DeviceListenerRequest {
    pub request: crate::session::ApprovalRequestMsg,
    pub response_tx: std::sync::mpsc::Sender<Result<crate::session::ApprovalResponseMsg>>,
    pub accept_timeout: std::time::Duration,
}

/// daemon-owned `device.sock` listener를 백그라운드 스레드로 시작한다.
/// 반환 시점에는 소켓이 이미 bind되어 있다. 요청 채널이 닫히면 listener도 종료된다.
#[cfg(feature = "remote")]
pub fn spawn_device_listener(
    path: PathBuf,
    daemon_private: Vec<u8>,
) -> Result<DeviceListenerHandle> {
    let listener = bind_device_listener(&path)?;
    let (request_tx, request_rx) = std::sync::mpsc::channel::<DeviceListenerRequest>();
    let thread = std::thread::spawn(move || {
        while let Ok(item) = request_rx.recv() {
            let response = run_daemon_listener_once_with_timeout(
                &listener,
                &daemon_private,
                &item.request,
                item.accept_timeout,
            );
            let _ = item.response_tx.send(response);
        }
    });
    Ok(DeviceListenerHandle { request_tx, thread })
}

#[cfg(feature = "remote")]
fn run_daemon_listener_once_with_timeout(
    listener: &std::os::unix::net::UnixListener,
    daemon_private: &[u8],
    request: &crate::session::ApprovalRequestMsg,
    timeout: std::time::Duration,
) -> Result<crate::session::ApprovalResponseMsg> {
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};

    listener.set_nonblocking(true)?;
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                return crate::session::run_daemon_request(&mut stream, daemon_private, request)
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    anyhow::bail!("디바이스 연결 시간 초과");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(err.into()),
        }
    }
}

/// RA-1 반복 device listener skeleton. `next_request`가 `Some`을 반환할 때마다
/// 다음 device 연결을 수락하고 해당 승인 요청을 `session::run_daemon_request`로 왕복한다.
/// `handle_response`가 `false`를 반환하면 정상 종료한다.
///
/// 다음 단계에서 `next_request`는 gate-flow pending approval queue로, `handle_response`는
/// nonce consume + approval validation + gate reply wakeup으로 대체된다.
#[cfg(feature = "remote")]
pub fn serve_device_loop<N, H>(
    path: &Path,
    daemon_private: &[u8],
    mut next_request: N,
    mut handle_response: H,
) -> Result<()>
where
    N: FnMut() -> Option<crate::session::ApprovalRequestMsg>,
    H: FnMut(Result<crate::session::ApprovalResponseMsg>) -> bool,
{
    let listener = bind_device_listener(path)?;

    while let Some(request) = next_request() {
        let response =
            crate::session::run_daemon_listener_once(&listener, daemon_private, &request);
        if !handle_response(response) {
            break;
        }
    }
    Ok(())
}
