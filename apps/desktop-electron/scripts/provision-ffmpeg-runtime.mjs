import { chmod, copyFile, mkdir, stat, writeFile } from "node:fs/promises";
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

const ffmpegSource = process.env.VE_FFMPEG_SOURCE ?? findBuildTool("ffmpeg");
const ffprobeSource = process.env.VE_FFPROBE_SOURCE ?? findBuildTool("ffprobe");

await mkdir(targetDir, { recursive: true });
await copyRuntimeBinary(ffmpegSource, join(targetDir, binaryName("ffmpeg")));
await copyRuntimeBinary(ffprobeSource, join(targetDir, binaryName("ffprobe")));

const manifest = {
  runtimeId,
  source: "buildMachineProvisioned",
  reviewStatus: "legalReviewPending",
  ffmpeg: binaryManifest(ffmpegSource, join(targetDir, binaryName("ffmpeg"))),
  ffprobe: binaryManifest(ffprobeSource, join(targetDir, binaryName("ffprobe")))
};

await writeFile(join(targetDir, "manifest.local.json"), `${JSON.stringify(manifest, null, 2)}\n`);

console.log(`Provisioned bundled FFmpeg runtime at ${targetDir}`);

function findBuildTool(name) {
  const output = execFileSync("sh", ["-lc", `command -v ${name}`], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"]
  }).trim();
  if (output.length === 0) {
    throw new Error(`Cannot provision bundled runtime: ${name} is not installed for the build machine.`);
  }
  return output;
}

async function copyRuntimeBinary(source, target) {
  const info = await stat(source).catch(() => null);
  if (info === null || !info.isFile()) {
    throw new Error(`Cannot provision bundled runtime: ${source} is not a file.`);
  }
  await copyFile(source, target);
  if (process.platform !== "win32") {
    await chmod(target, 0o755);
  }
}

function binaryName(name) {
  return process.platform === "win32" ? `${name}.exe` : name;
}

function binaryManifest(source, target) {
  const versionOutput = execFileSync(source, ["-version"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    maxBuffer: 1024 * 1024
  });
  const firstLine = versionOutput.split(/\r?\n/).find((line) => line.trim().length > 0) ?? "";
  const configureLine = versionOutput.split(/\r?\n/).find((line) => line.startsWith("configuration:")) ?? null;
  return {
    fileName: basename(target),
    bundlePath: `ffmpeg/${runtimeId}/${basename(target)}`,
    sourcePath: source,
    version: firstLine,
    configureLine,
    sha256: sha256(target)
  };
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}
