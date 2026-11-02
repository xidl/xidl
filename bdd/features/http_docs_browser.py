"""Browse built Starlight output through a local HTTP server and real Chromium."""

import functools
import json
import re
import threading
from contextlib import contextmanager
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


class DocumentationBrowser:
    def __init__(self, project):
        self.project = project
        self.external_requests = []

    @contextmanager
    def session(self):
        from playwright.sync_api import sync_playwright

        mount = self.project.root / "web"
        mount.mkdir(exist_ok=True)
        reference = mount / "reference"
        if not reference.exists():
            reference.symlink_to(self.project.site / "dist", target_is_directory=True)
        handler = functools.partial(QuietHandler, directory=mount)
        with (
            ThreadingHTTPServer(("127.0.0.1", 0), handler) as server,
            sync_playwright() as playwright,
        ):
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                browser = playwright.chromium.launch()
                try:
                    yield browser, f"http://127.0.0.1:{server.server_port}"
                finally:
                    browser.close()
            finally:
                server.shutdown()
                thread.join(timeout=5)

    def context(self, browser, **options):
        context = browser.new_context(**options)

        # No external service, analytics, CDN or real API request belongs to this journey.
        def local_only(route):
            if urlsplit(route.request.url).hostname == "127.0.0.1":
                route.continue_()
            else:
                self.external_requests.append(route.request.url)
                route.abort()

        context.route("**/*", local_only)
        return context

    def journey(self):
        from playwright.sync_api import expect

        with self.session() as (browser, origin):
            context = self.context(
                browser,
                viewport={"width": 1440, "height": 1000},
                permissions=["clipboard-read", "clipboard-write"],
            )
            page = context.new_page()
            errors = []
            resources = []
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.on(
                "response",
                lambda response: (
                    resources.append(response.url) if response.status >= 400 else None
                ),
            )
            page.goto(origin + "/reference/", wait_until="networkidle")
            expect(page.locator("h1")).to_have_text("Billing API")
            page.locator(
                'main a[href="/reference/changelog/billing/v1-to-v2/"]'
            ).first.click()
            expect(page.locator("h1")).to_contain_text("v1 → v2")
            panel = page.locator("details.xidl-idl-details").first
            expect(panel).not_to_have_attribute("open", "")
            panel.locator("summary").focus()
            page.keyboard.press("Enter")
            expect(panel).to_have_attribute("open", "")
            raw_markdown = (
                self.project.site / "public/changelog/billing/v1-to-v2.md"
            ).read_text()
            expected_declarations = re.findall(
                r"```idl[^\n]*\n(.*?)\n```", raw_markdown, re.S
            )[:2]
            assert len(expected_declarations) == 2
            for index, expected in enumerate(expected_declarations):
                code = panel.locator(".expressive-code").nth(index)
                assert code.locator(".ec-line").count() > 0
                code.get_by_role("button", name="Copy to clipboard").click()
                copied = page.evaluate("navigator.clipboard.readText()")
                assert copied.rstrip("\n") == expected.rstrip("\n"), (expected, copied)
                assert "struct Invoice" in copied
                assert "ApiVersion" not in copied
                assert "@doc" not in copied
                assert code.locator("span[style]").count() > 0
                assert code.locator("pre").evaluate(
                    "element => element.scrollWidth <= element.clientWidth"
                ), (
                    "the full IDL declaration must be readable without horizontal scrolling"
                )
            assert panel.locator(".del, .ins").count() > 0
            page.locator(
                'main a[href="/reference/api/billing/v2/types/Invoice/#field-payments"]'
            ).first.click()
            expect(page.locator("#field-payments")).to_be_visible()
            expect(page.locator("main")).to_contain_text(
                "Use the invoice ID for later lookups."
            )
            expect(page.locator("main p strong").filter(has_text="ID")).to_be_visible()
            expect(
                page.locator("main li").filter(
                    has_text="Read payment details from the selected API version."
                )
            ).to_be_visible()
            page.locator(
                '.expressive-code button[data-code="GET /invoices/inv_001"]'
            ).click()
            assert (
                page.evaluate("navigator.clipboard.readText()").rstrip("\n")
                == "GET /invoices/inv_001"
            )
            page.locator(
                'main a[href="/reference/api/billing/v1/types/Invoice/"]'
            ).first.click()
            expect(page.locator("#field-payment_intent")).to_be_visible()
            page.locator(
                'main a[href^="/reference/changelog/billing/v1-to-v2/#change-"]'
            ).first.click()
            expect(page.locator("h1")).to_contain_text("v1 → v2")
            anchor = page.evaluate("decodeURIComponent(location.hash.slice(1))")
            expect(page.locator(f"[id={json.dumps(anchor)}]")).to_be_visible()
            page.goto(
                origin
                + "/reference/api/payments/unversioned/operations/Payments~3a~3aretrieve/"
            )
            page.locator(
                'main a[href="/reference/api/billing/v2/types/InvoicePayment/"]'
            ).first.click()
            expect(page.locator("h1")).to_have_text("InvoicePayment")
            assert set(
                page.locator('main a[href*="/operations/"]').evaluate_all(
                    "elements => elements.map(element => element.getAttribute('href'))"
                )
            ) == self.project.shared_uses("/")
            raw = page.get_by_role("link", name="View raw Markdown")
            response = context.request.get(origin + raw.get_attribute("href"))
            assert response.ok and "# InvoicePayment" in response.text()
            page.keyboard.press("ControlOrMeta+k")
            search = page.get_by_role("dialog", name="Search").get_by_role(
                "textbox", name="Search"
            )
            expect(search).to_be_visible()
            search.fill("payments")
            # The first query initializes Pagefind's index and WebAssembly.
            expect(page.locator("dialog")).to_contain_text(
                "results for payments", timeout=15000
            )
            expect(
                page.locator('dialog a[href*="/api/payments/"]').first
            ).to_be_visible()
            expect(
                page.locator('dialog a[href*="/api/billing/"]').first
            ).to_be_visible()
            result = page.locator('dialog a[href*="/api/payments/"]').first
            destination = result.get_attribute("href")
            result.focus()
            page.keyboard.press("Enter")
            expect(page).to_have_url(origin + destination)
            page.wait_for_load_state("networkidle")
            expect(page.locator("h1")).to_be_visible()
            assert "/api/" in page.url
            page.keyboard.press("ControlOrMeta+k")
            search = page.get_by_role("dialog", name="Search").get_by_role(
                "textbox", name="Search"
            )
            search.fill("zzqvnotfound7625")
            expect(page.locator("dialog")).to_contain_text("No results", timeout=15000)
            page.keyboard.press("Escape")
            expect(search).not_to_be_visible()
            page.get_by_role("combobox").select_option("dark")
            expect(page.locator("html")).to_have_attribute("data-theme", "dark")
            assert page.evaluate("document.documentElement.scrollWidth <= innerWidth")
            assert not errors, errors
            assert not resources, resources
            context.close()

            mobile = self.context(
                browser,
                viewport={"width": 390, "height": 844},
                is_mobile=True,
                has_touch=True,
            )
            page = mobile.new_page()
            page.goto(origin + "/reference/changelog/billing/v1-to-v2/")
            assert page.evaluate("document.documentElement.scrollWidth <= innerWidth")
            page.locator("details.xidl-idl-details summary").first.click()
            expect(
                page.locator("details.xidl-idl-details[open] pre").first
            ).to_be_visible()
            assert page.evaluate("document.documentElement.scrollWidth <= innerWidth")
            page.get_by_role("button", name="Menu", exact=True).click()
            expect(
                page.get_by_role("link", name="Overview", exact=True)
            ).to_be_visible()
            mobile.close()

            no_script = self.context(browser, java_script_enabled=False)
            page = no_script.new_page()
            page.goto(origin + "/reference/changelog/billing/v1-to-v2/")
            panel = page.locator("details.xidl-idl-details").first
            panel.locator("summary").click()
            expect(panel.locator("pre").first).to_be_visible()
            page.locator(
                'main a[href="/reference/api/billing/v2/types/Invoice/#field-payments"]'
            ).first.click()
            expect(page.locator("#field-payments")).to_be_visible()
            no_script.close()
        self.project.browser_verified = True

    def narrow_search(self):
        from playwright.sync_api import expect

        with self.session() as (browser, origin):
            context = self.context(browser)
            page = context.new_page()
            page.goto(origin + "/reference/")
            page.get_by_role("button", name="Search").click()
            search = page.get_by_role("dialog", name="Search").get_by_role(
                "textbox", name="Search"
            )
            search.fill("payment_intent")
            # Pagefind also returns prefix matches; stale URLs must disappear even
            # when the remaining v2 payment fields match the broader search term.
            expect(page.locator("dialog")).to_contain_text(
                "results for payment_intent", timeout=15000
            )
            assert (
                page.locator(
                    'dialog a[href*="/api/payments/"], dialog a[href*="/v1/"]'
                ).count()
                == 0
            )
            search.fill("InvoicePayment")
            expect(
                page.locator('dialog a[href*="/api/billing/v2/"]').first
            ).to_be_visible()
            assert (
                page.locator(
                    'dialog a[href*="/api/payments/"], dialog a[href*="/v1/"]'
                ).count()
                == 0
            )
            context.close()

    def literal_content(self):
        from playwright.sync_api import expect

        with self.session() as (browser, origin):
            context = self.context(
                browser, permissions=["clipboard-read", "clipboard-write"]
            )
            page = context.new_page()
            page.goto(
                origin + "/reference/changelog/billing/v1-to-v2/",
                wait_until="networkidle",
            )
            assert page.evaluate("window.xidlUnsafe") is None
            assert (
                page.locator(
                    "main script:not([src]), main img[onerror], main [onclick]"
                ).count()
                == 0
            )
            assert page.locator("main script[src]").evaluate_all(
                "scripts => scripts.every(script => script.type === 'module' && new URL(script.src).pathname.startsWith('/reference/_astro/'))"
            )
            expect(page.locator("#author-heading")).to_be_visible()
            expect(page.locator("#request----response")).to_be_visible()
            expect(page.locator("main")).to_contain_text(
                "<script>window.xidlUnsafe = true</script>"
            )
            code = page.locator(".expressive-code").filter(has_text="exampleOnly()")
            code.get_by_role("button", name="Copy to clipboard").click()
            assert (
                page.evaluate(
                    "navigator.clipboard.readText()",
                ).rstrip("\n")
                == "<script>exampleOnly()</script>"
            )
            page.goto(origin + "/reference/api/billing/v2/types/Invoice/")
            assert page.evaluate("window.xidlDocUnsafe") is None
            assert page.locator("main script:not([src])").count() == 0
            expect(page.locator("main")).to_contain_text(
                "<script>window.xidlDocUnsafe = true</script>"
            )
            context.close()
        self.project.literal_verified = True

    def restored_search(self):
        from playwright.sync_api import expect

        with self.session() as (browser, origin):
            context = self.context(browser)
            page = context.new_page()
            page.goto(origin + "/reference/")
            page.get_by_role("button", name="Search").click()
            search = page.get_by_role("dialog", name="Search").get_by_role(
                "textbox", name="Search"
            )
            search.fill("payment_intent")
            result = page.locator('dialog a[href*="/api/billing/v1/"]').first
            expect(result).to_be_visible()
            result.click()
            expect(page.locator("#field-payment_intent")).to_be_visible()
            context.close()
