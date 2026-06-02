package json

import (
	"errors"
	"reflect"
	"testing"
)

func TestMarshalBasic(t *testing.T) {
	tests := []struct {
		val  any
		want string
	}{
		{nil, "null"},
		{true, "true"},
		{false, "false"},
		{123, "123"},
		{-456, "-456"},
		{uint(789), "789"},
		{3.14, "3.14"},
		{"hello", `"hello"`},
		{"hello <world> & \"co\"", `"hello \u003cworld\u003e \u0026 \"co\""`},
		{[]int{1, 2, 3}, "[1,2,3]"},
		{[2]string{"a", "b"}, `["a","b"]`},
		{map[string]int{"b": 2, "a": 1}, `{"a":1,"b":2}`},
		{[]byte("base64"), `"YmFzZTY0"`},
	}

	for _, tc := range tests {
		got, err := Marshal(tc.val)
		if err != nil {
			t.Errorf("Marshal(%v) failed: %v", tc.val, err)
			continue
		}
		if string(got) != tc.want {
			t.Errorf("Marshal(%v) = %s, want %s", tc.val, got, tc.want)
		}
	}
}

type SimpleStruct struct {
	A int    `xjson:"a"`
	B string `xjson:"b,omitempty"`
	C bool   `xjson:"-"`
	D int    `xjson:",string"`
}

type EmbeddedStruct struct {
	SimpleStruct
	E float64 `xjson:"e"`
}

type ConflictOuter struct {
	SimpleStruct
	A string `xjson:"a"` // shadows SimpleStruct.A
}

func TestMarshalStruct(t *testing.T) {
	s := SimpleStruct{A: 10, B: "", C: true, D: 42}
	got, err := Marshal(s)
	if err != nil {
		t.Fatalf("Marshal failed: %v", err)
	}
	want := `{"a":10,"D":"42"}` // B is omitempty and empty, C is skipped, D is string option
	if string(got) != want {
		t.Errorf("Marshal(SimpleStruct) = %s, want %s", got, want)
	}

	emb := EmbeddedStruct{
		SimpleStruct: SimpleStruct{A: 1, B: "test", D: 2},
		E:            5.5,
	}
	got, err = Marshal(emb)
	if err != nil {
		t.Fatalf("Marshal failed: %v", err)
	}
	want = `{"a":1,"b":"test","D":"2","e":5.5}`
	if string(got) != want {
		t.Errorf("Marshal(EmbeddedStruct) = %s, want %s", got, want)
	}

	co := ConflictOuter{
		SimpleStruct: SimpleStruct{A: 1},
		A:            "shadow",
	}
	got, err = Marshal(co)
	if err != nil {
		t.Fatalf("Marshal failed: %v", err)
	}
	want = `{"D":"0","a":"shadow"}`
	if string(got) != want {
		t.Errorf("Marshal(ConflictOuter) = %s, want %s", got, want)
	}
}

func TestUnmarshalBasic(t *testing.T) {
	t.Run("bool", func(t *testing.T) {
		var v bool
		if err := Unmarshal([]byte("true"), &v); err != nil || !v {
			t.Errorf("Unmarshal(true) = %v, %v", v, err)
		}
	})

	t.Run("int", func(t *testing.T) {
		var v int
		if err := Unmarshal([]byte("-123"), &v); err != nil || v != -123 {
			t.Errorf("Unmarshal(-123) = %v, %v", v, err)
		}
	})

	t.Run("uint", func(t *testing.T) {
		var v uint
		if err := Unmarshal([]byte("456"), &v); err != nil || v != 456 {
			t.Errorf("Unmarshal(456) = %v, %v", v, err)
		}
	})

	t.Run("float", func(t *testing.T) {
		var v float64
		if err := Unmarshal([]byte("123.45"), &v); err != nil || v != 123.45 {
			t.Errorf("Unmarshal(123.45) = %v, %v", v, err)
		}
	})

	t.Run("string", func(t *testing.T) {
		var v string
		if err := Unmarshal([]byte(`"hello\nworld"`), &v); err != nil || v != "hello\nworld" {
			t.Errorf("Unmarshal(string) = %q, %v", v, err)
		}
	})

	t.Run("slice", func(t *testing.T) {
		var v []int
		if err := Unmarshal([]byte("[1,2,3]"), &v); err != nil || !reflect.DeepEqual(v, []int{1, 2, 3}) {
			t.Errorf("Unmarshal(slice) = %v, %v", v, err)
		}
	})

	t.Run("map", func(t *testing.T) {
		var v map[string]int
		if err := Unmarshal([]byte(`{"a":1,"b":2}`), &v); err != nil || v["a"] != 1 || v["b"] != 2 {
			t.Errorf("Unmarshal(map) = %v, %v", v, err)
		}
	})

	t.Run("interface", func(t *testing.T) {
		var v any
		if err := Unmarshal([]byte(`{"a":1,"b":[true, "hello"]}`), &v); err != nil {
			t.Fatalf("Unmarshal to interface failed: %v", err)
		}
		m, ok := v.(map[string]any)
		if !ok {
			t.Fatalf("Expected map[string]any, got %T", v)
		}
		if m["a"].(float64) != 1 {
			t.Errorf("Expected m[a] = 1, got %v", m["a"])
		}
		arr := m["b"].([]any)
		if !arr[0].(bool) || arr[1].(string) != "hello" {
			t.Errorf("Unexpected array elements: %v", arr)
		}
	})

	t.Run("null pointer", func(t *testing.T) {
		var v *SimpleStruct = &SimpleStruct{A: 99}
		if err := Unmarshal([]byte("null"), &v); err != nil || v != nil {
			t.Errorf("Unmarshal(null) = %v, %v", v, err)
		}
	})

	t.Run("nested pointer allocation", func(t *testing.T) {
		var v **SimpleStruct
		if err := Unmarshal([]byte(`{"a":5}`), &v); err != nil || v == nil || *v == nil || (*v).A != 5 {
			t.Errorf("Unmarshal into nested pointer failed: %v, %v", v, err)
		}
	})
}

func TestUnmarshalStruct(t *testing.T) {
	data := []byte(`{"a":100,"b":"yes","d":"200"}`)
	var s SimpleStruct
	if err := Unmarshal(data, &s); err != nil {
		t.Fatalf("Unmarshal failed: %v", err)
	}
	if s.A != 100 || s.B != "yes" || s.D != 200 {
		t.Errorf("Unmarshal got %+v", s)
	}

	// Test case insensitive match
	data2 := []byte(`{"A":101,"B":"ok"}`)
	var s2 SimpleStruct
	if err := Unmarshal(data2, &s2); err != nil {
		t.Fatalf("Unmarshal failed: %v", err)
	}
	if s2.A != 101 || s2.B != "ok" {
		t.Errorf("Case insensitive match got %+v", s2)
	}
}

type customTextMarshaler struct {
	val string
}

func (c customTextMarshaler) MarshalText() ([]byte, error) {
	return []byte("custom:" + c.val), nil
}

func (c *customTextMarshaler) UnmarshalText(text []byte) error {
	s := string(text)
	if !reflect.ValueOf(s).IsValid() || len(s) < 7 || s[:7] != "custom:" {
		return errors.New("invalid format")
	}
	c.val = s[7:]
	return nil
}

func TestCustomMarshalers(t *testing.T) {
	t.Run("RawMessage", func(t *testing.T) {
		type Msg struct {
			Raw RawMessage `xjson:"raw"`
		}
		m := Msg{Raw: RawMessage(`{"nested":true}`)}
		got, err := Marshal(m)
		if err != nil {
			t.Fatalf("Marshal with RawMessage failed: %v", err)
		}
		want := `{"raw":{"nested":true}}`
		if string(got) != want {
			t.Errorf("Marshal(RawMessage) = %s, want %s", got, want)
		}

		var m2 Msg
		if err := Unmarshal(got, &m2); err != nil {
			t.Fatalf("Unmarshal with RawMessage failed: %v", err)
		}
		if string(m2.Raw) != `{"nested":true}` {
			t.Errorf("Unmarshal(RawMessage) got %s", m2.Raw)
		}
	})

	t.Run("TextMarshaler", func(t *testing.T) {
		c := customTextMarshaler{val: "foobar"}
		got, err := Marshal(c)
		if err != nil {
			t.Fatalf("Marshal TextMarshaler failed: %v", err)
		}
		if string(got) != `"custom:foobar"` {
			t.Errorf("Marshal TextMarshaler got %s", got)
		}

		var c2 customTextMarshaler
		if err := Unmarshal(got, &c2); err != nil {
			t.Fatalf("Unmarshal TextUnmarshaler failed: %v", err)
		}
		if c2.val != "foobar" {
			t.Errorf("Unmarshal TextUnmarshaler got %s", c2.val)
		}
	})
}

func TestMarshalIndent(t *testing.T) {
	v := map[string]any{
		"a": 1,
		"b": []any{2, 3},
	}
	got, err := MarshalIndent(v, "", "  ")
	if err != nil {
		t.Fatalf("MarshalIndent failed: %v", err)
	}

	want := "{\n" +
		"  \"a\": 1,\n" +
		"  \"b\": [\n" +
		"    2,\n" +
		"    3\n" +
		"  ]\n" +
		"}"

	if string(got) != want {
		t.Errorf("MarshalIndent got:\n%s\nwant:\n%s", got, want)
	}
}

type Profile struct {
	Age  int    `xjson:"age"`
	City string `xjson:"city,omitempty"`
}

type Address struct {
	Street string `xjson:"street"`
}

type User struct {
	Name    string   `xjson:"name"`
	Profile Profile  `xjson:"profile,flatten"`
	Address *Address `xjson:"address,flatten,omitempty"`
}

func TestFlattenTag(t *testing.T) {
	t.Run("marshal non-nil", func(t *testing.T) {
		u := User{
			Name: "Alice",
			Profile: Profile{
				Age:  30,
				City: "New York",
			},
			Address: &Address{
				Street: "123 Main St",
			},
		}

		got, err := Marshal(u)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}

		want := `{"name":"Alice","age":30,"city":"New York","street":"123 Main St"}`
		if string(got) != want {
			t.Errorf("Marshal(User) = %s, want %s", got, want)
		}
	})

	t.Run("marshal nil pointer", func(t *testing.T) {
		u := User{
			Name: "Bob",
			Profile: Profile{
				Age:  25,
				City: "Boston",
			},
			Address: nil,
		}

		got, err := Marshal(u)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}

		want := `{"name":"Bob","age":25,"city":"Boston"}`
		if string(got) != want {
			t.Errorf("Marshal(User with nil Address) = %s, want %s", got, want)
		}
	})

	t.Run("unmarshal non-nil", func(t *testing.T) {
		data := []byte(`{"name":"Alice","age":30,"city":"New York","street":"123 Main St"}`)
		var u User
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}

		if u.Name != "Alice" || u.Profile.Age != 30 || u.Profile.City != "New York" || u.Address == nil || u.Address.Street != "123 Main St" {
			t.Errorf("Unmarshal(User) got: %+v, Address: %+v", u, u.Address)
		}
	})

	t.Run("unmarshal nil pointer", func(t *testing.T) {
		data := []byte(`{"name":"Bob","age":25,"city":"Boston"}`)
		var u User
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}

		if u.Name != "Bob" || u.Profile.Age != 25 || u.Profile.City != "Boston" || u.Address != nil {
			t.Errorf("Unmarshal(User with missing Address) got: %+v, Address: %+v", u, u.Address)
		}
	})
}

type MapCatchAll struct {
	Type  string         `xjson:"type"`
	Extra map[string]any `xjson:",flatten"`
}

type MapCatchAllInt struct {
	Type  string         `xjson:"type"`
	Extra map[string]int `xjson:",flatten"`
}

type InterfaceCatchAll struct {
	Type  string `xjson:"type"`
	Extra any    `xjson:",flatten"`
}

type InvalidMapCatchAll struct {
	Type  string      `xjson:"type"`
	Extra map[int]any `xjson:",flatten"`
}

type MultipleCatchAll struct {
	Type   string         `xjson:"type"`
	Extra1 map[string]any `xjson:",flatten"`
	Extra2 map[string]any `xjson:",flatten"`
}

type StructAndCatchAll struct {
	Name    string         `xjson:"name"`
	Profile Profile        `xjson:"profile,flatten"`
	Extra   map[string]any `xjson:",flatten"`
}

type ConflictingStructs struct {
	Age1  AgeStruct1     `xjson:",flatten"`
	Age2  AgeStruct2     `xjson:",flatten"`
	Extra map[string]any `xjson:",flatten"`
}

type AgeStruct1 struct {
	Age int `xjson:"age"`
}

type AgeStruct2 struct {
	Age int `xjson:"age"`
}

func TestCatchAllFlatten(t *testing.T) {
	t.Run("map[string]any flatten marshal/unmarshal", func(t *testing.T) {
		// Marshal non-nil map
		m := MapCatchAll{
			Type: "click",
			Extra: map[string]any{
				"x": 10,
				"y": "twenty",
			},
		}
		got, err := Marshal(m)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		want := `{"type":"click","x":10,"y":"twenty"}`
		if string(got) != want {
			t.Errorf("Marshal got %s, want %s", got, want)
		}

		// Marshal nil map
		mNil := MapCatchAll{
			Type:  "click",
			Extra: nil,
		}
		gotNil, err := Marshal(mNil)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		wantNil := `{"type":"click"}`
		if string(gotNil) != wantNil {
			t.Errorf("Marshal got %s, want %s", gotNil, wantNil)
		}

		// Unmarshal
		var u MapCatchAll
		data := []byte(`{"type":"click","x":10,"y":"twenty"}`)
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}
		if u.Type != "click" {
			t.Errorf("got Type %q, want click", u.Type)
		}
		if len(u.Extra) != 2 || u.Extra["x"] != 10.0 || u.Extra["y"] != "twenty" {
			t.Errorf("got Extra %+v, want x=10, y=twenty", u.Extra)
		}
	})

	t.Run("map[string]int flatten marshal/unmarshal", func(t *testing.T) {
		m := MapCatchAllInt{
			Type: "count",
			Extra: map[string]int{
				"apples":  5,
				"oranges": 12,
			},
		}
		got, err := Marshal(m)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		want := `{"apples":5,"oranges":12,"type":"count"}`
		if string(got) != want {
			t.Errorf("Marshal got %s, want %s", got, want)
		}

		var u MapCatchAllInt
		data := []byte(`{"apples":5,"oranges":12,"type":"count"}`)
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}
		if u.Type != "count" || len(u.Extra) != 2 || u.Extra["apples"] != 5 || u.Extra["oranges"] != 12 {
			t.Errorf("got %+v", u)
		}
	})

	t.Run("invalid map key type", func(t *testing.T) {
		m := InvalidMapCatchAll{
			Type:  "invalid",
			Extra: map[int]any{1: "one"},
		}
		_, err := Marshal(m)
		if err == nil {
			t.Error("expected error for invalid map key type, got nil")
		}

		var u InvalidMapCatchAll
		data := []byte(`{"type":"invalid","1":"one"}`)
		err = Unmarshal(data, &u)
		if err == nil {
			t.Error("expected error for invalid map key type, got nil")
		}
	})

	t.Run("multiple catch-all fields", func(t *testing.T) {
		m := MultipleCatchAll{
			Type:   "multi",
			Extra1: map[string]any{"a": 1},
			Extra2: map[string]any{"b": 2},
		}
		_, err := Marshal(m)
		if err == nil {
			t.Error("expected error for multiple catch-all fields, got nil")
		}

		var u MultipleCatchAll
		data := []byte(`{"type":"multi","a":1,"b":2}`)
		err = Unmarshal(data, &u)
		if err == nil {
			t.Error("expected error for multiple catch-all fields, got nil")
		}
	})

	t.Run("named fields priority", func(t *testing.T) {
		// Marshal: "type" key in map should be ignored because Type is a named field
		m := MapCatchAll{
			Type: "click",
			Extra: map[string]any{
				"type": "ignored",
				"x":    10,
			},
		}
		got, err := Marshal(m)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		want := `{"type":"click","x":10}`
		if string(got) != want {
			t.Errorf("Marshal got %s, want %s", got, want)
		}

		// Unmarshal: "type" key should go to Type field, and not to Extra
		var u MapCatchAll
		data := []byte(`{"type":"click","x":10}`)
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}
		if u.Type != "click" {
			t.Errorf("got Type %q, want click", u.Type)
		}
		if _, ok := u.Extra["type"]; ok {
			t.Errorf("Extra contains 'type' key which should have gone to named field")
		}
		if u.Extra["x"] != 10.0 {
			t.Errorf("got Extra['x'] %v, want 10", u.Extra["x"])
		}
	})

	t.Run("any flatten marshal values", func(t *testing.T) {
		// nil value
		mNil := InterfaceCatchAll{Type: "nil", Extra: nil}
		gotNil, err := Marshal(mNil)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		if string(gotNil) != `{"type":"nil"}` {
			t.Errorf("Marshal got %s", gotNil)
		}

		// map value
		mMap := InterfaceCatchAll{Type: "map", Extra: map[string]any{"x": 10}}
		gotMap, err := Marshal(mMap)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		if string(gotMap) != `{"type":"map","x":10}` {
			t.Errorf("Marshal got %s", gotMap)
		}

		// struct value
		mStruct := InterfaceCatchAll{Type: "struct", Extra: Profile{Age: 30, City: "Paris"}}
		gotStruct, err := Marshal(mStruct)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		if string(gotStruct) != `{"age":30,"city":"Paris","type":"struct"}` {
			t.Errorf("Marshal got %s", gotStruct)
		}

		// struct pointer value
		mStructPtr := InterfaceCatchAll{Type: "struct", Extra: &Profile{Age: 30, City: "Paris"}}
		gotStructPtr, err := Marshal(mStructPtr)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		if string(gotStructPtr) != `{"age":30,"city":"Paris","type":"struct"}` {
			t.Errorf("Marshal got %s", gotStructPtr)
		}

		// invalid value (string)
		mInvalid := InterfaceCatchAll{Type: "invalid", Extra: "hello"}
		_, err = Marshal(mInvalid)
		if err == nil {
			t.Error("expected error for non-map/non-struct any flatten value, got nil")
		}
	})

	t.Run("any flatten unmarshal", func(t *testing.T) {
		var u InterfaceCatchAll
		data := []byte(`{"type":"click","x":10,"y":"twenty"}`)
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}
		if u.Type != "click" {
			t.Errorf("got Type %q", u.Type)
		}
		extraMap, ok := u.Extra.(map[string]any)
		if !ok {
			t.Fatalf("expected Extra to be map[string]any, got %T", u.Extra)
		}
		if len(extraMap) != 2 || extraMap["x"] != 10.0 || extraMap["y"] != "twenty" {
			t.Errorf("got Extra map %+v", extraMap)
		}
	})

	t.Run("struct-flatten and catch-all together", func(t *testing.T) {
		m := StructAndCatchAll{
			Name: "Alice",
			Profile: Profile{
				Age:  30,
				City: "Boston",
			},
			Extra: map[string]any{
				"city": "ignored", // should be ignored as City is a promoted named field
				"x":    10,
			},
		}
		got, err := Marshal(m)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		want := `{"age":30,"city":"Boston","name":"Alice","x":10}`
		if string(got) != want {
			t.Errorf("Marshal got %s, want %s", got, want)
		}

		var u StructAndCatchAll
		data := []byte(`{"name":"Alice","age":30,"city":"Boston","x":10}`)
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}
		if u.Name != "Alice" || u.Profile.Age != 30 || u.Profile.City != "Boston" {
			t.Errorf("got %+v", u)
		}
		if _, ok := u.Extra["city"]; ok {
			t.Error("Extra should not contain 'city'")
		}
		if u.Extra["x"] != 10.0 {
			t.Errorf("got Extra %+v", u.Extra)
		}
	})

	t.Run("conflicting structs and catch-all", func(t *testing.T) {
		// age is defined in both Age1 and Age2, so it's a conflict and dropped.
		// Therefore, during Marshal, only Extra's "age" is printed.
		// During Unmarshal, "age" in JSON goes into Extra.
		m := ConflictingStructs{
			Age1:  AgeStruct1{Age: 10},
			Age2:  AgeStruct2{Age: 20},
			Extra: map[string]any{"age": 30},
		}
		got, err := Marshal(m)
		if err != nil {
			t.Fatalf("Marshal failed: %v", err)
		}
		want := `{"age":30}`
		if string(got) != want {
			t.Errorf("Marshal got %s, want %s", got, want)
		}

		var u ConflictingStructs
		data := []byte(`{"age":30}`)
		if err := Unmarshal(data, &u); err != nil {
			t.Fatalf("Unmarshal failed: %v", err)
		}
		if u.Age1.Age != 0 || u.Age2.Age != 0 {
			t.Errorf("expected Age1 and Age2 to be 0 due to conflict, got Age1=%d, Age2=%d", u.Age1.Age, u.Age2.Age)
		}
		if u.Extra["age"] != 30.0 {
			t.Errorf("expected Extra to capture conflicting 'age' key, got %+v", u.Extra)
		}
	})
}
