#!/usr/bin/env sh
set -eu

API_URL="https://api.github.com/repos/xidl/xidl/releases/latest"
INSTALL_DIR="${HOME}/.local/bin"

os_name() {
  case "$(uname -s)" in
    Darwin) echo "apple-darwin" ;;
    Linux) echo "unknown-linux-musl" ;;
    *) echo "unsupported" ;;
  esac
}

arch_name() {
  case "$(uname -m)" in
    arm64|aarch64) echo "aarch64" ;;
    x86_64|amd64) echo "x86_64" ;;
    *) echo "unsupported" ;;
  esac
}

asset_name() {
  case "${OS}:${ARCH}" in
    apple-darwin:aarch64) echo "xidlc-aarch64-apple-darwin.tar.gz" ;;
    apple-darwin:x86_64) echo "xidlc-x86_64-apple-darwin.tar.gz" ;;
    unknown-linux-musl:aarch64) echo "xidlc-aarch64-unknown-linux-musl.tar.gz" ;;
    unknown-linux-musl:x86_64) echo "xidlc-x86_64-unknown-linux-musl.tar.gz" ;;
    *) echo "unsupported" ;;
  esac
}

fetch_url() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -H "Accept: application/vnd.github+json" -H "User-Agent: xidl-installer" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- --header="Accept: application/vnd.github+json" --header="User-Agent: xidl-installer" "$1"
  else
    echo "curl or wget is required" >&2
    exit 1
  fi
}

download_file() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -H "User-Agent: xidl-installer" "$1" -o "$2"
  else
    wget -qO "$2" --header="User-Agent: xidl-installer" "$1"
  fi
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$1" | awk '{print $NF}'
  else
    echo "sha256sum, shasum, or openssl is required" >&2
    exit 1
  fi
}

OS="$(os_name)"
ARCH="$(arch_name)"
ASSET="$(asset_name)"

if [ "${OS}" = "unsupported" ] || [ "${ARCH}" = "unsupported" ] || [ "${ASSET}" = "unsupported" ]; then
  echo "Unsupported platform: $(uname -s) / $(uname -m)" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to parse GitHub API response" >&2
  exit 1
fi

RELEASE_JSON="$(fetch_url "${API_URL}")"

INFO="$(RELEASE_JSON="${RELEASE_JSON}" python3 - "${ASSET}" <<'PY'
import json
import os
import re
import sys

target = sys.argv[1]
data = json.loads(os.environ["RELEASE_JSON"])
assets = data.get("assets", [])
body = data.get("body", "") or ""
tag = data.get("tag_name", "")

asset = None
for candidate in assets:
    if candidate.get("name") == target:
        asset = candidate
        break

asset_url = ""
sha = ""
if asset is not None:
    asset_url = asset.get("browser_download_url", "")
    digest = asset.get("digest", "") or ""
    if isinstance(digest, str) and digest.startswith("sha256:"):
        sha = digest.split(":", 1)[1].lower()

if not sha:
    lines = body.splitlines()
    for i, line in enumerate(lines):
        if target in line:
            for j in range(i + 1, min(i + 6, len(lines))):
                match = re.search(r"sha256[:\s]*([0-9a-fA-F]{64})", lines[j])
                if match:
                    sha = match.group(1).lower()
                    break
        if sha:
            break
    if not sha:
        for line in lines:
            match = re.search(rf"{re.escape(target)}.*sha256[:\s]*([0-9a-fA-F]{{64}})", line)
            if match:
                sha = match.group(1).lower()
                break

print(f"{asset_url}|{sha}|{tag}")
PY
)"

IFS='|' read -r URL SHA TAG <<EOF
${INFO}
EOF

if [ -z "${URL}" ]; then
  echo "Artifact not found in latest stable release: ${ASSET}" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
ARCHIVE_PATH="${TMP_DIR}/${ASSET}"
trap 'rm -rf "${TMP_DIR}"' EXIT HUP INT TERM

download_file "${URL}" "${ARCHIVE_PATH}"

if [ -n "${SHA}" ]; then
  ACTUAL_SHA="$(sha256_file "${ARCHIVE_PATH}")"
  if [ "${ACTUAL_SHA}" != "${SHA}" ]; then
    echo "sha256 mismatch: expected ${SHA}, got ${ACTUAL_SHA}" >&2
    exit 1
  fi
fi

mkdir -p "${INSTALL_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${INSTALL_DIR}"

BIN_PATH="${INSTALL_DIR}/xidlc"
if [ ! -x "${BIN_PATH}" ]; then
  echo "xidlc not found after extraction" >&2
  exit 1
fi

echo "Installed xidlc to ${BIN_PATH} (release ${TAG})"
case ":${PATH}:" in
  *:"${INSTALL_DIR}":*) ;;
  *)
    echo "Add ${INSTALL_DIR} to your PATH, e.g.:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac
