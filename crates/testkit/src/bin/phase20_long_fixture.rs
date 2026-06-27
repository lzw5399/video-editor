use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;

use project_store::{StdPlatformFileSystem, open_project_bundle, save_project_bundle};
use testkit::large_timeline::{
    PHASE20_PRODUCT_SEGMENTS_PER_TRACK, PHASE20_SEGMENT_DURATION_US, Phase20ProductMediaUris,
    build_phase20_product_timeline,
};

const USAGE: &str = "Usage: phase20_long_fixture --bundle <path> --video <path> --audio <path>";

#[derive(Debug)]
struct Args {
    bundle_path: PathBuf,
    video_uri: String,
    audio_uri: String,
}

fn main() {
    match run() {
        Ok(summary) => println!("{summary}"),
        Err(error) => {
            eprintln!("{error}");
            process::exit(error.exit_code());
        }
    }
}

fn run() -> Result<String, CliError> {
    let args = parse_args(env::args_os().skip(1).collect())?;
    let media_uris = Phase20ProductMediaUris::new(args.video_uri.clone(), args.audio_uri.clone());
    let fixture = build_phase20_product_timeline(media_uris).map_err(CliError::from_error)?;

    let saved = save_project_bundle(&StdPlatformFileSystem, &args.bundle_path, &fixture.draft)
        .map_err(CliError::from_error)?;
    let reopened = open_project_bundle(&StdPlatformFileSystem, &args.bundle_path)
        .map_err(CliError::from_error)?;
    if reopened.bundle.draft != fixture.draft {
        return Err(CliError::runtime(
            "materialized project did not reopen as the same canonical draft",
        ));
    }

    let tracks = fixture.draft.tracks.len();
    let total_segments = fixture
        .draft
        .tracks
        .iter()
        .map(|track| track.segments.len())
        .sum::<usize>();
    let duration_us = PHASE20_SEGMENT_DURATION_US
        .checked_mul(PHASE20_PRODUCT_SEGMENTS_PER_TRACK as u64)
        .ok_or_else(|| CliError::runtime("phase 20 duration overflowed"))?;

    Ok(format!(
        "{{\"bundlePath\":{},\"projectJsonPath\":{},\"tracks\":{},\"segmentsPerTrack\":{},\"totalSegments\":{},\"durationUs\":{},\"videoUri\":{},\"audioUri\":{}}}",
        json_string(&saved.bundle_path.to_string_lossy()),
        json_string(&saved.project_json_path.to_string_lossy()),
        tracks,
        PHASE20_PRODUCT_SEGMENTS_PER_TRACK,
        total_segments,
        duration_us,
        json_string(&args.video_uri),
        json_string(&args.audio_uri)
    ))
}

fn parse_args(values: Vec<OsString>) -> Result<Args, CliError> {
    let mut bundle_path = None;
    let mut video_uri = None;
    let mut audio_uri = None;
    let mut index = 0;

    while index < values.len() {
        let flag = values[index].to_string_lossy();
        let Some(value) = values.get(index + 1) else {
            return Err(CliError::usage(format!("missing value for {flag}")));
        };
        match flag.as_ref() {
            "--bundle" => {
                reject_duplicate(bundle_path.is_some(), "--bundle")?;
                bundle_path = Some(PathBuf::from(value));
            }
            "--video" => {
                reject_duplicate(video_uri.is_some(), "--video")?;
                video_uri = Some(path_arg_to_string(value, "--video")?);
            }
            "--audio" => {
                reject_duplicate(audio_uri.is_some(), "--audio")?;
                audio_uri = Some(path_arg_to_string(value, "--audio")?);
            }
            other => return Err(CliError::usage(format!("unknown argument {other}"))),
        }
        index += 2;
    }

    Ok(Args {
        bundle_path: bundle_path.ok_or_else(|| CliError::usage("missing --bundle"))?,
        video_uri: video_uri.ok_or_else(|| CliError::usage("missing --video"))?,
        audio_uri: audio_uri.ok_or_else(|| CliError::usage("missing --audio"))?,
    })
}

fn reject_duplicate(already_set: bool, flag: &'static str) -> Result<(), CliError> {
    if already_set {
        return Err(CliError::usage(format!("duplicate {flag}")));
    }
    Ok(())
}

fn path_arg_to_string(value: &OsString, flag: &'static str) -> Result<String, CliError> {
    value
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| CliError::usage(format!("{flag} must be valid UTF-8")))
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

#[derive(Debug)]
struct CliError {
    message: String,
    kind: CliErrorKind,
}

#[derive(Debug)]
enum CliErrorKind {
    Usage,
    Runtime,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: format!("{}\n\n{USAGE}", message.into()),
            kind: CliErrorKind::Usage,
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: CliErrorKind::Runtime,
        }
    }

    fn from_error(error: impl Error) -> Self {
        Self::runtime(error.to_string())
    }

    fn exit_code(&self) -> i32 {
        match self.kind {
            CliErrorKind::Usage => 2,
            CliErrorKind::Runtime => 1,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CliError {}
