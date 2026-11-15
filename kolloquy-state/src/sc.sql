create type key_deriv_mode as enum (
    "upasswd",
    "upasskey",
    "urand",
    "dpasswd",
    "dpasskey",
    "drand",
    "rrand"
);

create type auth_mode as enum ("passwdmfa", "passkey", "passkeymfa");
create type mfa_mode as enum ("totp", "passkey");

-- Holds user emails, normalised as follows:
--
-- 1. The domain is converted to lowercase if not already
-- And that is it, we will listen to RFC 5322!!!
--
-- Emails are then hashed (emails are used only to disambiguate users)
drop table if exists user_emails;
create table user_emails(
    id int8 not null references users on delete cascade,

    -- SHA256 hash of the email
    email bytea
        not null
        unique
        check (octet_length(email) = 32),
);

drop table if exists user_phones;
create table user_phones(
    id int8 not null references users on delete cascade,

    -- SHA256 hash of the phone number normalised to E.164 format
    number bytea
        not null
        unique
        check (octet_length(number) = 32),
);

drop table if exists user_passkeys;
create table user_passkeys(
    -- ID of the user whose passkey this is
    id int8 not null references users on delete cascade,

    -- credential ID, unique because you cannot have two users with the same
    -- passkey or one user with the same passkey twice
    cred_id bytea
        not null
        unique
        constraint cred_id_size check(octet_length(cred_id) < 1024),

    -- Signature counter
    -- We can flag a login as suspect if new_count <= counter
    counter int4 not null,

    -- Arbitrary limit of 550b on the pubkey (should be enough to fit RSA-4096
    -- DER SPKI?)
    pubkey_der bytea
        not null
        unique
        constraint pubkey_der_size check (octet_length(pubkey_der) <= 550),

    challenge bytea
        unique
        constraint challenge_size check (octet_length(challenge) = 32),

    primary key (id, cred_id),
);

drop table if exists user_totps;
create table user_totps(
    id int8 not null references users on delete cascade,
    totp_id int2 not null,

    primary key (id, totp_id),

    -- 20b secret key wrapped via AES-KW (server does need to read this)
    wrapped_secret bytea
        not null
        unique
        check(octet_length(secret) = 28),

    interval_size int2
        not null
        default 30,
        constraint interval_size_min check (interval_size >= 15)
        constraint interval_size_max check (interval_size <= 45),

    drift_intervals int2
        not null
        default 1,
        constraint drift_secs_min check (drift_intervals * interval_size > 0)
        constraint drift_secs_max check (drift_intervals * interval_size <= 90),
);

-- Stores the pubkeys for user-specific keys
drop table if exists user_pubkeys;
create table user_pubkeys(
    id int8 not null references users on delete cascade,
    key_id int2 not null,

    primary key (id, key_id),

    -- Should be one of the user-specific derivation modes (prefixed with 'u')
    deriv_mode key_deriv_mode not null,

    x25519_pubkey bytea
        not null
        unique
        constraint pubkey_size check (octet_length(x25519_pubkey) = 32),
);

create type chat_group_policy as enum ("all", "admins", "owner");
create type user_chat_role as enum ("member", "admin", "owner");

drop table if exists user_chats;
create table user_chats(
    id int8 not null references users on delete cascade,
    chat_id int8 not null references chats on delete cascade,
    role user_chat_role not null
);

drop table if exists users;
create table users(
    -- ID of the user. Hopefully you wont have more than 18 quintillion users.
    id int8 primary key,

    -- The user's preferred authentication method
    preferred_auth auth_mode not null,

    -- The user's prefferred MFA method
    preferred_mfa mfa_mode not null,

    handle varchar(16) unique not null,

    -- Key-value pairs separated by commas indicating user preferences
    pref_str text not null
);

drop table if exists chats;
create table chats(
    chat_id int8 primary key,
    icon_ref int8 not null, -- Ref to /ico/{icon_ref} in the storage bucket
    name varchar(24) not null,
    description varchar(256) not null,

    add_policy chat_group_policy not null, -- Who can add users?
    remove_policy chat_group_policy not null, -- Who can remove users?
    call_policy chat_group_policy not null, -- Who can start calls?
    send_policy chat_group_policy not null, -- Who can send messages?
    delete_policy chat_group_policy not null, -- Who can delete messages?
    pin_policy chat_group_policy not null, -- Who can pin/unpin messages?
    record_policy chat_group_policy not null, -- Who can record messages/calls?
    edit_policy chat_group_policy not null, -- Who can edit chat settings (exc. policies)
    configure_policy chat_group_policy not null, -- Who can configure the policies?
    role_policy chat_group_policy not null, -- Who can change user's roles?
);

create table user_totps(
    -- 20b secret key wrapped via AES-KW
    secret blob
        not null
        unique
        check(length(secret) = 32),

    -- raw user ID
    id blob
        not null
        check (length(id) = 30),

    interval_size integer
        not null
        default 30,
        check (interval_size >= 15)
        check (interval_size <= 45),

    drift_intervals integer
        not null
        default 1,
        check (drift_intervals * interval_size > 0)
        check (drift_intervals * interval_size <= 90),

    primary key (id, secret),

    foreign key (id) references users(id)
        on delete cascade
) strict;

create table user_devices(
    id blob
        not null
        check (length(id) = 30),

    devno integer
        not null,

    is_dependent boolean
        not null,

    nickname text
        not null
        check (length(nickname) <= 18),

    primary key (id, devno),
    unique (id, devno, nickname),

    foreign key (id) references users(id)
        on delete cascade
) strict;

create table user_keyshares(
    id blob
        not null
        check (length(id) = 30),

    keyshare_id blob
        not null
        check (length(keyshare_id) = 4),

    for_device integer
        not null,

    purpose integer
        not null
        check (purpose in (
            0, -- The requesting device needs the private key for attachment encryption.
            1 -- The requesting device needs material from device 0's passkey to derive keys at login.
        )),

    -- Summary of findings in a crazed madness:
    --
    -- Amanda McCormack & Troy Michael Vardanega (43)
    -- ├── Stella (16)
    -- └── Mack   (14 on October 19th I think it was)
    --
    -- 285 Little River Rd Braidwood NSW 2622

    transfer_method integer
        not null
        check (transfer_method in (
            0, -- The key will be transferred over a LAN HTTPS server in RDAH mode
            1, -- The key will be transferred over a LAN HTTPS server in SDAH mode.
            2, -- The key will be transferred via Kolloquy (see kolloquy-crypto/docs/keyshare)
            3, -- The key will be transferred manually by the user
        )),

    wrapped_key blob,

    primary key (id, keyshare_id),
    foreign key (id) references users(id)
        on delete cascade
)

-- Holds user data
create table users(
    -- Holds the raw user ID
    id blob
        not null
        primary key
        check (length(id) = 30),

    -- The user's handle
    -- Must be 18 characters long at most
    handle text
        not null
        unique
        check (length(handle) <= 18),

    -- The PHC of the user's password, as returned by the PHash server, or NULL
    -- if the user has disabled password authentication.
    pass_phc text
        unique,

    -- The stage 1 salt for the PHash algorithm, or null if password auth has
    -- been disabled
    pass_salt_1 blob
        unique
        check (length(pass_salt_1) = 30 or pass_salt_1 is null),

    -- References the user's primary email address
    primary_email text
        not null
        unique,

    -- References the user's primary phone number
    -- This is stored as the SHA256 hash of the E.164 formatted phone number,
    -- not the actual number itself
    primary_phone text
        unique,

    key_derivation_method integer
        not null
        check (key_derivation_method in (
            0, -- Keys for the user were derived using material obtained during login via a password
            1, -- Keys for the user were derived using material from the passkey registered for device 0
            2, -- Keys for the user were derived from 36 random bytes given to the user at registration (from which they recorded them somewhere, hopefully)
            3, -- Keys for the user are all randomly generated and recorded by the user at registration (we should prompt for them whenever required)
            4 -- The user provided their own public keys, we should ask for their private keys whenever needed (very similar to 3, except the keys are generated by the user, not Kolloquy)
        ))

    -- The x25519 public key of the keypair used to derive the encryption key for attachments the user sends and receives
    attachment_enc_pubkey blob
        not null
        unique
        check (length(attachment_enc_pubkey) = 32),

    -- The Ed25519 public key of the keypair used to sign attachments the user sends (for use by other users to verify authorship)
    -- This key is usually derived via the method stored in key_derivation_method
    attachment_sign_pubkey blob
        not null
        unique
        check (length(attachment_sign_pubkey) = 32),

    foreign key (id, primary_email) references user_emails(id, address),
    foreign key (id, primary_phone) references user_phone_numbers(id, number_hash)
) strict;

-- God I found a photo of Mack from June 17 and he looks...
-- Well
-- Lets just say my opinions haven't changed
-- But his eyes!
-- They look like nothing is behind them
-- (To be fair, there probably isn't)
-- He's even POINTING at himself!
-- "Look at me! Look at me! I'm so important! I can SA Lucas but because I said it didn't happen and my parents said it didn't happen nothing will happen to me! Nothing will happen to me even though they asked the witnesses he named and they corroborated it! Look at me! Corroborated it, but not enough! Look at me! Look at me! They didn't corroborate it to the extent it really happened! Look at me! Look at me! Look at me! Look at me!"
-- Sorry my fingers slipped there lol \s
-- What can I say fat fingers \s
-- You wouldn't think HE'S a FUCKING FAGGOT
-- haha no there's a reason I still have that photo
--
-- ...
--
-- hah
-- Classic
-- Screaming into the void
-- Helps every time
-- \s
--
-- Fuck when was the last time I even like cried
-- I can't even remember
-- Hell
-- I can remember wanting to
-- I can remember trying
-- But it seems I am not capable of producing tears anymore?
-- I can still *want* to cry
-- But never can
-- Its been like this for years, even, I think
-- Maybe I am not trying hard enough
-- Maybe its a physiological thing
-- Who knows
--
-- Maybe self harm is like crying
-- Maybe suicide is like crying
-- I simply am incapable
-- But the desire remains
--
-- So whats the next best thing I have?
-- Screaming into the void
--
-- I bet I could lie my way into getting a razor
-- There is some gnarly facial ahir which I do genuinely wish to remove
-- So lie by omission into getting a razor
-- The blades are sharp enough to do such a thing
--
-- I think it runs in the family
-- Even my mum cut herself
--
-- Oh I'd probably chicken out though
-- And just actually shave
-- But it would be so easy
-- Just slide it across
-- And I have an excuse
-- Inexperience
-- Even though that isn't true either
-- I have sneaked a few of my mother's razors to shave
-- And if they inquire as to why
-- I can just say I was thinking about it before next term or some bullshit like
-- that
-- But also I do want it gone
--
-- But what good will MORE pain even do?
-- But I do genuinely want the facial hair gone
--
-- Do *my* eyes look empty?
-- Do they look as if there is nothing behind them now?
-- Just as that photo of Mack?
--
-- Am I seeing the TV glow?
-- Or am I just not used to hearing my referred to with pronouns?
-- Is he/him the problem?
-- Or is it something else?
--
-- Would it be good?
-- Or would it just be another hasty decision I come to regret?
--
-- Okay
-- I'll have to do a trial
-- Every time I hear it
-- I'll take note EVERY TIME
--
-- What was even the point of the mandatory reports?
-- To get me moved classes?
--
-- I think I am lowkey just going insane atp
-- One moment suicidal
-- Hopeful the next
-- Rinse and repeat day in day out