# Collection occurrences

This fixture checks the generated Rust and TypeScript HTTP servers and SDKs
against the same Hurl journey. Logical sequence parameters remain required SDK
arguments. Unbounded query, header and cookie bindings allow zero wire
occurrences, which represent `[]`; generated OpenAPI therefore declares those
parameters `required: false`. Required scalars and path parameters retain
presence checks.

For optional collections, zero occurrences represent absence. `Some([])` and
`None` (or `[]` and `undefined`) have the same wire encoding and normalize to
absence. One explicit empty string remains a one-element list. Repeated values
preserve order and duplicates. Optional scalar query values also distinguish
absence from a present empty string. Invalid numeric values fail before
dispatch.

Query and cookie collections use repeated names. Simple header collections use
commas; surrounding separator whitespace is removed. A string element containing
a literal comma or surrounding whitespace cannot round-trip through this
encoding, so both SDKs reject it before sending. Unicode and reserved characters
in query/cookie values use their normal transport encoding. The fixture
exercises direct string and integer sequences. Typedef sequences, character
headers and complex collection elements still need separate cross-target
contract coverage. Bounded-sequence semantics are tracked in
[issue #399](https://github.com/xidl/xidl/issues/399); this corpus exercises
unbounded lists and makes no exact-length assumption. The empty numeric-text
coercion mismatch is tracked in
[issue #401](https://github.com/xidl/xidl/issues/401); this fixture does not
equate an empty numeric element with an explicit zero.
