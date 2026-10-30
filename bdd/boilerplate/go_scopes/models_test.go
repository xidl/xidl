package api

import (
	"reflect"
	"sources"
	"testing"

	json "github.com/xidl/xidl/golang/xidl-go-codec"
)

func entry() LedgerEntry {
	detail := LedgerDetail{Message: "first entry"}
	return LedgerEntry{
		LedgerBase: LedgerBase{Id: "entry-1"},
		Detail:     detail,
		Trail:      []LedgerDetail{detail},
		Labels:     map[string]LedgerDetail{"primary": detail},
		Pair:       LedgerDetails{detail, {Message: "second entry"}},
		State:      LedgerInitial,
		Root:       Detail{Code: 41},
		Origin:     sources.HttpApiDetail{Source: "imported"},
	}
}

func TestDeclaredScopesSurviveRoundTrips(t *testing.T) {
	value := LedgerHttpApiReceipt{
		Attempt: 3,
		Detail:  LedgerDetail{Message: "parent module"},
		Root:    Detail{Code: 42},
		Entry:   entry(),
	}
	wire, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	var decoded LedgerHttpApiReceipt
	if err := json.Unmarshal(wire, &decoded); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(value, decoded) {
		t.Fatalf("scope or wire shape changed: %s", wire)
	}
	if len(decoded.Entry.Pair) != int(LedgerWidth) {
		t.Fatal("array bound lost its declaring scope")
	}
	view := LedgerEntriesView{Message: "interface alias"}
	envelope := LedgerEntriesEnvelope{Entry: decoded.Entry, Detail: view}
	if envelope.Detail.Message != view.Message {
		t.Fatal("interface export lost its declaring scope")
	}
}

func TestUnionPayloadUsesTheDeclaredType(t *testing.T) {
	value := NewLedgerChangeDetail(LedgerDetail{Message: "changed"})
	wire, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	var decoded LedgerChange
	if err := json.Unmarshal(wire, &decoded); err != nil {
		t.Fatal(err)
	}
	detail, ok := decoded.AsDetail()
	if !ok || detail.Message != "changed" || decoded.Tag() != "open" {
		t.Fatalf("wrong discriminator or scoped payload: %s", wire)
	}
	for _, wire := range []string{
		`{"tag":"open","data":{"message":7}}`,
		`{"tag":"missing","data":{"message":"changed"}}`,
	} {
		if err := json.Unmarshal([]byte(wire), &decoded); err == nil {
			t.Fatalf("accepted invalid scoped union: %s", wire)
		}
	}
}

func TestScopedEnumAndArrayRejectOtherWireShapes(t *testing.T) {
	var state LedgerState
	if err := json.Unmarshal([]byte(`"missing"`), &state); err == nil {
		t.Fatal("accepted undeclared enum member")
	}
	var pair LedgerDetails
	if err := json.Unmarshal([]byte(`[{"message":"one"}]`), &pair); err == nil {
		t.Fatal("accepted an array shorter than the scoped bound")
	}
}
