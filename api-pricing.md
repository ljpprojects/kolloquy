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