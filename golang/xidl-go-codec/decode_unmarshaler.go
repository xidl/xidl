package json

import (
	"encoding"
	"reflect"
)

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
