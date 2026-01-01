#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT
mkdir -p "$TEST_DIR/bin" "$TEST_DIR/workspace/target/package"
printf '[workspace]\n' > "$TEST_DIR/workspace/Cargo.toml"

cat > "$TEST_DIR/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "metadata" ]]; then
  printf '%s\n' '{"packages":[{"name":"fixture-crate","version":"1.2.3"}]}'
elif [[ "$1" == "package" ]]; then
  printf 'fixture bytes\n' > target/package/fixture-crate-1.2.3.crate
elif [[ "$1" == "publish" ]]; then
  if [[ "${FAKE_PUBLISH_MODE:-success}" == "immutable" ]]; then
    printf 'error: crate version already exists\n' >&2
    exit 1
  elif [[ "${FAKE_PUBLISH_MODE:-success}" == "failure" ]]; then
    printf 'error: registry unavailable\n' >&2
    exit 1
  fi
fi
CARGO
chmod +x "$TEST_DIR/bin/cargo"

cat > "$TEST_DIR/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${FAKE_INDEX_JSON:?}"
CURL
chmod +x "$TEST_DIR/bin/curl"

checksum="$(printf 'fixture bytes\n' | sha256sum | awk '{print $1}')"
run() {
  PATH="$TEST_DIR/bin:$PATH" \
  CARGO_BIN="$TEST_DIR/bin/cargo" \
  CARGO_REGISTRIES_SIGMA_INDEX='sparse+https://registry.invalid/index/' \
  FAKE_INDEX_JSON="$1" FAKE_PUBLISH_MODE="${2:-success}" \
  "$ROOT/scripts/publish-fork-crate.sh" --workspace "$TEST_DIR/workspace" fixture-crate >/dev/null
}

run "{\"vers\":\"1.2.3\",\"cksum\":\"$checksum\"}" success
if run '{"vers":"1.2.3","cksum":"wrong"}' immutable; then
  printf '%s\n' 'mismatched checksum was incorrectly accepted' >&2
  exit 1
fi
run "{\"vers\":\"1.2.3\",\"cksum\":\"$checksum\"}" immutable
if run "{\"vers\":\"1.2.3\",\"cksum\":\"wrong\"}" failure; then
  printf '%s\n' 'non-immutable publish failure was incorrectly suppressed' >&2
  exit 1
fi
printf '%s\n' 'publish-fork-crate decision tests passed'
