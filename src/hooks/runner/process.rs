use anyhow::{Context, Result};
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
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

    let deadline = Instant::now() + timeout;
    let mut child = cmd.spawn().context("failed to start the hook")?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);

    let Some(status) = wait_until(&mut child, deadline, label)? else {
        return Ok(time_out(&mut child));
    };
    match (collect(stdout, deadline), collect(stderr, deadline)) {
        (Some(stdout), Some(stderr)) => Ok(Finish::Exited {
            status,
            stdout,
            stderr,
        }),
        _ => Ok(time_out(&mut child)),
    }
}

fn time_out(child: &mut Child) -> Finish {
    kill_tree(child);
    let _ = child.wait();
    Finish::TimedOut
}

fn wait_until(child: &mut Child, deadline: Instant, label: &str) -> Result<Option<ExitStatus>> {
    let start = Instant::now();
    let mut next_heartbeat = start + HEARTBEAT_INTERVAL;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        if now >= next_heartbeat {
            tracing::info!(
                "    [{label}] still running after {}s",
                (now - start).as_secs()
            );
            next_heartbeat += HEARTBEAT_INTERVAL;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    rx
}

fn collect(reader: Option<Receiver<Vec<u8>>>, deadline: Instant) -> Option<Vec<u8>> {
    let Some(reader) = reader else {
        return Some(Vec::new());
    };
    reader
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()
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
    if let Ok(group) = libc::pid_t::try_from(child.id()) {
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }
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
