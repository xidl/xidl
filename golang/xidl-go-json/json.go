package json

import (
	"bytes"
	"encoding"
	"encoding/base64"
	"fmt"
	"io"
	"math"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"sync"
	"unicode/utf8"
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

// Indent appends to dst an indented form of the JSON-encoded src.
func Indent(dst *bytes.Buffer, src []byte, prefix, indent string) error {
	var (
		depth   int
		inStr   bool
		escaped bool
	)

	writeIndent := func() {
		dst.WriteString(prefix)
		for i := 0; i < depth; i++ {
			dst.WriteString(indent)
		}
	}

	for i := 0; i < len(src); i++ {
		c := src[i]
		if inStr {
			dst.WriteByte(c)
			if escaped {
				escaped = false
			} else if c == '\\' {
				escaped = true
			} else if c == '"' {
				inStr = false
			}
			continue
		}

		switch c {
		case ' ', '\t', '\r', '\n':
			continue
		case '"':
			inStr = true
			dst.WriteByte(c)
		case '{', '[':
			dst.WriteByte(c)
			depth++
			dst.WriteByte('\n')
			writeIndent()
		case '}', ']':
			depth--
			dst.WriteByte('\n')
			writeIndent()
			dst.WriteByte(c)
		case ':':
			dst.WriteByte(':')
			dst.WriteByte(' ')
		case ',':
			dst.WriteByte(',')
			dst.WriteByte('\n')
			writeIndent()
		default:
			dst.WriteByte(c)
		}
	}
	return nil
}

// Unmarshal parses the JSON-encoded data and stores the result in the value pointed to by v.
func Unmarshal(data []byte, v any) error {
	rv := reflect.ValueOf(v)
	if rv.Kind() != reflect.Ptr || rv.IsNil() {
		return fmt.Errorf("json: Unmarshal(non-pointer %T)", v)
	}

	d := &decoder{data: data, idx: 0}
	if err := d.value(rv); err != nil {
		return err
	}

	d.skipSpace()
	if d.idx < len(d.data) {
		return fmt.Errorf("json: extra data after JSON value at offset %d", d.idx)
	}
	return nil
}

type fieldOptions string

func parseTag(tag string) (string, fieldOptions) {
	if idx := strings.Index(tag, ","); idx != -1 {
		return tag[:idx], fieldOptions(tag[idx+1:])
	}
	return tag, ""
}

func (o fieldOptions) Contains(option string) bool {
	if len(o) == 0 {
		return false
	}
	s := string(o)
	for s != "" {
		var next string
		i := strings.Index(s, ",")
		if i >= 0 {
			s, next = s[:i], s[i+1:]
		}
		if s == option {
			return true
		}
		s = next
	}
	return false
}

type field struct {
	name      string
	index     []int
	typ       reflect.Type
	tag       bool
	tagged    string
	omitempty bool
	asString  bool
}

var (
	fieldCacheMu sync.RWMutex
	fieldCache   = make(map[reflect.Type][]field)
)

func getFields(t reflect.Type) []field {
	fieldCacheMu.RLock()
	fields, ok := fieldCache[t]
	fieldCacheMu.RUnlock()
	if ok {
		return fields
	}

	fields = typeFields(t)

	fieldCacheMu.Lock()
	fieldCache[t] = fields
	fieldCacheMu.Unlock()

	return fields
}

func typeFields(t reflect.Type) []field {
	type queueEntry struct {
		typ   reflect.Type
		index []int
	}

	queue := []queueEntry{{typ: t, index: nil}}

	type foundField struct {
		f     field
		depth int
	}

	fieldsByName := make(map[string]foundField)

	for len(queue) > 0 {
		entry := queue[0]
		queue = queue[1:]

		depth := len(entry.index)

		for i := 0; i < entry.typ.NumField(); i++ {
			f := entry.typ.Field(i)

			isUnexported := f.PkgPath != ""
			if isUnexported {
				if !f.Anonymous {
					continue
				}
			}

			tag := f.Tag.Get("json")
			if tag == "-" {
				continue
			}

			tagName, opts := parseTag(tag)

			index := make([]int, len(entry.index)+1)
			copy(index, entry.index)
			index[len(entry.index)] = i

			ft := f.Type
			if ft.Kind() == reflect.Ptr {
				ft = ft.Elem()
			}

			isFlatten := opts.Contains("flatten")
			if (f.Anonymous && tagName == "") || isFlatten {
				if ft.Kind() == reflect.Struct {
					queue = append(queue, queueEntry{typ: ft, index: index})
					continue
				}
			}

			name := tagName
			if name == "" {
				name = f.Name
			}

			curr := field{
				name:      name,
				index:     index,
				typ:       f.Type,
				tag:       tag != "",
				tagged:    tagName,
				omitempty: opts.Contains("omitempty"),
				asString:  opts.Contains("string"),
			}

			existing, exists := fieldsByName[name]
			if !exists {
				fieldsByName[name] = foundField{f: curr, depth: depth}
				continue
			}

			if depth < existing.depth {
				fieldsByName[name] = foundField{f: curr, depth: depth}
				continue
			}
			if depth > existing.depth {
				continue
			}

			if curr.tag && !existing.f.tag {
				fieldsByName[name] = foundField{f: curr, depth: depth}
				continue
			}
			if !curr.tag && existing.f.tag {
				continue
			}

			existing.depth = -1
			fieldsByName[name] = existing
		}
	}

	var list []field
	for _, ff := range fieldsByName {
		if ff.depth != -1 {
			list = append(list, ff.f)
		}
	}

	sort.Slice(list, func(i, j int) bool {
		idx1 := list[i].index
		idx2 := list[j].index
		for k := 0; k < len(idx1) && k < len(idx2); k++ {
			if idx1[k] != idx2[k] {
				return idx1[k] < idx2[k]
			}
		}
		return len(idx1) < len(idx2)
	})

	return list
}

func marshalValue(val reflect.Value, buf *bytes.Buffer) error {
	if !val.IsValid() {
		buf.WriteString("null")
		return nil
	}

	if val.CanInterface() {
		if m, ok := val.Interface().(Marshaler); ok {
			if val.Kind() == reflect.Ptr && val.IsNil() {
				buf.WriteString("null")
				return nil
			}
			b, err := m.MarshalJSON()
			if err != nil {
				return err
			}
			buf.Write(b)
			return nil
		}
	}

	if val.CanAddr() {
		pv := val.Addr()
		if pv.CanInterface() {
			if m, ok := pv.Interface().(Marshaler); ok {
				b, err := m.MarshalJSON()
				if err != nil {
					return err
				}
				buf.Write(b)
				return nil
			}
		}
	}

	if val.CanInterface() {
		if m, ok := val.Interface().(encoding.TextMarshaler); ok {
			if val.Kind() == reflect.Ptr && val.IsNil() {
				buf.WriteString("null")
				return nil
			}
			b, err := m.MarshalText()
			if err != nil {
				return err
			}
			writeString(string(b), buf)
			return nil
		}
	}
	if val.CanAddr() {
		pv := val.Addr()
		if pv.CanInterface() {
			if m, ok := pv.Interface().(encoding.TextMarshaler); ok {
				b, err := m.MarshalText()
				if err != nil {
					return err
				}
				writeString(string(b), buf)
				return nil
			}
		}
	}

	switch val.Kind() {
	case reflect.Interface, reflect.Ptr:
		if val.IsNil() {
			buf.WriteString("null")
			return nil
		}
		return marshalValue(val.Elem(), buf)

	case reflect.Bool:
		if val.Bool() {
			buf.WriteString("true")
		} else {
			buf.WriteString("false")
		}
		return nil

	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		buf.WriteString(strconv.FormatInt(val.Int(), 10))
		return nil

	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		buf.WriteString(strconv.FormatUint(val.Uint(), 10))
		return nil

	case reflect.Float32, reflect.Float64:
		f := val.Float()
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return fmt.Errorf("json: unsupported value: %f", f)
		}
		buf.WriteString(strconv.FormatFloat(f, 'g', -1, 64))
		return nil

	case reflect.String:
		writeString(val.String(), buf)
		return nil

	case reflect.Slice:
		if val.IsNil() {
			buf.WriteString("null")
			return nil
		}
		if val.Type().Elem().Kind() == reflect.Uint8 {
			bytesVal := val.Bytes()
			encoded := base64.StdEncoding.EncodeToString(bytesVal)
			writeString(encoded, buf)
			return nil
		}
		fallthrough

	case reflect.Array:
		buf.WriteByte('[')
		n := val.Len()
		for i := 0; i < n; i++ {
			if i > 0 {
				buf.WriteByte(',')
			}
			if err := marshalValue(val.Index(i), buf); err != nil {
				return err
			}
		}
		buf.WriteByte(']')
		return nil

	case reflect.Map:
		if val.IsNil() {
			buf.WriteString("null")
			return nil
		}
		buf.WriteByte('{')
		keys := val.MapKeys()
		type keyStr struct {
			key reflect.Value
			str string
		}
		kStrs := make([]keyStr, 0, len(keys))
		for _, k := range keys {
			var s string
			switch k.Kind() {
			case reflect.String:
				s = k.String()
			case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
				s = strconv.FormatInt(k.Int(), 10)
			case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
				s = strconv.FormatUint(k.Uint(), 10)
			default:
				if k.CanInterface() {
					if tm, ok := k.Interface().(encoding.TextMarshaler); ok {
						b, err := tm.MarshalText()
						if err != nil {
							return err
						}
						s = string(b)
					} else if k.CanAddr() {
						if tm, ok := k.Addr().Interface().(encoding.TextMarshaler); ok {
							b, err := tm.MarshalText()
							if err != nil {
								return err
							}
							s = string(b)
						} else {
							return fmt.Errorf("json: unsupported map key type: %s", k.Type())
						}
					} else {
						return fmt.Errorf("json: unsupported map key type: %s", k.Type())
					}
				} else {
					return fmt.Errorf("json: unsupported map key type: %s", k.Type())
				}
			}
			kStrs = append(kStrs, keyStr{key: k, str: s})
		}
		sort.Slice(kStrs, func(i, j int) bool {
			return kStrs[i].str < kStrs[j].str
		})

		for i, kStr := range kStrs {
			if i > 0 {
				buf.WriteByte(',')
			}
			writeString(kStr.str, buf)
			buf.WriteByte(':')
			if err := marshalValue(val.MapIndex(kStr.key), buf); err != nil {
				return err
			}
		}
		buf.WriteByte('}')
		return nil

	case reflect.Struct:
		buf.WriteByte('{')
		fields := getFields(val.Type())
		first := true
		for _, f := range fields {
			fVal := val
			for _, idx := range f.index {
				if fVal.Kind() == reflect.Ptr {
					if fVal.IsNil() {
						fVal = reflect.Value{}
						break
					}
					fVal = fVal.Elem()
				}
				fVal = fVal.Field(idx)
			}

			if !fVal.IsValid() {
				continue
			}

			if f.omitempty && isEmptyValue(fVal) {
				continue
			}

			if !first {
				buf.WriteByte(',')
			}
			first = false

			writeString(f.name, buf)
			buf.WriteByte(':')

			if f.asString {
				var strBuf bytes.Buffer
				if err := marshalValue(fVal, &strBuf); err != nil {
					return err
				}
				writeString(strBuf.String(), buf)
			} else {
				if err := marshalValue(fVal, buf); err != nil {
					return err
				}
			}
		}
		buf.WriteByte('}')
		return nil

	default:
		return fmt.Errorf("json: unsupported type: %s", val.Type())
	}
}

func isEmptyValue(v reflect.Value) bool {
	if !v.IsValid() {
		return true
	}
	switch v.Kind() {
	case reflect.Array, reflect.Map, reflect.Slice, reflect.String:
		return v.Len() == 0
	case reflect.Bool:
		return !v.Bool()
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return v.Int() == 0
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return v.Uint() == 0
	case reflect.Float32, reflect.Float64:
		return v.Float() == 0
	case reflect.Interface, reflect.Ptr:
		return v.IsNil()
	}
	return false
}

func writeString(s string, buf *bytes.Buffer) {
	buf.WriteByte('"')
	start := 0
	for i := 0; i < len(s); {
		if b := s[i]; b < utf8.RuneSelf {
			if htmlSafeSet[b] {
				i++
				continue
			}
			if start < i {
				buf.WriteString(s[start:i])
			}
			switch b {
			case '\\', '"':
				buf.WriteByte('\\')
				buf.WriteByte(b)
			case '\n':
				buf.WriteByte('\\')
				buf.WriteByte('n')
			case '\r':
				buf.WriteByte('\\')
				buf.WriteByte('r')
			case '\t':
				buf.WriteByte('\\')
				buf.WriteByte('t')
			default:
				buf.WriteString(`\u00`)
				buf.WriteByte(hexTable[b>>4])
				buf.WriteByte(hexTable[b&0xF])
			}
			i++
			start = i
			continue
		}
		r, size := utf8.DecodeRuneInString(s[i:])
		if r == '\u2028' || r == '\u2029' {
			if start < i {
				buf.WriteString(s[start:i])
			}
			buf.WriteString(`\u202`)
			buf.WriteByte(hexTable[r&0xF])
			i += size
			start = i
			continue
		}
		i += size
	}
	if start < len(s) {
		buf.WriteString(s[start:])
	}
	buf.WriteByte('"')
}

var hexTable = "0123456789abcdef"

var htmlSafeSet = [utf8.RuneSelf]bool{
	' ':    true,
	'!':    true,
	'"':    false,
	'#':    true,
	'$':    true,
	'%':    true,
	'&':    false,
	'\'':   true,
	'(':    true,
	')':    true,
	'*':    true,
	'+':    true,
	',':    true,
	'-':    true,
	'.':    true,
	'/':    true,
	'0':    true,
	'1':    true,
	'2':    true,
	'3':    true,
	'4':    true,
	'5':    true,
	'6':    true,
	'7':    true,
	'8':    true,
	'9':    true,
	':':    true,
	';':    true,
	'<':    false,
	'=':    true,
	'>':    false,
	'?':    true,
	'@':    true,
	'A':    true,
	'B':    true,
	'C':    true,
	'D':    true,
	'E':    true,
	'F':    true,
	'G':    true,
	'H':    true,
	'I':    true,
	'J':    true,
	'K':    true,
	'L':    true,
	'M':    true,
	'N':    true,
	'O':    true,
	'P':    true,
	'Q':    true,
	'R':    true,
	'S':    true,
	'T':    true,
	'U':    true,
	'V':    true,
	'W':    true,
	'X':    true,
	'Y':    true,
	'Z':    true,
	'[':    true,
	'\\':   false,
	']':    true,
	'^':    true,
	'_':    true,
	'`':    true,
	'a':    true,
	'b':    true,
	'c':    true,
	'd':    true,
	'e':    true,
	'f':    true,
	'g':    true,
	'h':    true,
	'i':    true,
	'j':    true,
	'k':    true,
	'l':    true,
	'm':    true,
	'n':    true,
	'o':    true,
	'p':    true,
	'q':    true,
	'r':    true,
	's':    true,
	't':    true,
	'u':    true,
	'v':    true,
	'w':    true,
	'x':    true,
	'y':    true,
	'z':    true,
	'{':    true,
	'|':    true,
	'}':    true,
	'~':    true,
	'\x7f': true,
}

type decoder struct {
	data []byte
	idx  int
}

func (d *decoder) skipSpace() {
	for d.idx < len(d.data) {
		c := d.data[d.idx]
		if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
			d.idx++
		} else {
			break
		}
	}
}

func (d *decoder) value(v reflect.Value) error {
	d.skipSpace()
	if d.idx >= len(d.data) {
		return io.ErrUnexpectedEOF
	}

	if u, ok := indirectUnmarshaler(v); ok {
		start := d.idx
		if err := d.skipValue(); err != nil {
			return err
		}
		raw := d.data[start:d.idx]
		return u.UnmarshalJSON(raw)
	}

	if u, ok := indirectTextUnmarshaler(v); ok {
		s, err := d.parseString()
		if err != nil {
			return err
		}
		return u.UnmarshalText([]byte(s))
	}

	if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "null" {
		d.idx += 4
		if v.Kind() == reflect.Ptr {
			if v.Elem().CanSet() {
				v.Elem().Set(reflect.Zero(v.Elem().Type()))
			}
		} else {
			if v.CanSet() {
				v.Set(reflect.Zero(v.Type()))
			}
		}
		return nil
	}

	if v.Kind() == reflect.Ptr {
		if v.IsNil() {
			if !v.CanSet() {
				return fmt.Errorf("json: cannot set nil pointer")
			}
			v.Set(reflect.New(v.Type().Elem()))
		}
		return d.value(v.Elem())
	}

	switch v.Kind() {
	case reflect.Bool:
		b, err := d.parseBool()
		if err != nil {
			return err
		}
		v.SetBool(b)
		return nil

	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		i, err := d.parseInt()
		if err != nil {
			return err
		}
		v.SetInt(i)
		return nil

	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		u, err := d.parseUint()
		if err != nil {
			return err
		}
		v.SetUint(u)
		return nil

	case reflect.Float32, reflect.Float64:
		f, err := d.parseFloat()
		if err != nil {
			return err
		}
		v.SetFloat(f)
		return nil

	case reflect.String:
		s, err := d.parseString()
		if err != nil {
			return err
		}
		v.SetString(s)
		return nil

	case reflect.Slice:
		if v.Type().Elem().Kind() == reflect.Uint8 {
			s, err := d.parseString()
			if err != nil {
				return err
			}
			b, err := base64.StdEncoding.DecodeString(s)
			if err != nil {
				return err
			}
			v.Set(reflect.ValueOf(b))
			return nil
		}

		if d.data[d.idx] != '[' {
			return fmt.Errorf("json: expected array for slice at offset %d", d.idx)
		}
		d.idx++

		v.Set(reflect.MakeSlice(v.Type(), 0, 0))

		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ']' {
				d.idx++
				return nil
			}

			elem := reflect.New(v.Type().Elem()).Elem()
			if err := d.value(elem); err != nil {
				return err
			}
			v.Set(reflect.Append(v, elem))

			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ',' {
				d.idx++
				continue
			}
			if d.data[d.idx] == ']' {
				d.idx++
				return nil
			}
			return fmt.Errorf("json: expected comma or end of array at offset %d", d.idx)
		}

	case reflect.Array:
		if d.data[d.idx] != '[' {
			return fmt.Errorf("json: expected array at offset %d", d.idx)
		}
		d.idx++

		i := 0
		n := v.Len()
		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ']' {
				d.idx++
				return nil
			}

			if i < n {
				if err := d.value(v.Index(i)); err != nil {
					return err
				}
				i++
			} else {
				if err := d.skipValue(); err != nil {
					return err
				}
			}

			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ',' {
				d.idx++
				continue
			}
			if d.data[d.idx] == ']' {
				d.idx++
				return nil
			}
			return fmt.Errorf("json: expected comma or end of array at offset %d", d.idx)
		}

	case reflect.Map:
		if d.data[d.idx] != '{' {
			return fmt.Errorf("json: expected object for map at offset %d", d.idx)
		}
		d.idx++

		if v.IsNil() {
			v.Set(reflect.MakeMap(v.Type()))
		}

		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return nil
			}

			if d.data[d.idx] != '"' {
				return fmt.Errorf("json: expected object key at offset %d", d.idx)
			}
			keyStr, err := d.parseString()
			if err != nil {
				return err
			}

			mapKeyType := v.Type().Key()
			mapKey := reflect.New(mapKeyType).Elem()

			if u, ok := indirectTextUnmarshaler(mapKey); ok {
				if err := u.UnmarshalText([]byte(keyStr)); err != nil {
					return err
				}
			} else {
				switch mapKeyType.Kind() {
				case reflect.String:
					mapKey.SetString(keyStr)
				case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
					valInt, err := strconv.ParseInt(keyStr, 10, 64)
					if err != nil {
						return err
					}
					mapKey.SetInt(valInt)
				case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
					valUint, err := strconv.ParseUint(keyStr, 10, 64)
					if err != nil {
						return err
					}
					mapKey.SetUint(valUint)
				default:
					return fmt.Errorf("json: unsupported map key type: %s", mapKeyType)
				}
			}

			d.skipSpace()
			if d.idx >= len(d.data) || d.data[d.idx] != ':' {
				return fmt.Errorf("json: expected colon at offset %d", d.idx)
			}
			d.idx++

			mapVal := reflect.New(v.Type().Elem()).Elem()
			if err := d.value(mapVal); err != nil {
				return err
			}

			v.SetMapIndex(mapKey, mapVal)

			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ',' {
				d.idx++
				continue
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return nil
			}
			return fmt.Errorf("json: expected comma or end of object at offset %d", d.idx)
		}

	case reflect.Struct:
		if d.data[d.idx] != '{' {
			return fmt.Errorf("json: expected object for struct at offset %d", d.idx)
		}
		d.idx++

		fields := getFields(v.Type())
		fieldsMap := make(map[string]field)
		for _, f := range fields {
			fieldsMap[f.name] = f
		}

		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return nil
			}

			if d.data[d.idx] != '"' {
				return fmt.Errorf("json: expected object key at offset %d", d.idx)
			}
			key, err := d.parseString()
			if err != nil {
				return err
			}

			d.skipSpace()
			if d.idx >= len(d.data) || d.data[d.idx] != ':' {
				return fmt.Errorf("json: expected colon at offset %d", d.idx)
			}
			d.idx++

			f, ok := fieldsMap[key]
			if !ok {
				lowerKey := strings.ToLower(key)
				for _, fd := range fields {
					if strings.ToLower(fd.name) == lowerKey {
						f = fd
						ok = true
						break
					}
				}
			}

			if ok {
				fVal := v
				for _, idx := range f.index {
					if fVal.Kind() == reflect.Ptr {
						if fVal.IsNil() {
							if !fVal.CanSet() {
								return fmt.Errorf("json: cannot set embedded pointer field")
							}
							fVal.Set(reflect.New(fVal.Type().Elem()))
						}
						fVal = fVal.Elem()
					}
					fVal = fVal.Field(idx)
				}

				// Check if the next token is null first
				d.skipSpace()
				if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "null" {
					d.idx += 4
					if fVal.CanSet() {
						fVal.Set(reflect.Zero(fVal.Type()))
					}
				} else if f.asString {
					s, err := d.parseString()
					if err != nil {
						return err
					}
					subDec := &decoder{data: []byte(s)}
					if err := subDec.value(fVal); err != nil {
						return err
					}
				} else {
					if err := d.value(fVal); err != nil {
						return err
					}
				}
			} else {
				if err := d.skipValue(); err != nil {
					return err
				}
			}

			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ',' {
				d.idx++
				continue
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return nil
			}
			return fmt.Errorf("json: expected comma or end of object at offset %d", d.idx)
		}

	case reflect.Interface:
		if v.NumMethod() == 0 {
			if !v.IsNil() {
				elem := v.Elem()
				if elem.Kind() == reflect.Ptr || elem.Kind() == reflect.Map {
					return d.value(elem)
				}
			}
			var val any
			if err := d.any(&val); err != nil {
				return err
			}
			v.Set(reflect.ValueOf(val))
			return nil
		}
		return fmt.Errorf("json: cannot unmarshal into interface with methods: %s", v.Type())

	default:
		return fmt.Errorf("json: unsupported target type: %s", v.Type())
	}
}

func (d *decoder) any(val *any) error {
	d.skipSpace()
	if d.idx >= len(d.data) {
		return io.ErrUnexpectedEOF
	}
	c := d.data[d.idx]
	switch c {
	case '{':
		m := make(map[string]any)
		mv := reflect.ValueOf(&m).Elem()
		if err := d.value(mv); err != nil {
			return err
		}
		*val = m
		return nil
	case '[':
		var arr []any
		av := reflect.ValueOf(&arr).Elem()
		if err := d.value(av); err != nil {
			return err
		}
		*val = arr
		return nil
	case '"':
		s, err := d.parseString()
		if err != nil {
			return err
		}
		*val = s
		return nil
	case 't', 'f':
		b, err := d.parseBool()
		if err != nil {
			return err
		}
		*val = b
		return nil
	case 'n':
		if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "null" {
			d.idx += 4
			*val = nil
			return nil
		}
		return fmt.Errorf("json: invalid null value")
	default:
		if c == '-' || (c >= '0' && c <= '9') {
			f, err := d.parseFloat()
			if err != nil {
				return err
			}
			*val = f
			return nil
		}
		return fmt.Errorf("json: invalid character %q at offset %d", c, d.idx)
	}
}

func (d *decoder) skipValue() error {
	d.skipSpace()
	if d.idx >= len(d.data) {
		return io.ErrUnexpectedEOF
	}
	c := d.data[d.idx]
	switch c {
	case '{':
		d.idx++
		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return nil
			}
			if d.data[d.idx] != '"' {
				return fmt.Errorf("json: expected object key at offset %d", d.idx)
			}
			if err := d.skipString(); err != nil {
				return err
			}
			d.skipSpace()
			if d.idx >= len(d.data) || d.data[d.idx] != ':' {
				return fmt.Errorf("json: expected colon at offset %d", d.idx)
			}
			d.idx++
			if err := d.skipValue(); err != nil {
				return err
			}
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ',' {
				d.idx++
				continue
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return nil
			}
			return fmt.Errorf("json: expected comma or end of object at offset %d", d.idx)
		}
	case '[':
		d.idx++
		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ']' {
				d.idx++
				return nil
			}
			if err := d.skipValue(); err != nil {
				return err
			}
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == ',' {
				d.idx++
				continue
			}
			if d.data[d.idx] == ']' {
				d.idx++
				return nil
			}
			return fmt.Errorf("json: expected comma or end of array at offset %d", d.idx)
		}
	case '"':
		return d.skipString()
	case 't', 'f':
		if d.data[d.idx] == 't' {
			if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "true" {
				d.idx += 4
				return nil
			}
		} else {
			if d.idx+5 <= len(d.data) && string(d.data[d.idx:d.idx+5]) == "false" {
				d.idx += 5
				return nil
			}
		}
		return fmt.Errorf("json: invalid bool at offset %d", d.idx)
	case 'n':
		if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "null" {
			d.idx += 4
			return nil
		}
		return fmt.Errorf("json: invalid null at offset %d", d.idx)
	default:
		if c == '-' || (c >= '0' && c <= '9') {
			if _, err := d.parseFloat(); err != nil {
				return err
			}
			return nil
		}
		return fmt.Errorf("json: invalid character %q at offset %d", c, d.idx)
	}
}

func (d *decoder) skipString() error {
	if d.idx >= len(d.data) || d.data[d.idx] != '"' {
		return fmt.Errorf("json: expected string start at offset %d", d.idx)
	}
	d.idx++
	for d.idx < len(d.data) {
		c := d.data[d.idx]
		if c == '"' {
			d.idx++
			return nil
		}
		if c == '\\' {
			d.idx += 2
		} else {
			d.idx++
		}
	}
	return io.ErrUnexpectedEOF
}

func (d *decoder) parseInt() (int64, error) {
	d.skipSpace()
	start := d.idx
	if d.idx < len(d.data) && d.data[d.idx] == '-' {
		d.idx++
	}
	for d.idx < len(d.data) && d.data[d.idx] >= '0' && d.data[d.idx] <= '9' {
		d.idx++
	}
	if start == d.idx {
		return 0, fmt.Errorf("json: expected integer at offset %d", start)
	}
	numStr := string(d.data[start:d.idx])
	if d.idx < len(d.data) && (d.data[d.idx] == '.' || d.data[d.idx] == 'e' || d.data[d.idx] == 'E') {
		d.idx = start
		f, err := d.parseFloat()
		if err != nil {
			return 0, err
		}
		return int64(f), nil
	}
	i, err := strconv.ParseInt(numStr, 10, 64)
	if err != nil {
		return 0, err
	}
	return i, nil
}

func (d *decoder) parseUint() (uint64, error) {
	d.skipSpace()
	start := d.idx
	for d.idx < len(d.data) && d.data[d.idx] >= '0' && d.data[d.idx] <= '9' {
		d.idx++
	}
	if start == d.idx {
		return 0, fmt.Errorf("json: expected integer at offset %d", start)
	}
	numStr := string(d.data[start:d.idx])
	if d.idx < len(d.data) && (d.data[d.idx] == '.' || d.data[d.idx] == 'e' || d.data[d.idx] == 'E') {
		d.idx = start
		f, err := d.parseFloat()
		if err != nil {
			return 0, err
		}
		return uint64(f), nil
	}
	u, err := strconv.ParseUint(numStr, 10, 64)
	if err != nil {
		return 0, err
	}
	return u, nil
}

func (d *decoder) parseFloat() (float64, error) {
	d.skipSpace()
	start := d.idx
	if d.idx < len(d.data) && d.data[d.idx] == '-' {
		d.idx++
	}
	for d.idx < len(d.data) {
		c := d.data[d.idx]
		if (c >= '0' && c <= '9') || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
			d.idx++
		} else {
			break
		}
	}
	if start == d.idx {
		return 0, fmt.Errorf("json: expected number at offset %d", start)
	}
	numStr := string(d.data[start:d.idx])
	f, err := strconv.ParseFloat(numStr, 64)
	if err != nil {
		return 0, err
	}
	return f, nil
}

func (d *decoder) parseString() (string, error) {
	d.skipSpace()
	if d.idx >= len(d.data) || d.data[d.idx] != '"' {
		return "", fmt.Errorf("json: expected string start at offset %d", d.idx)
	}
	d.idx++
	var sb strings.Builder
	for d.idx < len(d.data) {
		c := d.data[d.idx]
		if c == '"' {
			d.idx++
			return sb.String(), nil
		}
		if c == '\\' {
			if d.idx+1 >= len(d.data) {
				return "", io.ErrUnexpectedEOF
			}
			escaped := d.data[d.idx+1]
			d.idx += 2
			switch escaped {
			case '"', '\\', '/', '\'':
				sb.WriteByte(escaped)
			case 'b':
				sb.WriteByte('\b')
			case 'f':
				sb.WriteByte('\f')
			case 'n':
				sb.WriteByte('\n')
			case 'r':
				sb.WriteByte('\r')
			case 't':
				sb.WriteByte('\t')
			case 'u':
				if d.idx+4 > len(d.data) {
					return "", io.ErrUnexpectedEOF
				}
				hexStr := string(d.data[d.idx : d.idx+4])
				d.idx += 4
				r, err := strconv.ParseUint(hexStr, 16, 64)
				if err != nil {
					return "", fmt.Errorf("json: invalid unicode escape %q", hexStr)
				}
				sb.WriteRune(rune(r))
			default:
				return "", fmt.Errorf("json: invalid escape sequence \\%c", escaped)
			}
		} else {
			sb.WriteByte(c)
			d.idx++
		}
	}
	return "", io.ErrUnexpectedEOF
}

func (d *decoder) parseBool() (bool, error) {
	d.skipSpace()
	if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "true" {
		d.idx += 4
		return true, nil
	}
	if d.idx+5 <= len(d.data) && string(d.data[d.idx:d.idx+5]) == "false" {
		d.idx += 5
		return false, nil
	}
	return false, fmt.Errorf("json: expected boolean at offset %d", d.idx)
}

func indirectUnmarshaler(v reflect.Value) (Unmarshaler, bool) {
	if v.Kind() != reflect.Ptr && v.CanAddr() {
		pv := v.Addr()
		if pv.CanInterface() {
			if u, ok := pv.Interface().(Unmarshaler); ok {
				return u, true
			}
		}
	}

	if v.CanInterface() {
		if v.Kind() == reflect.Ptr && v.IsNil() {
			if v.Type().Implements(reflect.TypeOf((*Unmarshaler)(nil)).Elem()) {
				v.Set(reflect.New(v.Type().Elem()))
				return v.Interface().(Unmarshaler), true
			}
		} else {
			if u, ok := v.Interface().(Unmarshaler); ok {
				return u, true
			}
		}
	}

	return nil, false
}

func indirectTextUnmarshaler(v reflect.Value) (encoding.TextUnmarshaler, bool) {
	if v.Kind() != reflect.Ptr && v.CanAddr() {
		pv := v.Addr()
		if pv.CanInterface() {
			if u, ok := pv.Interface().(encoding.TextUnmarshaler); ok {
				return u, true
			}
		}
	}

	if v.CanInterface() {
		if v.Kind() == reflect.Ptr && v.IsNil() {
			if v.Type().Implements(reflect.TypeOf((*encoding.TextUnmarshaler)(nil)).Elem()) {
				v.Set(reflect.New(v.Type().Elem()))
				return v.Interface().(encoding.TextUnmarshaler), true
			}
		} else {
			if u, ok := v.Interface().(encoding.TextUnmarshaler); ok {
				return u, true
			}
		}
	}

	return nil, false
}
