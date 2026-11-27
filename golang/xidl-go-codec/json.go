package json

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"reflect"
)

// Marshaler is the interface implemented by types that
// can marshal themselves into valid JSON.
type Marshaler interface {
	MarshalJSON() ([]byte, error)
}

// Unmarshaler is the interface implemented by types that
// can unmarshal a JSON description of themselves.
type Unmarshaler interface {
	UnmarshalJSON([]byte) error
}

// RawMessage is a raw encoded JSON value.
// It implements Marshaler and Unmarshaler and can be used to
// delay JSON decoding or precompute JSON encoding.
type RawMessage []byte

// MarshalJSON returns m as the JSON encoding of m.
func (m RawMessage) MarshalJSON() ([]byte, error) {
	if m == nil {
		return []byte("null"), nil
	}
	return m, nil
}

// UnmarshalJSON sets *m to a copy of data.
func (m *RawMessage) UnmarshalJSON(data []byte) error {
	if m == nil {
		return fmt.Errorf("json.RawMessage: UnmarshalJSON on nil pointer")
	}
	*m = append((*m)[0:0], data...)
	return nil
}

// Marshal returns the JSON encoding of v.
func Marshal(v any) ([]byte, error) {
	var buf bytes.Buffer
	if err := marshalValue(reflect.ValueOf(v), &buf); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// MarshalIndent is like Marshal but applies Indent to format the output.
func MarshalIndent(v any, prefix, indent string) ([]byte, error) {
	b, err := Marshal(v)
	if err != nil {
		return nil, err
	}
	var buf bytes.Buffer
	if err := Indent(&buf, b, prefix, indent); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// Unmarshal parses the JSON-encoded data and stores the result in the value pointed to by v.
func Unmarshal(data []byte, v any) error {
	rv := reflect.ValueOf(v)
	if rv.Kind() != reflect.Ptr || rv.IsNil() {
		return fmt.Errorf("json: Unmarshal(non-pointer %T)", v)
	}

	d := &decoder{data: data, idx: 0}
	if err := d.value(rv.Elem()); err != nil {
		return err
	}

	d.skipSpace()
	if d.idx < len(d.data) {
		return fmt.Errorf("json: extra data after JSON value at offset %d", d.idx)
	}
	return nil
}

// An Encoder writes JSON values to an output stream.
type Encoder struct {
	w io.Writer
}

// NewEncoder returns a new encoder that writes to w.
func NewEncoder(w io.Writer) *Encoder {
	return &Encoder{w: w}
}

// Encode writes the JSON encoding of v to the stream.
func (enc *Encoder) Encode(v any) error {
	b, err := Marshal(v)
	if err != nil {
		return err
	}
	b = append(b, '\n')
	_, err = enc.w.Write(b)
	return err
}

// A Decoder reads and decodes JSON values from an input stream.
type Decoder struct {
	dec *json.Decoder
}

// NewDecoder returns a new decoder that reads from r.
func NewDecoder(r io.Reader) *Decoder {
	return &Decoder{dec: json.NewDecoder(r)}
}

// Decode reads the next JSON-encoded value from its input and stores it in the value pointed to by v.
func (dec *Decoder) Decode(v any) error {
	var raw json.RawMessage
	if err := dec.dec.Decode(&raw); err != nil {
		return err
	}
	return Unmarshal(raw, v)
}
