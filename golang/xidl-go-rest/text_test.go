package xidlgohttp

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"reflect"
	"testing"

	"github.com/gin-gonic/gin"
)

func TestTextBindingPresenceDoesNotInventAnEmptyString(t *testing.T) {
	query := url.Values{"name": {""}}
	if value, err := QueryString(query, "name"); err != nil || value != "" {
		t.Fatalf("present empty query: %q %v", value, err)
	}
	if _, err := QueryString(query, "missing"); err == nil {
		t.Fatal("missing query accepted")
	}
	header := make(http.Header)
	header.Set("X-Name", "")
	if value, err := HeaderString(header, "X-Name"); err != nil || value != "" {
		t.Fatalf("present empty header: %q %v", value, err)
	}
	if _, err := HeaderString(header, "X-Missing"); err == nil {
		t.Fatal("missing header accepted")
	}
}

type textCount int16

func TestTextValuesRespectTheDeclaredType(t *testing.T) {
	var count textCount
	if err := ParseText("32767", &count); err != nil || count != 32767 {
		t.Fatalf("alias: %d %v", count, err)
	}
	if err := ParseText("32768", &count); err == nil {
		t.Fatal("text integer overflow was accepted")
	}
	var matrix [3]textCount
	if err := ParseText("0,1,2", &matrix); err != nil || matrix != [3]textCount{0, 1, 2} {
		t.Fatalf("array: %v %v", matrix, err)
	}
	for _, wire := range []string{"0,1", "0,1,2,3", "0,wrong,2"} {
		if err := ParseText(wire, &matrix); err == nil {
			t.Fatalf("accepted %q", wire)
		}
	}
	var optional *textCount
	if err := ParseText("0", &optional); err != nil || optional == nil || *optional != 0 {
		t.Fatalf("optional zero: %v %v", optional, err)
	}
	var flags []bool
	if err := ParseText("true,false", &flags); err != nil || !reflect.DeepEqual(flags, []bool{true, false}) {
		t.Fatalf("boolean collection: %v %v", flags, err)
	}
	for _, value := range []string{"1", "TRUE", "yes"} {
		var flag bool
		if err := ParseText(value, &flag); err == nil {
			t.Fatalf("accepted boolean %q", value)
		}
	}
	if wire, err := FormatText(matrix); err != nil || wire != "0,1,2" {
		t.Fatalf("array output: %q %v", wire, err)
	}
}

func TestCookieTextIsDecodedExactlyOnce(t *testing.T) {
	const value = "%2F+ %"
	encoded := EncodeCookieValue(value)
	request := httptest.NewRequest("GET", "/", nil)
	request.Header.Set("Cookie", "session="+encoded)
	context, _ := gin.CreateTestContext(httptest.NewRecorder())
	context.Request = request
	decoded, err := GinCookieString(context, "session")
	if err != nil || decoded != value {
		t.Fatalf("request cookie: %q %v", decoded, err)
	}
	var receipt string
	if err := ParseCookieText(encoded, &receipt); err != nil || receipt != value {
		t.Fatalf("response cookie: %q %v", receipt, err)
	}
	if err := ParseCookieText("%broken", &receipt); err == nil {
		t.Fatal("malformed cookie escape was accepted")
	}
}
