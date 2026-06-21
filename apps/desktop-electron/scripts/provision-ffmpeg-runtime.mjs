import { chmod, stat, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { basename, join, relative, resolve } from "node:path";
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

const ffmpegDependencies = auditRuntimeBinaryDependencies(ffmpegPath);
const ffprobeDependencies = auditRuntimeBinaryDependencies(ffprobePath);
const bundledLibraries = auditBundledRuntimeLibraries(targetDir);
adHocSignRuntimeFiles([
  ...bundledLibraries.map((library) => join(targetDir, library)),
  ffmpegPath,
  ffprobePath
]);

const manifest = {
  runtimeId,
  source: "bundledRuntimeDirectory",
  reviewStatus: "legalReviewPending",
  bundledLibraries,
  ffmpeg: binaryManifest(ffmpegPath, ffmpegDependencies),
  ffprobe: binaryManifest(ffprobePath, ffprobeDependencies)
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

function binaryManifest(target, dependencies) {
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
    dependencyAudit: {
      status: "passed",
      checkedWith: process.platform === "darwin" ? "otool -L" : "not-required-on-this-platform",
      dependencies
    },
    sha256: sha256(target)
  };
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function auditRuntimeBinaryDependencies(target) {
  if (process.platform !== "darwin") {
    return [];
  }

  const output = execFileSync("otool", ["-L", target], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    maxBuffer: 1024 * 1024
  });
  const dependencies = output
    .split(/\r?\n/)
    .slice(1)
    .map((line) => line.trim().split(/\s+/, 1)[0])
    .filter((line) => line.length > 0);
  const forbidden = dependencies.filter((dependency) => !isAllowedMacosRuntimeDependency(dependency));

  if (forbidden.length > 0) {
    throw new Error(
      [
        `Bundled FFmpeg runtime binary has local-machine dynamic dependencies: ${target}`,
        ...forbidden.map((dependency) => `  - ${dependency}`),
        "Use a self-contained/static runtime or bundle and rewrite all non-system dylibs before packaging."
      ].join("\n")
    );
  }

  return dependencies;
}

function auditBundledRuntimeLibraries(runtimeDir) {
  if (process.platform !== "darwin") {
    return [];
  }

  const libraryDir = join(runtimeDir, "lib");
  if (!existsSync(libraryDir)) {
    return [];
  }

  const libraries = listDylibs(libraryDir);
  for (const library of libraries) {
    auditRuntimeBinaryDependencies(library);
  }

  return libraries.map((library) => relative(runtimeDir, library)).sort();
}

function adHocSignRuntimeFiles(paths) {
  if (process.platform !== "darwin") {
    return;
  }

  for (const path of paths) {
    execFileSync("codesign", ["--force", "--sign", "-", path], {
      stdio: ["ignore", "ignore", "pipe"],
      maxBuffer: 1024 * 1024
    });
  }
}

function listDylibs(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return listDylibs(path);
    }
    return entry.isFile() && entry.name.endsWith(".dylib") ? [path] : [];
  });
}

function isAllowedMacosRuntimeDependency(dependency) {
  return (
    dependency.startsWith("/System/Library/") ||
    dependency.startsWith("/usr/lib/") ||
    dependency.startsWith("@executable_path/") ||
    dependency.startsWith("@loader_path/") ||
    dependency.startsWith("@rpath/")
  );
}
