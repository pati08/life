build-web:
    rm -rf dist
    mkdir dist
    RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" wasm-pack build life --target no-modules --debug --out-dir ../dist -- -Zbuild-std=std,panic_abort
    # RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" cargo build --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort -p life
    # wasm-bindgen \
    #     ./target/wasm32-unknown-unknown/debug/life.wasm \
    #     --out-dir \
    #     ./dist \
    #     --target \
    #     no-modules
    cp index.js dist
    cp index.html dist
    cp worker.js dist

build-web-release:
    rm -rf dist
    mkdir dist
    RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" wasm-pack build life --target no-modules --release --out-dir ../dist -- -Zbuild-std=std,panic_abort
    # RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" cargo build --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort -p life
    # wasm-bindgen \
    #     ./target/wasm32-unknown-unknown/debug/life.wasm \
    #     --out-dir \
    #     ./dist \
    #     --target \
    #     no-modules
    cp index.js dist
    cp index.html dist
    cp worker.js dist
