import { chmod, stat, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const platform = process.platform;
const arch = process.arch;
const runtimeId = `${platform}-${arch}`;
const targetDir = join(projectRoot, "runtime", "ffmpeg", runtimeId);
const ffmpegPath = join(targetDir, binaryName("ffmpeg"));
const ffprobePath = join(targetDir, binaryName("ffprobe"));

await validateRuntimeBinary(ffmpegPath);
await validateRuntimeBinary(ffprobePath);

const manifest = {
  runtimeId,
  source: "bundledRuntimeDirectory",
  reviewStatus: "legalReviewPending",
  ffmpeg: binaryManifest(ffmpegPath),
  ffprobe: binaryManifest(ffprobePath)
};

await writeFile(join(targetDir, "manifest.local.json"), `${JSON.stringify(manifest, null, 2)}\n`);

console.log(`Validated bundled FFmpeg runtime at ${targetDir}`);

async function validateRuntimeBinary(target) {
  const info = await stat(target).catch(() => null);
  if (info === null || !info.isFile()) {
    throw new Error(
      `Missing bundled FFmpeg runtime binary: ${target}. Place ffmpeg and ffprobe in apps/desktop-electron/runtime/ffmpeg/${runtimeId} before building.`
    );
  }
  if (process.platform !== "win32") {
    await chmod(target, 0o755);
  }
}

function binaryName(name) {
  return process.platform === "win32" ? `${name}.exe` : name;
}

function binaryManifest(target) {
  const versionOutput = execFileSync(target, ["-version"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    maxBuffer: 1024 * 1024
  });
  const firstLine = versionOutput.split(/\r?\n/).find((line) => line.trim().length > 0) ?? "";
  const configureLine = versionOutput.split(/\r?\n/).find((line) => line.startsWith("configuration:")) ?? null;
  return {
    fileName: basename(target),
    bundlePath: `ffmpeg/${runtimeId}/${basename(target)}`,
    version: firstLine,
    configureLine,
    sha256: sha256(target)
  };
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}
