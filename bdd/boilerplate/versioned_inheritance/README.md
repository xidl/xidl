# Inherited address contracts

The address example keeps one Head model and append-only delivery facts. Its
business rules run through Rust Chain and Route servers, independently generated
Rust/TypeScript SDKs, and a separate TypeScript adapter runtime journey.

The country-code change is inspired by Shopify's
[MailingAddress](https://shopify.dev/docs/api/storefront/2026-10/objects/MailingAddress):
`countryCode` is a deprecated string and `countryCodeV2` is a country enum. This
fixture owns its release dates, inheritance model and delivery rules; it does
not claim that Shopify removed either field on those dates.

| Release    | Address contract                                                                                               |
| ---------- | -------------------------------------------------------------------------------------------------------------- |
| 2026-01-01 | `line`, string `countryCode`, required `deliveryReference`, optional `deliveryNote`                            |
| 2026-04-01 | `street` replaces `line`; `countryCodeV2` replaces the string through an explicit hook                         |
| 2026-07-01 | Delivery facts leave Head; readers retain them for older versions. New callers can choose `signatureRequired`. |

`Identity`, `Postal` and `Address` carry ancestor identity, postal fields and
local priority. An empty marker and recursive prior anchors exercise base
construction. The PATCH hierarchy changes independently and uses the generated
stored bag. The shadowing `shipping::Identifier` is numeric; the inherited id
remains a string. The shared library uses local `Identifier` and recursive
`Anchor` references; inheritance preserves their declaring scope through package
mounts, independent SDK selection and imported consumers.

`VersionedMode` belongs to the example application. Chain uses generated JSON
dispatchers. Route selects one of three ordinary generated `AddressesServer`
routers and calls typed upcast/downcast functions. Both call `Store::update`.
The application resolves exact version headers, authorizes fixed test
credentials against the tenant, and serializes
current/read/convert/write/project as one transaction. Hooks perform no I/O and
select no version. This in-memory example uses a process-wide async mutex; a
deployed application would use its storage transaction boundary.

The shared Hurl journey checks 19 requests in each mode: old writes and new
reads, new writes and old reads, absent versus empty notes, zero priority, both
round-trip rules, unchanged historical rows, tenant/id isolation, missing
resources, missing required historical facts, and a refused country conversion
that cannot write. Every API response is checked against the requested version's
generated OpenAPI, including its version echo and content type. SDK journeys
also overlap old/new writes and check that each response uses the facts from its
own commit.

Current ordinary-target boundaries remain explicit:

- Rust ordinary routers ignore unknown JSON members; adapter wire decoders
  reject them. The mode-boundary step verifies both results and verifies that
  hidden fields do not change Head or history.
- Recursive anchors use a sequence. Direct optional recursion currently fails in
  ordinary Rust REST wire codecs
  ([#407](https://github.com/xidl/xidl/issues/407)).

Focused entry point, after building the pinned compiler and TypeScript runtimes:

```sh
behave bdd/features/version_inheritance.feature
```
