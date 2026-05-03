XIDLC_BIN ?= ../target/debug/xidlc

.PHONY: test test-python-codegen test-python-runtime

test: test-python-codegen test-python-runtime

test-python-codegen:
	RUSTC_WRAPPER= cargo build --manifest-path ../xidlc/Cargo.toml --bin xidlc --features "cli fmt"
	$(XIDLC_BIN) gen --out-dir "xidlc_examples/hello_world" python ../xidlc/tests/python/hello_world.idl; \
	$(XIDLC_BIN) gen --out-dir "xidlc_examples/http_security" python-http ../xidlc/tests/python-http/http_security.idl; \
	$(XIDLC_BIN) gen --out-dir "xidlc_examples/http_stream" python-http ../xidlc/tests/python-http/http_stream.idl; \

test-python-runtime:
	PYTHONPATH=. python3 -m unittest discover -s tests -v
