import { mkdir, rm } from "node:fs/promises";
import { basename, join } from "node:path";

const REPO_ROOT = join(process.cwd(), "../..");
const PHASE6_RESULTS_DIR = join(REPO_ROOT, "test-results", "phase6");
const MEDIA_FIXTURE_DIR = join(process.cwd(), "tests", "fixtures", "media");

export type Phase6MediaFixtures = {
  rootDir: string;
  bundlePath: string;
  videoPath: string;
  imagePath: string;
  audioPath: string;
  outputPath: string;
  videoName: string;
  imageName: string;
  audioName: string;
  expectedResolutionLabel: string;
  expectedWidth: number;
  expectedHeight: number;
  expectedFrameRate: string;
  expectedDurationSeconds: number;
  expectedTextContent: string;
};

export async function generatePhase6MediaFixtures(): Promise<Phase6MediaFixtures> {
  const rootDir = join(PHASE6_RESULTS_DIR, `workflow-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`);
  const exportDir = join(rootDir, "exports");
  const bundlePath = join(rootDir, "phase6-real-workflow.veproj");
  const videoPath = join(MEDIA_FIXTURE_DIR, "p0-moving-testsrc.mp4");
  const imagePath = join(MEDIA_FIXTURE_DIR, "p0-overlay-testsrc.png");
  const audioPath = join(MEDIA_FIXTURE_DIR, "p0-tone.wav");
  const outputPath = join(exportDir, "phase6-export.mp4");

  await mkdir(exportDir, { recursive: true });
  await mkdir(bundlePath, { recursive: true });
  await rm(outputPath, { force: true });

  return {
    rootDir,
    bundlePath,
    videoPath,
    imagePath,
    audioPath,
    outputPath,
    videoName: basename(videoPath),
    imageName: basename(imagePath),
    audioName: basename(audioPath),
    expectedResolutionLabel: "320x180",
    expectedWidth: 320,
    expectedHeight: 180,
    expectedFrameRate: "30/1",
    expectedDurationSeconds: 3,
    expectedTextContent: "真实工作流标题"
  };
}
