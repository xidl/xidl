package xidlgohttp

import (
	"errors"
	"io"
	"net/url"
	"reflect"
	"strconv"
	"strings"

	"github.com/vmihailenco/msgpack/v5"
	json "github.com/xidl/xidl/golang/xidl-go-codec"
)

type Codec interface {
	ContentType() string
	Encode(w io.Writer, value any) error
	Decode(r io.Reader, value any) error
}

type jsonCodec struct{}
type formCodec struct{}
type msgpackCodec struct{}

func MustCodecForMime(mime string) Codec {
	switch strings.ToLower(strings.TrimSpace(mime)) {
	case "", "application/json":
		return jsonCodec{}
	case "application/x-www-form-urlencoded":
		return formCodec{}
	case "application/msgpack":
		return msgpackCodec{}
	default:
		panic("unsupported mime type: " + mime)
	}
}

func (jsonCodec) ContentType() string { return "application/json" }
func (jsonCodec) Encode(w io.Writer, value any) error {
	return json.NewEncoder(w).Encode(value)
}
func (jsonCodec) Decode(r io.Reader, value any) error {
	return json.NewDecoder(r).Decode(value)
}

func (formCodec) ContentType() string { return "application/x-www-form-urlencoded" }
func (formCodec) Encode(w io.Writer, value any) error {
	values, err := structToValues(value)
	if err != nil {
		return err
	}
	_, err = io.WriteString(w, values.Encode())
	return err
}
func (formCodec) Decode(r io.Reader, value any) error {
	payload, err := io.ReadAll(r)
	if err != nil {
		return err
	}
	values, err := url.ParseQuery(string(payload))
	if err != nil {
		return err
	}
	return assignValues(value, values)
}

func (msgpackCodec) ContentType() string { return "application/msgpack" }
func (msgpackCodec) Encode(w io.Writer, value any) error {
	encoder := msgpack.NewEncoder(w)
	return encoder.Encode(value)
}
func (msgpackCodec) Decode(r io.Reader, value any) error {
	decoder := msgpack.NewDecoder(r)
	return decoder.Decode(value)
}

func structToValues(value any) (url.Values, error) {
	out := url.Values{}
	rv := reflect.ValueOf(value)
	if rv.Kind() == reflect.Pointer {
		rv = rv.Elem()
	}
	if rv.Kind() != reflect.Struct {
		return nil, errors.New("form encoding requires struct value")
	}
	rt := rv.Type()
	for i := 0; i < rv.NumField(); i++ {
		field := rt.Field(i)
		if !field.IsExported() {
			continue
		}
		name := field.Tag.Get("form")
		if name == "" {
			name = field.Tag.Get("json")
		}
		if name == "" {
			name = strings.ToLower(field.Name)
		}
		name = strings.Split(name, ",")[0]
		if name == "" || name == "-" {
			continue
		}
		encoded, err := formatReflectValue(rv.Field(i))
		if err != nil {
			return nil, err
		}
		for _, item := range encoded {
			out.Add(name, item)
		}
	}
	return out, nil
}

func assignValues(dst any, values url.Values) error {
	rv := reflect.ValueOf(dst)
	if rv.Kind() != reflect.Pointer || rv.IsNil() {
		return errors.New("form decoding requires non-nil pointer")
	}
	rv = rv.Elem()
	if rv.Kind() != reflect.Struct {
		return errors.New("form decoding requires pointer to struct")
	}
	rt := rv.Type()
	for i := 0; i < rv.NumField(); i++ {
		field := rt.Field(i)
		if !field.IsExported() {
			continue
		}
		name := field.Tag.Get("form")
		if name == "" {
			name = field.Tag.Get("json")
		}
		if name == "" {
			name = strings.ToLower(field.Name)
		}
		name = strings.Split(name, ",")[0]
		if name == "" || name == "-" {
			continue
		}
		if err := assignReflectValue(rv.Field(i), values[name]); err != nil {
			return err
		}
	}
	return nil
}

func assignReflectValue(dst reflect.Value, values []string) error {
	if !dst.CanSet() || len(values) == 0 {
		return nil
	}
	if dst.Kind() == reflect.Pointer {
		if dst.IsNil() {
			dst.Set(reflect.New(dst.Type().Elem()))
		}
		return assignReflectValue(dst.Elem(), values)
	}
	if dst.Kind() == reflect.Slice && dst.Type().Elem().Kind() != reflect.Uint8 {
		slice := reflect.MakeSlice(dst.Type(), 0, len(values))
		for _, item := range values {
			elem := reflect.New(dst.Type().Elem()).Elem()
			if err := assignReflectValue(elem, []string{item}); err != nil {
				return err
			}
			slice = reflect.Append(slice, elem)
		}
		dst.Set(slice)
		return nil
	}
	return assignScalar(dst, values[0])
}

func assignScalar(dst reflect.Value, value string) error {
	switch dst.Kind() {
	case reflect.String:
		dst.SetString(value)
	case reflect.Bool:
		parsed, err := strconv.ParseBool(value)
		if err != nil {
			return err
		}
		dst.SetBool(parsed)
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		parsed, err := strconv.ParseInt(value, 10, 64)
		if err != nil {
			return err
		}
		dst.SetInt(parsed)
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		parsed, err := strconv.ParseUint(value, 10, 64)
		if err != nil {
			return err
		}
		dst.SetUint(parsed)
	case reflect.Float32, reflect.Float64:
		parsed, err := strconv.ParseFloat(value, 64)
		if err != nil {
			return err
		}
		dst.SetFloat(parsed)
	default:
		return errors.New("unsupported scalar form field type: " + dst.Type().String())
	}
	return nil
}

func formatReflectValue(value reflect.Value) ([]string, error) {
	if value.Kind() == reflect.Pointer {
		if value.IsNil() {
			return nil, nil
		}
		return formatReflectValue(value.Elem())
	}
	if value.Kind() == reflect.Slice && value.Type().Elem().Kind() != reflect.Uint8 {
		out := make([]string, 0, value.Len())
		for i := 0; i < value.Len(); i++ {
			items, err := formatReflectValue(value.Index(i))
			if err != nil {
				return nil, err
			}
			out = append(out, items...)
		}
		return out, nil
	}
	switch value.Kind() {
	case reflect.String:
		return []string{value.String()}, nil
	case reflect.Bool:
		return []string{strconv.FormatBool(value.Bool())}, nil
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return []string{strconv.FormatInt(value.Int(), 10)}, nil
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return []string{strconv.FormatUint(value.Uint(), 10)}, nil
	case reflect.Float32, reflect.Float64:
		return []string{strconv.FormatFloat(value.Float(), 'f', -1, 64)}, nil
	default:
		return nil, errors.New("unsupported form field type: " + value.Type().String())
	}
}
