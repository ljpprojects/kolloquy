# Architecture

Your code to verify your Kolloquy account's email is "123456".
Do not share it with anyone.

It will expire in 15 minutes.

## Authentication

### Creating an Account

### With an Existing Account

Clients `POST` to `/auth/register` from `/signup`.

They send the following data:

- The email for the new account
- The display name for the new account
- The plaintext password for the new account

This **must** be done over a HTTPS connection. The server should not proceed if
the connection is insecure.

Example body:

```json
{
  "email": "john.doe@example.com",
  "display_name": "john.doe",
  "password": "supersecretpassword123"
}
```

The server will then:

1. Hash the plaintext password using SHA-1 without a salt
2. With the first 5 characters of the hash, use the AmIPwned API (`https://api.pwnedpasswords.com/range/...`) to check if the password has been in a data breach.
3. If it has, return code `422` with an error message.
4. Otherwise, Generate a random 256-bit salt.
5. Hash the password with the salt (appended), pepper (prepended), and secret (from Cloudflare Secrets Store, appended) using SHA256 (this is stage 1)
6. Send the stage 1 digest (base64 encoded) to the offload server (`phash.kolloquy.com`) with the body being JSON with the field `stage_1_digest`.
7. The offload server will compute the Argon2id hash of the stage 1 digest with secure parameters (this cannot be done on the worker) and its own pepper and secret. The server sends back the base64-encoded hash (as plain text, not JSON). This is the hash which is to be put into the DB.
8. Send an email from `no-reply@kolloquy.com` with a 6-digit code
9. Generate a random 128 bit single-use token and store it in the `kSESSIONS` KV under `verify:...` (ttl = 15 minutes) and the data being the email, display name, hashed password, and the code we sent
10. Set the `verify` cookie (secure, samesite strict, http only) to the new token with a max age of 15 minutes.
11. Return `201` (CREATED). The frontend should then redirect to `/verify`

At `/verify`, the user will enter the code from their email, and the client will make a request to `/auth/verify`.

Calls to `/auth/verify` should have:

- The 6-digit code that the user entered as a string

Example body:

```
{
  "code": "123456",
}
```

The server will then:

1. Read the `verify` cookie (or return code 401)
2. Read the data at `verify:{verify cookie here}` to obtain the new user's email, display name, password hash, and the code we sent
3. Ensure the `entered_code` matches the code we sent (if it doesn't return 422 and error body)
4. Add the user to the `kolloquy` DB under the `users` table (see schema in src)
5. Set the session cookie to a base64-encoded 64 bit random integer with a maximum age of 30 minutes.
6. Set the refresh token to a base64-encoded 128 bit random integer with a maximum age of 45 days.
7. Store the session in the `kSESSIONS` KV under `session:...` (ttl = 30 minutes) and the data being the user's email and display name.

### With an Existing Account

Clients `POST` to `/auth/login` from `/login`.

They send the following data:

- The email of their account
- The plaintext password

This **must** be done over a HTTPS connection. The server should not proceed if
the connection is insecure.

Example body:

```json
{
  "email": "john.doe@example.com",
  "password": "supersecretpassword123"
}
```

The server will then:

1. Retrieve the salt and hash for the user with the specified email
2. Hash the password with the salt (appended), pepper (prepended), and secret (from Cloudflare Secrets Store, appended) using SHA256 (this is stage 1)
3. Send the stage 1 digest (base64 encoded) to the offload server (`phash.kolloquy.com`) with the body being JSON with the field `stage_1_digest`.
4. The offload server will compute the Argon2id hash of the stage 1 digest with secure parameters (this cannot be done on the worker) and its own pepper and secret. The server sends back the base64-encoded hash (as plain text, not JSON).
5. On the worker again, compare the retrieved hash with the one returned from the offload server, returning `401` if they are inequal.
6. Set the session cookie to a base64-encoded 64 bit random integer with a maximum age of 30 minutes.
7. Set the refresh token to the base64-encoded result of prepending a random 64 bit integer to the user's identifier, with a maximum age of 45 days.
8. Store the session in the `kSESSIONS` KV under `session:...` (ttl = 30 minutes) and the data being the user's email and display name.
