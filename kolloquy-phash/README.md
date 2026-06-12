# Kolloquy P(assword)Hash(ing) Server

This is the server which Kolloquy uses to compute password hashes.

## Tested Targets

### `aarch64-apple-darwin`

Keyring set/get & demo case for hashing as intended.

### `aarch64-unknown-linux-gnu` & `aarch64-unknown-linux-musl`

Untested as of yet; need to setup raspberry pi (it is still being zeroed out ugh)

### `x86_64-unknown-linux-gnu` & `x86_64-unknown-linux-musl`

Keyring set/get & demo case for hashing work as intended.

Also holy shit why is that so GOOD at computing the hashes???
Like it just straight up crunched through them in not even 300ms for musl and gnu
Even the macbook I tested aarch64-apple-darwin on took like 2.5s each

### Windows? Are you fucking insane???

## Recommended Setup
