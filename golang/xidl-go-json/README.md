# xidl-go-json

A reflection-based JSON encoder/decoder for Go. It uses the `xjson` struct tag
and is wire-compatible with `encoding/json`.

## Tag syntax

```go
type Example struct {
    Field string `xjson:"name,option1,option2"`
}
```

| Option      | Description                                                  |
|-------------|--------------------------------------------------------------|
| `name`      | Override the JSON key name.                                  |
| `omitempty`  | Omit the field when it holds its zero value.                 |
| `string`    | Encode/decode a numeric or boolean value as a JSON string.   |
| `flatten`   | Promote child fields into the enclosing JSON object.         |
| `-`         | Skip the field unconditionally.                              |

## Flatten

The `flatten` option inlines a field's contents into the parent JSON object
instead of nesting them under a separate key. Three categories of types may
carry the option:

| Field type          | Category       | Allowed count per struct |
|---------------------|----------------|--------------------------|
| struct / \*struct    | struct flatten  | unlimited                |
| map[string]T        | catch-all       | at most **one**          |
| any (interface{})   | catch-all       | at most **one**          |

A struct may contain any number of struct-flatten fields **plus** at most one
catch-all flatten field. If more than one catch-all flatten field is declared,
`Marshal` and `Unmarshal` return an error.

### Struct flatten

A struct (or pointer-to-struct) field tagged with `flatten` behaves like a Go
embedded (anonymous) field: its children are promoted into the parent object.

```go
type Address struct {
    Street string `xjson:"street"`
    City   string `xjson:"city"`
}

type User struct {
    Name    string  `xjson:"name"`
    Address Address `xjson:"address,flatten"`
}

// Marshal(User{Name: "Alice", Address: Address{Street: "1st Ave", City: "NY"}})
// => {"name":"Alice","street":"1st Ave","city":"NY"}
```

Multiple struct-flatten fields are allowed and follow the same depth-based
conflict resolution rules as Go embedded structs: if two promoted fields share
the same JSON key at the same depth, both are silently dropped.

### Catch-all flatten (`map[string]T` / `any`)

A `map[string]T` or `any` field tagged with `flatten` acts as a **catch-all
bucket** for keys that do not match any named struct field.

```go
type Event struct {
    Type  string         `xjson:"type"`
    Extra map[string]any `xjson:",flatten"`
}

// Marshal(Event{Type: "click", Extra: map[string]any{"x": 10, "y": 20}})
// => {"type":"click","x":10,"y":20}
//
// Unmarshal({"type":"click","x":10,"y":20}, &Event{})
// => Event{Type: "click", Extra: map[string]any{"x": 10, "y": 20}}
```

#### Rules

1. **At most one catch-all per struct.** Declaring two or more `map` or `any`
   flatten fields in the same struct is an error.

2. **Named fields take priority.**
   - During **marshal**, map keys that collide with a named struct field's JSON
     key are silently skipped.
   - During **unmarshal**, a JSON key that matches a named struct field is
     always routed to that field; only unmatched keys are collected into the
     catch-all.

3. **Map key type must be `string`.** A flatten field of type `map[int]any` or
   any other non-string-keyed map is an error.

4. **`any` flatten — marshal behaviour.** The concrete value stored in the
   `any` field is inspected at runtime:
   - `nil` — no extra keys are emitted.
   - `map[string]T` — treated as a map catch-all (same as above).
   - struct — treated as a struct flatten.
   - anything else — returns an error.

5. **`any` flatten — unmarshal behaviour.** All unmatched keys are collected
   into a freshly allocated `map[string]any` and assigned to the field.

6. **Struct-flatten fields are resolved first.** When a struct contains both
   struct-flatten fields and a catch-all flatten field, the promoted struct
   fields are considered named fields for the purpose of rules 2 and 5.

### Conflict resolution summary

| Scenario                         | Marshal                                 | Unmarshal                              |
|----------------------------------|-----------------------------------------|----------------------------------------|
| Named field vs catch-all key     | Named field wins; map key skipped       | Named field wins; key not collected    |
| Two struct-flatten, same key     | Both dropped (same depth)               | Both dropped; key goes to catch-all   |
| Multiple catch-all flatten       | Error                                   | Error                                 |
| `any` flatten holds non-map/struct | Error                                 | N/A (always becomes `map[string]any`) |
