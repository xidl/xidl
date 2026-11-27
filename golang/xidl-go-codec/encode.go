package json

import (
	"bytes"
	"encoding"
	"fmt"
	"math"
	"reflect"
	"sort"
	"strconv"
)

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

	case reflect.Uint,
		reflect.Uint8,
		reflect.Uint16,
		reflect.Uint32,
		reflect.Uint64,
		reflect.Uintptr:
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
		buf.WriteByte('{')
		keys := val.MapKeys()
		type keyStr struct {
			key reflect.Value
			str string
		}
		kStrs := make([]keyStr, 0, len(keys))
		for _, k := range keys {
			if k.CanInterface() {
				if encoder, ok := k.Interface().(encoding.TextMarshaler); ok {
					text, err := encoder.MarshalText()
					if err != nil {
						return err
					}
					kStrs = append(kStrs, keyStr{key: k, str: string(text)})
					continue
				}
			}
			var s string
			switch k.Kind() {
			case reflect.String:
				s = k.String()
			case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
				s = strconv.FormatInt(k.Int(), 10)
			case reflect.Uint,
				reflect.Uint8,
				reflect.Uint16,
				reflect.Uint32,
				reflect.Uint64,
				reflect.Uintptr:
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
		sFields := getFields(val.Type())
		if sFields.catchAllErr != nil {
			return sFields.catchAllErr
		}

		buf.WriteByte('{')
		first := true
		namedKeys := make(map[string]bool)
		for _, f := range sFields.list {
			namedKeys[f.name] = true
		}

		for _, f := range sFields.list {
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

		if sFields.catchAll != nil {
			f := sFields.catchAll
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

			if fVal.IsValid() {
				if err := marshalCatchAll(fVal, namedKeys, &first, buf); err != nil {
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

func marshalCatchAll(
	v reflect.Value,
	namedKeys map[string]bool,
	first *bool,
	buf *bytes.Buffer,
) error {
	for v.Kind() == reflect.Interface || v.Kind() == reflect.Ptr {
		if v.IsNil() {
			return nil
		}
		v = v.Elem()
	}

	if !v.IsValid() {
		return nil
	}

	switch v.Kind() {
	case reflect.Map:
		if v.IsNil() {
			return nil
		}
		if v.Type().Key().Kind() != reflect.String {
			return fmt.Errorf(
				"json: catch-all flatten map key type must be string, got %s",
				v.Type().Key(),
			)
		}

		keys := v.MapKeys()
		type keyStr struct {
			key reflect.Value
			str string
		}
		kStrs := make([]keyStr, 0, len(keys))
		for _, k := range keys {
			s := k.String()
			if namedKeys[s] {
				continue
			}
			kStrs = append(kStrs, keyStr{key: k, str: s})
		}
		sort.Slice(kStrs, func(i, j int) bool {
			return kStrs[i].str < kStrs[j].str
		})

		for _, kStr := range kStrs {
			if !*first {
				buf.WriteByte(',')
			}
			*first = false

			writeString(kStr.str, buf)
			buf.WriteByte(':')
			if err := marshalValue(v.MapIndex(kStr.key), buf); err != nil {
				return err
			}
		}
		return nil

	case reflect.Struct:
		sFields := getFields(v.Type())
		if sFields.catchAllErr != nil {
			return sFields.catchAllErr
		}
		for _, f := range sFields.list {
			if namedKeys[f.name] {
				continue
			}
			fVal := v
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

			if !*first {
				buf.WriteByte(',')
			}
			*first = false

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

		if sFields.catchAll != nil {
			f := sFields.catchAll
			fVal := v
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
			if fVal.IsValid() {
				if err := marshalCatchAll(fVal, namedKeys, first, buf); err != nil {
					return err
				}
			}
		}
		return nil

	default:
		return fmt.Errorf("json: unsupported catch-all flatten type: %s", v.Type())
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
	case reflect.Uint,
		reflect.Uint8,
		reflect.Uint16,
		reflect.Uint32,
		reflect.Uint64,
		reflect.Uintptr:
		return v.Uint() == 0
	case reflect.Float32, reflect.Float64:
		return v.Float() == 0
	case reflect.Interface, reflect.Ptr:
		return v.IsNil()
	}
	return false
}
