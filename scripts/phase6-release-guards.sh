#!/usr/bin/env bash
set -euo pipefail

require_literal() {
  local literal="$1"
  shift

  if ! rg -F -n "$literal" "$@" >/dev/null; then
    printf 'Missing required release text: %s\n' "$literal" >&2
    exit 1
  fi
}

require_script() {
  local package_file="$1"
  local script_name="$2"

  node - "$package_file" "$script_name" <<'NODE'
const fs = require("node:fs");
const [packageFile, scriptName] = process.argv.slice(2);
const pkg = JSON.parse(fs.readFileSync(packageFile, "utf8"));
if (!pkg.scripts || typeof pkg.scripts[scriptName] !== "string" || pkg.scripts[scriptName].length === 0) {
  console.error(`${packageFile} is missing script ${scriptName}`);
  process.exit(1);
}
NODE
}

forbid_literal() {
  local literal="$1"
  shift

  if rg -F -n "$literal" "$@" >/dev/null; then
    printf 'Forbidden release text still present: %s\n' "$literal" >&2
    exit 1
  fi
}

require_packaged_runtime_forces_resources_path() {
  node - <<'NODE'
const fs = require("node:fs");
const source = fs.readFileSync("apps/desktop-electron/src/main/index.ts", "utf8");
const start = source.indexOf("function configureBundledRuntimeEnvironment");
const end = source.indexOf("function platformArchSegment", start);
if (start < 0 || end < 0) {
  console.error("main process must define configureBundledRuntimeEnvironment before platformArchSegment");
  process.exit(1);
}

const body = source.slice(start, end);
const packagedBranch = body.indexOf("if (app.isPackaged)");
const packagedAssignment = body.indexOf("process.env.VE_BUNDLED_FFMPEG_DIR = bundledRuntimeDir", packagedBranch);
const envGuard = body.indexOf("if (process.env.VE_BUNDLED_FFMPEG_DIR !== undefined)");
if (packagedBranch < 0 || packagedAssignment < 0 || envGuard < 0 || packagedAssignment > envGuard) {
  console.error("packaged app must force VE_BUNDLED_FFMPEG_DIR to process.resourcesPath before honoring dev/test env overrides");
  process.exit(1);
}
NODE
}

require_bundled_runtime_entries() {
  require_literal "extraResources:" apps/desktop-electron/electron-builder.yml
  require_literal "runtime/ffmpeg" apps/desktop-electron/electron-builder.yml
  require_literal "to: \"ffmpeg\"" apps/desktop-electron/electron-builder.yml
  require_literal "provision:ffmpeg-runtime" apps/desktop-electron/package.json
  test -f apps/desktop-electron/scripts/provision-ffmpeg-runtime.mjs
  forbid_literal "command -v" apps/desktop-electron/scripts/provision-ffmpeg-runtime.mjs
  forbid_literal "VE_FFMPEG_SOURCE" apps/desktop-electron/scripts/provision-ffmpeg-runtime.mjs
  forbid_literal "VE_FFPROBE_SOURCE" apps/desktop-electron/scripts/provision-ffmpeg-runtime.mjs
}

test -f docs/release-ffmpeg-manifest.md
test -f docs/third-party-notices.md
test -f docs/mvp-known-limits.md

require_literal "FFmpeg and ffprobe are bundled application resources" docs/release-ffmpeg-manifest.md docs/third-party-notices.md docs/mvp-known-limits.md
require_literal "VE_BUNDLED_FFMPEG_DIR" docs/release-ffmpeg-manifest.md docs/third-party-notices.md docs/mvp-known-limits.md
require_literal "bundledRuntime" docs/release-ffmpeg-manifest.md
require_literal "legalReviewPending" docs/release-ffmpeg-manifest.md docs/third-party-notices.md
require_literal "redistributableBuild: false" docs/release-ffmpeg-manifest.md
forbid_literal "external/user-provided" docs/release-ffmpeg-manifest.md docs/third-party-notices.md docs/mvp-known-limits.md
forbid_literal "No FFmpeg binary is bundled" docs/release-ffmpeg-manifest.md docs/third-party-notices.md docs/mvp-known-limits.md
forbid_literal "VE_FFMPEG_PATH" docs/release-ffmpeg-manifest.md docs/third-party-notices.md docs/mvp-known-limits.md
forbid_literal "VE_FFPROBE_PATH" docs/release-ffmpeg-manifest.md docs/third-party-notices.md docs/mvp-known-limits.md

require_literal "signing" docs/mvp-known-limits.md
require_literal "notarization" docs/mvp-known-limits.md
require_literal "Phases 7-13" docs/mvp-known-limits.md
require_literal "Jianying" docs/mvp-known-limits.md
require_literal "CapCut" docs/mvp-known-limits.md
require_literal "Kaipai" docs/mvp-known-limits.md
require_literal "mobile" docs/mvp-known-limits.md
require_literal "server" docs/mvp-known-limits.md

require_script apps/desktop-electron/package.json package:dir
require_script apps/desktop-electron/package.json test:packaged-smoke
require_script apps/desktop-electron/package.json test:runtime-diagnostics
require_script apps/desktop-electron/package.json test:real-workflow
require_script apps/desktop-electron/package.json test:packaged-real-workflow
require_script apps/desktop-electron/package.json test:packaged

require_script package.json test:phase6-packaging
require_script package.json test:phase6-runtime
require_script package.json test:phase6-release-gates
require_script package.json test:phase6

require_bundled_runtime_entries
require_packaged_runtime_forces_resources_path

bash scripts/phase5-source-guards.sh
git diff --exit-code schemas apps/desktop-electron/src/generated
