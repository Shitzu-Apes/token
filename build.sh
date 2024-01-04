#!/bin/bash
set -e
cd "`dirname $0`"

cargo build --release -p token --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

wasm-opt -O4 res/token.wasm -o res/token.wasm --strip-debug --vacuum
