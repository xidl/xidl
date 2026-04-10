GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path

.PHONY: test test-rust test-go test-go-codegen test-go-runtime test-update test-xidl-parser-coverage build-xtypes docs-dev docs-build

test-rust:
	cargo test --all -F transport-all -F fmt

test: test-rust test-go

test-go: test-go-codegen test-go-runtime

test-go-codegen:
	$(MAKE) -C golang test-go-codegen GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-go-runtime:
	$(MAKE) -C golang test-go-runtime GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-update:
	INSTA_UPDATE=always make test-rust

test-xidl-parser-coverage:
	cargo test -p xidl-parser
	cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 95 --out Stdout

build-xtypes:
	cargo r -p xidlc -- --lang rust --out-dir ./xidl-typeobject/src/ ./xidl-typeobject/idl/dds-xtypes_typeobject.idl

docs-dev:
	pnpm --dir docs start

docs-build:
	pnpm --dir docs build
