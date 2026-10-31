use std::fmt;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use wait_timeout::ChildExt;

const OUTPUT_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbeOutput {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProbeError {
    TimedOut,
    Failed(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimedOut => formatter.write_str("version query timed out"),
            Self::Failed(message) => write!(formatter, "version query failed: {message}"),
        }
    }
}

struct ProbeChild(Child);

impl Drop for ProbeChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn drain(mut stream: impl Read) -> Result<Vec<u8>, ProbeError> {
    let mut retained = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|error| ProbeError::Failed(error.to_string()))?;
        if count == 0 {
            return Ok(retained);
        }
        let keep = count.min(OUTPUT_LIMIT.saturating_sub(retained.len()));
        retained.extend_from_slice(&buffer[..keep]);
    }
}

fn spawn_reader(
    stream: impl Read + Send + 'static,
) -> Result<Receiver<Result<Vec<u8>, ProbeError>>, ProbeError> {
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name("bio-tool-version-reader".to_string())
        .spawn(move || {
            let _ = sender.send(drain(stream));
        })
        .map_err(|error| ProbeError::Failed(error.to_string()))?;
    Ok(receiver)
}

fn read_result(
    receiver: &Receiver<Result<Vec<u8>, ProbeError>>,
    deadline: Instant,
) -> Result<String, ProbeError> {
    let bytes = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| match error {
            std::sync::mpsc::RecvTimeoutError::Timeout => ProbeError::TimedOut,
            std::sync::mpsc::RecvTimeoutError::Disconnected => {
                ProbeError::Failed("version output reader stopped".to_string())
            }
        })??;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    let mut end = OUTPUT_LIMIT.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    Ok(text)
}

pub(crate) fn query_command(
    mut command: Command,
    timeout: Duration,
) -> Result<ProbeOutput, ProbeError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = ProbeChild(
        command
            .spawn()
            .map_err(|error| ProbeError::Failed(error.to_string()))?,
    );
    let deadline = Instant::now() + timeout;
    let stdout = spawn_reader(
        child
            .0
            .stdout
            .take()
            .ok_or_else(|| ProbeError::Failed("missing version stdout".to_string()))?,
    )?;
    let stderr = spawn_reader(
        child
            .0
            .stderr
            .take()
            .ok_or_else(|| ProbeError::Failed("missing version stderr".to_string()))?,
    )?;
    let status = child
        .0
        .wait_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| ProbeError::Failed(error.to_string()))?
        .ok_or(ProbeError::TimedOut)?;
    Ok(ProbeOutput {
        stdout: read_result(&stdout, deadline)?,
        stderr: read_result(&stderr, deadline)?,
        success: status.success(),
    })
}

pub(crate) fn query_path(path: &Path, timeout: Duration) -> Result<ProbeOutput, ProbeError> {
    let mut command = Command::new(path);
    command.arg("--version");
    query_command(command, timeout)
}

#[cfg(test)]
#[path = "tool_version_probe_tests.rs"]
mod tests;
