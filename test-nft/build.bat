cargo build --target wasm32-unknown-unknown --release
copy target\wasm32-unknown-unknown\release\test_nft.wasm target\out\main.wasm