# Deferred Items

## 19-14: Existing crop export limitation in reused template fixture

- **Found during:** Task 3 desktop template import product E2E.
- **Issue:** The base `fixtures/kaipai/positive/main-video.json` crop settings can produce an invalid FFmpeg crop against the small desktop test media (`Invalid too big or non positive size`) before Phase 19 retime/transition/filter export evidence is evaluated.
- **Scope decision:** Out of scope for 19-14 because the plan target was template fidelity for retime, transition, filter, report boundaries, and no provider ID leakage. The Task 3 fixture variant removes the unrelated crop so the planned product gate remains focused.
- **Suggested follow-up:** Add a focused crop export compiler guard that validates or clamps crop dimensions against decoded source dimensions before FFmpeg runtime execution, then re-enable crop coverage in a dedicated fixture.
