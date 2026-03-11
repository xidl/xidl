# Chapter 1: XCDR1 - PLAIN_CDR

## 1. When to Use This Serialization Method

- **Encoding version**: Encoding Version 1 (XCDR1).
- **Type extensibility**: Applicable to **FINAL** and **APPENDABLE** types.
- **Data kinds**: All primitive types, strings, sequences, arrays, maps, and
  structs/unions defined as FINAL or APPENDABLE.
- **Typical use**: The traditional compact binary format used in classic DDS
  systems, suitable for deterministic layouts and cases that do not require
  highly flexible member evolution (such as reordering).

## 2. Primitive Type Serialization

This follows traditional OMG CDR rules, with the key characteristic of **natural
alignment** (an N-byte type must be aligned to an N-byte boundary).

- **Byte / Boolean / Char8 / Int8 / UInt8**: 1 byte, alignment 1.
- **Int16 / UInt16**: 2 bytes, alignment 2.
- **Int32 / UInt32 / Float32 / Enum**: 4 bytes, alignment 4.
- **Int64 / UInt64 / Float64**: 8 bytes, alignment **8**.
- **Float128**: 16 bytes, alignment 8.
- **Endianness**: Both big-endian and little-endian are supported, typically
  indicated by the encapsulation header.

## 3. Composite Type Serialization

- **Enumeration**:
  - Serialized as the corresponding integer type (Int8, Int16, or Int32),
    depending on the `@bit_bound` annotation. The default is Int32.

- **Bitmask**:
  - Serialized as the corresponding unsigned integer (UInt8 to UInt64),
    depending on required bit width.

- **Structure**:
  - **Order**: Members are serialized strictly in IDL declaration order.
  - **Padding**: Padding bytes are inserted automatically between members to
    satisfy alignment for the next member.
  - **Optional members**: Before the member value, a **Parameter Header** is
    inserted (4 bytes: 2-byte ID + 2-byte length). If the member is absent, the
    header is still present but length is 0, and the value is omitted.

- **Union**:
  - Serialize the discriminator first, then serialize the selected member.

- **Array**:
  - Elements are serialized contiguously, without a length header.

- **Sequence**:
  - Serialize **Length** (UInt32) first, then serialize elements contiguously.

## 4. Example (ASCII Art)

**Scenario**: `struct Point { int16 x; double y; };` (little-endian)

- `x` (2 bytes) at offset 0
- Padding (6 bytes) to reach 8-byte boundary (because `y` is `double`, and V1
  requires 8-byte alignment)
- `y` (8 bytes) at offset 8

```asciiart
 Offset | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 ... 15 |
--------|----|----|----|----|----|----|----|----|-----------|
 Content|  x (Lo) |  x (Hi) | [PAD] .. [PAD]      |  y (double) ... |
--------|---------|---------|---------------------|-----------|
 Value  | 0x0A    | 0x00    | 0x00 .. 0x00        | 0x......  |
```

---

# Chapter 2: XCDR1 - PL_CDR (Parameterized CDR)

## 1. When to Use This Serialization Method

- **Encoding version**: Encoding Version 1 (XCDR1).
- **Type extensibility**: Applicable only to **MUTABLE** types.
- **Typical use**: Highly flexible evolution scenarios. Members may be added,
  removed, or reordered. Receivers can skip unknown members.

## 2. Primitive Type Serialization

Primitive types are not used standalone in PL_CDR. They appear as member values
inside composite types and are wrapped as parameters.

## 3. Composite Type Serialization

- **Structure (Mutable)**:
  - Uses **Parameter List** format.
  - **Parameter layout**: `Member ID (2 bytes)` + `Length (2 bytes)` +
    `Value (N bytes)` + `Padding`.
  - **Order**: Members may appear in any order.
  - **Missing members**: A missing member is treated as absent (optional) or
    resolved by default value rules.
  - **Sentinel**: The list must end with `PID_SENTINEL` (ID = 0x0001, Length =
    0).
  - **Alignment**: Each parameter ID must start on a 4-byte boundary.

- **Union (Mutable)**:
  - Similar to mutable struct: discriminator parameter and member parameter(s),
    ending with sentinel.

## 4. Example (ASCII Art)

**Scenario**: Mutable struct `{ @id(10) int16 x; }`

- Parameter 1: ID = 10 (0x000A), Length = 2 (int16), Value
- Parameter 2: Sentinel

```asciiart
 Offset | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 10 | 11 |
--------|----|----|----|----|----|----|----|----|----|----|----|----|
 Field  | MemberID| Length  | Value(x)| [PAD]   | MemberID| Length  |
--------|----|----|----|----|----|----|----|----|----|----|----|----|
 Hex    | 0A   00 | 02   00 | 10   00 | 00   00 | 01   00 | 00   00 |
--------|---------|---------|---------|---------|---------|---------|
 Meaning| ID=10   | Len=2   | x=16    | Align 4 | SENTINEL| Len=0   |
```

---

# Chapter 3: XCDR2 - PLAIN_CDR2

## 1. When to Use This Serialization Method

- **Encoding version**: Encoding Version 2 (XCDR2).
- **Type extensibility**: Applicable to **FINAL** types.
- **Typical use**: The modern default high-efficiency DDS format. Compared with
  V1, it reduces packet size by relaxing alignment requirements (less padding),
  and improves optional member representation.

## 2. Primitive Type Serialization

- **Alignment optimization**: The biggest difference from XCDR1.
- **Int64 / UInt64 / Float64 / Float128**: Alignment reduced to **4 bytes** (8
  bytes in V1).
- Other types (1, 2, 4 bytes) keep natural alignment.

- **Boolean**: Must be encoded strictly as 0x00 (false) or 0x01 (true).

## 3. Composite Type Serialization

- **Structure (Final)**:
  - **Order**: Declaration order.
  - **Optional members**: No parameter header. A **Boolean flag** (1 byte) is
    inserted before the member.
    - `1` (true): member exists, then member value is serialized.
    - `0` (false): member absent, member value is skipped.

- **Collections**:
  - Similar to V1, but element alignment follows V2 rules (maximum 4-byte
    alignment).

## 4. Example (ASCII Art)

**Scenario**: `struct Point { int16 x; double y; };` (little-endian, XCDR2)

- `x` (2 bytes) at offset 0
- Padding (2 bytes) to 4-byte boundary (note: `y` is `double`, but in XCDR2 only
  4-byte alignment is required, so offset 4 is enough, not V1 offset 8)
- `y` (8 bytes) at offset 4

```asciiart
 Offset | 00 | 01 | 02 | 03 | 04 ... 11        |
--------|----|----|----|----|-------------------|
 Content|  x (Lo) |  x (Hi) | [PAD]   |  y (double) ...   |
--------|---------|---------|---------|-----------|
 Value  | 0x0A    | 0x00    | 0x00 00 | 0x......  |
 Note   |         |         | Align 4 | 4-byte align OK |
```

---

# Chapter 4: XCDR2 - DELIMITED_CDR

## 1. When to Use This Serialization Method

- **Encoding version**: Encoding Version 2 (XCDR2).
- **Type extensibility**: Applicable to **APPENDABLE** types.
- **Typical use**: Scenarios that allow appending new fields at the end of a
  type. Receivers can skip unknown appended fields using a delimiter header.

## 2. Primitive Type Serialization

Primitive types themselves are FINAL and use PLAIN_CDR2. DELIMITED_CDR is mainly
used to wrap structs or other aggregate types.

## 3. Composite Type Serialization

- **Structure (Appendable)**:
  - **DHEADER (Delimiter Header)**: Serialize a **UInt32** before object data,
    indicating total byte length of the object body.
  - **Body**: Content after DHEADER is serialized using **PLAIN_CDR2** rules.

- **Mechanism**:
  - If a receiver finishes known fields and there are still bytes remaining
    within DHEADER length, it can skip the remaining bytes.

## 4. Example (ASCII Art)

**Scenario**: Appendable struct `{ int32 a; }` (body size = 4 bytes)

- DHEADER: UInt32, value = 4
- Body: `a` (4 bytes)

```asciiart
 Offset | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 |
--------|----|----|----|----|----|----|----|----|
 Field  | DHEADER (Len)     | Member 'a'        |
--------|-------------------|-------------------|
 Hex    | 04   00   00   00 | 78   56   34   12 |
--------|-------------------|-------------------|
 Meaning| Length = 4 bytes  | a = 0x12345678    |
```

---

# Chapter 5: XCDR2 - PL_CDR2

## 1. When to Use This Serialization Method

- **Encoding version**: Encoding Version 2 (XCDR2).
- **Type extensibility**: Applicable to **MUTABLE** types.
- **Typical use**: V2 mutable-type serialization, more efficient than V1
  (supports indexed skipping).

## 2. Primitive Type Serialization

Same as XCDR1 PL_CDR: primitive types appear as member values.

## 3. Composite Type Serialization

- **Structure (Mutable)**:
  - **DHEADER**: Begins with a **UInt32** total length.
  - **Member Header (EMHEADER)**: Each member has a variable-size header.
  - **EMHEADER1 (UInt32)**: Contains `Must Understand Flag` (1 bit) +
    `Length Code` (3 bits) + `Member ID` (28 bits).
  - **NEXTINT (UInt32, optional)**: Present when required by Length Code,
    storing extended length.

- **Value**:
  - Member value, encoded with PLAIN_CDR2 rules.

- **Termination**:
  - No sentinel is required because DHEADER defines total length.

## 4. Example (ASCII Art)

**Scenario**: Mutable struct `{ @id(1) int32 a; }`

- Assume `a` value is 0x11223344 (4 bytes).
- DHEADER: total length = EMHEADER (4) + value (4) = 8 bytes.
- EMHEADER1: ID = 1, LengthCode = 2 (indicates 4-byte length). Hex =
  `0x20000001` (little-endian bytes: `01 00 00 20`).

```asciiart
 Offset | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 10 | 11 |
--------|----|----|----|----|----|----|----|----|----|----|----|----|
 Field  | DHEADER (Total)   | EMHEADER1         | Value (a)         |
--------|-------------------|-------------------|-------------------|
 Hex    | 08   00   00   00 | 01   00   00   20 | 44   33   22   11 |
--------|-------------------|-------------------|-------------------|
 Meaning| Total Len = 8     | ID=1, LC=2(4bytes)| a = 0x11223344    |
```
