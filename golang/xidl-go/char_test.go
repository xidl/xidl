package xidlgo

import (
	json "github.com/xidl/xidl/golang/xidl-go-codec"
	"testing"
)

func TestCharacterWireRoundTrip(t *testing.T) {
	for _, character := range []Char{'A', '雪', '🚀', '\n', '\uFFFD'} {
		wire, err := json.Marshal(map[Char]Char{character: character})
		if err != nil {
			t.Fatal(err)
		}
		var decoded map[Char]Char
		if err := json.Unmarshal(wire, &decoded); err != nil {
			t.Fatal(err)
		}
		if decoded[character] != character {
			t.Fatalf("character changed: %s", wire)
		}
	}
	var value Char
	if err := json.Unmarshal([]byte(`"\ud83d\ude80"`), &value); err != nil || value != '🚀' {
		t.Fatalf("escaped non-BMP character: %U %v", value, err)
	}
}

func TestCharacterRejectsNonScalars(t *testing.T) {
	for _, wire := range []string{`""`, `"ab"`, `"e\u0301"`, `"\ud800"`, `"\udfff"`, `1`, `null`, string([]byte{'"', 0xff, '"'})} {
		var value Char
		if err := json.Unmarshal([]byte(wire), &value); err == nil {
			t.Fatalf("accepted %q", wire)
		}
	}
	for _, character := range []Char{-1, 0xD800, 0x110000} {
		if _, err := json.Marshal(character); err == nil {
			t.Fatalf("encoded invalid scalar %U", character)
		}
	}
}

func TestOptionalCharacterNullRemainsAbsent(t *testing.T) {
	value := Char('A')
	target := struct {
		Letter *Char `xjson:"letter,omitempty"`
	}{Letter: &value}
	if err := json.Unmarshal([]byte(`{"letter":null}`), &target); err != nil {
		t.Fatal(err)
	}
	if target.Letter != nil {
		t.Fatal("null must clear the optional pointer")
	}
}
