use super::{ParserInitializer, VersionOutput};
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

pub(super) struct ProductionInitializer;

fn drain(mut stream: impl Read) -> Result<Vec<u8>, String> {
    let mut retained = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(retained);
        }
        let keep = count.min((64 * 1024usize).saturating_sub(retained.len()));
        retained.extend_from_slice(&buffer[..keep]);
    }
}

struct VersionChild(Child);

impl Drop for VersionChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn spawn_drain(
    stream: impl Read + Send + 'static,
) -> Result<Receiver<Result<Vec<u8>, String>>, String> {
    let (tx, rx) = channel();
    thread::Builder::new()
        .name("bio-parser-version-drain".to_string())
        .spawn(move || {
            let _ = tx.send(drain(stream));
        })
        .map_err(|error| format!("Cannot start version output reader: {error}"))?;
    Ok(rx)
}

impl ParserInitializer for ProductionInitializer {
    fn run_version(&self, executable: &Path) -> Result<VersionOutput, String> {
        let mut command = Command::new(executable);
        command
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = VersionChild(
            command
                .spawn()
                .map_err(|error| format!("Version query failed: {error}"))?,
        );
        let stdout = child
            .0
            .stdout
            .take()
            .ok_or_else(|| "Missing version stdout".to_string())?;
        let stderr = child
            .0
            .stderr
            .take()
            .ok_or_else(|| "Missing version stderr".to_string())?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let out_rx = spawn_drain(stdout)?;
        let err_rx = spawn_drain(stderr)?;
        let status = child.0.wait_timeout(Duration::from_secs(5));
        if !matches!(status, Ok(Some(_))) {
            let _ = child.0.kill();
            let _ = child.0.wait();
            return Err("Installer version query timed out or failed".to_string());
        }
        let status = status
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Missing version exit status".to_string())?;
        let stdout = out_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| "Version output drain timed out".to_string())??;
        let stderr = err_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| "Version error drain timed out".to_string())??;
        if !status.success() {
            return Err(format!("Installer version query exited with {status}"));
        }
        let text = if stdout.is_empty() { stderr } else { stdout };
        let text = String::from_utf8_lossy(&text);
        let display = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .ok_or_else(|| "Installer returned no version".to_string())?
            .trim()
            .chars()
            .take(512)
            .collect::<String>();
        let version = display
            .split_whitespace()
            .find_map(|word| {
                let candidate = word.trim_start_matches('v');
                let mut parts = candidate.split('.');
                if (0..3).all(|_| {
                    parts.next().is_some_and(|part| {
                        !part.is_empty() && part.bytes().all(|value| value.is_ascii_digit())
                    })
                }) && parts.next().is_none()
                {
                    Some(candidate.to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| "Installer returned no recognized version".to_string())?;
        Ok(VersionOutput { display, version })
    }
}

#[cfg(not(windows))]
pub(super) fn replace(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::rename(source, destination).map_err(|error| error.to_string())
}

#[cfg(windows)]
pub(super) fn replace(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}
