GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path

.PHONY: test test-go test-go-codegen test-go-runtime test-update build-xtypes docs-dev docs-build

test:
	cargo test --all

test-go: test-go-codegen test-go-runtime

test-go-codegen:
	$(MAKE) -C golang test-go-codegen GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-go-runtime:
	$(MAKE) -C golang test-go-runtime GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-update:
	INSTA_UPDATE=always cargo test

build-xtypes:
	cargo r -p xidlc -- --lang rust --out-dir ./xidl-typeobject/src/ ./xidl-typeobject/idl/dds-xtypes_typeobject.idl

docs-dev:
	pnpm --dir docs start

docs-build:
	pnpm --dir docs build
