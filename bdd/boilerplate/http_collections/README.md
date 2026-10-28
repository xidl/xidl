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
exercises direct sequences, chained collection typedefs and scalar numeric
aliases. Character headers cover direct and typedef'd lists, repeated header
fields, scalar presence and ASCII wchar values. Delimiter and whitespace
characters cannot be simple header elements; the SDKs reject them before
sending. Encoded cookies retain these values and Unicode. Rust also checks
complete character parsing against empty, multiple-character and quoted text.
TypeScript's character schema currently accepts unrestricted strings; that
remaining validation difference is tracked in
[issue #320](https://github.com/xidl/xidl/issues/320). Complex collection
elements remain outside this fixture. Bounded-sequence semantics are tracked in
[issue #399](https://github.com/xidl/xidl/issues/399); this corpus exercises
unbounded lists and makes no exact-length assumption.

The [issue #401](https://github.com/xidl/xidl/issues/401) regression
distinguishes empty and whitespace-only numeric text from an explicit zero.
Scalar and sequence integers accept signed decimal digits; fractional, exponent,
hexadecimal and trailing-newline integer text fails. Floating-point headers
accept decimal fractions and exponents. Unsigned cookie text accepts an optional
plus sign and rejects a minus sign, including `-0`. Optional numeric absence
stays absent, while a supplied empty value fails. The same journey runs against
both generated servers; both SDKs also check zero and optional numeric values.
Numeric widths and nonfinite float policy are separate constraints from this
text-decoding regression.
