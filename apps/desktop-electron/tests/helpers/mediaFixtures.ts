import { execFile } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import { basename, join } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const REPO_ROOT = join(process.cwd(), "../..");
const PHASE6_RESULTS_DIR = join(REPO_ROOT, "test-results", "phase6");

export type Phase6MediaFixtures = {
  rootDir: string;
  bundlePath: string;
  videoPath: string;
  audioPath: string;
  outputPath: string;
  videoName: string;
  audioName: string;
};

export async function generatePhase6MediaFixtures(): Promise<Phase6MediaFixtures> {
  const rootDir = join(PHASE6_RESULTS_DIR, `workflow-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`);
  const mediaDir = join(rootDir, "external-media");
  const exportDir = join(rootDir, "exports");
  const bundlePath = join(rootDir, "phase6-real-workflow.veproj");
  const videoPath = join(mediaDir, "phase6-video.mp4");
  const audioPath = join(mediaDir, "phase6-bgm.wav");
  const outputPath = join(exportDir, "phase6-export.mp4");

  await mkdir(mediaDir, { recursive: true });
  await mkdir(exportDir, { recursive: true });
  await mkdir(bundlePath, { recursive: true });
  await rm(outputPath, { force: true });

  await runFfmpeg([
    "-hide_banner",
    "-y",
    "-f",
    "lavfi",
    "-i",
    "color=c=0x1f6feb:size=160x90:rate=30:duration=2",
    "-an",
    "-c:v",
    "mpeg4",
    "-q:v",
    "4",
    "-pix_fmt",
    "yuv420p",
    videoPath
  ]);

  await runFfmpeg([
    "-hide_banner",
    "-y",
    "-f",
    "lavfi",
    "-i",
    "sine=frequency=660:sample_rate=44100:duration=2",
    "-ac",
    "1",
    "-c:a",
    "pcm_s16le",
    audioPath
  ]);

  return {
    rootDir,
    bundlePath,
    videoPath,
    audioPath,
    outputPath,
    videoName: basename(videoPath),
    audioName: basename(audioPath)
  };
}

async function runFfmpeg(args: string[]): Promise<void> {
  const ffmpegPath = process.env.VE_FFMPEG_PATH ?? "ffmpeg";

  try {
    await execFileAsync(ffmpegPath, args, {
      timeout: 20_000,
      maxBuffer: 1024 * 1024
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Phase 06 media fixture generation failed with ${ffmpegPath}: ${message}`);
  }
}
