drop table if exists users;

create table users(
    id text primary key,
    email text unique,
    display_name text
        unique
        not null
        check (length(display_name) <= 18), -- Length should be 18 at most
    password_salt text
        not null
        check (length(password_salt) = 44), -- Length should always be 44
    password_hash text
        not null
        check (length(password_hash) = 44),

    -- This will always be 1 in practice because you are forced to verify your
    -- email to create an account and have it put to the DB
    email_verified integer,

    -- A comma-separated list of chat ids which the user is a participant of.
    -- If a user has a chat present in this list which does not name them as a participant, it should be removed from the list.
    participations text not null,

    -- A comma-separated list of chat ids that the user has been invited into
    pending_entrances text not null,

    creation_timestamp integer -- The UNIX timestamp (in UTC) of the time the account was put to the DB
) strict;

drop table if exists chats;
create table chats(
    id text primary key,

    -- Stores the user ids of participans as a comma separated string (i.e. users who have explcitly accepted an invitation to join)
    participants text not null,

    name text
        not null
        check (length(name) <= 50),

    secret text unique not null, -- The secret used to encrypt messages sent by participants in the chat in PEM format
    creation_timestamp integer -- The UNIX timestamp (in UTC) of the time the chat was created
) strict;
