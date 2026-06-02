GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path
XIDLC_BIN ?= ../target/debug/xidlc
XIDLC_SNAPSHOT_VERSION ?= 0.0.1
XIDLC_SNAPSHOT_HASH ?= snapshot

.PHONY: test test-go-codegen test-go-runtime

test: test-go-codegen test-go-runtime

test-go-codegen:
	XIDLC_VERSION=$(XIDLC_SNAPSHOT_VERSION) XIDLC_GIT_HASH=$(XIDLC_SNAPSHOT_HASH) RUSTC_WRAPPER= cargo build --bin xidlc -F cli,fmt
	tmpdir=$$(mktemp -d); \
	trap 'rm -rf "$$tmpdir"' EXIT; \
	$(XIDLC_BIN) gen --out-dir "$$tmpdir/hello_world" go-rest xidlc-examples/api/http/hello_world.idl; \
	diff -u "$$tmpdir/hello_world/hello_world_http.go" xidlc-examples/hello_world/hello_world_http.go; \
	$(XIDLC_BIN) gen --out-dir "$$tmpdir/http_media_types" go-rest xidlc-examples/api/http/http_media_types.idl; \
	diff -u "$$tmpdir/http_media_types/rest_media_types.go" xidlc-examples/http_media_types/rest_media_types.go; \
	diff -u "$$tmpdir/http_media_types/rest_media_types_http.go" xidlc-examples/http_media_types/rest_media_types_http.go; \
	$(XIDLC_BIN) gen --out-dir "$$tmpdir/http_stream" go-rest xidlc-examples/api/http/http_stream.idl; \
	diff -u "$$tmpdir/http_stream/http_stream_http.go" xidlc-examples/http_stream/http_stream_http.go; \
	$(XIDLC_BIN) gen --out-dir "$$tmpdir/http_security" go-rest xidlc-examples/api/http/http_security.idl; \
	diff -u "$$tmpdir/http_security/http_security_http.go" xidlc-examples/http_security/http_security_http.go

test-go-runtime:
	cd xidl-go-json && GOCACHE=$(GO_CACHE) GOPATH=$(GO_PATH) go test ./...
	cd xidlc-examples && GOCACHE=$(GO_CACHE) GOPATH=$(GO_PATH) go test ./...
