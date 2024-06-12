build-web:
    rm -rf dist
    rm -rf www/dist
    wasm-pack build life --debug
    cd www && npm install && npm run build
    cd server && cargo build
    mkdir dist
    cp target/debug/server dist
    cp -r www/dist dist/assets

build-web-release:
    rm -rf dist
    rm -rf www/dist
    wasm-pack build life --release
    cd www && npm install && npm run build
    cd server && cargo build --release
    mkdir dist
    cp target/release/server dist
    cp -r www/dist dist/assets
