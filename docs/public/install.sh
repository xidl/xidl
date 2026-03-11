#!/usr/bin/env sh
set -eu

API_URL="https://api.github.com/repos/xidl/xidl/releases/latest"

os_name() {
  case "$(uname -s)" in
    Darwin) echo "apple-darwin" ;;
    Linux) echo "unknown-linux-gnu" ;;
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

OS="$(os_name)"
ARCH="$(arch_name)"

if [ "${OS}" = "unsupported" ] || [ "${ARCH}" = "unsupported" ]; then
  echo "Unsupported platform: $(uname -s) / $(uname -m)" >&2
  exit 1
fi

if [ "${OS}" = "apple-darwin" ] && [ "${ARCH}" = "x86_64" ]; then
  echo "No x86_64 macOS release available. Use aarch64 macOS or build from source." >&2
  exit 1
fi

TAR="xidlc-${ARCH}-${OS}.tar.gz"

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

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to parse GitHub API response" >&2
  exit 1
fi

RELEASE_JSON="$(fetch_url "${API_URL}")"

INFO="$(printf '%s' "${RELEASE_JSON}" | python3 - "${TAR}" <<'PY'
import json
import re
import sys

target = sys.argv[1]

data = json.load(sys.stdin)
assets = data.get("assets", [])
body = data.get("body", "") or ""
tag = data.get("tag_name", "")

asset_url = ""
for asset in assets:
    if asset.get("name") == target:
        asset_url = asset.get("browser_download_url", "")
        break

sha = ""
lines = body.splitlines()
for i, line in enumerate(lines):
    if target in line:
        for j in range(i + 1, min(i + 6, len(lines))):
            m = re.search(r"sha256[:\s]*([0-9a-fA-F]{64})", lines[j])
            if m:
                sha = m.group(1).lower()
                break
    if sha:
        break
if not sha:
    for line in lines:
        m = re.search(rf"{re.escape(target)}.*sha256[:\s]*([0-9a-fA-F]{{64}})", line)
        if m:
            sha = m.group(1).lower()
            break

print(f"{asset_url}|{sha}|{tag}")
PY
)"

IFS='|' read -r URL SHA TAG <<EOF
${INFO}
