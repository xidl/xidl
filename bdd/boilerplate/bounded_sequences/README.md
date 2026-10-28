# Bounded invoice batches

The included billing module shares constant expressions, aliases, nested
batches, map values, fixed arrays and a negotiated JSON/binary export. Both
generated SDKs and Hurl exercise empty, partial, maximum and oversized input,
unchanged service state after rejection, invalid output, response metadata and
hostile providers.

The Rust SDK also covers bounded aliases inside JSON header/cookie maps, where
wire structs must compile and validate nested elements. TypeScript composite
text metadata decoding remains outside #399; its existing runtime passes JSON
text straight to object schemas (tracked by the broader #320 conformance work).
