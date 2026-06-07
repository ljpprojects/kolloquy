# API Pricing

## Measurements

The API uses the following measurements:

`b/mon` (_bytes per month_) for messages received

`b/rcp/mon` (_bytes per recipient per month_) for messages sent

`mb/mon` (_megabytes per month_) for attachments uploaded

`kb/ptp/mon` (_kilobytes per participant per month_) for video and audio data sent during calls.

NOTE: `encyrptions/month` was an unfair measure, as the developer has no control
over how many times a message may be needed to be reencrypted, but they do know
how many users they are sending to.

A month is 30 days.

## Costs

One API token (`tk`) is equivalent to `USD$0.05`.

### Messaging

It costs `0.001 tk/b/mon` (_0.02 API tokens per byte per month_) to receive
messages. This is equivalent to `1 tk/kb/mon`. The number of bytes is the total
number of bytes received in message content (not API results).

It costs `0.005 tk/b/rcp/mon` to send messages. The number of bytes is the total
amount of sent bytes in message content, and the number of recipients is the
median number of recipients each message is sent to. This is equivalent to
`5 tk/kb/rcp/mon`.

### Attachments

It costs `5 tk/mb/50rcp/mon` to upload attachments. The number of megabytes is the
total number of megabytes uploaded in attachments (not API request sizes). The
number of recipients is the median number of users who each attachment is sent
for. This is equivalent to `0.005 tk/kb/50rcp/mon`.

It is free to download attachments.

### Calling

It costs somethign idfk know yet

## Free tier

The free tier provides 80 API tokens for free (`USD$4`). Everything beyond that
is charged.

### Forward Free Tier

Kolloquy provides a _forward free tier_. Essentially, you can increase the free
tier limits for this 30-day preiod by decreasing the free tier limits for the
coming 30-day period.

How the forward free tier works is that you define a _period_ over which the tokens
you added to this period's free tier are taken back. The total is how many
tokens you gain to the free tier this period.

The period has a limit of `6`, and the `total` is limited by the equation
`80 - t/p - 0.05t >= 0`.

Now of course, this isn't quite loan-like enough yet, so there is a small penalty
for using the forward free tier, of `5%/period` (or `p.p.` if you rather lol).

Given a period, the formula to calculate the maximum total allowed is:
`80 / (1/p + 0.05)`,
or, in LaTeX: `\frac{80}{p^{-1}+0.05}`.

Given a period and a minumum desired balance for each subsequent period, the
maximum total can be found with this formula:
`(80 - b) / (1/p + 0.05)`, or, in LaTeX: `-\frac{b-80}{p^{-1}+0.05}`.

Given a total, the absolute minimum period for it can be calculated with this
formula: `t / (80 - 0.05t)`, or, in LaTeX: `\frac{t}{80-0.05t}`.

Given a total and a minumum desired balance for each subsequent period, the
minumum period for it can be calculated using this formula: `t / ( 80 - 0.05t - b)`,
or, in LaTeX: `\frac{t}{80-0.05t-b}`.

For example, if I predicted that I would use 100 API tokens this month, I could
take advantaghe of the _forward free tier_ and take 20 tokens from next preiod's
free tier to this period's free tier. The period for this would be `1` (for 1
period) and the total `20`. This would leave me with `60 tks` (minus penalty) for
the next period.

If 60 API tokens for the next period was not enough, I could increase the period
of the forward free tier to `2`. This would mean I have `70 tks` (minus penalty)
for the next 2cperiods. The formula for the new free tier limit in the coming
periods is simple: `80 - t/p - 0.05t`, where `t` is the `total` and `p` is
the period.

In total, I would have `69 tks` for the next two periods, or `74 tks` for the
next four periods if I increased the periods, or `75.66 tks` for the next six if
I maxed out the period.

For this example, here is a table showing how many periods the deductions last
and how much it is. Personally, I would elect to choose the 2 or 3 periods as
after that it is all diminishing returns.

| Periods | Free tier limit/period | Total penalties |
| ------- | ---------------------- | --------------- |
| `1`     | `59 tks`               | `1 tks`         |
| `2`     | `69 tks`               | `2 tks`         |
| `3`     | `72.33 tks` (floored)  | `3 tks`         |
| `4`     | `74 tks`               | `4 tks`         |
| `5`     | `75 tks`               | `5 tks`         |
| `6`     | `75.66 tks` (floored)  | `6 tks`         |

It is important to note that you *cannot* have concurrent forward free tier...
'benefits'. This means that in the subsequent two months in that example, you
would not be allowed to use the forward free tier functionality.

You can use the forward free tier and your API credits together.

You can apply the forward free tier at any time except the 3 days leading up to
and the day of the charge for usage over the free tier limits. This means if you
have a spike in usage and you can reasonably assume there will not be one in the
next month/s you can use the forward free tier to pay less or none that period.

## API Credits

You can purcahse API credits for a price of `USD$0.04/cr`. If you used 92 API
tokens in a billing period, 12 of them would be paid. If you had 10 API credits,
you would only pay for 2 tokens, but you would lose your API credits. You can
decide how many credits to use on each payment. This would save you `USD$0.10`.

When you first sign up, you will receive 100 free API credits.

## Other discounts

## Examples

Let's say "John" has an API which, over 30 days, sends a total of 50 messages
all of the exact same size (117b) and exactly to one recipient each time.

The message sending would cost `bytes * median(rcps) * months`, or `5850` 'units'.
Each 'unit' would cost `0.005 tks`. In total, this would cost `29.25 tks`. This
usage falls well within the free tier, so John would not be charged.

---

Let's say "Apol" has a disgusting API which sends advertisements to many people.
Let's say each advertisement was the same size (326 bytes), and to one recipient
for each one. Over 30 days, Apol sends 683 advertisements.

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

