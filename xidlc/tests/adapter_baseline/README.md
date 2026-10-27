# <<<<<<< HEAD These complete artifact manifests were regenerated from the branch that widened `@deprecated` sites (#369), which adds `deprecated` to the generated transport allow lists. The Rust manifests include the intentional #390 contract correction: absent required query, header and cookie scalars fail extraction instead of acquiring defaults, and empty sequences retain their zero-occurrence encoding. Unary and streaming handlers share metadata decoding. The TypeScript manifests retain the original `d98bc37a` baseline.

These complete artifact manifests use
`XIDLC_VERSION=0.0.1
XIDLC_GIT_HASH=snapshot` and both client and server
generation enabled. The manifests include the intentional #390 contract
correction: absent required query, header and cookie scalars fail extraction
instead of acquiring defaults; empty sequences retain their zero-occurrence
encoding. Unary and streaming handlers share metadata decoding. The #393
collection correction uses repeated-value Rust query codecs, preserves
explicitly empty optional values, and retains logical optionality in TypeScript
request descriptors. Simple string header collections reject elements their
comma encoding cannot represent. The tests compare paths and contents byte for
byte; adapter generation is never involved in producing the baseline.

>>>>>>> 13d80df1 (fix(http): preserve collection wire occurrences)

To regenerate after an intentional change to ordinary output, build `xidlc` in a
clean checkout of the new baseline, then run for each target and fixture:

```sh
xidlc gen --out-dir <empty-directory> <target> --client --server <fixture.idl>
```

with `XIDLC_VERSION=0.0.1 XIDLC_GIT_HASH=snapshot` and both client and server
generation enabled. Serialize the resulting `{filename: full_content}` map as
sorted JSON with two space indentation and a trailing newline. Targets are
`typescript-rest` and `rust-axum`; fixtures are the five IDL files identified by
manifest filenames. The tests compare paths and contents byte for byte; adapter
generation is never involved in producing the baseline.
