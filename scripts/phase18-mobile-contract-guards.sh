#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase18 mobile runtime contract violation: $1" >&2
  exit 1
}

CONTRACT_DOC="${PHASE18_MOBILE_CONTRACT_DOC:-docs/mobile-runtime-contracts.md}"
SMOKE_TEST="${PHASE18_MOBILE_CONTRACT_SMOKE_TEST:-crates/bindings_c/tests/mobile_contract_handles.rs}"
SMOKE_TEST_NAME="mobile_contract_handles_validate_owner_generation_and_release"

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required mobile runtime contract artifact ${file}"
}

require_regex() {
  local file="$1"
  local pattern="$2"
  local message="$3"
  if ! rg -n --pcre2 -i "$pattern" "$file" >/dev/null; then
    fail "$message in ${file}"
  fi
}

validate_contract_doc() {
  local contract_doc="${PHASE18_MOBILE_CONTRACT_DOC:-$CONTRACT_DOC}"
  require_file "$contract_doc"
  require_regex "$contract_doc" 'mobile runtime contract' "missing mobile runtime contract marker"
  require_regex "$contract_doc" 'Android|JNI' "missing Android JNI lifecycle coverage"
  require_regex "$contract_doc" 'background' "missing background lifecycle coverage"
  require_regex "$contract_doc" 'foreground' "missing foreground lifecycle coverage"
  require_regex "$contract_doc" 'Swift|ObjC|Objective-C|C import' "missing Swift/ObjC C import ownership coverage"
  require_regex "$contract_doc" 'sandbox|security-scoped|scoped storage' "missing sandboxed file access coverage"
  require_regex "$contract_doc" 'permission invalidation|permission revocation|revoked permission|stale permission' "missing sandbox permission invalidation coverage"
  require_regex "$contract_doc" 'file handle' "missing file handle ownership coverage"
  require_regex "$contract_doc" 'texture' "missing texture handle coverage"
  require_regex "$contract_doc" 'device' "missing texture/device identity coverage"
  require_regex "$contract_doc" 'cancel|cancellation' "missing cancellation coverage"
  require_regex "$contract_doc" 'explicit release|release explicitly|must release' "missing explicit release coverage"
  require_regex "$contract_doc" 'cascad|session close|close.*session' "missing cascading session close coverage"
}

validate_smoke_test() {
  local smoke_test="${PHASE18_MOBILE_CONTRACT_SMOKE_TEST:-$SMOKE_TEST}"
  require_file "$smoke_test"
  require_regex "$smoke_test" "$SMOKE_TEST_NAME" "missing bindings_c mobile handle smoke test ${SMOKE_TEST_NAME}"
  require_regex "$smoke_test" 'owner|generation|release|device' "mobile handle smoke test must cover owner, generation, release, and device facts"
}

validate_full() {
  validate_contract_doc
  validate_smoke_test
  echo "phase18 mobile runtime contract guards passed"
}

validate_smoke_only() {
  validate_smoke_test
  echo "phase18 mobile runtime contract smoke guard passed"
}

run_self_test() {
  local tmp_dir good_doc bad_doc good_test bad_test
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  good_doc="$tmp_dir/mobile-runtime-contracts.md"
  bad_doc="$tmp_dir/mobile-runtime-contracts-missing-permission.md"
  good_test="$tmp_dir/mobile_contract_handles.rs"
  bad_test="$tmp_dir/mobile_contract_handles_missing.rs"

  cat >"$good_doc" <<'DOC'
# Mobile Runtime Contracts

This is the mobile runtime contract for future adapters.

## Android JNI Lifecycle
Android JNI callers must attach on valid threads and handle background and foreground transitions.

## Swift/ObjC C Import Ownership
Swift and ObjC callers import the C ABI and must keep ownership explicit.

## Sandboxed File Handles
Sandboxed file handle permissions can have permission invalidation or permission revocation.

## Texture And Device Handles
Texture handles carry device identity and owner generation facts.

## Cancellation And Release
Cancellation must fail closed, and callers must release explicitly.

## Cascading Session Close
Cascading session close releases resources and reports leaks.
DOC
  sed '/permission invalidation/d;/permission revocation/d' "$good_doc" >"$bad_doc"
  cat >"$good_test" <<'RUST'
#[test]
fn mobile_contract_handles_validate_owner_generation_and_release() {
    let owner_generation_device_release = "owner generation device release";
    assert!(owner_generation_device_release.contains("release"));
}
RUST
  cat >"$bad_test" <<'RUST'
#[test]
fn unrelated_mobile_test() {
    assert!(true);
}
RUST

  PHASE18_MOBILE_CONTRACT_DOC="$good_doc" \
    PHASE18_MOBILE_CONTRACT_SMOKE_TEST="$good_test" \
    validate_full >/dev/null

  if (
    PHASE18_MOBILE_CONTRACT_DOC="$bad_doc" \
      PHASE18_MOBILE_CONTRACT_SMOKE_TEST="$good_test" \
      validate_full >/dev/null 2>&1
  ); then
    fail "self-test accepted a mobile contract missing sandbox permission invalidation"
  fi

  if (
    PHASE18_MOBILE_CONTRACT_DOC="$good_doc" \
      PHASE18_MOBILE_CONTRACT_SMOKE_TEST="$bad_test" \
      validate_smoke_only >/dev/null 2>&1
  ); then
    fail "self-test accepted a missing bindings_c mobile handle smoke test"
  fi

  echo "phase18 mobile runtime contract self-test passed"
  rm -rf "$tmp_dir"
  trap - RETURN
}

if [ "${1:-}" = "--" ]; then
  shift
fi

case "${1:-}" in
  --self-test)
    run_self_test
    ;;
  --smoke-only)
    validate_smoke_only
    ;;
  -h|--help)
    echo "Usage: bash scripts/phase18-mobile-contract-guards.sh [--self-test|--smoke-only]"
    ;;
  *)
    validate_full
    ;;
esac
