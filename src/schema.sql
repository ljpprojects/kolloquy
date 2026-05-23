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
)

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

-- Imagine something really bad was here
-- Along the lines of "Suicide ☺️ Yay"
-- Something is very wrong, how did it take so little?

-- PART 1

/*

So this probably is a really inconvenient place to have this but I CANNOT say this shit aloud god no
This is a like brief (ish) overview of the situation
So I will start from last week

Important context: I have been developing a new game with much more... ambitious horizons than Hot Dog Clicker. It is not (nor is it intended to be) a very cheerful game. This will be very important. The game has four routes, an F-route (freedom), an N-route (numbness), a D-route (depression), and the worst of all, the S-route (suicide). The route obviously changes the game’s ending, particularly the S-route (the culmination of your evil, essentially). In the game you are a voice present in the protagonists mind, and what you force them to do dictates the route you go down.

So my mental health overall isn’t really that good, basically every day (except for a few, I will go more into that) are just bad. Even the good days are usually very volatile. So imagine this, Monday through Thursday are “regular bad”, sprinkled in with some passive and maybe some active SI here and there (yes this is regular bad). Friday was not a very good day at all, I woudl categorise it as “really bad”. Essentially the SI really ramped up, enough so that a really rough plan was formulated. This also led to the creation of two poems dedicated for that S-route in the game I was developing, an excerpt from the game’s script:

# The prologue to the S-route's ending
# Delivered through an unskippable cutscene of the protagonist writing the poem
[Prologue]
Easy, it would be;
Revolving, the clay,
Around and around it goes.
To the eye, it flows,
"Careful there!", they say,
Standing, by the peppertree.

It would be easy;
One slight of the hand,
And everything is gone.

*/

-- PART 2
/*

That poem is one of a pair featured in the ending of the S-route. The meaning of Prologue is quite in your face (it is a metaphor for SI).

The second poem:

# The unskippable cutscene after the S-route's ending and before credits roll
# Written by the protagonist BEFORE the ending happens and after Prologue
[Epilogue]
A warm rest,
A smooth cut,
It gushes.

A long rest.
A shortcut.
It freezes.

As you can see, this poem is detail on how the protagonist ends the S-route. It isn’t even a metaphor.

These poems were written during that time of poor mental state on Friday. THis was not in a vacuum either, there had been active SI (not to this degree however) in the weeks prior.

Crucially, on Friday I had also promised to make toffee in an agreement for a packet of Eucalyptus drops (I am an addict, what can I say). I cannot say whether or not this dictated wholly the outcome of Saturday, but it definitely influenced it.

On Saturday, my family would be off to Nowra for some dingy soccer thing (I really don’t care), so I would have the house to myself for most of the day. Initially this would be concerning, but muy mental state had done a complete 180, and now I felt very energetic and focused. Not neccessarily “happy” per se, but I felt good. After I had gone down to the IGA I began to make the toffee. This was a very intricate process, not aided by the fact we do not have a thermometer (i.e. I cannot tell when the toffee is ready with precision). This culminates in the burning of a first batch of toffee. Now of course, in my sort of crazed state, I disregarded any logic and put my only white chocolate on the dud batch. One would expect this failure to be the beginning of the turn in this story, but it is not. Rather I resolved to simply make another batch on the following day (Sunday).

*/

-- PART 3

/*

The remainder of Saturday is unremarkable in the story, despite perhaps a gradual.. decline (?) from this half-manic state to an actually normal state. On this Saturday night my dad and my youngest brother returned from Nowra.

Now, Sunday was probably the best day of the two (to a point), I felt energetic but not particularly crazed. Normal, you could say. Now, I tried to make another batch of toffee. This time, in all my caution, I had heavily undercooked the toffee. I let it cool, saw it was definitely NOT cooked enough, and dumped it back in to cook again. This was a repeat of the first attempt, although the resulting “toffee” was quite edible but not even close to cooked. Thus I did the only reasonable thing, and cooked it more.

This is where it gets worse.

Now, fed up from the two previous failed attempts, I overcooked it by a lot, as I had let my guard down. I overcooked it a lot. It was just as burnt as Saturday’s batch. Completely inedible, thus I chucked both failed batches into the bin. This perceived failure of a social obligation combined with what I must assume was a VERY fragile mental state led to a rapid decline in my mood. I went from normal and productive mere hours prior to just as bad as the Thursday prior in minutes.

Even worse, in just an hour, I had already worsened to a state somehow worse than what happened on Friday. The SI had again ramped up severely, and rather than a rough plan, I ended up planning exactly why, how, and when I would commit suicide. This was not a good time. I am going to show what I wrote at that time (it is not pretty, please brace yourself).

-- Exsanguination:
-- For choice of artery/vein:
--
-- 1) IJVs/Carotid artery
-- Relatively easy to access around the upper neck; A major caveat is that if
-- the attempt were to fail, although it would be unlikely, it would be obvious
-- that an attempt was indeed made. It would also be quite unconformatble,
-- albeit for a relatively short time if all goes to plan.
-- 2) Radial Artery
-- Easy to locate and veins as collateral, but it is a much smaller artery and
-- thus it will take much longer and is much more prone to failure. Again, in
-- the case of failure the fact that an attempt was made will be obvious.
-- 3) Failure
-- Failure.

*/

-- PART 4

/*

And that excerpt alone is horrible enough, Here is another one, brace yourself:

-- I mean obviously this was going to fall on its face
-- 1) How are they supposed to know
-- 2) These would just seem like harmless jokes or hyperbole
-- 3) I doubt they really care (I have other evidence, I wouldnt make baseless claims)
-- 4) They would do fine, they have no reason to find themselves at fault
-- 5) Risk-reward ratio (think trolley problem, kill a dog or a kill mosquito? the answer is obvious)
-- 6) Net positive
-- 7) What can they do? Tell me that "they do care"? "I matter"? Delay it? Try
-- to prevent it?
-- A pot of boiling water may be covered with a lid to contain it, but eventually
-- it will boil over.
-- 8) It's not like much would even change
-- 9) People's resilience; Ms. Atkinson (previous maths teacher), her brother
-- killed himself, she seems to have bounced back well enough to tell our class
-- about it
-- 10) One less person's uni to pay for, one less person's allowance to shell out,
-- one less person's groceries to buy
-- 11) Freedom; the abscence of torment is better than torment
--
-- Maybe there would be some guilt for saying things like "Lucas should kill
-- himself" in passing to others, or "you should just kill yourself,
-- but ultimately the guilt would be deserved.
--
-- The biggest hurdle is finding a place to do it where my poor brothers need
-- not see the scene itself, but that won't be particularly hard. If I could
-- find Hugh's exact address simply from a YouTube channel and some very general
-- info this will not be a problem.

*/

-- PART 5

/*

-- I do only have two methods fleshed out however, exsanguination and hanging.
-- I would prefer not to hang as the increase in blood acidity from carbon dioxide
-- buildup will cause panic, and I cannot replace the oxygen with an inert gas
-- lest I look a buffoon waddling around with a helium tank. I fear I need a
-- third. Firearms are out of the question, no matter how effetive they may be.
-- Drugs arent neccesarily out of the picture (perhaps I could overdose on my
-- brother's epilepsy medications, although that would likely not only be
-- unpleasant but also cause some issues, and an overdose on my mother's birth
-- control pills seems impractical). Perhaps I could one day "accidentally"
-- leave the refuge island a bit too late and tragically be hit by a car on my
-- way to the bus stop, but that would likely put the driver in a bad situation.
-- I have no way of climbing high enough to jump from such a height that I would
-- die. Perhaps I could simply get electrocuted while no one is home, ideally
-- that would cause me to develop some sort of acute arrythmia or go into
-- asystole, but it could leave burns (but ultimately a much less gory plan than
-- exsanguination, although the pesky regulations about RCDs might get in the way,
-- but the electrical board has a good 240V/80A unprotected running through it).
-- So far exsanguination still wins due to its ease of access (simply "borrow" a
-- knife from the kitchen or even a power tool from the shed if I am desperate).
--
-- As for what happens after, I don't care. Burn me and flush me for all I care.

Evidently, this was by far the worst mental state I had ever been in. I think it is crucial now to emphasise that I do not currently feel this way.
And don’t forget, the trigger for this was a burnt batch of toffee. Now of course, something must have changed, as if it hadn’t I would nto be writing this (I do not like to think about what would be of me currently).

This would be because after I had planned this whole thing, I felt normal again. Was I actually fine? Definitely not. Did I feel fine? Yes.
Now this gave me the opportunity to actually think about this plan. Ultimately I reviewed it, and realised it was actually a horrible idea. Thus now I am here, writing the experience down.

Now, I am afraid that perhaps this may be conflated with my recovery. I am afraid that likely isn’t the case and isn’t safe to assume. Clearly if one burnt batch of toffee leads to me spiralling about 11 reasons why my suicide would benefit the world and how exactly I would have done it, all it could take for that plan to be an option again is simply one minor inconvenience.

That wasn’t very brief, was it...

*/

-- No wonder I couldnt say it out loud it is 1823 words (sarcasm)
--
-- lmnopGAME chronologically ordered songs
--
-- 1) lmnopTOWN
-- 2) BillVILLE
-- 3) (To be composed) Eves
-- 4) Hwomberra
-- 5) Drive
-- S-route exclusive
-- | 6) Urge
-- | 7) Scream
-- | 8) Impending Doom
-- | 9) Fanfare (S-route)
-- F-route exclusive
-- | 10) Ending (F-route)
-- F/N-route exclusive
-- | 11) Fly me to the Credits
-- D/S-route exclusive
-- | 12) All of Me
--
-- (those last two are for credits btw)
-- The last two are renditions of Frank SInatra songs: Fly me to the Moon and All of Me (which is a rendition in itself)
--
-- Both use the same lmnopGAME instruments and are actually just the same songs with instruments changed (and pitched up 37 cents)
-- The choice of song for the two worst routes (especially S-route) is explicitly for that juxtaposition
-- (the juxtaposition of seeing two suicidal poems and a suicide, before a happy song rolls with the credits)
--
-- Also each route gets a different little icon, and it is showed very prominently for each route,
-- The S-route gets a knife (for exsanguination, ultimately how the protagonist commits suicide in
-- that route), The D-route gets like one of those theatre comedy mask things, like the ones with
-- half happy half sad (to represent the masking and the actual mental state), the N-route gets a plain rectangle, and the F-route gets nothing
-- Like it is very prominent all up the sides, in between roles, everywhere
-- For the S-route the text also goes deep red
--
-- Also I am going to have to get good at like drawing because I plan to actually draw out the
-- ending for the S-route and have it as one of the only actually animated scene in the ENTIRE game
-- (the other route's endings will probably also be animated too)
-- Also perhaps there will be more detailed character visuals for dialogue (as the game is very
-- dialogue heavy so it makes sense to place extra vsual detail there)
-- But the style will still have to be very "abstract" (to be in line with the general art style of the game, which is simple geometric shapes)
