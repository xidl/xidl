package xidlgohttp

import (
	"fmt"
	"net/url"
	"reflect"
	"strings"
)

func textDestination(value any) (reflect.Value, error) {
	dst := reflect.ValueOf(value)
	if dst.Kind() != reflect.Pointer || dst.IsNil() {
		return reflect.Value{}, fmt.Errorf("HTTP text decoding requires a non-nil pointer")
	}
	target := dst.Elem()
	for target.Kind() == reflect.Pointer {
		if target.IsNil() {
			target.Set(reflect.New(target.Type().Elem()))
		}
		target = target.Elem()
	}
	return target, nil
}

// ParseText decodes one scalar or a comma-separated collection using its IDL type.
func ParseText(raw string, value any) error {
	target, err := textDestination(value)
	if err != nil {
		return err
	}
	if target.Kind() == reflect.Slice || target.Kind() == reflect.Array {
		return assignReflectValue(target, strings.Split(raw, ","))
	}
	return assignScalar(target, raw)
}

// ParseTextValues decodes repeated HTTP bindings without splitting element contents.
func ParseTextValues(raw []string, value any) error {
	target, err := textDestination(value)
	if err != nil {
		return err
	}
	return assignReflectValue(target, raw)
}

// DecodeCookieValue decodes one cookie value without treating a plus as a space.
func DecodeCookieValue(raw string) (string, error) {
	value, err := url.PathUnescape(raw)
	if err != nil {
		return "", fmt.Errorf("HTTP cookie value: %w", err)
	}
	return value, nil
}

// ParseCookieText decodes the URI-encoded value of an HTTP cookie.
func ParseCookieText(raw string, value any) error {
	decoded, err := DecodeCookieValue(raw)
	if err != nil {
		return err
	}
	return ParseText(decoded, value)
}

// FormatText encodes a scalar or collection without bypassing its text codec.
func FormatText(value any) (string, error) {
	parts, err := FormatTextValues(value)
	if err != nil {
		return "", fmt.Errorf("HTTP text encoding: %w", err)
	}
	return strings.Join(parts, ","), nil
}

// FormatTextValues encodes repeated bindings, preserving each element separately.
func FormatTextValues(value any) ([]string, error) {
	return formatReflectValue(reflect.ValueOf(value))
}
