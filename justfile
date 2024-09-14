build-web:
    rm -rf dist
    rm -rf www/dist
    wasm-pack build life --debug
    cd www && npm install && npm run build
    mkdir dist
    cp -r www/dist/* dist/

build-web-release:
    rm -rf dist
    rm -rf www/dist
    wasm-pack build life --release
    cd www && npm install && npm run build
    mkdir dist
    cp -r www/dist/* dist/

# The same as build-web-release, but without building the server
pages-ci:
    rm -rf dist
    rm -rf www/dist
    wasm-pack build life --release
    cd www && npm install && npm run build
    mkdir dist
    cp -r www/dist dist/assets
