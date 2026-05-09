#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/lsp-release"
REPO="${GITHUB_REPOSITORY:-entasislabs/grapheme}"
TAG=""
PUBLISH=0

TARGETS=()

usage() {
  cat <<EOF
Usage: scripts/release-lsp.sh [options]

Options:
  --target <triple>     Rust target triple to build (repeatable). Default: host target.
  --tag <tag>           Release tag (required with --publish).
  --repo <owner/repo>   GitHub repo for release uploads. Default: $REPO
  --publish             Upload built assets using gh release upload/create.
  -h, --help            Show this help.

Examples:
  scripts/release-lsp.sh
  scripts/release-lsp.sh --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu
  scripts/release-lsp.sh --target x86_64-unknown-linux-gnu --tag v0.1.0 --publish
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

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  HOST_TARGET="$(rustc -vV | awk '/host:/ {print $2}')"
  TARGETS=("$HOST_TARGET")
fi

if [[ $PUBLISH -eq 1 && -z "$TAG" ]]; then
  echo "--tag is required when --publish is set" >&2
  exit 1
fi

mkdir -p "$DIST_DIR"

asset_name_for_target() {
  local target="$1"
  case "$target" in
    x86_64-unknown-linux-gnu) echo "grapheme-lsp-linux-x64" ;;
    aarch64-unknown-linux-gnu) echo "grapheme-lsp-linux-arm64" ;;
    x86_64-apple-darwin) echo "grapheme-lsp-macos-x64" ;;
    aarch64-apple-darwin) echo "grapheme-lsp-macos-arm64" ;;
    x86_64-pc-windows-msvc|x86_64-pc-windows-gnu) echo "grapheme-lsp-windows-x64.exe" ;;
    aarch64-pc-windows-msvc|aarch64-pc-windows-gnullvm) echo "grapheme-lsp-windows-arm64.exe" ;;
    *)
      echo "Unsupported target for extension asset naming: $target" >&2
      return 1
      ;;
  esac
}

for target in "${TARGETS[@]}"; do
  echo "Building grapheme-lsp for $target"
  rustup target add "$target" >/dev/null
  cargo build -p grapheme-lsp --release --target "$target"

  asset_name="$(asset_name_for_target "$target")"
  src="$ROOT_DIR/target/$target/release/grapheme-lsp"
  if [[ "$asset_name" == *.exe ]]; then
    src+=".exe"
  fi

  dst="$DIST_DIR/$asset_name"
  cp "$src" "$dst"
  chmod +x "$dst" 2>/dev/null || true
  echo "Built $dst"
done

if [[ $PUBLISH -eq 1 ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh CLI is required for --publish" >&2
    exit 1
  fi

  if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
    echo "Release $TAG already exists in $REPO"
  else
    gh release create "$TAG" --repo "$REPO" --title "$TAG" --notes "Manual grapheme-lsp binary release"
  fi

  gh release upload "$TAG" "$DIST_DIR"/* --repo "$REPO" --clobber
  echo "Uploaded assets to $REPO@$TAG"
fi
