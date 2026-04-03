#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import textwrap
import tomllib
import urllib.error
import urllib.request
from pathlib import Path


DEFAULT_REPO = "xidl/xidl"
ROOT = Path(__file__).resolve().parent.parent
WORKSPACE_MANIFEST = ROOT / "Cargo.toml"
FORMULA_PATH = ROOT / "Formula" / "xidlc.rb"
SCOOP_PATH = ROOT / "packaging" / "scoop" / "xidlc.json"
WINGET_DIR = ROOT / "packaging" / "winget" / "manifests" / "x" / "xidl" / "xidlc"

PACKAGE_IDENTIFIER = "xidl.xidlc"
WINGET_MANIFEST_VERSION = "1.12.0"

WINDOWS_ASSET_CANDIDATES = {
    "64bit": [
        {
            "target": "x86_64-pc-windows-msvc",
            "archive": "zip",
            "winget_supported": True,
        },
        {
            "target": "x86_64-pc-windows-gnu",
            "archive": "tar.gz",
            "winget_supported": False,
        },
    ],
    "arm64": [
        {
            "target": "aarch64-pc-windows-msvc",
            "archive": "zip",
            "winget_supported": True,
        }
    ],
}

TAG_VERSION_RE = re.compile(
    r"^(?:refs/tags/)?v?(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)$"
)


class SyncError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Sync Homebrew / Scoop / winget manifests from a published "
            "xidlc GitHub release."
        )
    )
    parser.add_argument("--repo", default=DEFAULT_REPO, help="GitHub repo in owner/name form")
    parser.add_argument("--version", help="CLI version, defaults to workspace Cargo.toml")
    parser.add_argument("--tag", help="Release tag, defaults to v<version>")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify files are up to date without rewriting them",
    )
    return parser.parse_args()


def cli_version() -> str:
    with WORKSPACE_MANIFEST.open("rb") as handle:
        manifest = tomllib.load(handle)
    return manifest["workspace"]["package"]["version"]


def github_headers(token: str | None) -> dict[str, str]:
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "xidl-package-sync",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def fetch_json(url: str, token: str | None) -> object:
    request = urllib.request.Request(url, headers=github_headers(token))
    try:
        with urllib.request.urlopen(request) as response:
            return json.load(response)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise SyncError(f"request {url} failed with {exc.code}: {body}") from exc


def sha256_url(url: str, token: str | None) -> str:
    request = urllib.request.Request(url, headers=github_headers(token))
    digest = hashlib.sha256()
    try:
        with urllib.request.urlopen(request) as response:
            while True:
                chunk = response.read(1024 * 1024)
                if not chunk:
                    break
                digest.update(chunk)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise SyncError(f"download {url} failed with {exc.code}: {body}") from exc
    return digest.hexdigest()


def release_url(repo: str, tag: str) -> str:
    return f"https://api.github.com/repos/{repo}/releases/tags/{tag}"


def normalize_tag(tag: str) -> str:
    return tag.removeprefix("refs/tags/")


def version_from_tag(tag: str) -> str | None:
    match = TAG_VERSION_RE.fullmatch(tag.strip())
    if match is None:
        return None
    return match.group("version")


def formula_source_archive_url(repo: str, tag: str) -> str:
    return f"https://github.com/{repo}/archive/refs/tags/{tag}.tar.gz"


def asset_sha256(asset: dict[str, object], token: str | None) -> str:
    digest = asset.get("digest")
    if isinstance(digest, str) and digest.startswith("sha256:"):
        return digest.split(":", 1)[1]
    url = str(asset["browser_download_url"])
    return sha256_url(url, token)


def require_asset(assets: dict[str, dict[str, object]], name: str) -> dict[str, object]:
    asset = assets.get(name)
    if asset is None:
        raise SyncError(f"release asset {name} is missing")
    return asset


def render_formula(repo: str, tag: str, source_sha256: str) -> str:
    repo_url = f"https://github.com/{repo}"
    return textwrap.dedent(
        f"""\
        class Xidlc < Formula
          desc "XIDL compiler and multi-target code generator"
          homepage "{repo_url}"
          url "{formula_source_archive_url(repo, tag)}"
          sha256 "{source_sha256}"
          license "Apache-2.0"
          head "{repo_url}.git", branch: "master"

          depends_on "rust" => :build

          def install
            system "cargo", "install", *std_cargo_args(path: "xidlc")
          end

          test do
            assert_match version.to_s, shell_output("#{{bin}}/xidlc --version")
          end
        end
        """
    )


def render_scoop(repo: str, version: str, windows_assets: dict[str, dict[str, str]]) -> str:
    repo_url = f"https://github.com/{repo}"
    manifest = {
        "version": version,
        "description": "XIDL compiler and multi-target code generator",
        "homepage": repo_url,
        "license": "Apache-2.0",
        "architecture": {
            arch: {
                "url": asset["url"],
                "hash": asset["sha256"],
            }
            for arch, asset in windows_assets.items()
        },
        "bin": "xidlc.exe",
        "checkver": {
            "url": f"{repo_url}/releases",
            "regex": r"v([\d.]+)",
        },
        "autoupdate": {
            "architecture": {
                arch: {"url": asset["autoupdate_url"]}
                for arch, asset in windows_assets.items()
            }
        },
    }
    return json.dumps(manifest, indent=4, ensure_ascii=False) + "\n"


def render_winget_version(version: str) -> str:
    return textwrap.dedent(
        f"""\
        PackageIdentifier: {PACKAGE_IDENTIFIER}
        PackageVersion: {version}
        DefaultLocale: en-US
        ManifestType: version
        ManifestVersion: {WINGET_MANIFEST_VERSION}
        """
    )


def render_winget_default_locale(repo: str, version: str) -> str:
    repo_url = f"https://github.com/{repo}"
    return textwrap.dedent(
        f"""\
        PackageIdentifier: {PACKAGE_IDENTIFIER}
        PackageVersion: {version}
        PackageLocale: en-US
        Publisher: xidl
        PublisherUrl: https://github.com/xidl
        PublisherSupportUrl: {repo_url}/issues
        PackageName: xidlc
        PackageUrl: {repo_url}
        Moniker: xidlc
        License: Apache-2.0
        ShortDescription: XIDL compiler and multi-target code generator
        Description: xidlc compiles XIDL contracts into APIs, SDKs, protocol specs, and implementation scaffolding for multiple targets.
        Tags:
          - idl
          - codegen
          - openapi
          - jsonrpc
          - contract
        ManifestType: defaultLocale
        ManifestVersion: {WINGET_MANIFEST_VERSION}
        """
    )


def render_winget_installer(version: str, windows_assets: dict[str, dict[str, str]]) -> str:
    lines = [
        f"PackageIdentifier: {PACKAGE_IDENTIFIER}",
        f"PackageVersion: {version}",
        "Installers:",
    ]
    for arch, winget_arch in (("64bit", "x64"), ("arm64", "arm64")):
        asset = windows_assets.get(arch)
        if asset is None:
            continue
        lines.extend(
            [
                f"  - Architecture: {winget_arch}",
                "    InstallerType: zip",
                "    NestedInstallerType: portable",
                "    NestedInstallerFiles:",
                "      - RelativeFilePath: xidlc.exe",
                "        PortableCommandAlias: xidlc",
                "    Commands:",
                "      - xidlc",
                f"    InstallerUrl: {asset['url']}",
                f"    InstallerSha256: {asset['sha256'].upper()}",
            ]
        )
    lines.extend(
        [
            "ManifestType: installer",
            f"ManifestVersion: {WINGET_MANIFEST_VERSION}",
        ]
    )
    return "\n".join(lines) + "\n"


def write_text(path: Path, content: str, check: bool) -> bool:
    if path.exists():
        current = path.read_text()
        if current == content:
            print(f"unchanged {path.relative_to(ROOT)}")
            return False
    if check:
        raise SyncError(f"{path.relative_to(ROOT)} is out of date")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    print(f"updated {path.relative_to(ROOT)}")
    return True


def resolve_windows_assets(
    repo: str,
    assets: dict[str, dict[str, object]],
    token: str | None,
) -> dict[str, dict[str, str | bool]]:
    resolved: dict[str, dict[str, str | bool]] = {}
    repo_url = f"https://github.com/{repo}"
    for arch, candidates in WINDOWS_ASSET_CANDIDATES.items():
        for candidate in candidates:
            asset_name = f"xidlc-{candidate['target']}.{candidate['archive']}"
            asset = assets.get(asset_name)
            if asset is None:
                continue
            resolved[arch] = {
                "url": str(asset["browser_download_url"]),
                "sha256": asset_sha256(asset, token),
                "target": str(candidate["target"]),
                "archive": str(candidate["archive"]),
                "autoupdate_url": f"{repo_url}/releases/download/v$version/{asset_name}",
                "winget_supported": bool(candidate["winget_supported"]),
            }
            break
    if not resolved:
        raise SyncError("release payload is missing supported Windows assets")
    return resolved


def main() -> int:
    args = parse_args()
    token = os.getenv("GITHUB_TOKEN") or os.getenv("GH_TOKEN")
    tag = normalize_tag(args.tag) if args.tag else None
    derived_version = version_from_tag(tag) if tag else None

    if args.version is None:
        if tag is None:
            version = cli_version()
            tag = f"v{version}"
        elif derived_version is not None:
            version = derived_version
        else:
            raise SyncError(
                f"cannot derive version from tag {tag!r}; pass --version explicitly"
            )
    else:
        version = args.version
        if tag is None:
            tag = f"v{version}"
        elif derived_version is not None and derived_version != version:
            raise SyncError(
                f"tag {tag!r} resolves to version {derived_version!r}, "
                f"which does not match --version {version!r}"
            )

    release = fetch_json(release_url(args.repo, tag), token)
    if not isinstance(release, dict):
        raise SyncError(f"unexpected release payload type: {type(release)!r}")

    assets_list = release.get("assets")
    if not isinstance(assets_list, list):
        raise SyncError("release payload is missing assets")
    assets = {
        asset["name"]: asset
        for asset in assets_list
        if isinstance(asset, dict) and isinstance(asset.get("name"), str)
    }

    windows_assets = resolve_windows_assets(args.repo, assets, token)

    source_sha256 = sha256_url(formula_source_archive_url(args.repo, tag), token)

    changed = False
    changed |= write_text(FORMULA_PATH, render_formula(args.repo, tag, source_sha256), args.check)
    changed |= write_text(SCOOP_PATH, render_scoop(args.repo, version, windows_assets), args.check)

    winget_assets = {
        arch: asset
        for arch, asset in windows_assets.items()
        if bool(asset["winget_supported"])
    }
    if winget_assets:
        winget_version_dir = WINGET_DIR / version
        changed |= write_text(
            winget_version_dir / f"{PACKAGE_IDENTIFIER}.yaml",
            render_winget_version(version),
            args.check,
        )
        changed |= write_text(
            winget_version_dir / f"{PACKAGE_IDENTIFIER}.locale.en-US.yaml",
            render_winget_default_locale(args.repo, version),
            args.check,
        )
        changed |= write_text(
            winget_version_dir / f"{PACKAGE_IDENTIFIER}.installer.yaml",
            render_winget_installer(version, winget_assets),
            args.check,
        )
    else:
        print("skipped packaging/winget because no supported Windows zip assets were found")

    if args.check:
        print("package manager manifests are up to date")
    elif not changed:
        print("package manager manifests already in sync")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SyncError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
