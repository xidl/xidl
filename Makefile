test-update:
	INSTA_UPDATE=always cargo test

build-xtypes:
	cargo r -p xidlc -- --lang rust --out-dir ./xidl-typeobject/src/ ./xidl-typeobject/idl/dds-xtypes_typeobject.idl
