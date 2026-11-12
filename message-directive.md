# Message Directives

## Attachments

**Syntax**

```
@[attachment: {UUID}]
```

Where `{UUID}` is the UUID (v4) of the attachemnt, taking the format `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.

---

Links an attachment into the message.

If the attachment does not exist the message will be rejected.

If the attachment does not have all of the message's recipients allowed to view
the attachment, the message will fail to send.

## Links

```
@[link: {URI}]
```

Where `{URI}` is the URI of the site to link to.

---

Links a website into the message.

WHen the message is opened, the user will either see a button to go to the link
or an indicator that the link was hidden (if they have disabled links.)

## Mention

**Syntaxes**

```
@[mention: @{DISPLAY NAME}]
```

Where `{DISPLAY NAME}` is the display name of the mentioned user,

```
@[mention: #{ID}]
```

Where `{ID}` is the ID of the mentioned user,

```
@[mention: ~{EMAIL}]
```

Where `{EMAIL}` is the email of the mentioned user, or

```
@[mention: %{PHONE HASH}]
```

Where `{PHONE HASH}` is the SHA-256 hash of the user's phone number (E.164-formatted).

## Polls

**Syntax**

```
@[poll: {UUID}]
```

Where `{UUID}` is the UUID (v4) of the poll.