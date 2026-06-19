use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Default timeout for FFmpeg-family process probes and smoke executions.
pub const DEFAULT_PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

/// Run an external process with explicit arguments and a bounded wait time.
pub fn run_process_with_timeout(
    binary: &Path,
    args: &[OsString],
    timeout: Duration,
) -> io::Result<Output> {
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout_reader = child.stdout.take().map(spawn_output_reader);
    let stderr_reader = child.stderr.take().map(spawn_output_reader);
    let started = Instant::now();

    loop {
        if let Some(status) = child.try_wait()? {
            let stdout = join_output_reader(stdout_reader)?;
            let stderr = join_output_reader(stderr_reader)?;
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = join_output_reader(stdout_reader);
            let _ = join_output_reader(stderr_reader);
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

fn spawn_output_reader<R>(mut reader: R) -> thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

fn join_output_reader(
    reader: Option<thread::JoinHandle<io::Result<Vec<u8>>>>,
) -> io::Result<Vec<u8>> {
    let Some(reader) = reader else {
        return Ok(Vec::new());
    };

    reader
        .join()
        .map_err(|_| io::Error::other("process output reader thread panicked"))?
}
