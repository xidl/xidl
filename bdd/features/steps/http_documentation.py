from behave import given, then, when

from http_docs_project import DocumentationProject
from http_docs_browser import DocumentationBrowser
from http_docs_playground import DocumentationPlayground


@given("an isolated multi-root HTTP documentation project")
def isolated_project(context):
    context.documentation = DocumentationProject(context)


@when("I publish the manifest, its directory, and the reversed explicit roots")
def equivalent_roots(context):
    project = context.documentation
    project.generate()
    context.documentation_original = project.owned()
    for inputs in [
        [project.source],
        [project.source / "payments.idl", project.source / "billing.idl"],
    ]:
        project.generate(inputs=inputs)
        assert project.owned() == context.documentation_original


@then("all publication bytes and shared versioned contracts agree")
def shared_publication(context):
    context.documentation.assert_shared_contract()


@when("I build and browse the production Starlight project")
def build_and_browse(context):
    project = context.documentation
    project.build()
    DocumentationBrowser(project).journey()


@then("comparison navigation, search, copying, mobile, and no-script reading work")
def browser_verified(context):
    assert context.documentation.browser_verified


@when("I publish all roots, then only billing v2, twice")
def narrow_publication(context):
    project = context.documentation
    project.generate()
    context.documentation_original = project.owned()
    project.build()
    for name, content in [
        ("public/robots.txt", "User-Agent: *\nDisallow: /private/\n"),
        ("notes/user.md", "Private author notes\n"),
    ]:
        file = project.site / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content)
    project.generate(inputs=[project.source / "billing.idl"], versions=["v2"])
    narrow = project.owned()
    project.generate(inputs=[project.source / "billing.idl"], versions=["v2"])
    assert project.owned() == narrow


@then(
    "stale references, comparisons, and search entries disappear while user files survive"
)
def stale_cleanup(context):
    project = context.documentation
    files = project.owned()
    for stale in context.documentation_original.keys() - files.keys():
        assert not (project.site / stale).exists(), stale
    for name in files:
        assert (
            "/v1/" not in name and "/payments/" not in name and "v1-to-v2" not in name
        ), name
    assert (project.site / "notes/user.md").read_text() == "Private author notes\n"
    dist = project.build()
    assert (dist / "robots.txt").read_text().startswith("User-Agent:")
    assert not (dist / "api/billing/v1/index.html").exists()
    assert not (dist / "api/payments/unversioned/index.html").exists()
    assert not (dist / "changelog/billing/v1-to-v2/index.html").exists()
    assert not (dist / "playground/billing/v1/index.html").exists()
    assert not (dist / "playground/payments/unversioned/index.html").exists()
    assert not (dist / "openapi/billing/v1.json").exists()
    assert not (dist / "openapi/payments/unversioned.json").exists()
    assert (dist / "playground/billing/v2/index.html").is_file()
    assert b"payment_intent" not in files["public/llms-full.txt"]
    DocumentationBrowser(project).narrow_search()


@when("I restore the full manifest publication")
def restore_full(context):
    context.documentation.generate()


@then("the original generated publication is restored byte for byte")
def restored_publication(context):
    assert context.documentation.owned() == context.documentation_original
    context.documentation.assert_shared_contract()
    context.documentation.build()
    DocumentationBrowser(context.documentation).restored_search()


@when("I attempt invalid release notes, exclusions, settings, and example requests")
def invalid_releases(context):
    context.documentation.rejected_publications()


@then("each failure preserves the last complete publication")
def rejected_publication_verified(context):
    # Every attempt in the journey checks all owned bytes and the ownership manifest.
    assert context.documentation.owned()


@when("I publish author HTML and code examples after correcting the release")
def literal_author_content(context):
    project = context.documentation
    billing = project.source / "billing.idl"
    billing.write_text(
        billing.read_text().replace(
            "/// Invoice returned by the billing API.",
            "/// <script>window.xidlDocUnsafe = true</script>\n"
            "///\n/// Invoice returned by the billing API.",
        )
    )
    project.replace_note(
        "body",
        project.note_text.split("\n+++\n", 1)[1]
        + '\n\n<script>window.xidlUnsafe = true</script>\n\n<img src="missing" onerror="window.xidlUnsafe = true">\n\n## Author heading {#author-heading onclick="window.xidlUnsafe=true"}\n\n## Request -- response\n\n[Request notes](#request----response)\n\n```html\n<script>exampleOnly()</script>\n```\n',
    )
    project.generate()
    project.build()
    DocumentationBrowser(project).literal_content()


@then("author markup stays literal and code examples remain copyable")
def author_content_verified(context):
    assert context.documentation.literal_verified


@when("I browse generated version playgrounds against a local HTTP service")
def version_playgrounds(context):
    DocumentationPlayground(context.documentation).journey()


@then("successful and missing records preserve each API's selected version")
def playground_verified(context):
    assert context.documentation.playground_verified
