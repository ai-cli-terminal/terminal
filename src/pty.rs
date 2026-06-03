//! PTY Terminal Core (설계 §5 Execution Layer, M1/W2).
//!
//! 일반 셸 경로의 토대. portable-pty로 셸을 PTY에 띄우고 명령을 실행한다.
//! MVP는 단발성 실행(`shell -c <command>`)부터 시작해 이후 인터랙티브 세션으로 확장한다.
//!
//! 불변식(`docs/RULES.md` §1): 일반 셸 경로는 AI 계층을 거치지 않는다(최소 지연).

use std::io::{Read, Write};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

/// PTY로 실행한 명령의 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyOutput {
    /// PTY가 캡처한 출력(stdout+stderr 혼합, 셸 raw).
    pub output: String,
    /// 종료 코드.
    pub exit_code: u32,
}

/// `shell -c command`을 PTY 위에서 실행하고 출력을 수집한다.
///
/// PTY 기반이므로 셸은 자신이 터미널에 연결됐다고 인식한다(색·prompt 등 동작 일치).
pub fn run_in_pty(shell: &str, command: &str) -> anyhow::Result<PtyOutput> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(shell);
    cmd.arg("-c");
    cmd.arg(command);
    let mut child = pair.slave.spawn_command(cmd)?;

    // 자식이 PTY를 상속한 뒤 slave 핸들을 닫아야 읽기 측에 EOF가 전달된다.
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    let status = child.wait()?;
    Ok(PtyOutput {
        output: String::from_utf8_lossy(&buf).into_owned(),
        exit_code: status.exit_code(),
    })
}

/// PTY에서 `shell -c command`를 실행하며 출력을 청크 단위로 흘려보낸다.
/// Ctrl+C(SIGINT) 수신 시 자식을 kill하고 중단한다. 종료코드를 반환한다
/// (취소 시 130 = 128+SIGINT).
///
/// 리더 스레드가 블로킹 read로 청크를 bounded 채널(cap 64)에 보내고(소비가 느리면
/// `blocking_send`가 막혀 backpressure), current-thread 런타임이 채널 수신과 ctrl_c를
/// `select`한다. `child`는 select 종료 후 `wait`에서 다시 쓰므로 async 블록이 가변 차용만 한다.
pub fn run_in_pty_streaming(
    shell: &str,
    command: &str,
    mut on_chunk: impl FnMut(&str),
) -> anyhow::Result<i32> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(shell);
    cmd.arg("-c");
    cmd.arg(command);
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let reader_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let cancelled = runtime.block_on(async {
        loop {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some(bytes) => on_chunk(&String::from_utf8_lossy(&bytes)),
                    None => break false,
                },
                _ = tokio::signal::ctrl_c() => {
                    let _ = child.kill();
                    break true;
                }
            }
        }
    });

    let status = child.wait()?;
    let _ = reader_thread.join();
    Ok(if cancelled {
        130
    } else {
        status.exit_code() as i32
    })
}

/// 인터랙티브 PTY 세션. 입력을 쓰고 출력을 점진적으로 읽는다(인터랙티브 셸/TUI 토대).
pub struct PtySession {
    // master를 살려 둬야 reader/writer가 유효하다.
    _master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    reader: Box<dyn Read + Send>,
}

impl PtySession {
    /// `program args...`를 PTY에 띄운 인터랙티브 세션을 시작한다.
    pub fn spawn(program: &str, args: &[&str]) -> anyhow::Result<PtySession> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(program);
        for a in args {
            cmd.arg(a);
        }
        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let writer = pair.master.take_writer()?;
        let reader = pair.master.try_clone_reader()?;
        Ok(PtySession {
            _master: pair.master,
            child,
            writer,
            reader,
        })
    }

    /// 자식에게 입력을 보낸다.
    pub fn write_input(&mut self, data: &str) -> anyhow::Result<()> {
        self.writer.write_all(data.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }

    /// 사용 가능한 출력 한 덩어리를 읽는다(데이터가 올 때까지 블록).
    pub fn read_chunk(&mut self) -> anyhow::Result<String> {
        let mut buf = [0u8; 4096];
        let n = self.reader.read(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n]).into_owned())
    }

    /// 자식을 종료한다.
    pub fn kill(&mut self) -> anyhow::Result<()> {
        self.child.kill()?;
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn interactive_cat_echoes_input() {
        let mut s = PtySession::spawn("cat", &[]).unwrap();
        s.write_input("hello_session\n").unwrap();
        let mut acc = String::new();
        for _ in 0..20 {
            acc.push_str(&s.read_chunk().unwrap());
            if acc.contains("hello_session") {
                break;
            }
        }
        s.kill().unwrap();
        assert!(
            acc.contains("hello_session"),
            "interactive echo missing: {acc:?}"
        );
    }

    #[test]
    fn runs_echo_through_pty() {
        let out = run_in_pty("/bin/bash", "echo hello_pty").unwrap();
        assert!(
            out.output.contains("hello_pty"),
            "pty output should contain echoed text: {:?}",
            out.output
        );
        assert_eq!(out.exit_code, 0);
    }

    #[test]
    fn propagates_nonzero_exit_code() {
        let out = run_in_pty("/bin/bash", "exit 3").unwrap();
        assert_eq!(out.exit_code, 3);
    }

    #[test]
    fn streaming_accumulates_full_output() {
        let mut acc = String::new();
        let code = run_in_pty_streaming("/bin/bash", "printf 'one\\ntwo\\n'", |c| acc.push_str(c))
            .unwrap();
        assert!(acc.contains("one"), "{acc:?}");
        assert!(acc.contains("two"), "{acc:?}");
        assert_eq!(code, 0);
    }

    #[test]
    fn streaming_propagates_nonzero_exit() {
        let code = run_in_pty_streaming("/bin/bash", "exit 3", |_| {}).unwrap();
        assert_eq!(code, 3);
    }
}
