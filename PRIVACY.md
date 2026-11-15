# Privacy Policy

**Last updated: November 12th, 2026**

This is the privacy policy for the Kolloquy server personally hosted by me
("ljpprojects"), referred to as "Kolloquy.com", "Us", or "We".

## Information We Collect

This section details the data We collect when you use the service. This includes
data you give Us and data We infer. In the event of a data breach, all the data
stored on Our servers may be leaked. We will notify you if we become aware that
your data has been made available in a data breach via the official
`@kolloquyprivacy:kolloquy.com` account.

### Phone Numbers

You may link a phone number to your Kolloquy.com account. This number will be
sent to the server with encryption required (via HTTPS). The server shares your
phone number with Amazon Web Services, Inc. ("AWS") to send a one-time code to
verify that you own that phone number (using AWS Simple Notification Service).
Please read their privacy policy to know how AWS will use that data.

Once your phone number has been verified, it is cryptographically hashed (via
SHA256). We store this hash on Our servers. The nature of a hash prevents the
reversal from the hash to the original input without a brute-force method, so We
cannot read your phone number. In the event of a data breach, only the hash of
your phone number can be leaked. We will still notify you if we become aware
that any of your data has been made available in a data breach.

### Emails

You may link an email to your Kolloquy.com account. This email is sent to Our
servers. The server shares your email with AWS to send a
one-time code to verify that you own that email (using AWS Simple Email Service).
Please read their privacy policy to know how AWS will use this data.

Once your email has been verified, it is cryptographically hashed (via SHA256).
We store this hash on Our servers. The nature of a hash prevents the reversal
from the hash to the original input without a brute-force method, so We cannot
read your email. In the event of a data breach, only the hash of
your email will be leaked. We will still notify you if we become aware that any
of your data has been made available in a data breach.

### Identifiers

When you create a Kolloquy.com account, the server generates a User ID for you,
a unique 8-byte sequence which we use to identify your account. Your User ID is
stored on Our servers, and is publicly visible. We also require that you select
a publicly-visible handle for your kolloquy.com account, which we store on Our
servers.

You may also add a profile picture for your kolloquy.com account. Depending on
your account preferences, it may be publicly visible. If your account is set to
public, your profile picture will be stored unencrypted on our servers, and is
visible publicly to anyone with access to the internet. If your account is set
to friends-only (or any other mode which restricts visibility to a specific
group of users), your profile picture is end-to-end encrypted once per user who
is allowed to view your profile. The nature of end-to-end encryption prevents
Us and anyone who you have not granted access to view your profile from viewing
your profile picture.

The device that you use to register your account is also tracked by Our servers.
We generate a Device ID for it, a unique 8-byte sequence we use to identify that
device. We also create Device IDs for any other devices you add to your
Kolloquy.com account. You may add a nickname for the device when it is
registered with Our system. This is stored on Our servers and can be viewed from
any of your registered devices.

### Messages

When you send a message through Us, it is end-to-end encrypted on your device.
The nature of end-to-end encryption prevents Us and anyone who you have not
granted access to read your message from viewing your message. We store these
encrypted messages for as long as you specify in your account settings. This
defaults to 1 week.

You can also configure your account to delete messages from Our servers once
they have been read on all devices that can read them. In a chat, we use the
most permissive configuration from its participants. You may view the active
configuration for your messages in a chat at any time. You may delete a message
from Our servers at any time. This will make it not viewable to any participant
in that chat. The message will not be cleared from participants' on-device
message histories instantaneously, but will be removed as soon as possible.

Messages are stored on-device indefinitely. You can clear your on-device message
history or delete a specific message from your device's message history at any
time.

We also collect other metadata about your messages, which is not end-to-end
encrypted. This consists of the time it was sent, the User IDs (and Device IDs
if applicable) of recipients, and the cryptographic salt used in encrypting the
message.

### Preferences

We store your account preferences on Our servers unencrypted.

## Information We Share

As mentioned above, your email and phone number will be shared with AWS once to
send a one-time code to verify your ownership of the email or phone number.
Please read their privacy policy to know what they do with this information. We
only store a hash of your email or phone number on Our servers.

We may also need to share your data to meet applicable laws, regulations, legal
processes, or enforcable government requests. In these events, all of the data
we collect may be shared.

## Updates

This privacy policy was last updated on November 12th, 2026.

When We change the privacy policy, We will gain your explicit consent again
before you can continue to use Our services.

You can view all previous version of this privacy policy in the history of
the `PRIVACY.md` file at
[kolloquy's official git repository](https://github.com/ljpprojects/kolloquy).
The repository is hosted on GitHub, and their terms apply while using their site.

## Contacting Us

You may contact Us if you have questions about the privacy policy by sending an
email to `privacy@kolloquy.com`, or by sending a message to
`@kolloquyprivacy:kolloquy.com` via any kolloquy client. Please note that if you
contact us via a kolloquy client, the terms of that client apply, and if you
contact us via email, the terms of your email client apply.
