GO_CACHE ?= /tmp/xidl-go-cache
GO_PATH ?= /tmp/xidl-go-path
JINJA_TEMPLATES := $(shell find xidlc -type f -name '*.j2' | sort)
XIDLC_SNAPSHOT_VERSION ?= 0.0.1
XIDLC_SNAPSHOT_HASH ?= snapshot
BDD_FEATURES ?= bdd/features

.PHONY: test test-rust test-go test-go-codegen test-go-runtime test-update test-coverage test-xidl-parser-coverage test-xidl-jsonrpc-coverage test-xidl-rust-axum-coverage build-xtypes docs-dev docs-build

test: test-rust test-go test-bdd

test-rust: init
	XIDLC_VERSION=$(XIDLC_SNAPSHOT_VERSION) XIDLC_GIT_HASH=$(XIDLC_SNAPSHOT_HASH) cargo test --all -F transport-all -F fmt

test-go: test-go-runtime

test-bdd: init
	pip install -r bdd/requirements.txt
	behave $(BDD_FEATURES)

test-go-codegen:
	$(MAKE) -C golang test-go-codegen GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-go-runtime:
	$(MAKE) -C golang test-go-runtime GO_CACHE=$(GO_CACHE) GO_PATH=$(GO_PATH)

test-update:
	INSTA_UPDATE=always XIDLC_VERSION=$(XIDLC_SNAPSHOT_VERSION) XIDLC_GIT_HASH=$(XIDLC_SNAPSHOT_HASH) make test-rust

test-coverage:
	cargo test -p xidl-parser -p xidl-jsonrpc -p xidl-rust-axum --all-features
	$(MAKE) test-xidl-parser-coverage
	$(MAKE) test-xidl-jsonrpc-coverage
	$(MAKE) test-xidl-rust-axum-coverage

test-xidl-rust-axum-coverage:
	cargo tarpaulin --manifest-path xidl-rust-axum/Cargo.toml --packages xidl-rust-axum --all-features --include-files "xidl-rust-axum/src/*" --fail-under 80 --out Stdout

test-xidl-parser-coverage:
	cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --all-features --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 80 --out Stdout

test-xidl-jsonrpc-coverage:
	cargo tarpaulin --manifest-path xidl-jsonrpc/Cargo.toml --packages xidl-jsonrpc --all-features --include-files "xidl-jsonrpc/src/*" --exclude-files "xidl-jsonrpc/src/transport/io.rs" --fail-under 80 --out Stdout

fmt-jinja:
	cargo r -p xidlc -F cli -F fmt -- fmt -l jinja $(JINJA_TEMPLATES) -i
build-xtypes:
	cargo r -p xidlc -F cli -F fmt -- gen --out-dir ./xidl-typeobject/src/ rust ./xidl-typeobject/idl/dds_xtypes_typeobject.idl

docs-dev:
	pnpm --dir docs start

docs-build:
	pnpm --dir docs build

init:
	cd ./xidlc-examples && pnpm install
	cd ./typescript/xidl-typescript-codec && pnpm install && pnpm build
