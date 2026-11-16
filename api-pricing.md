# API Pricing

**!! This is a draft !!**

## Measurements

The API uses the following measurements:

`prd`, A billing period

`b`, bytes

`rcp`, recipient

`Mb`, megabytes

`msg`, message

`atc`, attachment

The default billing period is 30 days.

## Costs

One API token (`TK`) is equivalent to `AUD$0.025`.

We also use the measurements `mTK` (millitoken, a 1000th of a token), `cTK`
(centitoken, a 100th of a token), `dTK` (decitoken, a 10th of a token), and
`daTK` (decatoken, 10 tokens).

### Messaging

It costs `mTK1/b/msg` (_0.001 API tokens per byte_) to receive messages.
This is equivalent to `TK5/kb`. The number of bytes is the total number of
bytes received in message content (not API results).

It costs `mTK5/b/rcp/msg` to send messages. The number of bytes is the total
amount of sent bytes in message content. This is equivalent to `TK5/kb/rcp/mon`.

For example, receiving 20 128 byte messages costs `TK2.56`. Sending 20 128 byte
messages with 1 recipient each would cost `TK12.8`.

### Attachments

It costs `TK2/Mb/rcp/atc` to upload attachments. The number of megabytes is the
total number of megabytes uploaded in attachments (not API request sizes).

It costs `dTK5/Mb/atc` to download attachments.

For example, downloading 50 attachments of size 512kb would cost `TK12.8`.

## Free tier

The free tier provides 100 API tokens for free (`AUD$2.50`). Everything beyond that
is charged.

## API Credits

You can purchase API credits for a price of `AUD$0.04/cr`. If you used 128 API
tokens in a billing period, 28 of them would be paid. If you had 10 API credits,
you would only pay for 18 tokens, but you would lose your API credits. You can
decide how many credits to use on each payment.

You can also purchase API credits and have them sent to another account (a
"gift", if you will...)

When you first sign up, you will receive 100 free API credits.

## Other discounts

## Examples

Let's say "John" has an API which, over 30 days (which is what he has set his
billing period to), sends a total of 50 messages all of the exact same size
(117b) and exactly to one recipient each time, perhaps for OTP for his website.

John also has his API set up so that people can unsubscribe by sending a
message. You can control chats to reject messages over a certain length if you
have the neccessary permissions, so John has limited the message lengths to 11
bytes, just enough to allow an `UNSUBSCRIBE` message. Let's say 9 people send
exactly that message.

The sent messages would cost `0.005 * 117 * 1 * 50` tokens, totalling `TK29.25`.

The receieved message would cost `0.001 * 11 * 9` tokens, totalling `TK0.099`.

In total, John's API usage for message sending and receiving will cost
`TK29.349`, well within free tier allowances.

---

Let's say "Apol" has a disgusting API which sends advertisements to many people.
Let's say each advertisement was the same size (326 bytes), and to one recipient
for each one. Over 30 days, Apol sends 683 advertisements.

This would cost `0.001 * 326 * 1 * 683` tokens, totalling `TK222.658`. This
does not fall within the free tier allowance, so Apol pays for `TK122.658`. At
billing, tokens are rounded to the nearest token, so Apol owes `TK123`, or
`AUD$4.92`.

Apol's use of the Kolloquy API would also violate the terms of service, so if
the messages were reported for breaching the ToS, after a review Apol's API may
be suspended.

This would total `222658` (`326 * 683 * 1 * 1`) 'units'. This would total
`1113.29 tks`. This is well outside of the free tier limits. Applying the free
tier (`- 80`), Apol must pay for `1033.29 tks`. This would cost `USD$51.6645`.
Kolloquy discourages the use of the platform for advertising, and if enough
users report Apol's API, it will lose the ability to send messages.

---

Let's say "Gary Galah" has an API which sends newsletters to 1207 subscribers.
Let's assume each newsletter is a PDF with an average size of `689kb`. Over 30
days, Gary Galah writes 5 newsletters in individual messages to one recipient
each (thus sending 6035 messages in total).

Let's assume each message's contents is something akin to this:

```
@[attachment: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx]
```

Which would link an attachment. The message's size is `51b`.

Gary Galah uses the attachments feature to upload his 5 newsletters, totalling
`3.445mb` (`689 * 5 kb`), and designated for each of their 1207 subscribers.

This would total `415.8115 tks` (`3.445 * 5 * 1207 / 50 * 1`).

Gary Galah would also need to send 1207 `51b` messages, totalling another `307.785 tks`
(`0.005 * 51 * 1207 * 1 * 1`).

In total, Gary Galah used `723.5965 tks`. Subtracting the free tier allowance,
this would total `643.5965 tks` payable, or `USD$32.179825`.

But wait! Gary Galah has `103 crs`. They decide to use `95 crs` for this payment.
This means that `548.5965 tks` are payable now. Gary Galah will have saved `USD$0.63`
(`95 * 0.05 - 103 * 0.04`), factoring in the cost of all API credits, not just
those used.

Now, Gary Galah only pays `USD$27.429825`.

## The Major Problem

The major problem here is if this API pricing can make up for the costs of
hosting and maintaining Kolloquy (which is at LEAST $5/month due to Workers Paid,
but likely more over time as more data is stored in R2/D1). This assumes people
will actually use the API, and that Kolloquy will have adoption (this likely won't
be the case). Not to mention the costs for just hosting an app on the App Store
(AUD$149/yr). Perhaps the native Kolloquy app will be paid, but then more people
will use the website, and that doesn't reduce the costs much. Perhaps you will
need a subscription????? Maybe I will do that if operating costs become a problem.

Wait, I actually haven't spent a lot of time calculating these operating costs...
Hell I don't even know if Cloudflare Workers will be suitable for this workload,
especially with only like 128MiB of RAM or whatever

I could self-host it but then it would be slow and would not scale at all.
I could buy some EC2 instances or something but the costs will probably too high
as Kolloquy starts out. Also the ethics of Amazon are... questionable.

Self-hosting seems like the most reasonable option so far, as it would be
essentially free in terms of monetary cost (certainly not in the cost of labour
required to rewrite all of Kolloquy that has been written so far away from the
Cloudflare-central design).

Checkign EC2, for a machine with 4 threads and 4+ GiB of memory (equivalent to
the self-hosted setup) the minumum cost I could expect is... USD$69 A MONTH?!?!
WHAT THE FUCK

Maybe 2 instances with 2 vCPUs and 2GiB memory each? $17/month still!?
4 instances with 1 vCPU and 0.5GiB memory each? $8.76/month...

And none of that even counts R2, D1, egress, SES, or anything else at all...

Maybe if I have no other options

How about Cloudflare again... This time with Containers

Even the highest spec container has 0.5 vCPUs???? One thread and I only get 50%
of it?????? Bullshit

Definitely not

Uhhh

Self hosting it... is?
I wish I could use Cloudflare Workers but even if I did I would still be self
hosting the server to compute Argon2 hashes (although it would be less often)

Maybe I will continue the hybrid approach, so host as much as I can on the worker
but anything that needs actually decent specs defer immediately to my self-hosted
machine

But then the problem arises of having to have a little device up 24/7 ready to
hash (or return the cached hash, I did already design that whole server afterall)
of a password

Maybe I can just encourage passless authentication as it is A) more secure and
B) doesnt rely on my dingy little self hosted server

But 10ms of compute time will not be enough so $5/month is mandatory

Also again this still has labour costs I would have to basically reconfigure a
device from scratch specifically to be a secure server (last time I did something
like this it was to install Arch Linux and that went horribly, I had to reinstall
mutliple times and the working attempt took 3 hours)

R2 probably won't be an issue...
D1 probably won't be a problem either
SES probably won't be
Workers probably wont be either

So seems like the platforms I have been building for since last year are suitable

(I estimate atworst like $7/month and even that is probably too high)

