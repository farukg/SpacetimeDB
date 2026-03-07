#!/usr/bin/env bash
# fork-release.sh — Build and publish a fork release locally.
#
# Builds spacetimedb-cli + spacetimedb-standalone (Linux x86_64),
# packs the @spacetimedb/rescript tarball, and creates a GitHub release
# with all three assets.
#
# Usage:
#   ./scripts/fork-release.sh v2.0.0-fork4
#
# Prerequisites:
#   - Rust toolchain (see rust-toolchain.toml)
#   - Node.js 22+ with npm
#   - gh CLI authenticated
#   - sigma_rescript_codegen accessible at expected path (private dep)

set -euo pipefail

TAG="${1:?Usage: $0 <tag> (e.g. v2.0.0-fork4)}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${REPO_ROOT}/target/fork-release"

echo "=== Fork Release: ${TAG} ==="
echo "Repo: ${REPO_ROOT}"
echo ""

# ---------------------------------------------------------------------------
# 1. Validate tag format
# ---------------------------------------------------------------------------
if [[ ! "${TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-fork[0-9]+$ ]]; then
  echo "ERROR: Tag must match v*.*.*-fork* pattern (got: ${TAG})"
  exit 1
fi

# ---------------------------------------------------------------------------
# 2. Ensure working tree is clean
# ---------------------------------------------------------------------------
if ! git -C "${REPO_ROOT}" diff --quiet HEAD 2>/dev/null; then
  echo "WARNING: Working tree has uncommitted changes."
  read -rp "Continue anyway? [y/N] " confirm
  [[ "${confirm}" =~ ^[Yy]$ ]] || exit 1
fi

# ---------------------------------------------------------------------------
# 3. Build Rust binaries
# ---------------------------------------------------------------------------
echo ""
echo "--- Building spacetimedb-cli (release) ---"
cargo build --release -p spacetimedb-cli --manifest-path "${REPO_ROOT}/Cargo.toml"

echo ""
echo "--- Building spacetimedb-standalone (release) ---"
cargo build --release -p spacetimedb-standalone --manifest-path "${REPO_ROOT}/Cargo.toml"

# ---------------------------------------------------------------------------
# 4. Build ReScript SDK package
# ---------------------------------------------------------------------------
echo ""
echo "--- Building @spacetimedb/rescript ---"
RESCRIPT_DIR="${REPO_ROOT}/crates/bindings-rescript"
(
  cd "${RESCRIPT_DIR}"
  npm install --ignore-scripts 2>/dev/null || true
  npx rescript clean
  npx rescript build
  # Remove any prior tarball
  rm -f spacetimedb-rescript-*.tgz
  npm pack
)

# ---------------------------------------------------------------------------
# 5. Collect artifacts
# ---------------------------------------------------------------------------
echo ""
echo "--- Collecting artifacts ---"
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"

cp "${REPO_ROOT}/target/release/spacetimedb-cli" "${BUILD_DIR}/spacetimedb-cli"
cp "${REPO_ROOT}/target/release/spacetimedb-standalone" "${BUILD_DIR}/spacetimedb-standalone"
chmod +x "${BUILD_DIR}/spacetimedb-cli" "${BUILD_DIR}/spacetimedb-standalone"

TARBALL=$(find "${RESCRIPT_DIR}" -maxdepth 1 -name 'spacetimedb-rescript-*.tgz' | head -1)
if [[ -z "${TARBALL}" ]]; then
  echo "ERROR: No ReScript tarball found in ${RESCRIPT_DIR}"
  exit 1
fi
cp "${TARBALL}" "${BUILD_DIR}/"

echo "Artifacts:"
ls -lh "${BUILD_DIR}/"

# ---------------------------------------------------------------------------
# 6. Create/push tag if not already on remote
# ---------------------------------------------------------------------------
echo ""
echo "--- Tagging ---"
if git -C "${REPO_ROOT}" rev-parse "${TAG}" >/dev/null 2>&1; then
  echo "Tag ${TAG} already exists locally."
else
  echo "Creating tag ${TAG}..."
  git -C "${REPO_ROOT}" tag "${TAG}"
fi

if ! git -C "${REPO_ROOT}" ls-remote --tags origin "${TAG}" | grep -q "${TAG}"; then
  echo "Pushing tag ${TAG} to origin..."
  FORK_RELEASE_SCRIPT=1 git -C "${REPO_ROOT}" push origin "${TAG}"
else
  echo "Tag ${TAG} already on remote."
fi

# ---------------------------------------------------------------------------
# 7. Create GitHub release
# ---------------------------------------------------------------------------
echo ""
echo "--- Creating GitHub release ---"
if gh release view "${TAG}" --repo "$(git -C "${REPO_ROOT}" remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')" >/dev/null 2>&1; then
  echo "Release ${TAG} already exists. Uploading/overwriting assets..."
  gh release upload "${TAG}" \
    --repo "$(git -C "${REPO_ROOT}" remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')" \
    --clobber \
    "${BUILD_DIR}"/*
else
  gh release create "${TAG}" \
    --repo "$(git -C "${REPO_ROOT}" remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')" \
    --title "${TAG}" \
    --generate-notes \
    "${BUILD_DIR}"/*
fi

# ---------------------------------------------------------------------------
# 8. Upload to GitLab Generic Package Registry (if GITLAB_TOKEN is set)
# ---------------------------------------------------------------------------
if [[ -n "${GITLAB_TOKEN:-}" ]]; then
  echo ""
  echo "--- Uploading to GitLab Generic Package Registry ---"
  GITLAB_URL="${GITLAB_URL:-https://gitlab.next.myicecreamlab.com}"
  GITLAB_PROJECT_ID="${GITLAB_PROJECT_ID:-25}"
  
  for ARTIFACT in "${BUILD_DIR}"/*; do
    FILENAME=$(basename "$ARTIFACT")
    echo "Uploading ${FILENAME}..."
    HTTP_STATUS=$(curl -sS -o /dev/null -w '%{http_code}' \
      --header "PRIVATE-TOKEN: ${GITLAB_TOKEN}" \
      --upload-file "$ARTIFACT" \
      "${GITLAB_URL}/api/v4/projects/${GITLAB_PROJECT_ID}/packages/generic/spacetimedb-release/${TAG}/${FILENAME}")
    
    if [[ "$HTTP_STATUS" == "201" || "$HTTP_STATUS" == "200" ]]; then
      echo "  ✓ ${FILENAME} (HTTP ${HTTP_STATUS})"
    else
      echo "  ✗ ${FILENAME} failed (HTTP ${HTTP_STATUS})"
    fi
  done
  
  echo ""
  echo "GitLab download base: ${GITLAB_URL}/api/v4/projects/${GITLAB_PROJECT_ID}/packages/generic/spacetimedb-release/${TAG}/"
else
  echo ""
  echo "GITLAB_TOKEN not set — skipping GitLab upload"
fi

echo ""
echo "=== Done! Release ${TAG} published ==="
echo "https://github.com/$(git -C "${REPO_ROOT}" remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')/releases/tag/${TAG}"
