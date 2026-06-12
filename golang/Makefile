GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path

.PHONY: test test-go-runtime

test: test-go-runtime

test-go-runtime:
	cd xidl-go-codec && GOCACHE=$(GO_CACHE) GOPATH=$(GO_PATH) go test ./...
