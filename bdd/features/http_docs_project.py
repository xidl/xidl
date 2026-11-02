"""Real compiler/plugin publication fixtures; no API service or credentials required."""

import json
import os
import re
import shutil
import subprocess
import tempfile
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


@lru_cache(maxsize=1)
def documentation_binaries():
    result = subprocess.run(
        ["cargo", "build", "-p", "xidlc", "--features", "cli", "-p", "xidl-http-docs"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=900,
    )
    assert result.returncode == 0, result.stderr
    metadata = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    directory = Path(json.loads(metadata.stdout)["target_directory"]) / "debug"
    return directory


class DocumentationProject:
    def __init__(self, context):
        temporary = tempfile.TemporaryDirectory(prefix="xidl-http-docs-bdd-")
        context.add_cleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.source = self.root / "contracts"
        shutil.copytree(ROOT / "examples/documentation", self.source)
        self.site = self.root / "site"
        self.cwd = self.root / "unrelated-working-directory"
        self.cwd.mkdir()
        self.config = self.source / "xidl-http-docs.toml"
        self.note = self.source / "v1-to-v2.md"
        self.config_text = self.config.read_text().replace(
            'base_url = "/"', 'base_url = "/reference/"'
        )
        self.note_text = (
            self.note.read_text() + "\n\n[Upgrade instructions](#upgrade)\n"
        )
        self.config.write_text(self.config_text)
        self.note.write_text(self.note_text)
        self.bin = documentation_binaries()

    def generate(self, inputs=None, versions=(), succeeds=True, output=None):
        inputs = inputs or [self.source / "xidl.toml"]
        command = [str(self.bin / "xidlc"), "gen", "-o", str(output or self.site)]
        for version in versions:
            command.extend(["--api-version", version])
        command.extend(["http-docs", *map(str, inputs)])
        env = dict(os.environ, XIDL_HTTP_DOCS_CONFIG=str(self.config))
        env["PATH"] = str(self.bin) + os.pathsep + env.get("PATH", "")
        result = subprocess.run(
            command, cwd=self.cwd, env=env, capture_output=True, text=True, timeout=90
        )
        if succeeds:
            assert result.returncode == 0, result.stderr
        else:
            assert result.returncode != 0, "invalid publication unexpectedly succeeded"
        return result

    def owned(self):
        manifests = list(self.site.glob(".xidlc-*-outputs.json"))
        assert len(manifests) == 1, manifests
        manifest = manifests[0]
        paths = json.loads(manifest.read_text())
        return {
            path: (self.site / path).read_bytes() for path in [*paths, manifest.name]
        }

    def tree(self):
        return {
            str(path.relative_to(self.site)): path.read_bytes()
            for path in self.site.rglob("*")
            if path.is_file()
        }

    def build(self):
        for command in [["pnpm", "install", "--frozen-lockfile"], ["pnpm", "build"]]:
            result = subprocess.run(
                command, cwd=self.site, capture_output=True, text=True, timeout=300
            )
            assert result.returncode == 0, result.stdout + result.stderr
        return self.site / "dist"

    def assert_shared_contract(self):
        files = self.owned()
        description = (
            "Invoice returned by the billing API.\n\n"
            "Use the invoice **ID** for later lookups.\n\n"
            "- Read payment details from the selected API version.\n"
            "- Amounts use minor currency units.\n\n"
            "```http\nGET /invoices/inv_001\n```"
        )
        for version, field, field_description in [
            (
                "v1",
                "payment_intent",
                "Single payment intent associated with older invoice contracts.",
            ),
            ("v2", "payments", "Payments associated with the invoice."),
        ]:
            reference = files[f"public/api/billing/{version}/types/Invoice.md"].decode()
            assert description in reference
            document = json.loads(files[f"public/openapi/billing/{version}.json"])
            invoice = next(
                schema
                for schema in document["components"]["schemas"].values()
                if schema.get("title") == "Invoice"
            )
            assert invoice["description"] == description
            assert invoice["properties"][field]["description"] == field_description
            assert (
                document["paths"]["/invoices/{id}"]["get"]["description"]
                == "Retrieve one invoice by ID."
            )
        assert description.encode() in files["public/llms-full.txt"]
        shared = "public/api/billing/v2/types/InvoicePayment.md"
        assert shared in files
        assert not any(
            "payments/unversioned/types/InvoicePayment" in name for name in files
        )
        text = files[shared].decode()
        assert (
            set(re.findall(r"\]\(([^)]+/operations/[^)]+)\)", text))
            == self.shared_uses()
        )
        change = files["public/changelog/billing/v1-to-v2.md"].decode()
        assert change.count("### InvoicePayment — Added") == 1
        assert "payment\\_intent" in change and "Invoice.payments" in change
        assert "Author reviewed" in change and "Needs review" in change
        payment = files[
            "public/api/payments/unversioned/operations/Payments~3a~3aretrieve.md"
        ].decode()
        assert "/reference/api/billing/v2/types/InvoicePayment.md" in payment
        assert "X-API-Version" not in payment
        for name in ["public/index.md", "public/llms.txt", "public/llms-full.txt"]:
            assert b"/reference/" in files[name]
        assert b"changelog/billing/v1-to-v2.md" in files["public/llms.txt"]
        assert b"# InvoicePayment" in files["public/llms-full.txt"]
        assert b"# Payments::retrieve" in files["public/llms-full.txt"]

    def shared_uses(self, extension=".md"):
        return {
            f"/reference/api/{unit}/{version}/operations/{operation}~3a~3a{method}{extension}"
            for unit, version, operation in [
                ("billing", "v2", "Invoices"),
                ("payments", "unversioned", "Payments"),
            ]
            for method in ["retrieve", "list"]
        }

    def replace_note(self, field, value):
        import tomllib

        header, body = self.note_text.removeprefix("+++\n").split("\n+++\n", 1)
        metadata = tomllib.loads(header)
        if field == "body":
            body = value
        else:
            metadata[field] = value
        frontmatter = "\n".join(
            f"{key} = {json.dumps(item, ensure_ascii=False)}"
            for key, item in metadata.items()
        )
        self.note.write_text(f"+++\n{frontmatter}\n+++\n{body}")

    def rejected_publications(self):
        self.generate()
        original = self.tree()
        cases = [
            (
                "config",
                self.config_text + '\nexclude = ["billing::InvoicePayment"]\n',
                "references excluded type",
            ),
            (
                "config",
                self.config_text.replace('notes = ["v1-to-v2.md"]', "notes = []"),
                "missing Impact/Upgrade",
            ),
            ("reviewed", False, "unreviewed changes"),
            ("impact", "", "missing Impact/Upgrade"),
            ("upgrade", "", "missing Impact/Upgrade"),
            ("from", "v99", "unknown adjacent version pair"),
            (
                "config",
                self.config_text.replace(
                    'notes = ["v1-to-v2.md"]', 'notes = ["v1-to-v2.md", "v1-to-v2.md"]'
                ),
                "duplicate release notes",
            ),
            ("config", self.config_text + "\nstrcit = true\n", "unknown field"),
            (
                "config",
                self.config_text.replace('"/reference/"', '"//elsewhere.test/"'),
                "base_url",
            ),
            (
                "config",
                self.config_text + "\nindex_max_bytes = 0\n",
                "must be positive",
            ),
            (
                "config",
                self.config_text.replace(
                    '"/reference/"', '"https://docs.example.test//reference/"'
                ),
                "base_url deployment path",
            ),
            (
                "config",
                self.config_text + "\nindex_max_bytes = 1\n",
                "llms.txt exceeds",
            ),
            (
                "config",
                self.config_text + "\nfull_max_bytes = 1\n",
                "llms-full.txt exceeds",
            ),
            (
                "body",
                "[Upgrade](javascript:alert(1))",
                "unsupported documentation link",
            ),
        ]
        for field in ["body", "impact", "upgrade"]:
            cases.extend(
                [
                    (
                        field,
                        '```http title="Request"\nDELETE /not-a-published-route\n```',
                        "does not match a published HTTP route",
                    ),
                    (
                        field,
                        '```json title="Upgrade"\n{invalid}\n```',
                        "key must be a string",
                    ),
                ]
            )
        for field, value, error in cases:
            self.config.write_text(self.config_text)
            self.note.write_text(self.note_text)
            if field == "config":
                self.config.write_text(value)
            else:
                self.replace_note(field, value)
            result = self.generate(succeeds=False)
            assert error in result.stderr, (field, error, result.stderr)
            assert self.tree() == original, (
                field,
                "failed publication changed the actual output tree",
            )
        fresh = self.root / "never-published"
        self.generate(succeeds=False, output=fresh)
        assert not fresh.exists() or not list(fresh.rglob("*"))
        self.config.write_text(self.config_text)
        self.note.write_text(self.note_text)
        self.generate()
        assert self.tree() == original
