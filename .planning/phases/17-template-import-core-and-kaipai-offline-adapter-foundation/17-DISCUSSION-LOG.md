# Phase 17: Template Import Core And Kaipai Offline Adapter Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-24
**Phase:** 17-template-import-core-and-kaipai-offline-adapter-foundation
**Areas discussed:** Product target, ownership boundary, old branch reuse, import pipeline, supported subset, resource localization, report semantics, implementation order, verification gates

---

## Product Target

| Option | Description | Selected |
|--------|-------------|----------|
| Generic template import/rendering core | Core owns provider-neutral draft/template capabilities; Kaipai is one external adapter. | yes |
| Kaipai-first renderer | Build core around Kaipai formula semantics and reproduce Kaipai behavior directly. | |
| Pixel-perfect compatibility | Treat Kaipai visual parity as a strict reproduction target. | |

**User's choice:** Generic template import/rendering core.
**Notes:** The target is high-quality approximate rendering, not pixel-level Kaipai reproduction.

---

## Ownership Boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Canonical draft only in core | Core consumes `.veproj/project.json`, draft/material/track/segment/keyframe/filter/transition/text/sticker, local resources, render graph, preview, export, and provider-neutral reports. | yes |
| Raw provider data in core | Let raw Kaipai formula/template fields affect core render behavior. | |
| Android worker runtime dependency | Keep Android export as the product renderer. | |

**User's choice:** Canonical draft only in core.
**Notes:** Core must not contain Kaipai API, Android worker, raw formula, provider-specific render semantics, templateId/recipeId as render semantics, or Kaipai-specific `safe_area` logic.

---

## Old Branch Reuse

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve adapter assets, rewrite integration | Keep useful old adapter schemas, fixtures, localizer/report ideas, and tests while rebuilding the integration on current main. | yes |
| Directly merge old branch | Bring old branch code into main largely as-is. | |
| Ignore old branch | Rebuild without using old fixtures or findings. | |

**User's choice:** Preserve adapter assets, rewrite integration.
**Notes:** Old `origin/work/kaipai-adapter-poc` is older than current main and should be treated as asset/reference material, not a current architecture implementation.

---

## Import Pipeline

| Option | Description | Selected |
|--------|-------------|----------|
| Provider-neutral DraftImportPlan | Adapter parses/validates/localizes resources, emits an import plan and report, then project session applies it to canonical draft. | yes |
| Adapter writes draft directly | Adapter constructs or mutates `.veproj/project.json` directly. | |
| UI-side mapper | Electron/React maps formula fields into draft changes. | |

**User's choice:** Provider-neutral `DraftImportPlan`.
**Notes:** The target chain is `KaipaiFormulaBundle -> adapter_kaipai -> resource localizer -> DraftImportPlan -> project_session -> canonical Draft -> preview/export -> AdaptationReport`.

---

## First Supported Subset

| Option | Description | Selected |
|--------|-------------|----------|
| Focused approximate subset | Canvas, main video, PIP, basic stickers, text sticker style, BGM/audio, simple keyframes, fade/dissolve approximations, native-effect reporting. | yes |
| Full Kaipai formula coverage | Try to map every observed formula field in the first version. | |
| Only fixture validation | Stop at parse/report/localization without producing editable draft content. | |

**User's choice:** Focused approximate subset.
**Notes:** The subset must be good enough to render reasonably, preview, edit, export, and explain degradation.

---

## Generic Core Gaps

| Option | Description | Selected |
|--------|-------------|----------|
| Fill generic gaps only | Add `DraftImportPlan`, resource localization, overlay/sticker semantics, font closure, center-anchor rotation, constant speed mapping, and `AdaptationReport`. | yes |
| Add Kaipai-specific semantics | Encode provider-specific fields into core to improve first-version fidelity. | |
| Defer all core changes | Keep adapter as report-only until all downstream effects work exists. | |

**User's choice:** Fill generic gaps only.
**Notes:** The user emphasized that these capabilities should be generic editor/template rendering semantics, not Kaipai-specific patches.

---

## Implementation Order

| Option | Description | Selected |
|--------|-------------|----------|
| Backend-first offline path | Port adapter ideas, implement offline bundle/fixtures/report, localize resources, add import plan/mapper, integrate project session, add fixtures, then UI. | yes |
| UI-first import entry | Add desktop UI import/report surfaces before backend gates. | |
| Live provider first | Implement Kaipai API/provider calls before offline import is stable. | |

**User's choice:** Backend-first offline path.
**Notes:** UI/report panel comes last. First version should not need live Kaipai API or Android runtime.

---

## Verification Gates

| Option | Description | Selected |
|--------|-------------|----------|
| Source guards plus fixture preview/export | Core/render source guards, no raw formula in project, no Android/live API dependency, non-empty MP4 exports, correct layers/text/audio, explicit reports, no fallback success. | yes |
| Schema/report unit tests only | Validate adapter contracts without product preview/export evidence. | |
| Android oracle parity | Use Android output as acceptance evidence for product success. | |

**User's choice:** Source guards plus fixture preview/export.
**Notes:** Supported subset must use the realtime preview product path and export path, not old artifact fallback or Android oracle output.

## the agent's Discretion

- Exact crate/module names are left to planning as long as provider-neutral ownership stays intact.
- The planner may decide whether `AdaptationReport` evolves from old `CompatibilityReport` or becomes a renamed shared import-report contract.
- The planner may split the generic core gaps into multiple Phase 17 plans.

## Deferred Ideas

- Live Kaipai API/provider integration.
- Android worker replacement, ASR-to-word-list, and independent safe-area generation.
- Pixel-perfect proprietary effect parity.
- UI import entry and report panel until backend gates pass.
