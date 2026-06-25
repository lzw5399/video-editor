use std::env;
use std::path::PathBuf;
use std::process;
use std::thread;
use std::time::Duration;

use draft_model::ExportPreset;
use serde::Serialize;
use server_runtime::{
    ServerExportRequest, ServerRuntime, ServerRuntimeError, ServerRuntimeErrorKind,
    get_export_status, is_terminal_export_phase, open_project, start_export,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum CliEvent<T: Serialize> {
    Opened {
        project: T,
    },
    Started {
        status: T,
    },
    Status {
        status: T,
    },
    Error {
        kind: ServerRuntimeErrorKind,
        message: String,
    },
}

fn main() {
    if let Err(error) = run_cli() {
        print_event(&CliEvent::<()>::Error {
            kind: error.kind(),
            message: error.to_string(),
        });
        process::exit(1);
    }
}

fn run_cli() -> Result<(), ServerRuntimeError> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("export") => run_export(args.collect()),
        _ => Err(ServerRuntimeError::new(
            ServerRuntimeErrorKind::Internal,
            "usage: server_runtime export <bundle.veproj> <output.mp4> [h264-aac-draft|h264-aac-balanced]",
        )),
    }
}

fn run_export(args: Vec<String>) -> Result<(), ServerRuntimeError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(ServerRuntimeError::new(
            ServerRuntimeErrorKind::Internal,
            "usage: server_runtime export <bundle.veproj> <output.mp4> [h264-aac-draft|h264-aac-balanced]",
        ));
    }

    let bundle_path = PathBuf::from(&args[0]);
    let output_path = PathBuf::from(&args[1]);
    let preset = match args.get(2).map(String::as_str) {
        Some("h264-aac-balanced") | None => ExportPreset::H264AacBalanced,
        Some("h264-aac-draft") => ExportPreset::H264AacDraft,
        Some(value) => {
            return Err(ServerRuntimeError::new(
                ServerRuntimeErrorKind::Internal,
                format!("unknown export preset: {value}"),
            ));
        }
    };

    let runtime = ServerRuntime::new()?;
    let opened = open_project(&runtime, &bundle_path)?;
    print_event(&CliEvent::Opened {
        project: opened.clone(),
    });

    let started = start_export(
        &runtime,
        ServerExportRequest::new(opened.handle, output_path, preset),
    )?;
    print_event(&CliEvent::Started {
        status: started.clone(),
    });

    let job_id = started.status.job_id;
    loop {
        let status = get_export_status(&runtime, &job_id)?;
        print_event(&CliEvent::Status {
            status: status.clone(),
        });
        if is_terminal_export_phase(status.status.phase) {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }

    Ok(())
}

fn print_event<T: Serialize>(event: &CliEvent<T>) {
    match serde_json::to_string(event) {
        Ok(line) => println!("{line}"),
        Err(error) => println!(
            "{{\"type\":\"error\",\"kind\":\"internal\",\"message\":\"failed to serialize CLI event: {error}\"}}"
        ),
    }
}
