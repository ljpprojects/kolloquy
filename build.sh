#!/bin/zsh

# Script to build Kolloquy's server and scripts

#########################
###  Compile scripts  ###
#########################

echo "----- Compiling scripts -----"

rm client/dist/*.js

tsc || exit
cd client/dist || exit

for script in *.js; do
  out="${script%.js}.min.js"

  npx uglify-js --validate --keep-fnames -co "$out" "$script"
done

cd ../..

########################
### Build the server ###
########################

echo "----- Building the server -----"

export OPENSSL_DIR="$(brew --prefix)/opt/openssl@3"
export PKG_CONFIG_PATH="$OPENSSL_DIR/lib/pkgconfig:$PKG_CONFIG_PATH"
export PATH="$OPENSSL_DIR/bin:$PATH"
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar

export CARGO_BUILD_RUSTFLAGS="-C link-arg='-Bstatic' -C linker-flavor=ld -C linker=aarch64-linux-gnu-ld -L /usr/local/Cellar/aarch64-unknown-linux-gnu/13.3.0/toolchain/lib/gcc/aarch64-unknown-linux-gnu/13.3.0"

cd server || exit

cargo build --release --target aarch64-unknown-linux-gnu || exit

cd ..

#################
### Packaging ###
#################

echo "----- Packaging -----"

tar -cavf kolloquy.tar.gz server/target/aarch64-unknown-linux-gnu/release/server .env