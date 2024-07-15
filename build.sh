#!/bin/bash
cd vp_proxy
cargo run --features export-api > candid.did
cd ..
cargo build --release --target wasm32-unknown-unknown --features export-api