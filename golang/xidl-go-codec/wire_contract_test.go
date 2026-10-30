package json

import "testing"

func TestFixedArraysRejectLostOrInventedCells(t *testing.T) {
	for _, wire := range []string{`[]`, `[[1,2,3]]`, `[[1,2],[3,4,5]]`, `[[1,2,3],[4,5,6],[7,8,9]]`, `[[1,2,3,4],[5,6,7]]`, `[[1,null,3],[4,5,6]]`, `null`} {
		var grid [2][3]int32
		if err := Unmarshal([]byte(wire), &grid); err == nil {
			t.Fatalf("accepted malformed grid: %s", wire)
		}
	}
	var grid [2][3]int32
	if err := Unmarshal([]byte(`[[0,1,2],[3,4,5]]`), &grid); err != nil {
		t.Fatal(err)
	}
	if grid != [2][3]int32{{0, 1, 2}, {3, 4, 5}} {
		t.Fatal(grid)
	}
}

func TestIDLRequiredPresenceAndNull(t *testing.T) {
	type Input struct {
		Count int32   `xjson:"count"`
		Note  *string `xjson:"note,omitempty"`
	}
	for _, wire := range []string{`null`, `{}`, `{"Count":1}`, `{"count":null}`, `{"count":1.5}`, `{"count":1e0}`, `{"count":2147483648}`} {
		var input Input
		if err := Unmarshal([]byte(wire), &input); err == nil {
			t.Fatalf("accepted %s", wire)
		}
	}
	var input Input
	if err := Unmarshal([]byte(`{"count":0,"note":null}`), &input); err != nil {
		t.Fatal(err)
	}
	if input.Count != 0 || input.Note != nil {
		t.Fatal(input)
	}
}

func TestOctetsUseNumericArrays(t *testing.T) {
	for _, input := range [][]byte{nil, {}, {0, 127, 255}} {
		wire, err := Marshal(input)
		if err != nil {
			t.Fatal(err)
		}
		if len(wire) == 0 || wire[0] != '[' {
			t.Fatalf("octet wire: %s", wire)
		}
		var output []byte
		if err := Unmarshal(wire, &output); err != nil {
			t.Fatal(err)
		}
		if len(output) != len(input) {
			t.Fatalf("octet count changed: %s", wire)
		}
	}
	for _, wire := range []string{`"AH//"`, `[256]`, `[-1]`, `[null]`, `null`} {
		var output []byte
		if err := Unmarshal([]byte(wire), &output); err == nil {
			t.Fatalf("accepted %s", wire)
		}
	}
}
