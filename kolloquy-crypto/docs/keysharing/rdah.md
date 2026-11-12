# Remote Device As Host Keysharing

## Session creation

Create a session, S, using the instructions in the dedicated session document.

The RD cannot continue until the session is fully initialised. This requires the
user to now move to the source device (SD).

On the SD, with the newly-created keyshare session selected:

1. Generate an X25519 keypair
2. `PUT` to `ks.kolloquy.com/rdah` with the `SSID` cookie and `id` query parameter set, with a body matching the schema in `schemas/ks/PUT-rdah.req.json`