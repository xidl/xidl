GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path
XIDLC_BIN ?= ../target/debug/xidlc

.PHONY: test test-go-codegen test-go-runtime

test: test-go-codegen test-go-runtime

test-go-codegen:
	cargo build -p xidlc
	tmpdir=$$(mktemp -d); \
	trap 'rm -rf "$$tmpdir"' EXIT; \
	$(XIDLC_BIN) --lang go-http --out-dir "$$tmpdir/hello_world" xidlc-examples/api/http/hello_world.idl; \
	diff -u "$$tmpdir/hello_world/hello_world_http.go" xidlc-examples/hello_world/hello_world_http.go; \
	$(XIDLC_BIN) --lang go-http --out-dir "$$tmpdir/http_media_types" xidlc-examples/api/http/http_media_types.idl; \
	diff -u "$$tmpdir/http_media_types/http_media_types.go" xidlc-examples/http_media_types/http_media_types.go; \
	diff -u "$$tmpdir/http_media_types/http_media_types_http.go" xidlc-examples/http_media_types/http_media_types_http.go; \
	$(XIDLC_BIN) --lang go-http --out-dir "$$tmpdir/http_stream" xidlc-examples/api/http/http_stream.idl; \
	diff -u "$$tmpdir/http_stream/http_stream_http.go" xidlc-examples/http_stream/http_stream_http.go; \
	$(XIDLC_BIN) --lang go-http --out-dir "$$tmpdir/http_security" xidlc-examples/api/http/http_security.idl; \
	diff -u "$$tmpdir/http_security/http_security_http.go" xidlc-examples/http_security/http_security_http.go

test-go-runtime:
	cd xidlc-examples && GOCACHE=$(GO_CACHE) GOPATH=$(GO_PATH) go test ./...
