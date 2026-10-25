The address service assembles the same contract as
[`../versioned_inheritance.idl`](../versioned_inheritance.idl). That single-file
contract remains the generator comparison oracle and snapshot input.

- `service/xidl.toml` selects one entry file and mounts editable sibling
  libraries.
- `calendar` owns the timeline; `identity` owns shared types and historical
  facts.
- `shipping` owns address models, errors and operations. Includes inside a
  module keep its scope; nested includes resolve relative to the including file.
- `readers` imports the service as a separate compilation unit. The old reader
  has only an interface; the Head reader also has a local DTO. Both depend on
  the service's generated types and validators. Their OpenAPI responses
  reference the corresponding external version documents.

Dependencies are declared explicitly in each root manifest. Mounting a path does
not recursively load another manifest's dependencies. `#include` splices
definitions into the current unit, while `import` generates another unit. Nested
imported types use the complete `unit::version::module::Type` reference. The
imported model fixes the consumer's shape; HTTP version selection remains
application policy in `ts/imports.ts`.

`version_inheritance.feature` runs the existing Head store, adapters, Hurl
journey and independently selected Rust/TypeScript SDKs from these packages in
both Chain and Route modes. The imported readers additionally verify that Head
bytes fail the old response validator and that reads leave the business audit
intact. No business implementation or Hurl journey is duplicated for the package
layout.

The address evolution is inspired by Shopify's deprecated string `countryCode`
and enum `countryCodeV2` in
[MailingAddress](https://shopify.dev/docs/api/storefront/2026-10/objects/MailingAddress).
Release dates, inheritance and retained delivery history are fixture policy.

The retained `deliveryReference` uses the root string alias
`::DeliveryReference`, while `geo` deliberately declares a same-named integer
alias. Old writes and responses must continue to use the root string contract. A
numeric PATCH is rejected before storage changes in both modes. The operations
also use absolute exception paths; the existing HTTP and SDK journeys exercise
their 403/404/422 responses after version projection.

The address also carries a recorded handoff chain: an optional `Anchor.parent`
points to another checkpoint, alongside the existing `previous` sequence. Old
and Head SDKs read finite chains with omitted leaves. A PATCH can replace the
parent chain through the shared Head store; omission preserves it. This current
handoff chain is separate from append-only retired delivery facts in `history`.
The shared journey checks nested round trips, repeated writes, deep malformed
leaves and cross-tenant refusal through both Chain and Route fronts. Imported
readers retain the same recursive parent data and generated OpenAPI shape.
