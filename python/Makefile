XIDLC_BIN ?= ../target/debug/xidlc

.PHONY: test test-python-codegen test-python-runtime

test: test-python-codegen test-python-runtime

test-python-codegen:
	cargo build --manifest-path ../xidlc/Cargo.toml --bin xidlc --features "cli fmt"
	$(XIDLC_BIN) --lang python --out-dir "xidlc_examples/hello_world" ../xidlc/tests/python/hello_world.idl; \
	$(XIDLC_BIN) --lang python-http --out-dir "xidlc_examples/http_security" ../xidlc/tests/python-http/http_security.idl; \
	$(XIDLC_BIN) --lang python-http --out-dir "xidlc_examples/http_stream" ../xidlc/tests/python-http/http_stream.idl; \

test-python-runtime:
	PYTHONPATH=. python3 -m unittest discover -s tests -v
