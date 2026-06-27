---
phase: 17
slug: template-import-core-and-kaipai-offline-adapter-foundation
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-24
---

# Phase 17 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Offline Kaipai bundle to adapter | Strict, sanitized JSON/resource evidence enters only `adapter_kaipai`. | Provider evidence, local file refs, report provenance |
| Adapter to draft import | Adapter emits provider-neutral `DraftImportPlan` plus `AdaptationReport`; it does not mutate project/session state. | Canonical materials/tracks/segments and bounded report evidence |
| Resource localizer to `.veproj` bundle | Renderable resources are copied under `resources/template-import/...` and indexed as project resources. | Local files, sha256 evidence, bundle-relative refs |
| Project session to canonical draft | Rust project session applies validated import plans and persists `.veproj/project.json`. | Draft replacement, revision, view model, resource index refs |
| Renderer UI to Rust import API | Desktop UI sends only a narrow import request and displays bounded report copy. | Session id, expected revision, local paths, sanitized report summary |
| Preview/export evidence | Product success must come from realtime preview/export gates, not fallback artifacts or provider runtimes. | Render graph work, FFmpeg export jobs, generated test media |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-17-01 | Tampering | `AdaptationReport` taxonomy | mitigate | Required status enum, report snapshots, and mapper/export/product tests prevent unsupported/native effects from being hidden as support. | closed |
| T-17-02 | Information Disclosure | Report provenance fields | mitigate | External references stay in bounded provenance; fixture/report scanners and UI E2E reject secret/provider leakage. | closed |
| T-17-03 | Tampering | Core/render/session source boundaries | mitigate | `scripts/phase17-source-guards.sh` blocks provider/raw formula/live API/Android dependencies outside adapter/import boundary. | closed |
| T-17-04 | Repudiation | Product success evidence | mitigate | Phase 17 and no-product-fallback guards reject mock/artifact/CPU/Android evidence as preview/export success. | closed |
| T-17-SC | Tampering | Package installs and dependency supply chain | mitigate | No unreviewed package install occurred; `sha2` use follows the recorded package legitimacy audit and Cargo lockfile flow. | closed |
| T-17-05 | Tampering | Resource path canonicalization | mitigate | Localizer tests reject traversal, absolute destinations, symlink escapes, duplicate destinations, and unsafe refs. | closed |
| T-17-06 | Information Disclosure | Remote URLs and credentials | mitigate | Localizer/parser reject remote runtime refs; fixture scanners reject credential-like fields and signed URL shapes. | closed |
| T-17-07 | Tampering | Resource integrity | mitigate | `sha2::Sha256` validation reports mismatches before resources enter renderable import plans. | closed |
| T-17-08 | Denial of Service | Resource copy set | mitigate | Fixture sets are bounded and invalid entries fail/report before draft/session mutation. | closed |
| T-17-09 | Tampering | `DraftImportPlan` validation | mitigate | Import plan validation rejects invalid material refs, timeranges, track ordering, and raw provider semantics before session mutation. | closed |
| T-17-10 | Information Disclosure | Plan fields | mitigate | Raw formula/provenance evidence is excluded from canonical plan fields and persisted project JSON. | closed |
| T-17-11 | Tampering | Remote runtime refs | mitigate | Import-plan and source guards reject URL-style runtime refs in canonical draft semantics. | closed |
| T-17-12 | Repudiation | Unsupported feature status | mitigate | Adaptation reports require explicit approximated/dropped/missing/native-effect statuses for unsupported behavior. | closed |
| T-17-13 | Tampering | Bundle parser | mitigate | Strict serde contracts, schema tests, and unsafe evidence rejection guard offline bundle parsing. | closed |
| T-17-14 | Information Disclosure | Fixture corpus | mitigate | Sanitized fixture scans reject credentials, account IDs, cookies, signed URLs, and remote provider evidence. | closed |
| T-17-15 | Tampering | Old branch reuse | mitigate | Old branch was inspected as evidence only; current-main integration was rewritten and source guards block legacy leakage. | closed |
| T-17-16 | Repudiation | Native effects in input | mitigate | Parser preserves native effect evidence only for report classification; mapper reports `needsNativeEffect`/dropped. | closed |
| T-17-17 | Tampering | Fixture/report supported subset | mitigate | Explicit fixture/report catalog and mixed-template regression cover supported subset mapping. | closed |
| T-17-18 | Repudiation | Approximation reporting | mitigate | Report snapshots cover supported, approximated, dropped, missingResource, and needsNativeEffect cases. | closed |
| T-17-19 | Tampering | Resource refs in mapped materials | mitigate | Mapper consumes only localizer-available project-relative refs and reports/drops missing or unsafe resources. | closed |
| T-17-20 | Information Disclosure | Provider provenance | mitigate | External refs remain report evidence; source guards block raw formula leakage into core/render/session/export paths. | closed |
| T-17-21 | Tampering | Project session revision | mitigate | `importKaipaiFormulaBundle` requires `expectedRevision` and rejects stale imports before applying or saving. | closed |
| T-17-22 | Tampering | Atomic project write and resource index update | mitigate | Project-session tests cover rollback on stale/mapping/index failures, including cleanup of copied resources on index failure. | closed |
| T-17-23 | Information Disclosure | Project JSON | mitigate | Tests inspect saved `.veproj/project.json` for raw formula, provider IDs, remote runtime refs, and URLs. | closed |
| T-17-24 | Repudiation | Import response | mitigate | Import response returns `AdaptationReport` with the updated view model. | closed |
| T-17-25 | Tampering | Export transform compiler | mitigate | Transform snapshots and parity tests verify rotation angle, center anchor, full-canvas rotation, and layer order. | closed |
| T-17-26 | Repudiation | Rotation support classification | mitigate | Diagnostics distinguish supported static center-anchor rotation from unsupported animated/non-center cases. | closed |
| T-17-27 | Tampering | Provider-specific workaround | mitigate | Rotation support is implemented generically in render/FFmpeg paths; source guards block adapter-specific render semantics. | closed |
| T-17-28 | Information Disclosure | Export artifacts | accept | Test artifacts stay in temp/test-results paths and contain generated fixture media only. | closed |
| T-17-29 | Repudiation | Preview/export evidence | mitigate | Fixture export/preview tests require visible/text/audio/layer media assertions plus no-fallback evidence. | closed |
| T-17-30 | Tampering | Imported project JSON | mitigate | Source guards and tests inspect persisted project JSON for remote/provider runtime semantics. | closed |
| T-17-31 | Denial of Service | Fixture export jobs | mitigate | Testkit runtime timeouts and bundled FFmpeg runtime checks bound fixture export jobs. | closed |
| T-17-32 | Information Disclosure | Test artifacts | mitigate | Generated/sanitized repository-owned fixtures are used and scanned for secret-like data. | closed |
| T-17-33 | Tampering | Renderer import request | mitigate | Renderer sends only path/session/revision request; Rust validates and owns mutation. | closed |
| T-17-34 | Information Disclosure | Report panel | mitigate | Product UI displays bounded report copy and hides raw formula JSON, secrets, signed URLs, account IDs, and provenance internals. | closed |
| T-17-35 | Repudiation | UI import success | mitigate | Packaged Electron E2E requires report, timeline, preview/export, clean saved project, and no-fallback assertions. | closed |
| T-17-36 | Tampering | Stale session UI mutation | mitigate | UI passes current project-session revision; Rust stale-revision gate is authoritative. | closed |
| T-17-37 | Tampering | Mapper supported subset | mitigate | Every mapper output validates through `DraftImportPlan` and `draft_model` validation. | closed |
| T-17-38 | Repudiation | Approximation reporting | mitigate | Mapper and fixture export tests assert required report statuses against snapshots, including native-effect degradation. | closed |
| T-17-39 | Tampering | Resource refs in mapped materials | mitigate | Mapper consumes localizer results only and reports/drops resources that cannot be localized. | closed |
| T-17-40 | Information Disclosure | Provider provenance | mitigate | Provider references remain bounded report evidence and are blocked from canonical draft/render/session semantics. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-17-01 | T-17-28 | Export test artifacts contain generated fixture media only and are written to temp/test-results paths, not committed product/runtime state. | Phase 17 plan | 2026-06-24 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-24 | 41 | 41 | 0 | Codex |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-24
