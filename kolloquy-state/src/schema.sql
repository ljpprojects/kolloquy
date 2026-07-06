-- The MFA chain goes like this:
-- 1. Passkeys/WebAuthn (preferred, can replace passwords entirely)
-- 2. TOTP + Password
-- 3. Password + Code to another authenticated device (try to use instead of email)
-- 4. Email + Password (last resort)
-- 5. Password reset (code to other authenticated device)
-- 6. Password reset (code to email)

create table users(
    id text
        not null
        primary key
        check (length(id) = 40),
    display_name text
        unique
        not null
        check (length(display_name) <= 18), -- Length should be 18 at most

    -- All password PHC strings will be unqiue because of the salt
    -- If this is null then the account is a passwordless account and must have
    -- a passkey (if it doesn't the state is invalid but a password reset should
    -- be initiated nonetheless)
    password_phc text
        unique,

    -- The user's primary verified email. Email is never used as a valid MFA method.
    -- Email will NOT be used to send promotional messages.
    email text unique not null,

    -- A base64 encoded SHA-256 hash of the user's verified phone number (E.164
    -- formatted). It is sent to the backend once to verify it belongs to the
    -- user but is not stored.
    -- It can still be used to lookup users by phone number thanks to the format
    -- normalisation.
    phone_number text unique,

    -- Public key for the keypair used to E2E encrypt attachments specifically
    -- This would somehow have to be deriveable across devices, which is simple
    -- for passauth, but for passlessauthweb????
    -- Wait I have an idea
    -- So the main device (device 0) will derive a seperate key to encrypt the
    -- attachments for passlessauthweb, and then other devices can use their
    -- keypairs to E2EE transfer that so they can decrypt it too???
    -- Each other device can then store the attachment locally (but cannot upload
    -- it, I have limits)
    -- Thus if device 0 is replaced, and the attachment is stored locally for
    -- the new device, it can quickly replace the key for every other device and
    -- reencrypt attachments????? If it isnt cached on the new device... maybe I
    -- will have a recovery feature to GET the private key from device 0 BEFORE
    -- you can remove it, so you can give THAT to the new device 0
    -- It will also make you take note of the secret key when you add another
    -- device
    -- Device 0 is an insert for the device with the lowest device_num
    attachment_pubkey text not null unique
        check(length(attachment_pubkey) = 43),
) strict;

create table user_devices(
    devno integer not null,
    id text not null,

    -- If this is true the device has its own key pair
    -- This would mainly be false for "auxillary devices" (e.g. smartwatches)
    -- that can rely on another device to decrypt messages for it
    is_independent boolean not null,

    nickname text not null
        check (length(nickname) <= 18), -- The nickname either given to the device by the user or inferred

    primary key(devno, id),

    foreign key(id) references users(id)
        on delete cascade
) strict;

-- Stores the webauthn data of passkeys for users
-- If a user has a passkey (or many passkeys)
-- A user can have 3 passkeys at most (as each passkey has a different key)
create table user_passkeys(
    credential_id text not null, -- Base64url-encoded PublicKeyCredential.id
    user_id text not null,       -- ID of the user whose passkey this is

    -- signature counter; if new_count <= counter then the login is flagged and
    -- will require MFA (or block if no MFA is available).
    counter integer,

    public_key text not null,
    webauthn_challenge text,    -- Stores the challenge for WebAuthn registration/login

    primary key(credential_id, user_id),

    foreign key(user_id) references users(id)
        on delete cascade
) strict;

-- The second toffee batch is made and it definitely isnt burnt but it might be
-- closer to taffee than toffee (although it is still warm so who knows)
-- - 15:26 Sunday
-- Too bad I used all the white chocolate
-- I think it might be slightly undercooked because after a few minutes it didn't
-- thicken nearly as much as the definite toffee (although when it cools I can
-- just through it back into the pot)
-- Holy shit I got a relatively good bit (still tastes burnt) from yesterdays
-- batch and initially it actually tased like toffee
-- Also I wasnt expecting toffee to be so brittle
-- Its been 6 inutes in the fridge now that should be enough
--
-- Update 15:54
-- was indeed undercooked; cooked more
--
-- This is no high (its normal)
-- GRADUAL
--
-- 16:07 yeah its no toffee but ill take it it tastes good but it is quite soft
-- (I like it though)
-- I don't think it would be chewy enough to call taffy though
-- Based on the feel it maybe got to the soft/firm-ball stage???
--
-- OK fuck this more cooking
--
-- Oh fucking great now its fucking burnt
-- FUCK
-- Now everything is fucking burnt, two batches and myself (figuratively and literally, refer to yesterday)
-- In the bin it goes, yay
-- Thats enough of this bullshit
-- I tried to fucking warn him
--
-- Just because a fire burns gradually does not mean pouring diesel on it won't
-- make it burn faster
--
-- Ugh
--
-- The thing with diesel on a fire is that eventually it will all be burnt, and
-- the fire will burn gradually again, but it will ultimately have less fuel.
-- Crucially, it will still have fuel (for a time).
--
-- I still have at least 500g of dark brown sugar though so maybe more butterscotch
-- sauce (and actual heavy cream)

-- Stores totp info for a user
create table users_totp(
    wrapped_secret text not null unique
        check (length(wrapped_secret) = 43), -- The 20b secret (wrapped by a secret global key using AES-KW),
    id text not null,

    primary key(wrapped_secret, id),

    -- I couldn't bring myself to do it (say it aloud, that is)
    -- Also on further inspection that QR code led to a REFERRAL form
    -- Like it REQUIRED referrer's details
    -- That won't stop me from putting it in the only assessment I have upcoming
    -- though
    -- Also the whole counsellor thing is referred to as "someone to talk to"
    -- but that will probably be insufficient
    -- Crisis lines are good and all... until I find myself in a crisis I guess

    foreign key(id) references users(id)
        on delete cascade
);

create table users_keyshare(
    id text,
    devno_wants_device0_auth integer,
    auth_purpose text
        check (text is null or text in ("attachment_privkey", "passlessauthweb_material")),
    keyshare_fulfilled boolean,

    primary key(devno_wants_device0_auth, id),

    foreign key(id)
        references users(id)
);

-- Chats are limited to 6 or less members, as every message is reencrypted for
-- each recipient (using an ECDH keypair for each user)
create table chats(
    id text primary key,       -- The ID of the chat (30b of entropy)
    name text
        not null
        check (length(name) <= 50),
) strict;

-- Stores the user's keypair for each of their devices for each of their chats
-- A user can have a maximum of 4 registered devices to prevent the explosion of
-- key pairs.
--
-- This would leave room for each user to have a phone, laptop, smartwatch (that
-- is set up to independently read messages, but could also depend on the phone),
-- and another device to have keys for every chat.
--
-- Assuming each keypair takes up 96 bytes, if a user had 4 devices, and 10 chats
-- each with 6 members who all have 4 devices (including themselves),
-- that totals 200 keypairs per device (18.75KiB on each of their devices). If
-- each chat had 100 messages with an average size of 128b that would be 120KiB
-- of precious space in R2 (excluding other metadata).
--
-- This would get out of hand very quickly, so the strict max of 4 devices/user
-- and 6 users/chat is enforced to A) maintain speed and B) prevent chats consuming
-- a massive amount of storage.
--
-- If a key does not exist for a chat (a lookup by (devno, user_id, chat_id)
-- yields no results) then the device does not have permission to access that
-- chat
-- We can use this also to check if a user participates in a chat, query by (user_id, chat_id) and if rows show up the user participates in the chat
-- And we also use this to see which chats a user is in by querying by user_id
create table chats_devices_keys(
    device_num integer not null,         -- The number designated for this device
    user_id text not null,               -- The ID of the user whose device it is
    chat_id text not null,               -- The ID of the chat they key is for
    message_counter integer not null,    -- A counter that is incremented for every message sent
    send_allowed boolean not null,       -- Is this device allowed to send messages?
    public_key text not null unique,     -- The public key of the derived signing keypair (base64)
    derivation_salt text not null unique -- Salt to use for PKDF2 (32 bytes, base64)
    derivation_method text not null      -- The method used by the client to derive the private key
        check (derivation_method in ("passauth", "passlessauthweb")),

    -- This is how the E2EE private key is derived for the two methods:
    -- passauth:
    --   The initial material comprises:
    --     - The password itself (after it has been verified)
    --     - The saved or given device number
    --     - The chat ID (decoded)
    --     - The user ID (decoded)
    --     - Kolloquy Auth 1.0.0 magic number (0xafaf1e1e090901FF)
    --   With this material the salt (derivation_salt) is used to derive 256 bits
    --   which are used as the private key. If the user is joining the chat for
    --   the first time the public key is computed on the client and sent to the
    --   server for storage.
    -- passlessauthweb:
    --   A check to see if the client supports the PRF extension for WebAuthn
    --   should be undertaken before any passkeys are allowed to be added. If
    --   the client does NOT support the PRF extension it should not be allowed
    --   to add a passkey.
    --
    --   After the user authenticates, we can derive a key using HKDF, using the
    --   salt (derivation_salt) and setting 'info' to the following:
    --     - The saved or given device number
    --     - The chat ID (decoded)
    --     - The user ID (decoded)
    --     - Kolloquy Auth 1.0.0 magic number

    primary key(device_num, user_id, chat_id), -- So many dimensions

    foreign key (user_id) references users(id)
        on delete cascade,

    foreign key (chat_id) references chats(id)
        on delete cascade,

    foreign key (device_num, user_id) references user_devices(devno, id)
        on delete cascade,

    foreign key (chat_id, user_id) references chats_participants(chat_id, user_id)
        on delete cascade
);

-- Stores chat IDs and the IDs of users who have been invited to join them
-- We look up chats_invitees by invitee_id to find the chats the user is invited to
-- We look up chats_invitees by chat_id to find the users invited to the chat
-- Marker table; stores no real data
create table chats_invitees(
    chat_id text not null,
    invitee_id text not null,

    primary key(chat_id, invitee_id),

    -- If the chat is deleted all invitation records need to be deleted too
    foreign key(chat_id) references chats(id)
        on delete cascade,

    -- If the invitee is deleted then there is no one to join the chat and thus
    -- we need to delete the invitation record too
    foreign key(invitee_id) references users(id)
        on delete cascade
) strict;