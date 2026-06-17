use std::io;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Default timeout for FFmpeg-family process probes and smoke executions.
pub const DEFAULT_PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

/// Run an external process with explicit arguments and a bounded wait time.
pub fn run_process_with_timeout(
    binary: &Path,
    args: &[String],
    timeout: Duration,
) -> io::Result<Output> {
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output();
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{} timed out after {} ms",
                    binary.display(),
                    timeout.as_millis()
                ),
            ));
        }

        thread::sleep(Duration::from_millis(10));
    }
}
