# Key Management

## On-Device Key Storage

When using the iOS, iPadOS, watchOS, or macOS Kolloquy applications, the keys
for E2E chat encryption are stored in iCloud Keychain.

When a chat private key is stored, it is marked as `ThisDeviceOnly`, as each
user's device has a seperate key pair. When a chat is opened, the app checks to
see if the key is in iCloud Keychain. If it is (and the additional version info
is up to date), then it doesn't need to be rederived (which requires sign in).

The attachment E2EE private key is also stored in iCloud Keychain, and is marked
`synchronizable`, as it is not unique across devices.