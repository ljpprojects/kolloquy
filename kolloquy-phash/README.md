# Kolloquy P(assword)Hash(ing) Server

This is the server which Kolloquy uses to compute password hashes.

## Tested Targets

- `aarch64-apple-darwin`

## Recommended Setup

There should be a separate user for running the Kolloquy PHash server, named
`klqy`. The provided nginx config expects this. The `kolloquy-phash` binary
should be executed as this user, with the cwd set to `/home/klqy`.

The layout of the `klqy` user's home directory should look like this (assuming
all files and directories are owned by `klqy` unless specified otherwise):

```
/home/klqy          (0700)
├── .env            (SECRET config file, 0400, can't even trust ourselves with it)
├── kolloquy-phash  (The PHash server binary, should NOT be a link under the default apparmor rules, 4500)
└── phash           (0600)
    ├── mtls.crt    (Certificate for MTLS keypair, PEM format, 0400)
    ├── mtls.key    (Private key of the MTLS keypair, PEM format, 0400)
    ├── mtls-ca.crt (Certificate for the CA keypair used to create the MTLS keypair, 0400)
    ├── ssl.crt     (Certificate for the SSL keypair, PEM format, 0400)
    └── ssl.key     (Private key of the SSL keypair, PEM format, 0400)
└── cache           (Directory to use for cache, 0600)
```

(Technically anything more restrctive than `0700` doesn't really matter)