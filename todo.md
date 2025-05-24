# Todo

## Features
* Password Reset
* Email verification
* API

## Security Fixes

***

### SF000—Input Verification Amendment I
Verify all inputs on the frontend and backend.
This means additionally verifying handles when inputted for
chat creation, and the age input on account registration.

**Status: 🕒**

**Priority: ⭐️⭐️⭐️⭐️⭐️** (out of 5 stars)

***

### SF001—SSID Amendment I
* Improve SSID issuance in the backend
* Decrease the size of SSIDs from 64 bytes to 22 bytes (before base-64 encoding)
* Use a customised base-64 encoding where forward slashes become dots

SSIDs should live for exactly 1 800 seconds, or 30 minutes.

A server issuing an SSID should respond with this header:

```
Set-Cookie: __Host-SSID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; Max-Age=1800; Secure; HttpOnly; Path=/; SameSite=Strict
```

SSIDs also must be verified every time they are used. For example, a server issuing an SSID would perform these actions:
1. Generate SSID
2. Generate a SHA-2 checksum
3. Issue the SSID to the client using the header format above
4. Whenever the SSID is used, check for any invalidation conditions (checksum non-matching, expired)
5. Handle the request appropriately

**Status: 🧪**

**Priority: ⭐️⭐️⭐️⭐️⭐️** (out of 5 stars)

***

### SF002—Protocol & CORS Upgrade I
While the separation of the server is underway
(as defined by [BI002—Server Separation](#bi002server-separation)), migrate to
HTTPS, and update CORS to only allow Cloudflare origins.

**Status: ❌**

**Priority: ⭐️⭐️⭐️⭐️⭐️️** (out of 5 stars)

***

## Backend Infrastructure

***

### BI000—Network Efficiency Amendment I
Update the backend to send read-only links to avatars,
as part of the API, instead of sending the avatar directly,
via https://api.kolloquy.com/user/@xxx.

Requires completion of [BI002](#bi002server-separation).

**Status: ❌**

**Priority: ⭐️⭐️** (out of 5 stars)

***

### BI001—Authentication Amendment I
Update the backend to send STRTs
(Short-Term Refresh Tokens) when authenticating, alongside SSIDs.

STRTs are one-use-only tokens sent alongside the SSID
to generate a new SSID when the current one expires.
They are sent alongside the SSID with every request,
automatically renewing the SSID and rotating after every use.

STRTs should live for exactly 1 209 600 seconds,
or about 2 weeks.

A server issuing an STRT should respond with this header:

```
Set-Cookie: __Host-STRT=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; Max-Age=1209600; Secure; HttpOnly; Path=/; SameSite=Strict
```

**Status: ❌**

**Priority: ⭐⭐️** (out of 5 stars)

***

### BI002—Server Separation
Update the backend to split it into three separate parts, as defined below.

The first part is the router, and its purpose is to route
data ingress and egress to the correct destination.

The second part is the main server, which will server client-facing web pages.

The third part is the API server, which will handle all
backend APIs, including authentication, API calls, and data fetching.

The router will be the only publicly exposed server,
running on port 443.
The main server and API server should reject any 
ingress not from the router.
The main server should run on port 8443,
and the API server should run on port 7443.

All running servers, even development ones, should use HTTPS.

**Status: ❌**

**Priority: ⭐️** (out of 5 stars)

***

### BI003—Client Websocket Independence Amendment I

When connecting to a Kolloquy Chat, ensure that the user
is never on another user's Websocket connection. This means:

* separating each user's Websocket connections
* storing every user's Websocket connection
* deleting a user's Websocket connection
* implementing thread-safety for ingress and egress via these connections

**Status: ❌**

**Priority: ⭐️⭐️⭐️⭐️** (out of 5 stars)

***