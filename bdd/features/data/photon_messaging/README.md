# Photon messaging regression corpus

Source: user-supplied `xidl-versioned-fixture.zip`, SHA-256
`bd4a8f1c8315b9c1f2f346b59b4e8fa01101519c5b8b24f4c17790abac8f5fe6`.

The IDL and negative inputs are preserved verbatim. The original negative
expectations and 0.104.0 release hashes retain their values with repository JSON
formatting. Parser tests own projection and round-trip assertions. Compiler
tests own complete OpenAPI snapshots. BDD owns real CLI exit codes,
deterministic output, version selection and business flows.

The archive checker and its substring-based known-gap exemptions are
deliberately not a test gate: command failures must fail, and each supported
behavior has an explicit assertion. No command here updates the historical
hashes.

The archived expectations are evidence of the original report, not a new product
specification. In particular:

- Bare `@version` declares no transport location or default header.
- Version maps retain declaration replacements, including `legacy_send`, and
  contain only direct facts; inherited/reference changes are not propagated.
- `lint` and `gen` remain independent. N12/N15 are lint errors.
- Correcting the v2026_09 `Idempotency-Key` binding changes that version's old
  digest; the new regression snapshot records the correction, while this
  historical lock remains unchanged.
- Go versioned generation, raw TypeScript upgrades and union wire formats are
  independent target capabilities or known limitations. Inherited transport
  fields are tracked separately in
  [#361](https://github.com/xidl/xidl/issues/361); these snapshots do not
  establish a new inheritance wire format.
