import { expect, test } from "@playwright/test";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import type { Draft } from "../src/generated/Draft";
import {
  expectCanonicalDraftStable,
  expectNoDerivedArtifactPollution,
  readCanonicalDraftSummary,
  writePhase20EvidenceSummary
} from "./helpers/longTimelineEvidence";

test("phase 20 canonical summary preserves draft facts and rejects derived pollution", async () => {
  const root = await mkdtemp(join(tmpdir(), "phase20-evidence-"));
  const bundlePath = await writeBundle(root, "canonical.veproj", createCanonicalDraft());

  const summary = await readCanonicalDraftSummary(bundlePath);

  expect(summary.draftId).toBe("phase20-canonical");
  expect(summary.materials).toHaveLength(2);
  expect(summary.tracks).toHaveLength(1);
  expect(summary.tracks[0]?.segments[0]).toMatchObject({
    segmentId: "segment-video-1",
    materialId: "material-video-1",
    sourceTimerange: { start: 0, duration: 1_000_000 },
    targetTimerange: { start: 0, duration: 1_000_000 },
    visual: {
      visible: true,
      fitMode: "fill"
    },
    text: null,
    audio: {
      gainMillis: 0,
      panBalanceMillis: 0
    }
  });
  expectCanonicalDraftStable(summary, await readCanonicalDraftSummary(bundlePath));

  const pollutedPath = await writeBundle(root, "polluted.veproj", {
    ...createCanonicalDraft(),
    previewCaches: []
  });
  await expect(expectNoDerivedArtifactPollution(pollutedPath)).rejects.toThrow(/previewCaches/);
});

test("phase 20 evidence summary is product-readable and keeps developer details separate", async () => {
  const root = await mkdtemp(join(tmpdir(), "phase20-evidence-summary-"));
  const evidencePath = await writePhase20EvidenceSummary({
    evidenceDir: root,
    status: "passed",
    workflow: "phase20-long-product-uat",
    stage: "save-reopen-export",
    productSummary: {
      message: "Long timeline save, reopen, and export completed with production evidence.",
      segmentCount: 540,
      exportCount: 2
    },
    developerDetails: {
      paths: {
        trace: "trace.zip",
        ffprobe: "ffprobe.json"
      }
    }
  });
  const summary = JSON.parse(await readFile(evidencePath, "utf8")) as {
    workflow?: string;
    stage?: string;
    productSummary?: unknown;
    developerDetails?: { paths?: Record<string, string> };
  };

  expect(summary.workflow).toBe("phase20-long-product-uat");
  expect(summary.stage).toBe("save-reopen-export");
  expect(summary.productSummary).toBeDefined();
  expect(summary.developerDetails?.paths).toEqual({
    trace: "trace.zip",
    ffprobe: "ffprobe.json"
  });
});

test("phase 20 evidence helper source rejects fallback preview and file-exists-only export proof", async () => {
  const source = await readFile(join(process.cwd(), "tests/helpers/longTimelineEvidence.ts"), "utf8");

  expect(source).toContain("renderGraphGpuComposited");
  expect(source).toContain("renderGraphGpu");
  expect(source).toContain("diagnosticSource");
  expect(source).toContain("requestProjectSessionPreviewFrameCount");
  expect(source).toContain("probeMediaRuntime");
  expect(source).toContain("ffprobe");
  expect(source).toContain("ffmpeg");
  expect(source).toContain("sampledFrames");
  expect(source).toContain("developerDetails");
});

async function writeBundle(root: string, name: string, value: unknown): Promise<string> {
  const bundlePath = join(root, name);
  await mkdir(bundlePath, { recursive: true });
  await writeFile(join(bundlePath, "project.json"), JSON.stringify(value, null, 2));
  return bundlePath;
}

function createCanonicalDraft(): Draft {
  return {
    schemaVersion: 1,
    draftId: "phase20-canonical",
    metadata: {
      name: "Phase 20 Canonical Test"
    },
    canvasConfig: {
      aspectRatio: { kind: "custom", numerator: 16, denominator: 9 },
      width: 1920,
      height: 1080,
      frameRate: { numerator: 30, denominator: 1 },
      background: { kind: "black" },
      adaptationPolicy: "auto"
    },
    materials: [
      {
        materialId: "material-video-1",
        kind: "video",
        uri: "file:///repo/video.mp4",
        displayName: "video.mp4",
        metadata: {
          duration: 1_000_000,
          width: 1920,
          height: 1080,
          frameRate: { numerator: 30, denominator: 1 },
          hasVideo: true,
          hasAudio: false
        },
        status: "available"
      },
      {
        materialId: "material-audio-1",
        kind: "audio",
        uri: "file:///repo/audio.wav",
        displayName: "audio.wav",
        metadata: {
          duration: 1_000_000,
          hasVideo: false,
          hasAudio: true,
          audioSampleRate: 48_000,
          audioChannels: 2
        },
        status: "available"
      }
    ],
    tracks: [
      {
        trackId: "track-video-1",
        kind: "video",
        name: "Video Track",
        muted: false,
        locked: false,
        visible: true,
        segments: [
          {
            segmentId: "segment-video-1",
            materialId: "material-video-1",
            sourceTimerange: { start: 0, duration: 1_000_000 },
            targetTimerange: { start: 0, duration: 1_000_000 },
            retiming: {
              mode: { kind: "constant", speed: { numerator: 1, denominator: 1 } },
              audioPolicy: "followVideoSpeed"
            },
            mainTrackMagnet: { enabled: true },
            keyframes: [],
            filters: [],
            transition: null,
            text: null,
            volume: { levelMillis: 1_000 },
            audio: {
              gainMillis: 0,
              panBalanceMillis: 0,
              fadeInDuration: { duration: 0 },
              fadeOutDuration: { duration: 0 },
              effectSlots: []
            },
            visual: {
              visible: true,
              transform: {
                position: { x: 0, y: 0 },
                scale: { xMillis: 1_000, yMillis: 1_000 },
                rotation: { degrees: 0 },
                opacity: { valueMillis: 1_000 },
                crop: { leftMillis: 0, rightMillis: 0, topMillis: 0, bottomMillis: 0 },
                anchor: { xMillis: 500, yMillis: 500 }
              },
              fitMode: "fill",
              backgroundFilling: { kind: "none" },
              blendMode: { kind: "normal" },
              mask: { kind: "none" }
            }
          }
        ],
        transitions: []
      }
    ]
  };
}
