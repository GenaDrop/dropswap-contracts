set NEAR_ENV=mainnet
set CONTRACT_ID=v1.havenswap.near

near deploy --wasmFile target/out/main.wasm --accountId %CONTRACT_ID%