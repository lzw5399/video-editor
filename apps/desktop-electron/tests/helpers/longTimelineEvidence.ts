import { expect, type Page, type TestInfo } from "@playwright/test";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, join } from "node:path";
import { promisify } from "node:util";

import type { Draft, Material, Segment, Track } from "../../src/generated/Draft";
import type { Phase20LongTimelineFixtures } from "./longTimelineFixture";
import {
  captureVisiblePreviewEvidence,
  readNativeCommandObservations,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  readTaskRuntimeTelemetry,
  requestProjectSessionPreviewFrameCount,
  type PreviewEvidence,
  type ProductJourneyAppController,
  type TaskRuntimeTelemetryResponse
} from "./userJourney";

const execFileTextAsync = promisify(execFile);

const PROJECT_JSON_FILE = "project.json";
const FORBIDDEN_DERIVED_KEYS = new Set([
  "renderGraph",
  "renderGraphs",
  "ffmpegScript",
  "ffmpegScripts",
  "previewCache",
  "previewCaches",
  "previewFrame",
  "previewFrames",
  "thumbnail",
  "thumbnails",
  "waveform",
  "waveforms",
  "proxyFile",
  "proxyFiles",
  "export",
  "exports",
  "exportJob",
  "exportJobs",
  "runtime",
  "runtimeHandle",
  "runtimeHandles",
  "absoluteTempOutputPath"
]);

type NativeCommandObservations = Awaited<ReturnType<typeof readNativeCommandObservations>>;
type ProjectSessionCalls = Awaited<ReturnType<typeof readProjectSessionCalls>>;
type RealtimePreviewHostCalls = Awaited<ReturnType<typeof readRealtimePreviewHostCalls>>;

export type CanonicalMaterialSummary = Pick<Material, "materialId" | "kind" | "uri" | "displayName" | "metadata" | "status">;

export type CanonicalSegmentSummary = Pick<
  Segment,
  | "segmentId"
  | "materialId"
  | "sourceTimerange"
  | "targetTimerange"
  | "retiming"
  | "mainTrackMagnet"
  | "keyframes"
  | "filters"
  | "transition"
  | "text"
  | "volume"
  | "audio"
  | "visual"
>;

export type CanonicalTrackSummary = Pick<Track, "trackId" | "kind" | "name" | "muted" | "locked" | "visible" | "transitions"> & {
  segments: CanonicalSegmentSummary[];
};

export type CanonicalDraftSummary = {
  schemaVersion: Draft["schemaVersion"];
  draftId: Draft["draftId"];
  metadata: Draft["metadata"];
  canvasConfig: Draft["canvasConfig"];
  revision: number | null;
  materialCount: number;
  trackCount: number;
  segmentCount: number;
  materials: CanonicalMaterialSummary[];
  tracks: CanonicalTrackSummary[];
};

export type Phase20PreviewProductionEvidenceInput = {
  before: PreviewEvidence;
  after: PreviewEvidence;
  nativeCommandObservationsBefore?: NativeCommandObservations;
  nativeCommandObservationsAfter?: NativeCommandObservations;
  frameRequestsBefore?: number;
  frameRequestsAfter?: number;
};

export type Phase20ExportMediaExpectation = {
  outputPath: string;
  expectedWidth: number;
  expectedHeight: number;
  expectedFrameRate: string;
  expectedDurationSeconds: number;
  expectedDurationToleranceSeconds?: number;
  sampleTimesSeconds?: number[];
  editPointSeconds?: number[];
  minDistinctSampleHashes?: number;
  evidenceDir?: string;
};

export type Phase20SampledFrameEvidence = {
  timeSeconds: number;
  bytes: number;
  sha256: string;
};

export type Phase20ExportMediaEvidence = {
  outputPath: string;
  ffprobePath: string;
  ffmpegPath: string;
  durationSeconds: number;
  video: {
    width: number;
    height: number;
    avgFrameRate: string;
  };
  audio: {
    codecType: string;
  };
  sampledFrames: Phase20SampledFrameEvidence[];
  ffprobeJsonPath?: string;
  sampledFramesJsonPath?: string;
};

export type Phase20EvidenceSummaryInput = {
  evidenceDir: string;
  status: "passed" | "failed";
  workflow: string;
  stage: string;
  productSummary: Record<string, unknown>;
  developerDetails: Record<string, unknown>;
  fileName?: string;
};

export type Phase20FailureEvidenceInput = {
  evidenceDir?: string;
  fixtures?: Pick<Phase20LongTimelineFixtures, "evidenceDir" | "bundlePath" | "exportPaths">;
  workflow: string;
  stage: string;
  error: unknown;
  page?: Page;
  app?: ProductJourneyAppController;
  testInfo?: TestInfo;
  bundlePaths?: string[];
  exportPaths?: string[];
};

type RuntimeBinary = {
  path: string;
  source?: string | { kind?: string };
};

type RuntimeDiscovery = {
  ffprobe: RuntimeBinary;
  ffmpeg: RuntimeBinary;
};

type FfprobeOutput = {
  format?: {
    duration?: string;
  };
  streams?: Array<{
    codec_type?: string;
    width?: number;
    height?: number;
    avg_frame_rate?: string;
  }>;
};

export async function readCanonicalDraftSummary(bundlePath: string): Promise<CanonicalDraftSummary> {
  const { projectJsonPath, value } = await readProjectJson(bundlePath);
  await assertNoDerivedArtifactKeys(value, projectJsonPath);
  const draft = value as Draft;

  const materials = draft.materials.map((material) => ({
    materialId: material.materialId,
    kind: material.kind,
    uri: material.uri,
    displayName: material.displayName,
    metadata: canonicalClone(material.metadata),
    status: material.status
  }));
  const tracks = draft.tracks.map((track) => ({
    trackId: track.trackId,
    kind: track.kind,
    name: track.name,
    muted: track.muted,
    locked: track.locked,
    visible: track.visible,
    transitions: canonicalClone(track.transitions ?? []),
    segments: track.segments.map(canonicalSegmentSummary)
  }));

  return {
    schemaVersion: draft.schemaVersion,
    draftId: draft.draftId,
    metadata: canonicalClone(draft.metadata),
    canvasConfig: canonicalClone(draft.canvasConfig),
    revision: readRevisionFact(value),
    materialCount: materials.length,
    trackCount: tracks.length,
    segmentCount: tracks.reduce((count, track) => count + track.segments.length, 0),
    materials,
    tracks
  };
}

export function expectCanonicalDraftStable(
  before: CanonicalDraftSummary,
  after: CanonicalDraftSummary,
  message = "Phase 20 save/reopen must preserve normalized canonical draft facts"
): void {
  expect(after, message).toEqual(before);
}

export async function expectNoDerivedArtifactPollution(bundlePath: string): Promise<void> {
  const { projectJsonPath, value } = await readProjectJson(bundlePath);
  await assertNoDerivedArtifactKeys(value, projectJsonPath);
}

export function expectPhase20PreviewProductionEvidence({
  before,
  after,
  nativeCommandObservationsBefore,
  nativeCommandObservationsAfter,
  frameRequestsBefore,
  frameRequestsAfter
}: Phase20PreviewProductionEvidenceInput): void {
  const beforeFrameRequests =
    frameRequestsBefore ??
    (nativeCommandObservationsBefore === undefined ? undefined : requestProjectSessionPreviewFrameCount(nativeCommandObservationsBefore));
  const afterFrameRequests =
    frameRequestsAfter ??
    (nativeCommandObservationsAfter === undefined ? undefined : requestProjectSessionPreviewFrameCount(nativeCommandObservationsAfter));

  if (beforeFrameRequests === undefined || afterFrameRequests === undefined) {
    throw new Error("Phase 20 preview evidence requires artifact preview-frame request counts before and after playback");
  }

  expect(after.hostState?.ok, "Phase 20 preview evidence requires an ok realtime host state").toBe(true);
  expect(after.hostState?.productReady, "Phase 20 preview evidence requires product-ready preview").toBe(true);
  expect(after.hostState?.fallbackActive, "Phase 20 preview evidence must not use fallback").toBe(false);
  expect(after.hostState?.backend, "Phase 20 preview backend must be renderGraphGpu").toBe("renderGraphGpu");
  expect(after.hostState?.diagnosticSource, "Phase 20 preview success must not use diagnostic sources").toBe("none");
  expect(
    after.hostState?.contentEvidence?.source,
    "Phase 20 preview success requires renderGraphGpuComposited evidence"
  ).toBe("renderGraphGpuComposited");
  expect(after.hostState?.frameDisplay, "Phase 20 preview success must not be a runtime frame artifact").toBeNull();
  expect(after.hostState?.contentEvidence?.digest).not.toBe(before.hostState?.contentEvidence?.digest ?? null);
  expect(after.visibleCenterHash, "Phase 20 preview visible center pixels must change").not.toBe(before.visibleCenterHash);
  expect(
    afterFrameRequests,
    "Phase 20 preview success must not increase requestProjectSessionPreviewFrame artifact reads"
  ).toBe(beforeFrameRequests);
}

export async function expectPhase20ExportMedia(
  page: Page,
  expectation: Phase20ExportMediaExpectation
): Promise<Phase20ExportMediaEvidence> {
  await expectFileExists(expectation.outputPath);
  const runtime = await readBundledRuntimeDiscovery(page);
  const probe = await readFfprobeJson(runtime.ffprobe.path, expectation.outputPath);
  const videoStream = probe.streams?.find((stream) => stream.codec_type === "video");
  const audioStream = probe.streams?.find((stream) => stream.codec_type === "audio");
  const durationSeconds = Number(probe.format?.duration ?? "0");

  expect(videoStream?.width, "Phase 20 export width must match the project canvas").toBe(expectation.expectedWidth);
  expect(videoStream?.height, "Phase 20 export height must match the project canvas").toBe(expectation.expectedHeight);
  expect(videoStream?.avg_frame_rate, "Phase 20 export frame rate must match the project canvas").toBe(expectation.expectedFrameRate);
  expect(audioStream, "Phase 20 export must contain audio").toBeDefined();
  expect(durationSeconds, "Phase 20 export duration must be positive").toBeGreaterThan(0);
  const tolerance = expectation.expectedDurationToleranceSeconds ?? 0.75;
  expect(durationSeconds).toBeGreaterThan(expectation.expectedDurationSeconds - tolerance);
  expect(durationSeconds).toBeLessThan(expectation.expectedDurationSeconds + tolerance);

  const sampledFrames = await sampleExportFrames(runtime.ffmpeg.path, expectation.outputPath, sampleTimesFor(expectation, durationSeconds));
  expect(sampledFrames.length, "Phase 20 export validation must sample frames, not only metadata").toBeGreaterThanOrEqual(3);
  expect(
    new Set(sampledFrames.map((frame) => frame.sha256)).size,
    "Phase 20 sampled export frames must show time-varying media evidence"
  ).toBeGreaterThanOrEqual(expectation.minDistinctSampleHashes ?? 2);

  const evidence: Phase20ExportMediaEvidence = {
    outputPath: expectation.outputPath,
    ffprobePath: runtime.ffprobe.path,
    ffmpegPath: runtime.ffmpeg.path,
    durationSeconds,
    video: {
      width: videoStream?.width ?? 0,
      height: videoStream?.height ?? 0,
      avgFrameRate: videoStream?.avg_frame_rate ?? ""
    },
    audio: {
      codecType: audioStream?.codec_type ?? ""
    },
    sampledFrames
  };

  if (expectation.evidenceDir !== undefined) {
    await mkdir(expectation.evidenceDir, { recursive: true });
    const baseName = sanitizeForFile(basename(expectation.outputPath, ".mp4"));
    const ffprobeJsonPath = join(expectation.evidenceDir, `${baseName}-ffprobe.json`);
    const sampledFramesJsonPath = join(expectation.evidenceDir, `${baseName}-sampled-frames.json`);
    await writeJson(ffprobeJsonPath, probe);
    await writeJson(sampledFramesJsonPath, sampledFrames);
    evidence.ffprobeJsonPath = ffprobeJsonPath;
    evidence.sampledFramesJsonPath = sampledFramesJsonPath;
  }

  return evidence;
}

export async function writePhase20EvidenceSummary(input: Phase20EvidenceSummaryInput): Promise<string> {
  await mkdir(input.evidenceDir, { recursive: true });
  const fileName = input.fileName ?? `${sanitizeForFile(input.workflow)}-${sanitizeForFile(input.stage)}-${input.status}.json`;
  const evidencePath = join(input.evidenceDir, fileName);
  await writeJson(evidencePath, {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    workflow: input.workflow,
    stage: input.stage,
    status: input.status,
    productSummary: input.productSummary,
    developerDetails: input.developerDetails
  });
  return evidencePath;
}

export async function collectPhase20FailureEvidence(input: Phase20FailureEvidenceInput): Promise<string> {
  const evidenceDir = input.evidenceDir ?? input.fixtures?.evidenceDir;
  if (evidenceDir === undefined) {
    throw new Error("Phase 20 failure evidence requires evidenceDir or fixtures.evidenceDir");
  }
  await mkdir(evidenceDir, { recursive: true });

  const stage = sanitizeForFile(input.stage);
  const screenshotPath = input.page === undefined ? undefined : join(evidenceDir, `${stage}-failure-screenshot.png`);
  if (input.page !== undefined && screenshotPath !== undefined) {
    await input.page.screenshot({ path: screenshotPath, fullPage: true }).catch(() => undefined);
  }

  const [
    previewEvidence,
    taskRuntimeTelemetry,
    nativeCommandObservations,
    projectSessionCalls,
    realtimePreviewHostCalls,
    canonicalDrafts,
    exportFiles
  ] = await Promise.all([
    captureOptional(input.page, (page) => captureVisiblePreviewEvidence(page, input.app)),
    captureOptional(input.page, (page) => readTaskRuntimeTelemetry(page)),
    captureOptional(input.app, (app) => readNativeCommandObservations(app)),
    captureOptional(input.app, (app) => readProjectSessionCalls(app)),
    captureOptional(input.app, (app) => readRealtimePreviewHostCalls(app)),
    collectCanonicalSummaries([...(input.bundlePaths ?? []), ...(input.fixtures === undefined ? [] : [input.fixtures.bundlePath])]),
    collectExportFileFacts([...(input.exportPaths ?? []), ...(input.fixtures?.exportPaths ?? [])])
  ]);

  return writePhase20EvidenceSummary({
    evidenceDir,
    status: "failed",
    workflow: input.workflow,
    stage: input.stage,
    productSummary: {
      message: errorMessage(input.error),
      workflow: input.workflow,
      stage: input.stage
    },
    developerDetails: {
      error: errorDetails(input.error),
      paths: {
        screenshot: screenshotPath,
        playwrightOutputDir: input.testInfo?.outputDir
      },
      previewEvidence,
      taskRuntimeTelemetry,
      nativeCommandObservations,
      projectSessionCalls,
      realtimePreviewHostCalls,
      canonicalDrafts,
      exportFiles
    }
  });
}

async function readProjectJson(bundlePath: string): Promise<{ projectJsonPath: string; value: unknown }> {
  const projectJsonPath = bundlePath.endsWith(PROJECT_JSON_FILE) ? bundlePath : join(bundlePath, PROJECT_JSON_FILE);
  const value = JSON.parse(await readFile(projectJsonPath, "utf8")) as unknown;
  return { projectJsonPath, value };
}

async function assertNoDerivedArtifactKeys(value: unknown, projectJsonPath: string): Promise<void> {
  const violations = collectForbiddenKeys(value);
  expect(
    violations,
    `${projectJsonPath} must contain only canonical draft facts, not derived runtime/export/cache artifacts`
  ).toEqual([]);
}

function collectForbiddenKeys(value: unknown): string[] {
  const violations: string[] = [];

  function visit(current: unknown, path: string): void {
    if (Array.isArray(current)) {
      current.forEach((entry, index) => visit(entry, `${path}[${index}]`));
      return;
    }
    if (!isRecord(current)) {
      return;
    }
    for (const [key, child] of Object.entries(current)) {
      const childPath = path === "$" ? `$.${key}` : `${path}.${key}`;
      if (FORBIDDEN_DERIVED_KEYS.has(key)) {
        violations.push(childPath);
      }
      visit(child, childPath);
    }
  }

  visit(value, "$");
  return violations;
}

function canonicalSegmentSummary(segment: Segment): CanonicalSegmentSummary {
  return {
    segmentId: segment.segmentId,
    materialId: segment.materialId,
    sourceTimerange: canonicalClone(segment.sourceTimerange),
    targetTimerange: canonicalClone(segment.targetTimerange),
    retiming: canonicalClone(segment.retiming),
    mainTrackMagnet: canonicalClone(segment.mainTrackMagnet),
    keyframes: canonicalClone(segment.keyframes),
    filters: canonicalClone(segment.filters),
    transition: canonicalClone(segment.transition ?? null),
    text: canonicalClone(segment.text ?? null),
    volume: canonicalClone(segment.volume),
    audio: canonicalClone(segment.audio),
    visual: canonicalClone(segment.visual)
  };
}

function readRevisionFact(value: unknown): number | null {
  if (!isRecord(value)) {
    return null;
  }
  const revision = value.revision ?? value.projectRevision ?? value.draftRevision;
  return typeof revision === "number" ? revision : null;
}

async function readBundledRuntimeDiscovery(page: Page): Promise<RuntimeDiscovery> {
  const runtime = await page.evaluate(async () => {
    type CommandResultEnvelope<T> = {
      ok: boolean;
      data: T | null;
      error: { message?: string } | null;
    };
    type RuntimeBinaryResult = {
      path?: string;
      source?: string | { kind?: string };
    };
    type RuntimeResult = {
      ffprobe?: RuntimeBinaryResult;
      ffmpeg?: RuntimeBinaryResult;
    };
    const api = (window as typeof window & {
      videoEditorCore?: {
        probeMediaRuntime: () => Promise<CommandResultEnvelope<RuntimeResult>>;
      };
    }).videoEditorCore;
    return api?.probeMediaRuntime();
  });

  if (runtime?.ok !== true || runtime.data?.ffprobe?.path === undefined || runtime.data.ffmpeg?.path === undefined) {
    throw new Error(`Unable to read bundled ffprobe/ffmpeg paths from app runtime: ${JSON.stringify(runtime)}`);
  }

  for (const [kind, binary] of Object.entries({
    ffprobe: runtime.data.ffprobe,
    ffmpeg: runtime.data.ffmpeg
  })) {
    const source = binary.source;
    expect(typeof source === "string" ? source : source?.kind, `Phase 20 ${kind} must come from bundled runtime discovery`).toBe(
      "bundled"
    );
    expect(binary.path, `Phase 20 ${kind} path must not come from Homebrew or PATH fallback`).not.toContain("/opt/homebrew");
  }

  return {
    ffprobe: runtime.data.ffprobe as RuntimeBinary,
    ffmpeg: runtime.data.ffmpeg as RuntimeBinary
  };
}

async function readFfprobeJson(ffprobePath: string, outputPath: string): Promise<FfprobeOutput> {
  const { stdout } = await execFileTextAsync(
    ffprobePath,
    ["-v", "error", "-print_format", "json", "-show_format", "-show_streams", outputPath],
    {
      timeout: 30_000,
      maxBuffer: 4 * 1024 * 1024
    }
  );
  return JSON.parse(stdout) as FfprobeOutput;
}

function sampleTimesFor(expectation: Phase20ExportMediaExpectation, durationSeconds: number): number[] {
  const base = expectation.sampleTimesSeconds ?? [0.5, durationSeconds / 2, Math.max(0.5, durationSeconds - 0.5)];
  const times = [...base, ...(expectation.editPointSeconds ?? [])]
    .map((time) => Math.max(0, Math.min(durationSeconds - 0.05, time)))
    .filter((time) => Number.isFinite(time));
  return [...new Set(times.map((time) => Number(time.toFixed(3))))].sort((left, right) => left - right);
}

async function sampleExportFrames(ffmpegPath: string, outputPath: string, sampleTimesSeconds: number[]): Promise<Phase20SampledFrameEvidence[]> {
  return Promise.all(sampleTimesSeconds.map((timeSeconds) => sampleExportFrame(ffmpegPath, outputPath, timeSeconds)));
}

async function sampleExportFrame(ffmpegPath: string, outputPath: string, timeSeconds: number): Promise<Phase20SampledFrameEvidence> {
  const { stdout } = await execFileBuffer(ffmpegPath, [
    "-hide_banner",
    "-loglevel",
    "error",
    "-ss",
    timeSeconds.toFixed(3),
    "-i",
    outputPath,
    "-frames:v",
    "1",
    "-f",
    "image2pipe",
    "-vcodec",
    "png",
    "pipe:1"
  ]);
  expect(stdout.length, `Phase 20 sampled frame at ${timeSeconds}s must produce image bytes`).toBeGreaterThan(1024);
  return {
    timeSeconds,
    bytes: stdout.length,
    sha256: createHash("sha256").update(stdout).digest("hex")
  };
}

async function execFileBuffer(file: string, args: string[]): Promise<{ stdout: Buffer; stderr: Buffer }> {
  return new Promise((resolve, reject) => {
    execFile(
      file,
      args,
      {
        encoding: "buffer",
        timeout: 30_000,
        maxBuffer: 16 * 1024 * 1024
      },
      (error, stdout, stderr) => {
        const stdoutBuffer = Buffer.isBuffer(stdout) ? stdout : Buffer.from(stdout);
        const stderrBuffer = Buffer.isBuffer(stderr) ? stderr : Buffer.from(stderr);
        if (error !== null) {
          reject(
            new Error(
              `${file} ${args.join(" ")} failed: ${error.message}\nstdout=${stdoutBuffer.toString("utf8")}\nstderr=${stderrBuffer.toString("utf8")}`
            )
          );
          return;
        }
        resolve({ stdout: stdoutBuffer, stderr: stderrBuffer });
      }
    );
  });
}

async function expectFileExists(path: string): Promise<void> {
  await expect(access(path).then(
    () => true,
    () => false
  )).resolves.toBe(true);
}

async function collectCanonicalSummaries(bundlePaths: string[]): Promise<Array<CanonicalDraftSummary | { bundlePath: string; error: string }>> {
  return Promise.all(
    [...new Set(bundlePaths)].map(async (bundlePath) =>
      readCanonicalDraftSummary(bundlePath).catch((error: unknown) => ({
        bundlePath,
        error: errorMessage(error)
      }))
    )
  );
}

async function collectExportFileFacts(exportPaths: string[]): Promise<Array<{ path: string; exists: boolean; size?: number }>> {
  return Promise.all(
    [...new Set(exportPaths)].map(async (path) => {
      const exists = await access(path).then(
        () => true,
        () => false
      );
      return { path, exists };
    })
  );
}

async function captureOptional<TTarget, TResult>(
  target: TTarget | undefined,
  collect: (target: TTarget) => Promise<TResult>
): Promise<TResult | { error: string } | null> {
  if (target === undefined) {
    return null;
  }
  return collect(target).catch((error: unknown) => ({ error: errorMessage(error) }));
}

async function writeJson(path: string, value: unknown): Promise<void> {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function canonicalClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function errorDetails(error: unknown): Record<string, unknown> {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      stack: error.stack
    };
  }
  return {
    message: String(error)
  };
}

function sanitizeForFile(value: string): string {
  return value.replace(/[^A-Za-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "") || "phase20";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
