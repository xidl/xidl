package json

import (
	"fmt"
	"reflect"
	"sort"
	"strings"
	"sync"
)

type fieldOptions string

func parseTag(tag string) (string, fieldOptions) {
	if idx := strings.Index(tag, ","); idx != -1 {
		return tag[:idx], fieldOptions(tag[idx+1:])
	}
	return tag, ""
}

func structTag(f reflect.StructField) (string, bool) {
	if tag, ok := f.Tag.Lookup("xjson"); ok {
		return tag, tag != ""
	}
	if tag, ok := f.Tag.Lookup("json"); ok {
		return tag, tag != ""
	}
	return "", false
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
	required  bool
}

type structFields struct {
	list        []field
	catchAll    *field
	catchAllErr error
}

func (fields structFields) validatePresence(value reflect.Value, seen map[string]bool) error {
	for _, field := range fields.list {
		if !field.required || seen[field.name] {
			continue
		}
		parent := value
		optionalParentAbsent := false
		for _, index := range field.index[:len(field.index)-1] {
			parent = parent.Field(index)
			if parent.Kind() == reflect.Ptr {
				if parent.IsNil() {
					optionalParentAbsent = true
					break
				}
				parent = parent.Elem()
			}
		}
		if !optionalParentAbsent {
			return fmt.Errorf("json: %s requires field %q", value.Type(), field.name)
		}
	}
	return nil
}

var (
	fieldCacheMu sync.RWMutex
	fieldCache   = make(map[reflect.Type]structFields)
)

func getFields(t reflect.Type) structFields {
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

func typeFields(t reflect.Type) structFields {
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
	var catchAlls []field

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

			tag, tagged := structTag(f)
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
				if ft.Kind() == reflect.Map || ft.Kind() == reflect.Interface {
					curr := field{
						name:      f.Name,
						index:     index,
						typ:       f.Type,
						tag:       tagged,
						tagged:    tagName,
						omitempty: opts.Contains("omitempty"),
						asString:  opts.Contains("string"),
						required:  f.Tag.Get("xjson") != "" && !opts.Contains("omitempty") && f.Type.Kind() != reflect.Ptr,
					}
					catchAlls = append(catchAlls, curr)
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
				tag:       tagged,
				tagged:    tagName,
				omitempty: opts.Contains("omitempty"),
				asString:  opts.Contains("string"),
				required:  f.Tag.Get("xjson") != "" && !opts.Contains("omitempty") && f.Type.Kind() != reflect.Ptr,
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

	if len(catchAlls) > 1 {
		return structFields{
			list:        list,
			catchAllErr: fmt.Errorf("json: multiple catch-all flatten fields in %s", t),
		}
	}

	var ca *field
	if len(catchAlls) == 1 {
		c := catchAlls[0]
		ct := c.typ
		if ct.Kind() == reflect.Ptr {
			ct = ct.Elem()
		}
		if ct.Kind() == reflect.Map {
			if ct.Key().Kind() != reflect.String {
				return structFields{
					list: list,
					catchAllErr: fmt.Errorf(
						"json: catch-all flatten map key type must be string in %s, got %s",
						t,
						ct.Key(),
					),
				}
			}
		}
		ca = &c
	}

	return structFields{
		list:     list,
		catchAll: ca,
	}
}
