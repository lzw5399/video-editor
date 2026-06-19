use std::ffi::OsString;
use std::io::{self, Write};
use std::time::Duration;

use media_runtime::run_process_with_timeout;

#[test]
fn process_timeout_runner_drains_large_stdout_and_stderr_while_waiting() {
    let current_exe = std::env::current_exe().expect("test binary path should resolve");
    let output = run_process_with_timeout(
        &current_exe,
        &[
            OsString::from("--ignored"),
            OsString::from("--exact"),
            OsString::from("process_helper_emits_large_output"),
            OsString::from("--nocapture"),
        ],
        Duration::from_secs(2),
    )
    .expect("large child output should not deadlock the timeout runner");

    assert!(output.status.success());
    assert!(
        output.stdout.len() > 128 * 1024,
        "stdout should include the helper payload"
    );
    assert!(
        output.stderr.len() > 128 * 1024,
        "stderr should include the helper payload"
    );
}

#[test]
#[ignore]
fn process_helper_emits_large_output() {
    let payload = vec![b'x'; 256 * 1024];
    io::stdout()
        .write_all(&payload)
        .expect("helper should write stdout payload");
    io::stderr()
        .write_all(&payload)
        .expect("helper should write stderr payload");
}
