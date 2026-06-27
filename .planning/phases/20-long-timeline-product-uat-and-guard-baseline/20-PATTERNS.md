# Phase 20: Long Timeline Product UAT And Guard Baseline - Pattern Map

**Mapped:** 2026-06-28
**Files analyzed:** 12 expected new/modified files
**Analogs found:** 12 / 12

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/testkit/src/large_timeline.rs` | utility / fixture builder | batch, transform | `crates/testkit/src/large_timeline.rs` | exact |
| `crates/testkit/Cargo.toml` | config | dependency config | `crates/testkit/Cargo.toml` | exact |
| `crates/testkit/tests/long_timeline_product_fixture.rs` | test | batch, file-I/O, transform | `crates/testkit/tests/large_timeline_incremental.rs`; `crates/testkit/tests/template_import_exports.rs` | role-match |
| `crates/testkit/tests/large_timeline_incremental.rs` | test | batch, transform | `crates/testkit/tests/large_timeline_incremental.rs` | exact |
| `apps/desktop-electron/tests/helpers/longTimelineFixture.ts` | utility | file-I/O, process orchestration | `apps/desktop-electron/tests/helpers/mediaFixtures.ts` | role-match |
| `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts` | utility | file-I/O, transform, process orchestration | `apps/desktop-electron/tests/helpers/realWorkflow.ts`; `apps/desktop-electron/tests/helpers/userJourney.ts` | role-match |
| `apps/desktop-electron/tests/helpers/userJourney.ts` | utility | event-driven, request-response | `apps/desktop-electron/tests/helpers/userJourney.ts` | exact |
| `apps/desktop-electron/tests/helpers/realWorkflow.ts` | utility | file-I/O, request-response | `apps/desktop-electron/tests/helpers/realWorkflow.ts` | exact |
| `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts` | test | event-driven, request-response, file-I/O | `apps/desktop-electron/tests/real-workflow.spec.ts`; `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` | role-match |
| `scripts/phase20-source-guards.sh` | utility / guard script | batch, source scan | `scripts/phase19-source-guards.sh` | role-match |
| `scripts/no-product-fallback-guards.sh` | utility / guard script | batch, source scan | `scripts/no-product-fallback-guards.sh` | exact |
| `package.json` | config | batch orchestration | `package.json` Phase 16/19 scripts | role-match |

## Pattern Assignments

### `crates/testkit/src/large_timeline.rs` (utility, batch/transform)

**Analog:** `crates/testkit/src/large_timeline.rs`

**Imports pattern** (lines 3-8):
```rust
use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasBackground, Draft, DraftCanvasConfig,
    Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, Segment, SourceTimerange,
    TargetTimerange, TextSegment, TextSegmentSource, TextStyle, TextWrapping, Track, TrackId,
    TrackKind, validate_draft,
};
```

**Config builder pattern** (lines 24-63):
```rust
impl LargeTimelineConfig {
    pub fn new(segments_per_track: usize) -> Self {
        Self {
            segments_per_track,
            ..Self::default()
        }
    }

    pub fn with_track_mix(
        mut self,
        include_video: bool,
        include_audio: bool,
        include_text: bool,
    ) -> Self {
        self.include_video = include_video;
        self.include_audio = include_audio;
        self.include_text = include_text;
        self
    }
}
```

**Core build/validate pattern** (lines 136-161):
```rust
pub fn build_large_timeline(
    config: LargeTimelineConfig,
) -> Result<LargeTimelineDraft, LargeTimelineError> {
    validate_config(&config)?;

    let mut draft = Draft::new("phase13-large-timeline-draft", "Phase 13 Large Timeline");
    draft.canvas_config = config.canvas_config.clone();

    if config.include_video {
        push_track_with_segments(&mut draft, &config, TrackKind::Video)?;
    }
    if config.include_audio {
        push_track_with_segments(&mut draft, &config, TrackKind::Audio)?;
    }
    if config.include_text {
        push_track_with_segments(&mut draft, &config, TrackKind::Text)?;
    }

    validate_draft(&draft).map_err(|error| LargeTimelineError::new(error.to_string()))?;
    let localized_edit = localized_edit_target(&draft, &config)?;

    Ok(LargeTimelineDraft {
        draft,
        localized_edit,
    })
}
```

**Validation/error pattern** (lines 190-218):
```rust
fn validate_config(config: &LargeTimelineConfig) -> Result<(), LargeTimelineError> {
    if config.segments_per_track == 0 {
        return Err(LargeTimelineError::new(
            "segments_per_track must be greater than zero",
        ));
    }
    if config.segments_per_track > MAX_SEGMENTS_PER_TRACK {
        return Err(LargeTimelineError::new(format!(
            "segments_per_track must be <= {MAX_SEGMENTS_PER_TRACK}"
        )));
    }
    if config.track_count() == 0 {
        return Err(LargeTimelineError::new(
            "at least one video, audio, or text track must be enabled",
        ));
    }
    if config.segment_duration.get() == 0 {
        return Err(LargeTimelineError::new(
            "segment_duration must be greater than zero microseconds",
        ));
    }
    checked_target_range(last_index, config.segment_duration, config.target_stride)?;
    Ok(())
}
```

**Apply to Phase 20:** Add Phase 20 constants/helpers around `180`, `1000`, and diagnostic `3000` segments per track, but keep integer microsecond duration (`1_000_000`) and `validate_draft` in Rust. If a helper in `src/large_timeline.rs` saves `.veproj` bundles, `project_store` must move from dev-dependency to dependency or the saver must live in a test/support binary instead.

---

### `crates/testkit/Cargo.toml` (config, dependency config)

**Analog:** `crates/testkit/Cargo.toml`

**Current dependency boundary** (lines 12-28):
```toml
[dependencies]
audio_engine = { path = "../audio_engine" }
draft_model = { path = "../draft_model" }
engine_core = { path = "../engine_core" }
ffmpeg_compiler = { path = "../ffmpeg_compiler" }
media_runtime = { path = "../media_runtime" }
media_runtime_desktop = { path = "../media_runtime_desktop" }
preview_service = { path = "../preview_service" }
render_graph = { path = "../render_graph" }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
tempfile = "3.27.0"

[dev-dependencies]
project_store = { path = "../project_store" }
realtime_preview_runtime = { path = "../realtime_preview_runtime" }
```

**Apply to Phase 20:** Keep `project_store` as a dev-dependency if only integration tests materialize/open the long bundle. Move it to `[dependencies]` only if a public `testkit::large_timeline` helper saves `.veproj` bundles from library code.

---

### `crates/testkit/tests/long_timeline_product_fixture.rs` (test, batch/file-I/O/transform)

**Analogs:** `crates/testkit/tests/large_timeline_incremental.rs`, `crates/testkit/tests/template_import_exports.rs`, `crates/project_store/tests/project_bundle.rs`

**Imports pattern for large timeline structural tests** (large_timeline_incremental.rs lines 1-16):
```rust
use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasBackground, DirtyDomain, DirtyRange,
    DirtyRangeSource, Draft, DraftCanvasConfig, Microseconds, RationalFrameRate, SegmentOpacity,
    SegmentVolume, TargetTimerange, TextSegment, TrackKind, validate_draft,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    PreviewInvalidationRequest, invalidate_preview_cache,
};
use render_graph::{
    OutputDimensions, RenderGraphDiff, RenderGraphSnapshot, RenderOutputProfile, build_render_graph,
};
use testkit::large_timeline::{
    LargeTimelineConfig, MAX_SEGMENTS_PER_TRACK, assert_no_track_overlaps, build_large_timeline,
};
```

**Deterministic fixture test pattern** (large_timeline_incremental.rs lines 18-35):
```rust
#[test]
fn large_timeline_incremental_fixture_is_deterministic_and_valid() {
    let config = LargeTimelineConfig::new(240).with_localized_edit_index(120);

    let first = build_large_timeline(config.clone()).expect("first fixture should build");
    let second = build_large_timeline(config).expect("second fixture should build");

    validate_draft(&first.draft).expect("large draft should validate");
    assert_no_track_overlaps(&first.draft).expect("large draft tracks should not overlap");
    assert_eq!(
        serde_json::to_value(&first.draft).expect("first draft serializes"),
        serde_json::to_value(&second.draft).expect("second draft serializes"),
        "large timeline fixtures should be deterministic"
    );
    assert_eq!(first.draft.tracks.len(), 3);
}
```

**Save/open canonical bundle pattern** (template_import_exports.rs lines 235-245):
```rust
let saved = save_project_bundle(&StdPlatformFileSystem, &bundle_path, &applied.draft)
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
if reopened.bundle.draft != applied.draft {
    return Err(RenderCompareError::Assertion(
        "saved imported draft should reopen as the same canonical draft".to_owned(),
    ));
}
let project_json = fs::read_to_string(saved.project_json_path)?;
```

**Derived artifact rejection pattern** (project_bundle.rs lines 163-179):
```rust
#[test]
fn open_project_bundle_rejects_derived_artifact_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("derived-artifacts.veproj");
    let mut value = serde_json::to_value(Draft::new("draft-001", "Derived artifact leak"))
        .expect("draft should serialize");
    value["previewCaches"] = json!([]);
    write_project_json(&bundle_path, value);

    let error = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect_err("derived artifact field should fail");

    assert!(
        matches!(error, ProjectStoreError::SemanticValidation { .. }),
        "unexpected error: {error}"
    );
}
```

**Apply to Phase 20:** New Rust fixture/materialization tests should assert `180 x 3` product scale and `1000 x 3` blocking scale, save through `project_store`, reopen to the same canonical draft, and scan `project.json` for forbidden derived/runtime fields. Use non-blocking `3000 x 3` diagnostics only outside the default gate.

---

### `crates/testkit/tests/large_timeline_incremental.rs` (test, batch/transform)

**Analog:** `crates/testkit/tests/large_timeline_incremental.rs`

**Bounded graph diff/cache invalidation pattern** (lines 94-210):
```rust
#[test]
fn large_timeline_incremental_localized_move_has_bounded_graph_diff_dirty_ranges_and_cache_scope() {
    let fixture = build_large_timeline(
        LargeTimelineConfig::new(360)
            .with_localized_edit_index(180)
            .with_target_stride(Microseconds::new(250_000)),
    )
    .expect("large timeline fixture should build");
    let full_range = full_draft_range(&fixture.draft);
    let previous = snapshot_for(&fixture.draft, full_range.clone());

    let mut edited = fixture.draft.clone();
    let moved_start = fixture.localized_edit.target_timerange.start.get() + 100_000;
    segment_mut(
        &mut edited,
        fixture.localized_edit.track_kind,
        fixture.localized_edit.segment_index,
    )
    .target_timerange = TargetTimerange::new(
        moved_start,
        fixture.localized_edit.target_timerange.duration,
    );
    assert_no_track_overlaps(&edited).expect("localized move should stay inside the segment gap");

    let current = snapshot_for(&edited, full_range);
    let diff = RenderGraphDiff::between(
        &previous,
        &current,
        &dirty_ranges,
        &[DirtyDomain::Timing, DirtyDomain::GraphSnapshot],
    );
    assert!(diff.changed.len() <= 16);
    assert!(diff.unchanged.len() > diff.changed.len() * 100);
    assert_eq!(invalidated_ids(&result), vec!["old-range", "current-range"]);
}
```

**Shared bounded-change helper** (lines 390-427):
```rust
fn assert_localized_change_is_bounded(
    draft: &Draft,
    full_range: TargetTimerange,
    label: &str,
    domain: DirtyDomain,
    edit: impl FnOnce(&mut Draft),
) {
    let previous = snapshot_for(draft, full_range.clone());
    let mut edited = draft.clone();
    edit(&mut edited);
    validate_draft(&edited).expect("localized edit should keep the draft valid");
    let current = snapshot_for(&edited, full_range);
    let diff = RenderGraphDiff::between(
        &previous,
        &current,
        &[dirty_range(42_000_000, 100_000, DirtyRangeSource::Current)],
        &[domain, DirtyDomain::GraphSnapshot],
    );

    assert!(diff.added.is_empty(), "{label} should not add graph nodes");
    assert!(diff.removed.is_empty(), "{label} should not remove graph nodes");
    assert!(diff.changed.len() <= 16);
    assert!(diff.unchanged.len() > diff.changed.len() * 100);
    assert!(diff.dirty_domains.contains(&domain));
}
```

**Apply to Phase 20:** Extend these tests or add a sibling test file. Do not gate on wall-clock timing in Rust; gate on bounded graph diff, dirty ranges, and cache invalidation. Capture wall-clock only as diagnostic output if needed.

---

### `apps/desktop-electron/tests/helpers/longTimelineFixture.ts` (utility, file-I/O/process)

**Analog:** `apps/desktop-electron/tests/helpers/mediaFixtures.ts`

**Imports/path constants pattern** (lines 1-7):
```typescript
import { mkdir, rm } from "node:fs/promises";
import { basename, join } from "node:path";

const REPO_ROOT = join(process.cwd(), "../..");
const PHASE6_RESULTS_DIR = join(REPO_ROOT, "test-results", "phase6");
const MEDIA_FIXTURE_DIR = join(process.cwd(), "tests", "fixtures", "media");
```

**Fixture return shape and deterministic setup pattern** (lines 8-56):
```typescript
export type Phase6MediaFixtures = {
  rootDir: string;
  bundlePath: string;
  videoPath: string;
  imagePath: string;
  audioPath: string;
  outputPath: string;
  expectedWidth: number;
  expectedHeight: number;
  expectedFrameRate: string;
  expectedDurationSeconds: number;
};

export async function generatePhase6MediaFixtures(): Promise<Phase6MediaFixtures> {
  const rootDir = join(PHASE6_RESULTS_DIR, `workflow-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`);
  const exportDir = join(rootDir, "exports");
  const bundlePath = join(rootDir, "phase6-real-workflow.veproj");
  const outputPath = join(exportDir, "phase6-export.mp4");

  await mkdir(exportDir, { recursive: true });
  await mkdir(bundlePath, { recursive: true });
  await rm(outputPath, { force: true });

  return {
    rootDir,
    bundlePath,
    outputPath,
    expectedWidth: 320,
    expectedHeight: 180,
    expectedFrameRate: "30/1",
    expectedDurationSeconds: 6
  };
}
```

**Apply to Phase 20:** Create `test-results/phase20/<run-id>/`, `exports/`, and a generated `.veproj` path. The helper may shell out to an existing/new Rust test/helper only for fixture materialization; it must not construct the 540-segment draft in TypeScript. Return expected semantic counts: `segmentsPerTrack: 180`, `trackKinds: ["video", "audio", "text"]`, expected duration about `180` seconds, and export paths for two exports.

---

### `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts` (utility, file-I/O/transform/process)

**Analogs:** `apps/desktop-electron/tests/helpers/realWorkflow.ts`, `apps/desktop-electron/tests/helpers/userJourney.ts`, `crates/project_store/tests/project_bundle.rs`

**ffprobe export metadata pattern** (realWorkflow.ts lines 512-535):
```typescript
async function expectExportMedia(path: string, fixtures: Phase6MediaFixtures, page: Page): Promise<void> {
  const ffprobePath = await readBundledFfprobePath(page);
  const { stdout } = await execFileAsync(
    ffprobePath,
    ["-v", "error", "-print_format", "json", "-show_format", "-show_streams", path],
    {
      timeout: 20_000,
      maxBuffer: 1024 * 1024
    }
  );
  const probe = JSON.parse(stdout) as {
    format?: { duration?: string };
    streams?: Array<{ codec_type?: string; width?: number; height?: number; avg_frame_rate?: string }>;
  };
  const videoStream = probe.streams?.find((stream) => stream.codec_type === "video");
  const audioStream = probe.streams?.find((stream) => stream.codec_type === "audio");
  expect(videoStream?.width).toBe(fixtures.expectedWidth);
  expect(videoStream?.height).toBe(fixtures.expectedHeight);
  expect(videoStream?.avg_frame_rate).toBe(fixtures.expectedFrameRate);
  expect(audioStream, "export should contain an audio stream").toBeDefined();
  const duration = Number(probe.format?.duration ?? "0");
  expect(duration).toBeGreaterThan(fixtures.expectedDurationSeconds - 0.35);
  expect(duration).toBeLessThan(fixtures.expectedDurationSeconds + 0.35);
  await expectFileExists(join(fixtures.bundlePath, "project.json"));
}
```

**Bundled runtime path pattern** (realWorkflow.ts lines 538-559):
```typescript
async function readBundledFfprobePath(page: Page): Promise<string> {
  const runtime = await page.evaluate(() => {
    const api = (window as unknown as {
      videoEditorCore?: {
        probeMediaRuntime: () => Promise<{
          ok: boolean;
          data: null | { ffprobe?: { path?: string; source?: string | { kind?: string } } };
          error: null | { message?: string };
        }>;
      };
    }).videoEditorCore;
    return api?.probeMediaRuntime();
  });

  if (runtime?.ok !== true || runtime.data?.ffprobe?.path === undefined) {
    throw new Error(`Unable to read bundled ffprobe path from app runtime: ${JSON.stringify(runtime)}`);
  }
  const source = runtime.data.ffprobe.source;
  expect(typeof source === "string" ? source : source?.kind).toBe("bundled");
  expect(runtime.data.ffprobe.path).not.toContain("/opt/homebrew");
  return runtime.data.ffprobe.path;
}
```

**Visible preview evidence pattern** (userJourney.ts lines 490-501):
```typescript
export async function captureVisiblePreviewEvidence(
  page: Page,
  app: ProductJourneyAppController | undefined
): Promise<PreviewEvidence> {
  const evidence = await capturePreviewEvidence(page);
  if (process.platform !== "darwin" || app === undefined) {
    return evidence;
  }
  return {
    ...evidence,
    visibleCenterHash: hashBuffer(await captureVisiblePreviewCenter(page, app))
  };
}
```

**Semantic canonical comparison source pattern** (project_bundle.rs lines 27-39, 163-179):
```rust
#[test]
fn round_trip_save_open_preserves_semantic_draft_equality() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("round-trip.veproj");
    let draft = populated_draft("media/missing-video.mp4");

    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft).expect("draft should save");
    let opened =
        open_project_bundle(&StdPlatformFileSystem, &bundle_path).expect("saved draft should open");

    assert_eq!(opened.bundle.draft, draft);
    assert_eq!(opened.warnings.len(), 1);
}
```

**Apply to Phase 20:** `longTimelineEvidence.ts` should normalize `project.json` into canonical materials/tracks/segments/timing/visual/audio/text/revision facts, assert forbidden derived/runtime keys are absent, write lightweight success JSON, and on failure collect trace/screenshot/video paths, telemetry, project summaries, native command observations, ffprobe JSON, and sampled frame evidence. Export proof must use bundled runtime paths, not `PATH`.

---

### `apps/desktop-electron/tests/helpers/userJourney.ts` (utility, event-driven/request-response)

**Analog:** `apps/desktop-electron/tests/helpers/userJourney.ts`

**Open generated project through product entry** (lines 640-680, 698-703):
```typescript
export async function launchOpenedProductJourneyApp(
  projectBundlePath: string,
  openMaterialFiles: string[] = [],
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ProductJourneyAppController; page: Page }> {
  await expectFileExists(join(projectBundlePath, "project.json"));
  await Promise.all(openMaterialFiles.map((filePath) => expectFileExists(filePath)));
  const productEnv = {
    VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
    VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
    VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
    VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE: projectBundlePath,
    ...env
  };
  // launches app, clicks "打开项目", waits for workspace
}

export async function openProjectFromProductEntry(app: ProductJourneyAppController, page: Page): Promise<void> {
  await expectProductEntry(page);
  const nextCount = (await countProjectSessionCommand(app, "openProjectSession")) + 1;
  await page.getByRole("button", { name: "打开项目" }).click();
  await waitForProjectSessionCommandCount(app, "openProjectSession", nextCount);
}
```

**Workspace no-debug-surface assertions** (lines 705-717):
```typescript
export async function expectProductWorkspace(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.getByRole("button", { name: "导入素材" })).toBeVisible();
  await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
  await expect(page.locator('[aria-label="预览窗口"]')).toBeVisible();
  await expect(page.locator('[aria-label="属性检查器"]')).toBeVisible();
  await expect(page.locator('[aria-label="时间线"]')).toBeVisible();

  await expect(page.getByLabel("预览产物")).toHaveCount(0);
  await expect(page.getByText("草稿包路径")).toHaveCount(0);
  await expect(page.getByText("素材路径")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "导入路径" })).toHaveCount(0);
}
```

**Core UI action pattern** (lines 753-769, 912-943, 945-972):
```typescript
export async function addMaterialToTimeline(
  app: ProductJourneyAppController,
  page: Page,
  materialPath: string
): Promise<void> {
  const materialName = basename(materialPath);
  const nextCount = (await countProjectSessionIntent(app, "addTimelineSegmentIntent")) + 1;
  const materialRow = page.getByRole("article", { name: `素材 ${materialName}` });
  await expect(materialRow).toBeVisible({ timeout: 10_000 });
  const addButton = materialRow.getByRole("button", { name: `添加 ${materialName} 到时间线` });
  await expect(addButton).toBeEnabled({ timeout: 60_000 });
  await addButton.click();
  await waitForProjectSessionIntentCount(app, "addTimelineSegmentIntent", nextCount);
  await waitForProjectSessionIntentSuccess(app, "addTimelineSegmentIntent", nextCount);
}
```

```typescript
export async function seekTimelinePlayhead(page: Page, app: ProductJourneyAppController, targetTimeUs: number): Promise<void> {
  const frameRequestsBefore = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  const projectCallsBefore = (await readProjectSessionCalls(app)).length;
  await clickTimelineRulerAt(page, targetTimeUs);
  await waitForPlayheadScrubToSettle(app, projectCallsBefore);
  await expect
    .poll(async () => parseTimecodeToMicroseconds((await page.getByLabel("当前时间码").textContent()) ?? ""), {
      timeout: 10_000
    })
    .toBeGreaterThanOrEqual(targetTimeUs - TIMELINE_RULER_CLICK_TOLERANCE_US);
  expect(
    requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app)),
    "product seek must not fall back to preview artifact frame requests"
  ).toBe(frameRequestsBefore);
}
```

**Scheduler telemetry reader pattern** (lines 1334-1379):
```typescript
export async function readTaskRuntimeTelemetry(page: Page): Promise<TaskRuntimeTelemetryResponse> {
  const result = await page.evaluate(async () => {
    const api = (window as typeof window & {
      videoEditorCore?: {
        getTaskRuntimeTelemetry: () => Promise<CommandResultEnvelope<TaskRuntimeTelemetryResult>>;
      };
    }).videoEditorCore;
    return api?.getTaskRuntimeTelemetry();
  });

  expect(result?.ok, `getTaskRuntimeTelemetry failed: ${JSON.stringify(result?.error ?? null)}`).toBe(true);
  expect(result?.data, "getTaskRuntimeTelemetry must return scheduler telemetry data").not.toBeNull();
  return result.data as TaskRuntimeTelemetryResponse;
}

export function requestProjectSessionPreviewFrameCount(calls: NativeCommandObservation[]): number {
  return calls.filter((call) => call.command === "requestProjectSessionPreviewFrame").length;
}
```

**Apply to Phase 20:** Reuse existing exported helpers wherever possible. Add only missing long-timeline-specific helpers for selection/scroll/zoom/move/trim/split/undo/redo if current helpers do not expose them. Every helper should wait for project-session intent counts/results, use visible UI roles/selectors, and preserve no-preview-artifact assertions.

---

### `apps/desktop-electron/tests/helpers/realWorkflow.ts` (utility, file-I/O/request-response)

**Analog:** `apps/desktop-electron/tests/helpers/realWorkflow.ts`

**Workflow structure and no fallback command checks** (lines 63-116):
```typescript
export async function runRealImportPreviewExportWorkflow(
  app: ElectronApplication,
  page: Page,
  fixtures: Phase6MediaFixtures
): Promise<RealWorkflowResult> {
  await enterProjectFromProductEntryIfNeeded(page, app);
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();

  await importMaterials(page, app, [
    { name: fixtures.videoName },
    { name: fixtures.imageName },
    { name: fixtures.audioName }
  ]);

  await verifyRealtimePreviewPlayback(page, app);
  await exportDraft(page, app, fixtures);

  const calls = await readNativeCommandObservations(app);
  const projectCalls = await readProjectSessionCalls(app);
  expect(calls.filter((call) => call.command === "requestProjectSessionPreviewFrame")).toHaveLength(0);
  expect(calls.filter((call) => call.command === "requestProjectSessionPreviewSegment")).toHaveLength(0);

  return {
    calls,
    realtimePreviewHostCalls: await readRealtimePreviewHostCalls(app),
    outputPath: fixtures.outputPath
  };
}
```

**Export modal pattern** (lines 329-396):
```typescript
async function exportDraft(
  page: Page,
  app: ElectronApplication,
  fixtures: Phase6MediaFixtures
): Promise<void> {
  const nextStartCount = (await countCommand(app, "startExport")) + 1;
  const outputPath = fixtures.outputPath;
  await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("输出路径").fill(outputPath);
  await expect(dialog.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await dialog.getByRole("button", { name: "开始导出" }).click();
  await waitForCommandCount(page, app, "startExport", nextStartCount);
  // poll status, fail with progress/log/validation/native command details
  await expect(dialog.getByLabel("导出进度")).toContainText("已完成", { timeout: 5_000 });
  await expect(dialog.getByLabel("输出校验")).toContainText(fixtures.expectedResolutionLabel);
  await expect(dialog.getByLabel("输出校验")).toContainText("含音频");
  await expectFileExists(outputPath);
  await expectExportMedia(outputPath, fixtures, page);
}
```

**Apply to Phase 20:** Reuse this export modal/probe structure but expand evidence: two exports, bundled `ffprobe`, sampled semantic frames around start/middle/tail/edit points, and failure messages containing product summary plus telemetry/native/export details. Keep file-exists checks as a prerequisite, not the success condition.

---

### `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts` (test, event-driven/request-response/file-I/O)

**Analogs:** `apps/desktop-electron/tests/real-workflow.spec.ts`, `apps/desktop-electron/tests/product-scheduler-stress.spec.ts`

**Packaged open/reopen pattern** (real-workflow.spec.ts lines 39-63):
```typescript
test("packaged no-mock import-preview-export workflow", async () => {
  const fixtures = await generatePhase6MediaFixtures();
  const { app, page } = await launchPackagedApp({
    ...REAL_RUNTIME_TEST_ENV,
    VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: fixtures.bundlePath,
    VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([fixtures.videoPath, fixtures.imagePath, fixtures.audioPath])
  });

  try {
    await runRealImportPreviewExportWorkflow(app, page, fixtures);
  } finally {
    await app.close();
  }

  const reopened = await launchPackagedApp({
    ...REAL_RUNTIME_TEST_ENV,
    VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([]),
    VIDEO_EDITOR_TEST_OPEN_PROJECT_BUNDLE: fixtures.bundlePath
  });
  try {
    await assertReopenedProjectState(reopened.page, fixtures);
  } finally {
    await reopened.app.close();
  }
});
```

**Packaged launch helper pattern** (packagedApp.ts lines 13-22, 25-38):
```typescript
export async function launchPackagedApp(env: NodeJS.ProcessEnv = {}): Promise<PackagedAppLaunch> {
  const executablePath = await findPackagedExecutable();
  const poisonPath = await createPoisonRuntimePath();
  const app = await electron.launch({
    executablePath,
    env: sanitizedPackagedEnv(poisonPath, env)
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page, executablePath };
}

function sanitizedPackagedEnv(poisonPath: string, overrides: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  return {
    HOME: process.env.HOME,
    PATH: poisonPath,
    ...overrides
  };
}
```

**Scheduler/product pressure pattern** (product-scheduler-stress.spec.ts lines 66-156):
```typescript
const telemetryAfterPressure = await waitForSchedulerTelemetryProgress(page, telemetryBeforePressure);
const editStartedAt = Date.now();
await selectTimelineSegment(page, USER_JOURNEY_LONG_MOVING_VIDEO);
await updateSelectedVisualThroughInspector(page, app, {
  positionX: 96,
  positionY: -48,
  scaleX: 1180,
  scaleY: 1180,
  rotation: 6,
  opacity: 900,
  fitMode: "填充"
});
const inspectorEditMs = Date.now() - editStartedAt;

expect(renderGraphGpuComposited, "stress playback must use renderGraphGpuComposited product evidence").toBe(true);
expect(fallbackActive, "stress playback must not report fallbackActive").toBe(false);
expect(previewAfterPressure.hostState?.backend, "stress playback backend must remain renderGraphGpu").toBe("renderGraphGpu");
expect(frameRequestsAfterPressure, "scheduler stress must not use requestProjectSessionPreviewFrame artifact fallback").toBe(
  frameRequestsBeforePlay
);
expect(inspectorEditMs, "inspector edit command must stay responsive under scheduler pressure").toBeLessThanOrEqual(2_500);
expect(queueLatencyUs.p95 ?? 0, "queue latency p95 should remain bounded for product stress").toBeLessThanOrEqual(2_000_000);
expect(telemetryAfterPressure.rejectedCount, "stress workflow should not reject normal product work").toBe(0);
expect(telemetryAfterPressure.fallbackCount, "stress workflow must not use fallback scheduler success").toBe(0);
```

**Apply to Phase 20:** The spec should generate/open the Rust-owned long `.veproj`, execute selection, scroll/zoom, scrub/play, move, trim, split, undo/redo, inspector visual edit, save/reopen twice, and export twice. Use packaged Electron as the blocking test. Dev/diagnostic variants may exist, but the phase gate must run packaged evidence.

---

### `scripts/phase20-source-guards.sh` (utility/guard script, batch/source scan)

**Analog:** `scripts/phase19-source-guards.sh`

**Script helper pattern** (lines 4-20, 22-39):
```bash
fail() {
  echo "phase19 source guard violation: $1" >&2
  exit 1
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required Phase 19 artifact ${file}"
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
}
```

**Negative injection self-test pattern** (lines 65-82):
```bash
assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase19Violation.tsx"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase19Violation.tsx" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "$source" | sed 's|^|// |' >"$tmp_dir/CommentOnly.tsx"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.tsx" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}
```

**Wave/full mode pattern** (lines 190-230, 475-601):
```bash
require_wave0_files() {
  require_file "scripts/phase19-source-guards.sh"
  require_file "package.json"
  require_fixed "package.json" "\"test:phase19-rust\""
  require_fixed "package.json" "\"test:phase19-source-guards\""
  require_fixed "package.json" "\"test:phase19-desktop\""
  require_fixed "package.json" "\"test:phase19\""
  require_fixed "package.json" "bash scripts/phase19-source-guards.sh"
}

scan_no_fallback_success() {
  fail_matches \
    "product code/tests must not count DOM, artifact, CPU, mock, debug, fallback, or legacy output as Phase 19 success" \
    "$FALLBACK_SUCCESS_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}" \
    "${PRODUCT_TEST_DIRS[@]}"
  bash scripts/no-product-fallback-guards.sh >/dev/null
}
```

**Apply to Phase 20:** Require `product-long-timeline-uat.spec.ts`, `longTimelineFixture.ts`, `longTimelineEvidence.ts`, package scripts, Rust large-timeline gate text, and `bash scripts/no-product-fallback-guards.sh`. Add rejection patterns for UI-built 540 segment loops, file-exists-only export success, fallback/mock/artifact/CPU/DOM/native-video/first-frame success, direct FFmpeg construction outside compiler/runtime, and TypeScript-generated canonical long draft semantics.

---

### `scripts/no-product-fallback-guards.sh` (utility/guard script, batch/source scan)

**Analog:** `scripts/no-product-fallback-guards.sh`

**Fail-if-match pattern** (lines 4-13):
```bash
fail_if_matches() {
  local label="$1"
  local pattern="$2"
  shift 2

  if rg -n "$pattern" "$@"; then
    echo "no-product-fallback violation: ${label}" >&2
    exit 1
  fi
}
```

**Required scheduler evidence pattern** (lines 88-108):
```bash
SCHEDULER_STRESS_SPEC="apps/desktop-electron/tests/product-scheduler-stress.spec.ts"
if [ -f "$SCHEDULER_STRESS_SPEC" ]; then
  fail_if_matches \
    "Product scheduler stress success must not be satisfied by test runtime/export/artifact/audio mocks" \
    'VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS|VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS|VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS|VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES:\s*"1"|mockSchedulerSuccess|artifactSchedulerSuccess|cpuProbeSchedulerSuccess|domOnlySchedulerSuccess' \
    "$SCHEDULER_STRESS_SPEC"

  for required in \
    'renderGraphGpuComposited' \
    'captureVisiblePreviewEvidence' \
    'requestProjectSessionPreviewFrameCount' \
    'getTaskRuntimeTelemetry' \
    'queueLatencyUs' \
    'resourceSaturationCount' \
    'fallbackActive' \
    'visibleCenterHash'; do
    if ! rg -q "$required" "$SCHEDULER_STRESS_SPEC"; then
      echo "no-product-fallback violation: scheduler stress success must require ${required}" >&2
      exit 1
    fi
  done
fi
```

**Apply to Phase 20:** Extend this script or call Phase 20 guard from it so `product-long-timeline-uat.spec.ts` must contain `launchPackagedApp`, `renderGraphGpuComposited`, `captureVisiblePreviewEvidence`, `requestProjectSessionPreviewFrameCount`, `getTaskRuntimeTelemetry`, `queueLatencyUs`, `fallbackCount`, `ffprobe`, sampled frame evidence, and two reopen/export cycles. Fail if the long UAT uses mock runtime capability success or file-exists-only export assertions.

---

### `package.json` (config, batch orchestration)

**Analog:** `package.json`

**Phase aggregate script pattern** (lines 84-87, 104-107):
```json
"test:phase16-rust": "cargo test -p task_runtime -- --nocapture && cargo test -p bindings_node --test scheduler_preview_audio -- --nocapture && cargo test -p bindings_node --test scheduler_export -- --nocapture && cargo test -p bindings_node --test scheduler_artifact_probe -- --nocapture && cargo test -p bindings_node --test scheduler_runtime -- --nocapture",
"test:phase16-source-guards": "bash scripts/phase16-source-guards.sh",
"test:phase16-desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-scheduler-stress.spec.ts --reporter=line",
"test:phase16": "pnpm run test:phase16-rust && pnpm run test:phase16-source-guards && pnpm run test:no-product-fallback && pnpm --filter @video-editor/desktop test:runtime-diagnostics && pnpm run test:phase16-desktop && pnpm run test:contracts",

"test:phase19-rust": "cargo test -p draft_model production_effects_contracts -- --nocapture && cargo test -p draft_commands retiming_commands -- --nocapture && cargo test -p draft_commands transition_commands -- --nocapture && cargo test -p engine_core retiming -- --nocapture && cargo test -p audio_engine dsp_timeline -- --nocapture && cargo test -p render_graph production_effects -- --nocapture && cargo test -p realtime_preview_runtime production_effects -- --nocapture && cargo test -p ffmpeg_compiler production_effects -- --nocapture && cargo test -p testkit production_effects -- --nocapture",
"test:phase19-source-guards": "bash scripts/phase19-source-guards.sh",
"test:phase19-desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/production-effects.spec.ts tests/ui-regression.spec.ts --reporter=line --workers=1 && pnpm --filter @video-editor/desktop exec playwright test tests/workspace.spec.ts --grep \"Phase 11 runtime boundary docs\" --reporter=line --workers=1",
"test:phase19": "pnpm run test:phase19-source-guards && pnpm run test:no-product-fallback && pnpm run test:phase19-rust && pnpm run test:phase19-desktop && cargo check --workspace --locked && pnpm run test:contracts"
```

**Desktop package script pattern** (apps/desktop-electron/package.json lines 15-23):
```json
"package:dir": "pnpm run clean:build && pnpm run build && electron-builder --dir --config electron-builder.yml --publish=never",
"test:packaged-real-workflow": "pnpm run package:dir && playwright test tests/real-workflow.spec.ts --grep \"packaged\"",
"test:product-user-journey": "pnpm run build && playwright test tests/product-user-journey.spec.ts",
"test:real-workflow": "pnpm run build && playwright test tests/real-workflow.spec.ts --grep \"dev\"",
"test:runtime-diagnostics": "pnpm run build && playwright test tests/runtime-diagnostics.spec.ts"
```

**Apply to Phase 20:** Add root scripts, likely:

```json
"test:phase20-rust": "cargo test -p testkit large_timeline_incremental -- --nocapture && cargo test -p testkit long_timeline_product_fixture -- --nocapture",
"test:phase20-source-guards": "bash scripts/phase20-source-guards.sh",
"test:phase20-desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts --reporter=line --workers=1",
"test:phase20": "pnpm run test:phase20-rust && pnpm run test:phase20-source-guards && pnpm run test:no-product-fallback && pnpm run test:phase20-desktop && cargo check --workspace --locked && pnpm run test:contracts"
```

Keep the `3000 segments/track` pressure run outside the blocking aggregate unless the final plan explicitly marks it non-blocking and isolates failures.

## Shared Patterns

### Rust-Owned Canonical Bundle

**Sources:** `crates/project_store/src/bundle.rs`, `crates/project_store/src/paths.rs`
**Apply to:** Rust fixture materialization and save/reopen evidence.

**Canonical save/open pattern** (bundle.rs lines 30-57, 75-104):
```rust
pub fn save_project_bundle(
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
) -> Result<ProjectBundle, ProjectStoreError> {
    let bundle_path = bundle_path.as_ref();
    let project_json_path = project_json_path(bundle_path);

    validate_draft(draft).map_err(|source| semantic_error(&project_json_path, source))?;
    validate_material_uris(bundle_path, draft)?;
    let contents = serde_json::to_string_pretty(draft).map_err(|error| {
        ProjectStoreError::InvalidProjectJson {
            path: project_json_path.clone(),
            message: error.to_string(),
        }
    })?;
    fs.write_string(&project_json_path, &format!("{contents}\n"))?;
    Ok(ProjectBundle { bundle_path: bundle_path.to_path_buf(), project_json_path, draft: draft.clone() })
}
```

```rust
pub fn open_project_bundle(
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
) -> Result<ProjectBundleOpenResult, ProjectStoreError> {
    let project_json_path = project_json_path(bundle_path);
    let contents = fs.read_to_string(&project_json_path)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;
    let draft = migrate_draft_json(value)
        .map_err(|source| draft_validation_error(&project_json_path, source))?;
    let warnings = collect_warnings(fs, bundle_path, &draft)?;
    Ok(ProjectBundleOpenResult { bundle: ProjectBundle { bundle_path: bundle_path.to_path_buf(), project_json_path, draft }, warnings })
}
```

**Material URI validation pattern** (paths.rs lines 25-57, 88-107):
```rust
pub fn classify_material_uri(
    bundle_path: impl AsRef<Path>,
    uri: &str,
) -> Result<MaterialUri, ProjectStoreError> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        return invalid_uri(uri, "URI must not be empty");
    }
    let path = Path::new(trimmed);
    if is_absolute_material_path(trimmed, path) { /* external absolute */ }
    if has_uri_scheme(trimmed) { /* external URI */ }
    validate_bundle_relative_path(trimmed)?;
    Ok(MaterialUri {
        kind: MaterialUriKind::InBundleRelative,
        uri: trimmed.to_owned(),
        resolved_path: Some(bundle_path.as_ref().join(path)),
    })
}
```

### Product Preview Evidence

**Sources:** `apps/desktop-electron/tests/helpers/userJourney.ts`, `apps/desktop-electron/tests/product-scheduler-stress.spec.ts`
**Apply to:** Every Phase 20 preview/scrub/play assertion.

**Product playback helper pattern** (userJourney.ts lines 316-344, 346-436):
```typescript
export async function waitForCompositedPreviewEvidence(
  page: Page,
  app?: ProductJourneyAppController,
  timeoutMs = 8_000,
  afterTargetTimeUs = -1
): Promise<PreviewEvidence> {
  const deadline = Date.now() + timeoutMs;
  let lastEvidence: PreviewEvidence | null = null;

  while (Date.now() < deadline) {
    lastEvidence = await capturePreviewEvidence(page);
    const evidence = lastEvidence.hostState?.contentEvidence;
    if (
      evidence?.source === "renderGraphGpuComposited" &&
      evidence.targetTimeMicroseconds > afterTargetTimeUs
    ) {
      return lastEvidence;
    }
    await page.waitForTimeout(250);
  }
  throw new Error(`Timed out waiting for composited preview evidence...`);
}
```

```typescript
expect(after.hostState?.ok, "product playback requires an ok realtime host state").toBe(true);
expect(after.hostState?.productReady, "product playback requires product-ready realtime preview").toBe(true);
expect(after.hostState?.fallbackActive, "product playback must not be a fallback path").toBe(false);
expect(after.hostState?.backend, "product playback success backend must be renderGraphGpu").toBe("renderGraphGpu");
expect(after.hostState?.diagnosticSource, "product playback success must not come from diagnostic sources").toBe("none");
expect(after.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
expect(visibleMotion.visibleCenterHash).not.toBe(visibleBefore.visibleCenterHash);
expect(frameRequestsAfterPlay).toBe(frameRequestsBeforePlay);
expect(after.hostState?.frameDisplay).toBeNull();
```

### Scheduler Telemetry

**Sources:** `apps/desktop-electron/src/main/nativeBinding.ts`, `crates/realtime_preview_runtime/src/telemetry.rs`, `apps/desktop-electron/tests/helpers/userJourney.ts`
**Apply to:** Long UAT pressure and responsiveness budgets.

**Native binding telemetry response shape** (nativeBinding.ts lines 247-276):
```typescript
export type TaskRuntimeTelemetryResponse = {
  status: "ready" | "degraded" | "unavailable";
  submittedCount: number;
  completedCount: number;
  rejectedCount: number;
  staleRejectedCount: number;
  fallbackCount: number;
  resourceSaturationCount: number;
  queueLatencyUs: TaskRuntimeTelemetrySummary;
  waitTimeUs: TaskRuntimeTelemetrySummary;
  runTimeUs: TaskRuntimeTelemetrySummary;
  jobDurationUs: TaskRuntimeTelemetrySummary;
};
```

**Realtime scheduler snapshot propagation** (telemetry.rs lines 135-143):
```rust
pub fn record_scheduler_snapshot(&mut self, snapshot: &SchedulerTelemetrySnapshot) {
    self.scheduler_queue_latency_p95_us = snapshot.queue_latency_us.p95;
    self.scheduler_queue_depth = snapshot.current_queue_depth;
    self.scheduler_resource_saturation_count = snapshot.resource_saturation_count;
    self.scheduler_rejected_count = snapshot.rejected_count;
    self.scheduler_canceled_count = snapshot.canceled_count;
    self.scheduler_stale_rejected_count = snapshot.stale_rejected_count;
    self.scheduler_snapshot = Some(snapshot.clone());
}
```

### Source Guard And Package Script Gates

**Sources:** `scripts/phase19-source-guards.sh`, `scripts/no-product-fallback-guards.sh`, `package.json`
**Apply to:** Phase 20 guard and aggregate wiring.

Required guard patterns:
- `require_file` and `require_fixed` for expected artifacts and scripts.
- `assert_pattern_rejects` self-tests for each forbidden success pattern.
- Comment-stripped `rg --pcre2` scans.
- `bash scripts/no-product-fallback-guards.sh` chained into Phase 20 guard and root aggregate.
- Packaged desktop command must include `pnpm --filter @video-editor/desktop package:dir` before Playwright.

## No Analog Found

No Phase 20 target lacks an analog. The weakest single-file analog is `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts`, which should combine patterns from `realWorkflow.ts`, `userJourney.ts`, and `project_store` tests instead of copying one file wholesale.

## Metadata

**Analog search scope:** `crates/testkit`, `crates/project_store`, `crates/realtime_preview_runtime`, `apps/desktop-electron/tests`, `apps/desktop-electron/src/main`, `scripts`, `package.json`.
**Files scanned/read:** 23 files and manifests/scripts.
**Pattern extraction date:** 2026-06-28.

