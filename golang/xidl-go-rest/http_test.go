package xidlgohttp

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestRequireAccept(t *testing.T) {
	tests := []struct {
		name   string
		accept string
		mime   string
		wantOK bool
	}{
		{
			name:   "missing accept header",
			accept: "",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "exact application/json",
			accept: "application/json",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "star slash star",
			accept: "*/*",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "browser compound header with star slash star q param",
			accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "type wildcard token with q param",
			accept: "application/*;q=0.9",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "wildcard subtype json",
			accept: "*/json",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "case insensitive media type",
			accept: "Application/JSON",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "json with charset parameter",
			accept: "application/json;charset=utf-8",
			mime:   "application/json",
			wantOK: true,
		},
		{
			name:   "non matching media type",
			accept: "text/plain",
			mime:   "application/json",
			wantOK: false,
		},
		{
			name:   "compound header without wildcard or json",
			accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp",
			mime:   "application/json",
			wantOK: false,
		},
		{
			name:   "empty required mime always accepted",
			accept: "text/plain",
			mime:   "",
			wantOK: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, "/", nil)
			if tt.accept != "" {
				req.Header.Set("Accept", tt.accept)
			}
			err := RequireAccept(req, tt.mime)
			if tt.wantOK && err != nil {
				t.Fatalf("RequireAccept(accept=%q, mime=%q) returned error: %v", tt.accept, tt.mime, err)
			}
			if !tt.wantOK && err == nil {
				t.Fatalf("RequireAccept(accept=%q, mime=%q) expected error, got nil", tt.accept, tt.mime)
			}
		})
	}
}
