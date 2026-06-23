# Kolloquy P(assword)Hash(ing) Server

This is the server which Kolloquy uses to compute password hashes.

## Tested Targets

### `aarch64-apple-darwin`

Keyring set/get & demo case for hashing work as intended.

### `aarch64-unknown-linux-gnu` & `aarch64-unknown-linux-musl` (the big ones)

Keyring set/get & demo case for hashing work as intended.

Again, the hashing is the same speed as on my x86-64 linux machine, which says
something about the macbook, as even a raspberry pi 5 is beating it...
Setting it up was such a pain holy shit
I did not give the rootfs enough space...

```
$ df -h
Filesystem        Size  Used Avail Use% Mounted on
/dev/sda2         3.9G  3.2G  507M  87% /
/dev/sda1        1022M   66M  957M   7% /boot
/dev/mapper/home   20G  2.8G   16G  15% /home
```

### `x86_64-unknown-linux-gnu` & `x86_64-unknown-linux-musl`

Keyring set/get & demo case for hashing work as intended.

Also holy shit why is that so GOOD at computing the hashes???
Like it just straight up crunched through them in not even 300ms for musl and gnu
Even the macbook I tested aarch64-apple-darwin on took like 2.5s each

### Windows? Are you fucking insane???

## Recommended Setup
