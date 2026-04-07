XIDLC_BIN ?= ../target/debug/xidlc

.PHONY: test test-python-codegen test-python-runtime

test: test-python-codegen test-python-runtime

test-python-codegen:
	RUSTC_WRAPPER= cargo build --manifest-path ../xidlc/Cargo.toml --bin xidlc --features "cli fmt"
	tmpdir=$$(mktemp -d); \
	trap 'rm -rf "$$tmpdir"' EXIT; \
	$(XIDLC_BIN) --lang python --out-dir "$$tmpdir/hello_world" ../xidlc/tests/python/hello_world.idl; \
	diff -u -B "$$tmpdir/hello_world/hello_world.py" xidlc_examples/hello_world/hello_world.py; \
	$(XIDLC_BIN) --lang python-http --out-dir "$$tmpdir/http_security" ../xidlc/tests/python-http/http_security.idl; \
	diff -u -B "$$tmpdir/http_security/http_security.py" xidlc_examples/http_security/http_security.py; \
	diff -u -B "$$tmpdir/http_security/http_security_http.py" xidlc_examples/http_security/http_security_http.py; \
	$(XIDLC_BIN) --lang python-http --out-dir "$$tmpdir/http_stream" ../xidlc/tests/python-http/http_stream.idl; \
	diff -u -B "$$tmpdir/http_stream/http_stream_http.py" xidlc_examples/http_stream/http_stream_http.py

test-python-runtime:
	PYTHONPATH=. python3 -m unittest discover -s tests -v
