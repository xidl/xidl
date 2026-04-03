from __future__ import annotations

import argparse
import importlib.util
import os
import unittest
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "sync-package-manifests.py"
SPEC = importlib.util.spec_from_file_location("sync_package_manifests", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"failed to load {SCRIPT_PATH}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SyncPackageManifestsTests(unittest.TestCase):
    def test_version_from_tag_supports_common_release_forms(self) -> None:
        self.assertEqual(MODULE.version_from_tag("v0.31.0"), "0.31.0")
        self.assertEqual(MODULE.version_from_tag("refs/tags/v0.31.0"), "0.31.0")
        self.assertEqual(MODULE.version_from_tag("0.31.0"), "0.31.0")
        self.assertIsNone(MODULE.version_from_tag("nightly"))

    def test_main_derives_version_from_tag_and_uses_formula_archive_url(self) -> None:
        writes: list[tuple[Path, str, bool]] = []
        args = argparse.Namespace(
            repo="xidl/xidl",
            version=None,
            tag="v0.31.0",
            check=False,
        )

        with (
            mock.patch.dict(os.environ, {}, clear=True),
            mock.patch.object(MODULE, "parse_args", return_value=args),
            mock.patch.object(MODULE, "fetch_json", return_value={"assets": []}),
            mock.patch.object(
                MODULE,
                "resolve_windows_assets",
                return_value={
                    "64bit": {
                        "url": "https://example.com/xidlc-x86_64-pc-windows-gnu.tar.gz",
                        "sha256": "a" * 64,
                        "target": "x86_64-pc-windows-gnu",
                        "archive": "tar.gz",
                        "autoupdate_url": "https://example.com/v$version/xidlc.tar.gz",
                        "winget_supported": False,
                    }
                },
            ),
            mock.patch.object(MODULE, "sha256_url", return_value="b" * 64) as sha256_url,
            mock.patch.object(
                MODULE,
                "write_text",
                side_effect=lambda path, content, check: writes.append((path, content, check))
                or True,
            ),
        ):
            self.assertEqual(MODULE.main(), 0)

        sha256_url.assert_called_once_with(
            "https://github.com/xidl/xidl/archive/refs/tags/v0.31.0.tar.gz",
            None,
        )

        rendered = {path: content for path, content, _ in writes}
        self.assertIn(MODULE.FORMULA_PATH, rendered)
        self.assertIn(MODULE.SCOOP_PATH, rendered)
        self.assertIn(
            'url "https://github.com/xidl/xidl/archive/refs/tags/v0.31.0.tar.gz"',
            rendered[MODULE.FORMULA_PATH],
        )
        self.assertIn('"version": "0.31.0"', rendered[MODULE.SCOOP_PATH])

    def test_main_rejects_mismatched_tag_and_version(self) -> None:
        args = argparse.Namespace(
            repo="xidl/xidl",
            version="0.32.0",
            tag="v0.31.0",
            check=False,
        )

        with (
            mock.patch.dict(os.environ, {}, clear=True),
            mock.patch.object(MODULE, "parse_args", return_value=args),
        ):
            with self.assertRaises(MODULE.SyncError):
                MODULE.main()


if __name__ == "__main__":
    unittest.main()
