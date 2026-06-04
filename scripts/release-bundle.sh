#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/release-bundle"
LSP_DIST_DIR="$ROOT_DIR/dist/lsp-release"
EXT_DIR="$ROOT_DIR/extensions/grapheme-vscode"
REPO="${GITHUB_REPOSITORY:-entasislabs/grapheme}"
TAG=""
PUBLISH=0
SKIP_LSP=0
SKIP_VSIX=0

TARGETS=()

cd "$ROOT_DIR"

usage() {
  cat <<EOF
Usage: scripts/release-bundle.sh [options]

Builds release artifacts for both:
1) grapheme-lsp binaries
2) grapheme-vscode VSIX package

Options:
  --target <triple>     Rust target triple for LSP build (repeatable). Default: host target.
  --tag <tag>           Release tag (required with --publish).
  --repo <owner/repo>   GitHub repo for uploads. Default: $REPO
  --publish             Upload all built artifacts to GitHub release.
  --skip-lsp            Skip LSP binary builds.
  --skip-vsix           Skip VSIX packaging.
  -h, --help            Show help.

Examples:
  scripts/release-bundle.sh
  scripts/release-bundle.sh --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu
  scripts/release-bundle.sh --tag v0.5.0 --publish
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      [[ $# -lt 2 ]] && { echo "--target requires a value" >&2; exit 1; }
      TARGETS+=("$2")
      shift 2
      ;;
    --tag)
      [[ $# -lt 2 ]] && { echo "--tag requires a value" >&2; exit 1; }
      TAG="$2"
      shift 2
      ;;
    --repo)
      [[ $# -lt 2 ]] && { echo "--repo requires a value" >&2; exit 1; }
      REPO="$2"
      shift 2
      ;;
    --publish)
      PUBLISH=1
      shift
      ;;
    --skip-lsp)
      SKIP_LSP=1
      shift
      ;;
    --skip-vsix)
      SKIP_VSIX=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ $SKIP_LSP -eq 1 && $SKIP_VSIX -eq 1 ]]; then
  echo "Nothing to do: both --skip-lsp and --skip-vsix were set" >&2
  exit 1
fi

if [[ $PUBLISH -eq 1 && -z "$TAG" ]]; then
  echo "--tag is required when --publish is set" >&2
  exit 1
fi

mkdir -p "$DIST_DIR"

if [[ $SKIP_LSP -eq 0 ]]; then
  echo "Building LSP release assets"
  LSP_ARGS=()
  for target in "${TARGETS[@]}"; do
    LSP_ARGS+=(--target "$target")
  done
  "$ROOT_DIR/scripts/release-lsp.sh" "${LSP_ARGS[@]}"

  shopt -s nullglob
  for asset in "$LSP_DIST_DIR"/*; do
    cp "$asset" "$DIST_DIR/"
  done
  shopt -u nullglob
fi

if [[ $SKIP_VSIX -eq 0 ]]; then
  echo "Building VSIX"
  pushd "$EXT_DIR" >/dev/null
  npm install
  npm run build
  npx --yes @vscode/vsce package --allow-missing-repository
  shopt -s nullglob
  VSIX_FILES=(*.vsix)
  if [[ ${#VSIX_FILES[@]} -eq 0 ]]; then
    echo "No VSIX package found after build" >&2
    exit 1
  fi
  latest_vsix="${VSIX_FILES[-1]}"
  cp "$latest_vsix" "$DIST_DIR/"
  shopt -u nullglob
  popd >/dev/null
fi

if [[ $PUBLISH -eq 1 ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh CLI is required for --publish" >&2
    exit 1
  fi

  if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
    echo "Release $TAG already exists in $REPO"
  else
    gh release create "$TAG" --repo "$REPO" --title "$TAG" --notes "Grapheme LSP + VSIX bundle"
  fi

  gh release upload "$TAG" "$DIST_DIR"/* --repo "$REPO" --clobber
  echo "Uploaded bundle assets to $REPO@$TAG"
fi

echo "Bundle artifacts in: $DIST_DIR"
ls -lh "$DIST_DIR"
