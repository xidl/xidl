#!/bin/sh
set -e

TARGET=wasm32-unknown-emscripten
OUT_DIR=target/$TARGET/release
PUBLIC_DIR=xidl-playground/public/wasm

RUSTFLAGS='-C link-arg=-sEXPORTED_FUNCTIONS=["_xidlc_generate_json","_xidlc_free_string"] -C link-arg=-sEXPORTED_RUNTIME_METHODS=["ccall","cwrap","UTF8ToString"] -C link-arg=-sMODULARIZE=1 -C link-arg=-sEXPORT_NAME=xidlcModule' \
  cross +nightly build -p xidlc --target $TARGET --profile release

mkdir -p $PUBLIC_DIR
cp $OUT_DIR/xidlc.js $PUBLIC_DIR/xidlc.js
cp $OUT_DIR/xidlc.wasm $PUBLIC_DIR/xidlc.wasm

echo "WASM artifacts copied to $PUBLIC_DIR"
