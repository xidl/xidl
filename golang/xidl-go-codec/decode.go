package json

import (
	"fmt"
	"io"
	"reflect"
	"strconv"
	"strings"
)

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

	if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "null" {
		if v.Kind() == reflect.Ptr || v.Kind() == reflect.Interface {
			v.Set(reflect.Zero(v.Type()))
			d.idx += 4
			return nil
		}
		// A RawMessage or another explicit codec may own a null wire value.
		if decoder, ok := indirectUnmarshaler(v); ok {
			d.idx += 4
			return decoder.UnmarshalJSON([]byte("null"))
		}
		return fmt.Errorf("json: null is not a value of %s", v.Type())
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
		if v.OverflowInt(i) {
			return fmt.Errorf("json: integer %d overflows %s", i, v.Type())
		}
		v.SetInt(i)
		return nil

	case reflect.Uint,
		reflect.Uint8,
		reflect.Uint16,
		reflect.Uint32,
		reflect.Uint64,
		reflect.Uintptr:
		u, err := d.parseUint()
		if err != nil {
			return err
		}
		if v.OverflowUint(u) {
			return fmt.Errorf("json: integer %d overflows %s", u, v.Type())
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
				if i != n {
					return fmt.Errorf("json: fixed array requires %d elements, got %d", n, i)
				}
				return nil
			}

			if i < n {
				if err := d.value(v.Index(i)); err != nil {
					return err
				}
				i++
			} else {
				return fmt.Errorf("json: fixed array requires %d elements, got more", n)
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
				if i != n {
					return fmt.Errorf("json: fixed array requires %d elements, got %d", n, i)
				}
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

		sFields := getFields(v.Type())
		if sFields.catchAllErr != nil {
			return sFields.catchAllErr
		}

		fieldsMap := make(map[string]field)
		for _, f := range sFields.list {
			fieldsMap[f.name] = f
		}

		seen := make(map[string]bool)

		var catchAllMap reflect.Value

		for {
			d.skipSpace()
			if d.idx >= len(d.data) {
				return io.ErrUnexpectedEOF
			}
			if d.data[d.idx] == '}' {
				d.idx++
				return sFields.validatePresence(v, seen)
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
				for _, fd := range sFields.list {
					if !fd.required && strings.ToLower(fd.name) == lowerKey {
						f = fd
						ok = true
						break
					}
				}
			}

			if ok {
				seen[f.name] = true
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

				d.skipSpace()
				if f.asString && !(d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "null") {
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
				if sFields.catchAll != nil {
					if !catchAllMap.IsValid() {
						f := sFields.catchAll
						catchAllVal := v
						for _, idx := range f.index {
							if catchAllVal.Kind() == reflect.Ptr {
								if catchAllVal.IsNil() {
									if !catchAllVal.CanSet() {
										return fmt.Errorf("json: cannot set embedded pointer field")
									}
									catchAllVal.Set(reflect.New(catchAllVal.Type().Elem()))
								}
								catchAllVal = catchAllVal.Elem()
							}
							catchAllVal = catchAllVal.Field(idx)
						}

						realCatchAll := catchAllVal
						for realCatchAll.Kind() == reflect.Ptr {
							if realCatchAll.IsNil() {
								realCatchAll.Set(reflect.New(realCatchAll.Type().Elem()))
							}
							realCatchAll = realCatchAll.Elem()
						}

						if realCatchAll.Kind() == reflect.Map {
							if realCatchAll.IsNil() {
								realCatchAll.Set(reflect.MakeMap(realCatchAll.Type()))
							}
							catchAllMap = realCatchAll
						} else if realCatchAll.Kind() == reflect.Interface {
							if realCatchAll.IsNil() || realCatchAll.Elem().Kind() != reflect.Map {
								m := make(map[string]any)
								realCatchAll.Set(reflect.ValueOf(m))
							}
							catchAllMap = realCatchAll.Elem()
						} else {
							return fmt.Errorf("json: unsupported catch-all type: %s", realCatchAll.Type())
						}
					}

					valType := catchAllMap.Type().Elem()
					mapVal := reflect.New(valType).Elem()
					if err := d.value(mapVal); err != nil {
						return err
					}
					catchAllMap.SetMapIndex(reflect.ValueOf(key), mapVal)
				} else {
					if err := d.skipValue(); err != nil {
						return err
					}
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
				return sFields.validatePresence(v, seen)
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
