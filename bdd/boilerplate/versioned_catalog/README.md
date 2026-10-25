# Catalog filtering across representations

This reduced business contract is inspired by Shopify's
[2026-09-15 Catalog media change](https://shopify.dev/changelog/posts/filter-catalog-search-results-by-media-type).
The published example adds video/model media, combines media filters with AND,
and ignores unknown media types with a `not_found` message. Stable clients can
filter on rich media while their responses still contain images only.

This fixture expresses that business change with two concrete XIDL versions. It
is a small acceptance application, not an implementation of Shopify's API. One
immutable Head catalog owns all assets. Filtering runs there before pure
projection removes non-image assets for the old contract. In particular, a
video-only product remains a match with an empty old media list. Filtering after
projection would incorrectly remove it. Both `VersionedMode::{Chain, Route}` use
the same catalog and hooks. Route selects generated per-version HTTP routers;
Chain constructs an envelope for generated dispatch. Only the front ends differ.

The shared Hurl journey checks repeated AND filters, reordered and duplicated
requirements, unknown filters, tenant isolation, errors, and version headers.
Each actual response is checked against its selected generated OpenAPI document.
Explicit checks also ensure that media `kind` never leaks into old responses.
Independent Rust and TypeScript SDKs are generated for each version, make
concurrent old/new reads, and verify that repeated historical projections leave
the Head result unchanged. Empty filter arrays exercise zero-occurrence query
encoding through both SDKs and both application modes.

The fixture deduplicates unknown-type messages in first-occurrence order and
treats an explicitly empty string as an unknown type; these are its own stated
business rules. A query value containing a comma is one value, not two filters.
The application resolves exact version dates and defaults to `2026-10-01`;
generated adapters do not own that policy. Tenant cookies are supplied fixture
identities. Production authentication and persistent catalog IO remain
application responsibilities. No network resource or SaaS credential is needed
to run it.

The same Head also accepts a separately submitted single-media preference.
`GET /catalog?remember=...` returns the usual search body and an optional
`bookmark` response cookie; `PUT /catalog/preference?media=...` returns only
that cookie with a 204 and no body. Neither operation uses the preference to
filter the catalog. An omitted preference emits no `Set-Cookie`, an explicit
empty string writes `bookmark=`, and `image`, `video`, or `model_3d` writes that
value. Unknown preferences and tenants are denied without writing a cookie.
Repeating the same update is deterministic and never changes Head assets.

This response metadata pattern is inspired by Hydrogen's application-owned
[cart ID cookie setter](https://github.com/Shopify/hydrogen/blob/2026-04/packages/hydrogen/src/cart/cartSetIdDefault.ts).
The reduced preference rules above belong to this fixture. An empty cookie value
is valid under
[RFC 6265 §4.1.1](https://www.rfc-editor.org/rfc/rfc6265.html#section-4.1.1); it
does not delete the cookie. Expiration, browser persistence and production
cookie attributes remain application policy. Hurl checks raw headers, empty 204
bodies and rejected updates; both independently selected SDK generations check
the three decoded presence states through Chain and Route. The Chain front end
uses the same published `DeserializeFactory::deserialize_present` text decoder
as generated routers, so supplied empty text stays present.
