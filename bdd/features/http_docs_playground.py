"""Exercise generated Scalar pages against a local, contract-aware HTTP service."""

import json
import re
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from http_docs_browser import DocumentationBrowser


class PlaygroundApi(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_OPTIONS(self):
        self.respond(204, None)

    def do_GET(self):
        version = self.headers.get("X-API-Version")
        self.server.requests.append((self.path, version))
        if self.path == "/invoices/inv_1" and version in ["2026-01-01", "2026-04-01"]:
            invoice = {"id": "inv_1"}
            if version == "2026-01-01":
                invoice["payment_intent"] = "pi_1"
            else:
                invoice["payments"] = [{"id": "pay_1", "amount": 2500}]
            self.respond(200, invoice)
        elif self.path == "/payments/pay_1" and version is None:
            self.respond(200, {"id": "pay_1", "amount": 2500})
        else:
            self.respond(404, {"code": 404, "msg": "Record not found"})

    def respond(self, status, value):
        self.send_response(status)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, OPTIONS")
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        if value is not None:
            self.wfile.write(json.dumps(value).encode())


class DocumentationPlayground:
    def __init__(self, project):
        self.project = project
        self.browser = DocumentationBrowser(project)

    def journey(self):
        with ThreadingHTTPServer(("127.0.0.1", 0), PlaygroundApi) as service:
            service.requests = []
            thread = threading.Thread(target=service.serve_forever, daemon=True)
            thread.start()
            try:
                for name in ["billing.idl", "payments.idl"]:
                    path = self.project.source / name
                    path.write_text(
                        f"#pragma xidlc service http://127.0.0.1:{service.server_port} BDD\n"
                        + path.read_text()
                    )
                self.project.config.write_text(
                    self.project.config_text.replace(
                        'base_url = "/reference/"',
                        'base_url = "https://docs.example.test/reference/"',
                    )
                )
                self.project.generate()
                self.assert_documents()
                self.project.build()
                self.browse(service)
            finally:
                service.shutdown()
                thread.join(timeout=5)
        self.project.playground_verified = True

    def assert_documents(self):
        canonical = "https://docs.example.test/reference/playground/billing/v1/"
        for path in ["changelog/billing/v1-to-v2.md", "llms-full.txt"]:
            assert canonical in (self.project.site / "public" / path).read_text()
        for unit, version in [
            ("billing", "v1"),
            ("billing", "v2"),
            ("payments", "unversioned"),
        ]:
            path = self.project.site / f"public/openapi/{unit}/{version}.json"
            document = json.loads(path.read_text())

            # Each downloadable version works without any sibling JSON document.
            def references(value):
                if isinstance(value, dict):
                    if "$ref" in value:
                        ref = value["$ref"]
                        assert ref.startswith("#/components/schemas/"), ref
                        assert (
                            ref.removeprefix("#/components/schemas/")
                            in document["components"]["schemas"]
                        )
                    for child in value.values():
                        references(child)
                elif isinstance(value, list):
                    for child in value:
                        references(child)

            references(document)
            if unit == "payments":
                assert "X-API-Version" not in path.read_text()
                assert any(
                    schema.get("title") == "InvoicePayment"
                    for schema in document["components"]["schemas"].values()
                )

    def browse(self, service):
        from playwright.sync_api import expect

        with self.browser.session() as (browser, origin):
            context = self.browser.context(browser, color_scheme="light")
            context.add_init_script("localStorage.setItem('colorMode', 'light')")
            page = context.new_page()
            errors = []
            failures = []
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.on(
                "response",
                lambda response: (
                    failures.append(response.url)
                    if response.status >= 400
                    and not response.url.startswith(
                        f"http://127.0.0.1:{service.server_port}/"
                    )
                    else None
                ),
            )
            page.goto(origin + "/reference/changelog/billing/v1-to-v2/")
            expect(
                page.get_by_role("link", name="Explore 2026-01-01 HTTP API")
            ).to_have_attribute("href", "/reference/playground/billing/v1/")
            page.goto(
                origin + "/reference/api/billing/v1/operations/Invoices~3a~3aretrieve/"
            )
            for name in ["View raw Markdown", "AI index"]:
                link = page.get_by_role("link", name=name, exact=True)
                assert link.get_attribute("href").startswith("/reference/")
                assert context.request.get(origin + link.get_attribute("href")).ok
            page.get_by_role("link", name="Open version playground").click()
            expect(page.locator("h1")).to_contain_text("2026-01-01")
            expect(page.locator(".scalar-api-reference")).to_contain_text(
                "payment_intent"
            )
            assert not service.requests, "opening a playground must not call the API"
            self.theme(page, "dark")
            self.send(page, "/invoices/{id}", "inv_1", 200, "payment_intent")
            self.close_client(page)
            page.get_by_role(
                "link", name="Billing API · 2026-04-01", exact=True
            ).first.click()
            expect(page.locator("h1")).to_contain_text("2026-04-01")
            expect(page.locator(".scalar-api-reference")).to_contain_text("payments")
            expect(page.locator(".scalar-api-reference")).not_to_contain_text(
                "payment_intent"
            )
            assert len(service.requests) == 1
            expect(page.locator("body")).to_have_class(re.compile(r"\bdark-mode\b"))
            self.theme(page, "light")
            self.theme(page, "auto", "light")
            self.send(page, "/invoices/{id}", "inv_1", 200, "pay_1")
            page.emulate_media(color_scheme="dark")
            expect(page.locator("html")).to_have_attribute("data-theme", "dark")
            expect(page.locator("body")).to_have_class(re.compile(r"\bdark-mode\b"))
            expect(page.get_by_role("dialog")).to_contain_text("pay_1")
            assert len(service.requests) == 2
            self.send(
                page, "/invoices/{id}", "missing", 404, "Record not found", opened=True
            )
            self.close_client(page)
            page.emulate_media(color_scheme="light")
            expect(page.locator("html")).to_have_attribute("data-theme", "light")
            expect(page.locator("body")).to_have_class(re.compile(r"\blight-mode\b"))
            page.get_by_role(
                "link", name="Payments API · unversioned", exact=True
            ).first.click()
            expect(page.locator("h1")).to_contain_text("unversioned")
            expect(page.locator(".scalar-api-reference")).not_to_contain_text(
                "X-API-Version"
            )
            assert len(service.requests) == 3
            self.send(page, "/payments/{id}", "pay_1", 200, "2500")
            assert service.requests == [
                ("/invoices/inv_1", "2026-01-01"),
                ("/invoices/inv_1", "2026-04-01"),
                ("/invoices/missing", "2026-04-01"),
                ("/payments/pay_1", None),
            ]
            assert not self.browser.external_requests, self.browser.external_requests
            assert not errors, errors
            assert not failures, failures
            assert page.evaluate("localStorage.getItem('colorMode')") == "light"
            context.close()

    def theme(self, page, mode, resolved=None):
        from playwright.sync_api import expect

        page.get_by_role("banner").get_by_role("combobox").select_option(mode)
        expected = resolved or mode
        expect(page.locator("html")).to_have_attribute("data-theme", expected)
        expect(page.locator("body")).to_have_class(re.compile(rf"\b{expected}-mode\b"))

    def send(self, page, route, value, status, text, opened=False):
        from playwright.sync_api import expect

        if not opened:
            page.locator(".scalar-api-reference").get_by_role("link").filter(
                has_text=route
            ).first.click()
            # A scrolled Scalar sidebar must leave Starlight's fixed header clickable.
            page.locator("header").get_by_role("button", name="Search").first.click()
            expect(page.get_by_role("dialog", name="Search")).to_be_visible()
            page.keyboard.press("Escape")
            page.get_by_role("button", name="Test Request").filter(
                has_text=f"(get {route})"
            ).click()
        dialog = page.get_by_role("dialog")
        dialog.get_by_role("combobox", name="Value", exact=True).first.fill(value)
        with page.expect_response(
            lambda response: (
                response.request.method == "GET"
                and response.url.endswith(route.replace("{id}", value))
            )
        ) as response:
            dialog.get_by_role("button", name="Send get request").click()
        assert response.value.status == status
        expect(dialog).to_contain_text(text)

    def close_client(self, page):
        from playwright.sync_api import expect

        page.get_by_role("dialog").get_by_role(
            "button", name="Close Client", exact=True
        ).click()
        expect(page.get_by_role("dialog")).not_to_be_visible()
