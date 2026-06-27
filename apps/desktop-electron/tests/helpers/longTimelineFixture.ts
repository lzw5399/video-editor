import { execFile } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import { basename, join } from "node:path";
import { promisify } from "node:util";

import { USER_JOURNEY_LONG_AV_VIDEO, USER_JOURNEY_LONG_TONE_AUDIO } from "./userJourney";

const execFileAsync = promisify(execFile);

const REPO_ROOT = join(process.cwd(), "../..");
const PHASE20_RESULTS_DIR = join(REPO_ROOT, "test-results", "phase20");
const MATERIALIZER_COMMAND = "cargo run -p testkit --bin phase20_long_fixture";
const PRODUCT_SEGMENTS_PER_TRACK = 180;
const PRODUCT_TRACK_COUNT = 3;
const PRODUCT_SEGMENT_DURATION_US = 1_000_000;
const PRODUCT_TOTAL_SEGMENTS = PRODUCT_SEGMENTS_PER_TRACK * PRODUCT_TRACK_COUNT;
const PRODUCT_DURATION_US = PRODUCT_SEGMENTS_PER_TRACK * PRODUCT_SEGMENT_DURATION_US;

export type Phase20LongTimelineScale = {
  segmentsPerTrack: number;
  trackCount: number;
  totalSegments: number;
  segmentDurationUs: number;
  durationUs: number;
};

export type Phase20MaterializerSummary = {
  bundlePath: string;
  projectJsonPath: string;
  tracks: number;
  segmentsPerTrack: number;
  totalSegments: number;
  durationUs: number;
  videoUri: string;
  audioUri: string;
};

export type Phase20LongTimelineFixtures = {
  runId: string;
  rootDir: string;
  exportsDir: string;
  evidenceDir: string;
  bundlePath: string;
  exportPaths: readonly [string, string];
  firstExportPath: string;
  secondExportPath: string;
  videoPath: string;
  audioPath: string;
  videoName: string;
  audioName: string;
  expectedWidth: number;
  expectedHeight: number;
  expectedFrameRate: string;
  expectedDurationSeconds: number;
  expectedScale: Phase20LongTimelineScale;
  materializerSummary: Phase20MaterializerSummary;
};

export type GeneratePhase20LongTimelineFixtureOptions = {
  runId?: string;
};

export async function generatePhase20LongTimelineFixture(
  options: GeneratePhase20LongTimelineFixtureOptions = {}
): Promise<Phase20LongTimelineFixtures> {
  const runId = options.runId ?? `long-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  const rootDir = join(PHASE20_RESULTS_DIR, runId);
  const exportsDir = join(rootDir, "exports");
  const evidenceDir = join(rootDir, "evidence");
  const bundlePath = join(rootDir, "phase20-long-product.veproj");
  const firstExportPath = join(exportsDir, "phase20-long-first-export.mp4");
  const secondExportPath = join(exportsDir, "phase20-long-second-export.mp4");
  const videoPath = USER_JOURNEY_LONG_AV_VIDEO;
  const audioPath = USER_JOURNEY_LONG_TONE_AUDIO;

  await mkdir(rootDir, { recursive: true });
  await mkdir(exportsDir, { recursive: true });
  await mkdir(evidenceDir, { recursive: true });
  await rm(firstExportPath, { force: true });
  await rm(secondExportPath, { force: true });

  const materializerSummary = await runPhase20Materializer({
    bundlePath,
    videoPath,
    audioPath
  });

  return {
    runId,
    rootDir,
    exportsDir,
    evidenceDir,
    bundlePath,
    exportPaths: [firstExportPath, secondExportPath],
    firstExportPath,
    secondExportPath,
    videoPath,
    audioPath,
    videoName: basename(videoPath),
    audioName: basename(audioPath),
    expectedWidth: 1920,
    expectedHeight: 1080,
    expectedFrameRate: "30/1",
    expectedDurationSeconds: PRODUCT_DURATION_US / 1_000_000,
    expectedScale: {
      segmentsPerTrack: PRODUCT_SEGMENTS_PER_TRACK,
      trackCount: PRODUCT_TRACK_COUNT,
      totalSegments: PRODUCT_TOTAL_SEGMENTS,
      segmentDurationUs: PRODUCT_SEGMENT_DURATION_US,
      durationUs: PRODUCT_DURATION_US
    },
    materializerSummary
  };
}

async function runPhase20Materializer({
  bundlePath,
  videoPath,
  audioPath
}: {
  bundlePath: string;
  videoPath: string;
  audioPath: string;
}): Promise<Phase20MaterializerSummary> {
  const { stdout } = await execFileAsync(
    "cargo",
    ["run", "-p", "testkit", "--bin", "phase20_long_fixture", "--", "--bundle", bundlePath, "--video", videoPath, "--audio", audioPath],
    {
      cwd: REPO_ROOT,
      timeout: 180_000,
      maxBuffer: 8 * 1024 * 1024
    }
  );
  const summary = parseMaterializerSummary(stdout);

  if (
    summary.bundlePath !== bundlePath ||
    summary.videoUri !== videoPath ||
    summary.audioUri !== audioPath ||
    summary.tracks !== PRODUCT_TRACK_COUNT ||
    summary.segmentsPerTrack !== PRODUCT_SEGMENTS_PER_TRACK ||
    summary.totalSegments !== PRODUCT_TOTAL_SEGMENTS ||
    summary.durationUs !== PRODUCT_DURATION_US
  ) {
    throw new Error(
      `${MATERIALIZER_COMMAND} returned unexpected Phase 20 fixture facts: ${JSON.stringify(summary)}`
    );
  }

  return summary;
}

function parseMaterializerSummary(stdout: string): Phase20MaterializerSummary {
  const summary = JSON.parse(stdout.trim()) as Partial<Phase20MaterializerSummary>;
  for (const key of [
    "bundlePath",
    "projectJsonPath",
    "tracks",
    "segmentsPerTrack",
    "totalSegments",
    "durationUs",
    "videoUri",
    "audioUri"
  ] as const) {
    if (summary[key] === undefined) {
      throw new Error(`Phase 20 materializer summary is missing ${key}: ${stdout}`);
    }
  }
  return summary as Phase20MaterializerSummary;
}
