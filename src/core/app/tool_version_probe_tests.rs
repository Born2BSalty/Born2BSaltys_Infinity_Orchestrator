use super::*;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

fn child_command(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        "app::tool_version_probe::tests::probe_child",
        "--nocapture",
    ]);
    command.env("BIO_TOOL_PROBE_CHILD", mode);
    command
}

#[test]
fn bounded_query_drains_both_streams_without_waiting_for_pipe_space() {
    let output = query_command(child_command("pipes"), Duration::from_secs(5)).unwrap();
    assert!(output.success);
    assert!(output.stdout.contains("probe_out"));
    assert!(output.stderr.contains("probe_err"));
    assert!(output.stdout.len() <= 64 * 1024);
    assert!(output.stderr.len() <= 64 * 1024);
}

#[test]
fn hung_version_process_is_killed_before_returning() {
    let started = Instant::now();
    assert!(matches!(
        query_command(child_command("hang"), Duration::from_millis(100)),
        Err(ProbeError::TimedOut)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn nonzero_version_query_retains_output_without_claiming_success() {
    let output = query_command(child_command("failed"), Duration::from_secs(5)).unwrap();
    assert!(!output.success);
    assert!(output.stdout.contains("mod_installer 15.0.1"));
    assert!(output.stderr.contains("deliberate failure"));
}

#[test]
fn nonexistent_executable_returns_a_bounded_query_error() {
    let path = std::env::temp_dir().join(format!(
        "bio_absent_tool_{}_{}.exe",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(matches!(
        query_path(&path, Duration::from_secs(1)),
        Err(ProbeError::Failed(_))
    ));
}

#[test]
fn probe_child() {
    match std::env::var("BIO_TOOL_PROBE_CHILD").as_deref() {
        Ok("hang") => std::thread::sleep(Duration::from_secs(30)),
        Ok("pipes") => {
            let stdout = std::thread::spawn(|| {
                let mut output = std::io::stdout().lock();
                output.write_all(b"probe_out\n").unwrap();
                for _ in 0..24 {
                    output.write_all(&[b'o'; 8192]).unwrap();
                }
                output.flush().unwrap();
            });
            let stderr = std::thread::spawn(|| {
                let mut output = std::io::stderr().lock();
                output.write_all(b"probe_err\n").unwrap();
                for _ in 0..24 {
                    output.write_all(&[b'e'; 8192]).unwrap();
                }
                output.flush().unwrap();
            });
            stdout.join().unwrap();
            stderr.join().unwrap();
        }
        Ok("failed") => {
            println!("mod_installer 15.0.1");
            eprintln!("deliberate failure");
            std::process::exit(7);
        }
        _ => {}
    }
}
