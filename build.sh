set -euo pipefail

while [[ -e .lock ]]; do
    echo "Exiting: lock is held."

    exit 0
done

touch .lock;

function recompile {
    cargo clean -p kolloquy --target wasm32-unknown-unknown --release
    cargo build --release --target wasm32-unknown-unknown
}

function check_wasm {
    wasm-objdump -x target/wasm32-unknown-unknown/release/kolloquy.wasm > /dev/null \
    || recompile
}

function optimise_wasm {
    wasm-opt \
        -ifwl -iit -lmu -uim -pii=4 \
        --enable-bulk-memory \
        --enable-nontrapping-float-to-int \
        --disable-fp16 \
        --disable-gc \
        --optimize-for-js \
        -O4 \
        -o temp.wasm index_bg.wasm

    rm index_bg.wasm

    wasm-opt \
        --enable-bulk-memory \
        --enable-nontrapping-float-to-int \
        --flatten \
        -cw \
        --strip-debug \
        --strip-producers \
        -c -Oz -o index_bg.wasm temp.wasm

    rm temp.wasm
}

cargo build --release --target wasm32-unknown-unknown && check_wasm

rm -rf build

mkdir build

# BUILD FRONTEND

cd frontend
tsc

../node_modules/html-minifier-terser/cli.js \
    --collapse-whitespace --use-short-doctype \
    --file-ext html --input-dir . --output-dir \
    ../dist

ls css/*.css | xargs -I "{}" ../node_modules/clean-css-cli/bin/cleancss -dO3 -o "../dist/{}" "{}"

cd ..

# BUILD SERVER

cd build

cp ../target/wasm32-unknown-unknown/release/kolloquy.wasm index.wasm

wasm-bindgen --no-typescript \
    --remove-producers-section --remove-name-section \
    --target bundler --out-dir . index.wasm

# Do not optimise wasm unless this is a release build
if [[ -n "${RELEASE_BUILD+x}" ]]; then
    optimise_wasm
fi

cp ../custom-shim.mjs shim.mjs

../node_modules/esbuild/bin/esbuild --minify --bundle shim.mjs --outfile=index.js --allow-overwrite --external:cloudflare:workers --external:./index_bg.wasm --format=esm

rm shim.mjs
rm index_bg.js

cd ..

rm .lock
