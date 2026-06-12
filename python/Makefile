XIDLC_BIN ?= ../target/debug/xidlc

.PHONY: test test-python-codegen test-python-runtime

test: test-python-codegen test-python-runtime

test-python-codegen:
	RUSTC_WRAPPER= cargo build --manifest-path ../xidlc/Cargo.toml --bin xidlc --features "cli fmt"
	rm -rf tests/generated
	mkdir -p tests/generated
	touch tests/generated/__init__.py
	$(XIDLC_BIN) gen --out-dir "tests/generated/hello_world" python ../xidlc/tests/python/hello_world.idl
	touch tests/generated/hello_world/__init__.py
	$(XIDLC_BIN) gen --out-dir "tests/generated/http_security" python-rest ../xidlc/tests/python-http/http_security.idl
	touch tests/generated/http_security/__init__.py
	$(XIDLC_BIN) gen --out-dir "tests/generated/http_stream" python-rest ../xidlc/tests/python-http/http_stream.idl
	touch tests/generated/http_stream/__init__.py

test-python-runtime:
	python3 -m pip install -r requirements.txt
	PYTHONPATH=. python3 -m unittest discover -s tests -v
