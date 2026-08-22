#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

PUBLISH_ORDER=(
  "grapheme-signatures"
  "grapheme-artifact"
  "grapheme-stdlib"
  "grapheme-aot-container"
  "grapheme-runtime"
  "grapheme-compiler"
  "grapheme-sdk"
  "grapheme-cli"
  "grapheme-lsp"
)

DRY_RUN=1
YES=0
ALLOW_DIRTY=0
NO_VERIFY=0
WAIT_SECONDS=20
FROM_CRATE=""

usage() {
  cat <<EOF
Usage: scripts/publish-crates.sh [options]

Publishes workspace crates to crates.io in dependency-safe order.
Default mode is dry-run.
Crate versions that already exist on crates.io are skipped.

Options:
  --publish              Perform real publish (default: dry-run).
  --dry-run              Force dry-run publish checks (default).
  --from <crate>         Resume from a specific crate name in publish order.
  --yes                  Skip confirmation prompt.
  --allow-dirty          Pass --allow-dirty to cargo publish.
  --no-verify            Pass --no-verify to cargo publish.
  --wait-seconds <n>     Wait time between real publishes (default: 20).
  -h, --help             Show this help.

Publish order:
$(printf '  - %s\n' "${PUBLISH_ORDER[@]}")

Examples:
  scripts/publish-crates.sh
  scripts/publish-crates.sh --publish
  scripts/publish-crates.sh --publish --from grapheme-compiler --yes
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --publish)
      DRY_RUN=0
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --from)
      [[ $# -lt 2 ]] && { echo "--from requires a crate name" >&2; exit 1; }
      FROM_CRATE="$2"
      shift 2
      ;;
    --yes)
      YES=1
      shift
      ;;
    --allow-dirty)
      ALLOW_DIRTY=1
      shift
      ;;
    --no-verify)
      NO_VERIFY=1
      shift
      ;;
    --wait-seconds)
      [[ $# -lt 2 ]] && { echo "--wait-seconds requires a numeric value" >&2; exit 1; }
      WAIT_SECONDS="$2"
      if ! [[ "$WAIT_SECONDS" =~ ^[0-9]+$ ]]; then
        echo "--wait-seconds must be a non-negative integer" >&2
        exit 1
      fi
      shift 2
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

if [[ -n "$FROM_CRATE" ]]; then
  FOUND=0
  for crate in "${PUBLISH_ORDER[@]}"; do
    if [[ "$crate" == "$FROM_CRATE" ]]; then
      FOUND=1
      break
    fi
  done

  if [[ $FOUND -ne 1 ]]; then
    echo "Unknown crate for --from: $FROM_CRATE" >&2
    exit 1
  fi
fi

if [[ $YES -ne 1 ]]; then
  echo "Publish mode: $([[ $DRY_RUN -eq 1 ]] && echo DRY-RUN || echo REAL)"
  echo "Workspace: $ROOT_DIR"
  echo "Order: ${PUBLISH_ORDER[*]}"
  if [[ -n "$FROM_CRATE" ]]; then
    echo "Resuming from: $FROM_CRATE"
  fi
  read -r -p "Continue? [y/N] " reply
  if [[ ! "$reply" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to check existing crates.io versions" >&2
  exit 1
fi

if [[ $DRY_RUN -eq 0 ]] && [[ $YES -ne 1 ]]; then
  echo "Ensure you are authenticated: cargo login"
fi

COMMON_FLAGS=()
if [[ $ALLOW_DIRTY -eq 1 ]]; then
  COMMON_FLAGS+=("--allow-dirty")
fi
if [[ $NO_VERIFY -eq 1 ]]; then
  COMMON_FLAGS+=("--no-verify")
fi
if [[ $DRY_RUN -eq 1 ]]; then
  COMMON_FLAGS+=("--dry-run")
fi

SKIP=0
if [[ -n "$FROM_CRATE" ]]; then
  SKIP=1
fi

crate_version_is_published() {
  local crate="$1"
  local version="$2"
  local status

  if ! status="$(curl \
    --silent \
    --show-error \
    --output /dev/null \
    --write-out '%{http_code}' \
    --user-agent 'grapheme-release-script (https://github.com/entasislabs/grapheme)' \
    "https://crates.io/api/v1/crates/$crate/$version")"; then
    echo "Failed to check crates.io for $crate $version" >&2
    return 2
  fi

  case "$status" in
    200) return 0 ;;
    404) return 1 ;;
    *)
      echo "Unexpected crates.io response for $crate $version: HTTP $status" >&2
      return 2
      ;;
  esac
}

for crate in "${PUBLISH_ORDER[@]}"; do
  if [[ $SKIP -eq 1 ]]; then
    if [[ "$crate" != "$FROM_CRATE" ]]; then
      echo "Skipping $crate (before --from target)"
      continue
    fi
    SKIP=0
  fi

  package_id="$(cargo pkgid -p "$crate")"
  version="${package_id##*#}"
  version="${version##*@}"

  if crate_version_is_published "$crate" "$version"; then
    echo
    echo "=== Skipping $crate $version (already published) ==="
    continue
  else
    check_status=$?
    if [[ $check_status -ne 1 ]]; then
      exit "$check_status"
    fi
  fi

  echo
  echo "=== Publishing $crate $version ==="
  if [[ ${#COMMON_FLAGS[@]} -eq 0 ]]; then
    cargo publish -p "$crate"
  else
    cargo publish -p "$crate" "${COMMON_FLAGS[@]}"
  fi

  if [[ $DRY_RUN -eq 0 && "$crate" != "grapheme-lsp" && "$WAIT_SECONDS" -gt 0 ]]; then
    echo "Waiting ${WAIT_SECONDS}s for crates.io index propagation..."
    sleep "$WAIT_SECONDS"
  fi
done

echo
if [[ $DRY_RUN -eq 1 ]]; then
  echo "Dry-run publish checks completed."
else
  echo "Publish sequence completed."
fi
