#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT
mkdir -p "$TEST_DIR/bin" "$TEST_DIR/workspace" "$TEST_DIR/shared target/package"
export FAKE_TARGET_DIR="$TEST_DIR/shared target"
printf '[workspace]\n' > "$TEST_DIR/workspace/Cargo.toml"

cat > "$TEST_DIR/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
case "$1" in
  metadata)
    jq -n --arg target "$FAKE_TARGET_DIR" --arg name "$FAKE_CRATE_NAME" \
      '{packages:[{name:$name,version:"1.2.3"}],target_directory:$target}'
    ;;
  package)
    if [[ "$FAKE_PACKAGE_MODE" == failure ]]; then
      echo 'error: package generation failed' >&2
      exit 42
    fi
    printf 'fixture bytes\n' > "$FAKE_TARGET_DIR/package/$FAKE_CRATE_NAME-1.2.3.crate"
    ;;
  publish)
    case "$FAKE_PUBLISH_MODE" in
      success) ;;
      immutable) echo 'error: crate version already exists' >&2; exit 1 ;;
      failure) echo 'error: registry unavailable' >&2; exit 1 ;;
      forbidden) echo 'error: matching bytes must skip publication' >&2; exit 1 ;;
    esac
    ;;
  *) echo "unexpected Cargo argument: $1" >&2; exit 64 ;;
esac
CARGO
chmod +x "$TEST_DIR/bin/cargo"

cat > "$TEST_DIR/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
case "${!#}" in
  */"$FAKE_INDEX_PATH") ;;
  *) echo 'incorrect Cargo sparse-index path' >&2; exit 64 ;;
esac
if [[ "$FAKE_INDEX_MODE" == concurrent && ! -e "$FAKE_INDEX_CALLS" ]]; then
  touch "$FAKE_INDEX_CALLS"
  printf '{}\n'
else
  printf '%s\n' "$FAKE_INDEX_JSON"
fi
CURL
chmod +x "$TEST_DIR/bin/curl"

checksum="$(printf 'fixture bytes\n' | sha256sum | awk '{print $1}')"
run() {
  local name="${3:-fixture-crate}"
  rm -f "$TEST_DIR/index-calls"
  PATH="$TEST_DIR/bin:$PATH" \
  CARGO_BIN= CARGO="$TEST_DIR/bin/cargo" CARGO_TOOLCHAIN=fixture-toolchain \
  KELLNR_REGISTRY_INDEX='sparse+https://registry.invalid/index/' \
  FAKE_CRATE_NAME="$name" FAKE_INDEX_PATH="${4:-fi/xt/fixture-crate}" \
  FAKE_INDEX_MODE="${5:-static}" FAKE_INDEX_CALLS="$TEST_DIR/index-calls" \
  FAKE_INDEX_JSON="$1" FAKE_PUBLISH_MODE="${2:-success}" FAKE_PACKAGE_MODE="${6:-success}" \
  "$ROOT/scripts/publish-fork-crate.sh" --workspace "$TEST_DIR/workspace" "$name" >/dev/null
}

for specification in a:1/a ab:2/ab abc:3/a/abc fixture-crate:fi/xt/fixture-crate; do
  run "{\"vers\":\"1.2.3\",\"cksum\":\"$checksum\"}" forbidden \
    "${specification%%:*}" "${specification#*:}"
done
index="$(printf '{"vers":"1.2.3","cksum":"%s"}\n{"vers":"1.2.4","cksum":"newer"}\n' "$checksum")"
run "$index" forbidden
run '{"vers":"1.2.2","cksum":"absent"}' success
if run '{"vers":"1.2.3","cksum":"wrong"}' immutable; then
  echo 'mismatched checksum was incorrectly accepted' >&2
  exit 1
fi
run "{\"vers\":\"1.2.3\",\"cksum\":\"$checksum\"}" immutable fixture-crate fi/xt/fixture-crate concurrent
if run '{"vers":"1.2.3","cksum":"wrong"}' immutable fixture-crate fi/xt/fixture-crate concurrent; then
  echo 'concurrent publication with different bytes was incorrectly accepted' >&2
  exit 1
fi
if run '{"vers":"1.2.3","cksum":"wrong"}' failure; then
  echo 'non-immutable publish failure was incorrectly suppressed' >&2
  exit 1
fi
if run "{\"vers\":\"1.2.3\",\"cksum\":\"$checksum\"}" forbidden fixture-crate fi/xt/fixture-crate static failure; then
  echo 'failed packaging incorrectly reused stale crate bytes' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 42 ]]; then
    echo "package failure exit status was lost: $status" >&2
    exit 1
  fi
fi
printf '%s\n' 'publish-fork-crate target, sparse-index and publication decision tests passed'
