# Keysharing Sessions

## Creation

### RDAH

The RD should randomly generate an X25519 keypair and a 30 byte salt, and pick a
random port between `15579` and `56568`.

The last 4 bytes of the public key, when base-16 encoded, form the ID of the
keyshare.

The RD then will `POST` to `ks.kolloquy.com/rdah` with the user's `SSID` cookie
set. The `callback` query parameter should be set to the authority of the URI
where the RDAH server is accessible (e.g. `192.168.1.55:22222`). The `id` query
parameter should be set to the ID of the session (which should have been created
prior). The body of the request should match the schema in `schemas/ks/POST-rdah.req.json`.

From here, the Keyshare server should update the user's keysharing requests with
the new RDAH request, and send a response matching `schemas/ks/POST-rdah.res.json`.

### SDAH

