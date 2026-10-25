# Versioned invoice payments

This reduced business contract is inspired by Stripe's
[2025-03-31 partial invoice payments change](https://docs.stripe.com/changelog/basil/2025-03-31/add-support-for-multiple-partial-payments-on-invoices).
Stripe removed singular invoice payment references and introduced invoice
payments. This fixture uses that change to exercise a singular `payment_intent`
becoming a `payments` collection. It is an XIDL acceptance fixture, with its own
business rules and HTTP errors.

`VersionedMode::{Chain, Route}` changes the HTTP entry point. Chain constructs
wire envelopes and calls generated dispatch adapters. Route selects the
generated `v1::InvoicesServer` or `v2::InvoicesServer`, then uses typed adjacent
request and outcome conversions. Both paths use the same Head-only `Ledger` and
pure `Hooks`. The application reads the version header, chooses an exact
version, defaults a missing header to the concrete `2025-04-01` version, and
echoes it. Generated adapters contain no HTTP version-selection policy.

The shared Hurl journey checks missing required inputs before business dispatch,
invalid amounts, balance limits, duplicate payment intents, conflicting retries,
tenant isolation, empty lists, declared errors and unsupported projections. A
second partial payment commits before its response is projected. The old shape
cannot represent two payment intents, so the application returns its declared
422 error with `Invoice.payment_intent`. Retries reuse the committed Head
snapshot; they cannot capture the payment twice. Replaying the first payment
after later changes still returns that first payment's original outcome.

Rust and TypeScript clients are generated independently with `--api-version v1`
and `--api-version v2`. They call the same running endpoint. Concurrent initial
requests share an idempotency key across versions and must produce one commit.
Each SDK uses its own invoice so it exercises fresh writes. Every actual Hurl
response body is checked against the appropriate generated OpenAPI document,
including validation and business errors. Selected OpenAPI documents also have
an owning-crate insta snapshot and must equal their complete-history document.

The cookie identifies a fixture tenant. This demonstrates isolation of invoice
and idempotency state; a production application must authenticate its own tenant
identity. State is deliberately in memory and reset for each scenario, with no
external payment provider or database. The rules reject a singular projection of
several payments; they do not choose a replacement payment intent.

Run from the repository root after installing the normal BDD prerequisites:

```sh
XIDLC_VERSION=0.0.1 XIDLC_GIT_HASH=snapshot cargo build -p xidlc --features cli
behave bdd/features/versioned_billing.feature bdd/features/http_required_parameters.feature
XIDLC_VERSION=0.0.1 XIDLC_GIT_HASH=snapshot cargo test -p xidlc --all-features --test versioned_billing
```

The required-parameters fixture covers unary query/header/cookie scalars,
optional omission, explicit zero/true/empty values, and server-stream inputs.
The Rust SDK also verifies explicit false. TypeScript false decoding belongs to
the separate fix in [PR #391](https://github.com/xidl/xidl/pull/391). A missing
path segment is a routing 404. Rust inbound query deserialization defaults
optional members and sequences; an empty sequence contributes no parameter
occurrences. Header/cookie sequences also decode zero occurrences as empty
lists. OpenAPI presence declarations and empty-list serialization need the
separate cross-target contract decision tracked in
[issue #393](https://github.com/xidl/xidl/issues/393).

OpenAPI schema validation checks declared fields, media types and response
headers. The generated object schemas permit additional properties. Hurl
therefore explicitly checks the absence of `payments` in legacy successes and
the absence of `payment_intent` in current successes to prove that versions do
not leak each other's fields.

The generator crate passes strict clippy. Compiling emitted Rust artifacts with
the same lint gate currently exposes generator warnings tracked in
[issue #394](https://github.com/xidl/xidl/issues/394); normal generated builds
and the HTTP/SDK journeys pass.
