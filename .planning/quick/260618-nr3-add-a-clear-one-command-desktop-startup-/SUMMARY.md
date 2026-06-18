---
status: complete
completed: 2026-06-18T09:06:15Z
---

# Summary

Added `desktop:open` as a root package script alias for the existing desktop startup path and documented `corepack pnpm run desktop:open` as the recommended one-command launcher.

Verification:

- `node -e "JSON.parse(require('fs').readFileSync('package.json','utf8')); console.log('package ok')"`
- `pnpm run --silent | rg 'desktop:open|desktop'`

