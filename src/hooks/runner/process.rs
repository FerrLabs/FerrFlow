use anyhow::{Context, Result};
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

pub(super) enum Finish {
    Exited {
        status: ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    TimedOut,
}

pub(super) fn run_bounded(
    mut cmd: Command,
    capture: bool,
    timeout: Duration,
    label: &str,
) -> Result<Finish> {
    cmd.stdin(Stdio::null());
    if capture {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    }
    own_process_group(&mut cmd);

    let mut child = cmd.spawn().context("failed to start the hook")?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);

    match wait_until(&mut child, timeout, label)? {
        Some(status) => Ok(Finish::Exited {
            status,
            stdout: collect(stdout),
            stderr: collect(stderr),
        }),
        None => {
            kill_tree(&mut child);
            let _ = child.wait();
            Ok(Finish::TimedOut)
        }
    }
}

fn wait_until(child: &mut Child, timeout: Duration, label: &str) -> Result<Option<ExitStatus>> {
    let start = Instant::now();
    let mut next_heartbeat = HEARTBEAT_INTERVAL;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return Ok(None);
        }
        if elapsed >= next_heartbeat {
            tracing::info!("    [{label}] still running after {}s", elapsed.as_secs());
            next_heartbeat += HEARTBEAT_INTERVAL;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        buf
    })
}

fn collect(reader: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

#[cfg(unix)]
fn own_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(windows)]
fn own_process_group(_cmd: &mut Command) {}

#[cfg(unix)]
fn kill_tree(child: &mut Child) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}

#[cfg(windows)]
fn kill_tree(child: &mut Child) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &child.id().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}
