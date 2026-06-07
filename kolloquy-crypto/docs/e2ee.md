# Kolloquy E2EE

## Message Encryption

Each participant's device with access to a chat has the following stored by the
server:

- A `message_counter`, counting how many messages they have sent
- A `public_key`, the public key of the derived X25519 key pair for the device to access the chat.
- A `derivation_salt`, which was used as the salt in the derivation in the X25519 key pair

### Derivation mode `passauth`

Scrypt (from a WASM library in the browser or native library) is used to derive
the message's X25519 private key, from the following low entropy material:

- Password (after successful authentication)
- Chat ID
- Device number
- Derivation salt (as the salt)

This is the root key.

The public key is then computed from the newly derived private key and sent to
the server for storage.

### Derivation mode `passlessauthweb`

This is very similar to the process for attachment key derivation.

If the device deriving the user's X25519 key pair (deriving device) for this
chat is device 0, then the client should force the user to sign in again via the
device's associated passkey. The PRF extension is used to get 32 bytes of
material.

If the device is not device 0, we must request authentication on device 0 using
its passkey. 

With the following material, we concatenate the user id (which is essentially
30b of entropy), the chat id (which is another 30b of entropy), and XOR the
device number. In total, this gives us 60b of IKM. We then use HKDF and our salt
to derive the X25519 private key (the root key).

We compute the public key and send it off to the server for storage.

### Deriving Message Keys

From the X25519 root key and recipients' public keys, an HMAC-SHA384 key is
derived using HKDF, with the salt as the `derivation_salt` again. This is the
signing key.

We also derive an AES-GCM key from the X25519 key pair. This is the encryption
key.

The client bundles a newly generated salt alongside the message data for use in
the NEXT message's signing and encryption key derivation process.

When the client does send a new message, the message's new signing key is
derived from the previous sent message's signing key and the bundled salt, using
HKDF. The new encryption key is derived from the previous sent message's
encryption key using HKDF with the bundled salt from the previous message. This
is repeated for every subsequent message. These keys are never saved, so the full
chain must be reconstructed every time the chat is read.

With the new message, the sender's `message_counter` PRE-INCREMENT is signed
with the PREVIOUS message's signing key, or if it is the first message, the
current signing key. `message_counter` should be incremented and stored in the
DB _after_ the message is received by recipients. The `message_count` at the time
of sending should be sent with the message, in the encrypted headers (which are
encrypted with the previous message's encryption key, or the current if this is
the first message).

This guarantees that if the current message's signing/encryption keys are obtained,
only future messages can be forged/read. Of course, that is still quite bad, so
ideally it would be best for the user to either rotate keys and lose old messages
or delete the chat entirely.

### Message parts

Messages are made up of 3 parts: the plaintext headers, encrypted headers, and
encrypted body.

The plaintext headers contain the following information:

- Recipient user ID
- Sender user ID
- The IV used to encrypt the encrypted headers
- The nonce used to encrypt the encrypted headers

The encrypted headers are E2E encrypted using the previous message's encryption
key (or the current if this is the first message). It keeps the 16 byte
authentication tag appended to it in the output of `SubtleCrypto.encrypt`. It
contains the following information:

- Sender's `message_count` at the time of signing
- The siganture of the `message_count` at the time of sending
- The plaintext message's signature
- The IV used to encrypt the encrypted body
- The nonce used to encrypt the encrypted body
- The salt for the next message's signing and encryption key

The encrypted body is the E2E encrypted body of the message itself. It keeps the
16 byte authentication tag appended to it in the output of `SubtleCrypto.encrypt`.

## Attachment key pair

The attachment key pair is unique per-user. In other words, every user has one
and each fo their devices use it.

Since there is only one attachment key, it must be derived from only one of the
user's sign in methods. When the user creates an account, after they have had the
option to add passkeys and associate them with devices, they get asked:

```
Which method of login would you like to use to securely encrypt attachments you
send and receive? This cannot be changed later.

- My password
[if a passkey was added] - The passkey for this device (device 0)
- Secret code (which I must copy down and store securely)
```

If the method chosen is their password, then the derivation is simple across
devices. If a key cannot be found, or is invalid, force a reauthentication and
re-derive the private key from the salt the server returns on successful auth
and the password material.

If the method chosen is a secret code, the user will be asked to confirm. If they
do confirm, then a random private key is generated on the client, and they are
forced to write it down and confirm they have done so. Then when the application
would normally need to re-derive a key the user (after needing to re-authenticate
anyway) will have to re-enter their attachment secret key.

If the method chosen is to use device 0's passkey, then the private key will be
derived after a re-authentication, using the PRF extension with WebAuthn or
native equivalent. Whenever the client needs to re-derive the attachment key, the
client sets the `devno_needs_device0_auth` column for the user (in the `user_keyshare`
table) to the current device number (from `NULL`). The user is then asked to
login on device 0. The server should log out of a session if one is active for
device 0. When the user logs in on device 0 using their passkey, the private key
is derived, and the `devno_needs_device0_auth` column is set to NULL, and the
`wrapped_attachment_privkey` field is set to the derived private key (from `NULL`),
wrapped with a key derived using ECDH from the requesting device's
(`devno_needs_device0_auth`) public key and device 0's private key (which is
different from the attachment key). The user then goes back to the original device,
and continues to the next menu. The wrapped private key is sent to the requesting
device by the server (and `wrapped_attachment_privkey` is set to `NULL`). The
requesting device then derives the key to unwrap the private key using ECDH with
device 0's public key and their device's private key (derived after reauthenticating
on the requesting device). The private key is unwrapped and the attachments can
be viewed again from that device (and hopefully the key can be added to iCloud
Keychain or equivalent, set to synchronise across devices, and be shared to
other devices without the need for rederivement).

If the user elects to not use a secret code, they will still be presented with a
backup of the dervied private key to be written down. They can use this if they
cannot use the preferred derivation method when the private key is needed.

## Key Rotation

Whenever a user leaves a chat, the keys must be rotated to ensure they cannot
access any messages in the chat. 