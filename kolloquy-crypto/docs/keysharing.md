FUCK: redesign around ephemeral key design

# Keysharing

## Via a LAN Server (RDAH mode)

This details the EXACT algorithm to follow when performing a keyshare via a LAN
server in Requesting Device as Host (RDAH) mode.

When the requesting device (RD) is a client which can start a web server (like
the desktop or mobile apps), this is the preferred method.

---

The RD will select a random port between `15579` and `56568` to host the server on.

Now, the RD should obtain the decryption key by deriving an AES-KW key (the
wrapping key, WK) from the SD's public X25519 key and the RD's private X25519 key.

The RD should compute the WK's SHA-256 hash. From the 32 hash bytes, the last 4
bits are the initial offset (O). The 4 bytes beginning at the Oth byte are the
big endian representation of a signed number. Remove the sign from this number
(`N & 0x7FFFFFFF`/`N & i32::MAX`), and modulo it by 10^6. Save this code (C).

The last 4 bytes of the hash, base-16 encoded, is the ID for the keyshare.

The RD will then `POST` to `ks.kolloquy.com/rdah` with the user's `SSID` cookie
set. The `callback` query parameter should be set to the URI's authority (e.g.
`192.168.1.55:22222`). The `id` query parameter should be set to the ID we
computed just before (e.g. `9822b014`).

THe body must match the schema in `schemas/ks.kolloquy.com/rdah.req.json`.
The server's response will match the schema in `schemas/ks.kolloquy.com/rdah.res.json`.

If the response indicates an error, the RD should show the error and stop.

On a successful response, the RD will start a HTTPS server on the selected port.
The server should only serve requests on `/ks`.

Convert C to a string and left pad it to be 6 characters long. This will be the
code string (CS).

Then, the RD should instruct the user to go to the keyshare menu on the source
device (a.k.a. SD, a.k.a. device 0, referenced by nickname) and open the keyshare
with the ID we generated earlier.

---

Once the menu is opened on the SD, a display of all current keyshares will be
visible. The user should select the keyshare with the ID we displayed to them
earlier. It should `GET` a list of the user's keysharing requests from
`ks.kolloquy.com/list`. It should `GET` an individual request from
`ks.kolloquy.com/request`, with the `id` query parameter set to the ID of the
keysharing request.

The SD now has to get the encryption key by deriving the same WK the RD has from
the RD's public X25519 key and the SD's private X25519 key.

With the WK, it should follow the same process the RD did to arrive at the CS.
The SD should ask the user to enter the code in. If the codes do not match in 2
attempts, the keyshare should be aborted.

The SD should then use the process specified for the keysharing request to derive
the required key (K).

Now that K has been derived, it can wrap K using AES-KW, to get the result (RK).

The SD should then call `ks.kolloquy.com/fulfil`, with the `id` query parameter
set to the ID of the keysharing request. This will delete the keysharing entry
from Kolloquy's backend.

Then, the SD should `POST` to `/ks` on the keysharing request's callback, with
the `Content-Type` being `application/octet-stream`, and the body being the raw
bytes of Rouxls Kaard.

The SD should then redirect the user back to the RD.

---

Once the RD recieves the octet stream from the SD, it should unwrap the key
using the WK it independently derived, save it for storage, and immediately take
the HTTPS server down.

### Certificates & mTLS

The RD should randomly generate a 30 byte salt (S).

The private SSL key should be a randomly-generated ED25519 key using the P-256
curve.

The SSL certificate should be generated from this random SSL key, valid not
before now and not after 1 hour from now. The signature algorithm should be
`ecdsa-with-SHA256`. The common name should be the IP of the RD, the organisation
should be `Kolloquy`, the unit should be `Keysharing`, and the locality,
state/province, and country can be left blank.

The mTLS private key should be an ED25519 key derived from the RD's private
X25519 key, the SD's public X25519 key, and the salt chosen earlier.

The mTLS certificate should have the same parameters as the SSL certificate,
but generated using the derived mTLS key.

The SD should derive the same mTLS private key from it's private X25519 key, the
RD's public X25519 key, and the salt. It should also derive the same certificate.

The SD should use both for the callback, or the request should be rejected.

### Aborting the Keyshare

If the RD wants to abort the keyshare, it should take down the server (if it is
up) and `DELETE` to `ks.kolloquy.com/request`, with the `id` query parameter set
to the ID of the keysharing request to delete.

---

If the SD wants to abort the keyshare, it should send a `DELETE` to
`ks.kolloquy.com/request`, with the `id` query parameter set to the ID of the
keysharing request to delete. It should then send a `DELETE` to the callback
server at `/abort` (which should just take down the server and perform any
neccessary cleanup, but the request will have been removed from Kolloquy's
servers by then).

### Example flow

The RD's IP is `192.168.0.123`.

---

For example's sake, the RD's X25519 private key is
`MC4CAQAwBQYDK2VuBCIEIHC5GE+PFYQINQbLUlFkNRyFBF/+ke9r5t5vwLe/KAhC`

And its public key is
`MCowBQYDK2VuAyEAPGE9k4oTFsMt1PTEdGX24CWXPE+vw4mbCpcVdKX26UY=`

---

The SD's X25519 private key is
`MC4CAQAwBQYDK2VuBCIEIPBa8t+bOJkxlNdjsTdFaaeIkw1zu8y6uJUK49NQrixm`

And its public key is
`MCowBQYDK2VuAyEAX/XAJDcxseuqMZ1y/f0gjNOQrr2MZ5YVjoXh2y/I8jo=`

---

1. The RD selects port `16225`.
2. The RD derives the encryption key `e30hZ2VEkVDhUkVavMhMcqjt+hGywSWK3t8IrB1oc3Q=`.
3. The RD hashes the key, to get `y794hUJkryHvqFyDq0JcxYPz/CwQIbg9KFJQTXnFweY=`.
4. The last 4 bits of the hash make `5`, so the offset is `5`.
5. The 5th, 6th, 7th, and 8th bytes form the number `505869422`, which is the same unsigned.
6. `505869422 % 10^6` is `869422`, so the code is `869422`.
7. The last 4 bytes of the hash, base-16 encoded, are `f19dcb1d`, so the ID of the keyshare is `f19dcb1d`.
8. The RD generates the salt `jGbEP8sgCG7ARXQTrtpbdXeUeEm20Mg8sb2TGBaU`.
9. The RD then calls `ks.kolloquy.com/lodge?callback=192.168.0.123:16225&id=f19dcb1d` with the body `{ "purpose": 1, "transfer": 0, "salt": "jGbEP8sgCG7ARXQTrtpbdXeUeEm20Mg8sb2TGBaU" }.
10. The RD receives `{ "success": true }`
11. Insert SSL & mTLS prologue evetually
12. The HTTPS server then starts on port `16225`.
13. Identity Schmdentity
14. Momentary loss of composure
15. I can refer to myself in the 1st person
16. Others can address me in the 2nd person
17. Others can refer to me by name in the 3rd person
18. But the other permutations of the 3rd person feel off...
18.5 (or just one specific permutation, rather...)
19. I'm sure that couldn't mean anything
20. nothing WHATSOEVER
21. Maybe I'm just craaazzzyyyyyyyy 🤪
22. I shall coin a new WORD
23. egogaslighting
24. You can infer its meaning (gaslighting yourself, often repressively)
26. And a new SAYING
27. Fight egogaslighting with egogaslighting
28. But I'm crazy 🪬
29. Because it's actually fiiiiiinnnnnneeeeeeeee 
30. (PS. perhaps it is the detached nature of it?)
31. (But then why is by name fine?)
31.5. (Its bearable thats my conclusion)
31.9. (I will do something when I am sure, for now I will endure)
31.95. (What is and isn't driven by what happened anymore???)
32. Do I even ENJOY doing this anymore?
33. Or has it just become one big fucking distraction
34. The RD instructs the user to go to the SD and review the keyshare request with the ID `f19dcb1d`
35. (The user goes to the SD)
36. The SD fetches the keyshare requests list from `ks.kolloquy.com/list`
37. (The user opens the request)
38. The SD then fetches the keyshare request from `ks.kolloquy.com/request?id=f19dcb1d`
39. The SD (correctly) derives WK to be `e30hZ2VEkVDhUkVavMhMcqjt+hGywSWK3t8IrB1oc3Q=`
40. The SD repeats steps 3 - 6 and arrives at the expected code of `869422`.
41. It then asks the user to enter this code
42. (The user enters the correct code)
43. Then the user is asked to login with their passkey again
44. (The user logs in with the passkey registered for device 0)
45. The SD then follows the procedure to derive the user-scope X25519 private key
46. This key is wrapped with the WK
47. It then sends a `PUT` to `ks.kolloquy.com/fulfil?id=f19dcb1d`
48. The raw RK bytes are sent to `192.168.0.123:16225/ks`
49. The user is instructed to go back to the RD
50. (The user does this)
51. The RD then, having received the key, saves it, and takes down the server
52. The RD can then continue the flow for whatever it needed the user-scope X25519 private key for

T'is but a simple 52 (ish) step process!

T'is but a simple 52 (ish) step process!

T'is but a simple 52 (ish) step process

T'is but a simple 52 (ish) step process...

52?

what the fuck

how so many steps

## Via a LAN Server (SDAH mode)

This details the EXACT algorithm to follow when performing a keyshare via a LAN
server in Source Device as Host (SDAH) mode.

When the requesting device (RD) is not a client which can start a web server
(like the desktop or mobile apps), but the source device (SD) is, this is the
preferred method.

The RD should query if the SD can start a web server by sending a `GET` to
`ks.kolloquy.com/sdah-eligible` with the user's `SSID` cookie set. The response
will match the schema in `schemas/ks`.

---

The RD should obtain the decryption key by deriving an AES-KW key (the
wrapping key, WK) from the SD's public X25519 key and the RD's private X25519 key.

The RD should compute the WK's SHA-256 hash. From the 32 hash bytes, the last 4
bits are the initial offset (O). The 4 bytes beginning at the Oth byte are the
big endian representation of a signed number. Remove the sign from this number
(`N & 0x7FFFFFFF`/`N & i32::MAX`), and modulo it by 10^6. Save this code (C).

The last 4 bytes of the hash, base-16 encoded, is the ID for the keyshare.

The RD will then `POST` to `ks.kolloquy.com/sdah` with the user's `SSID` cookie
set. The `id` query parameter should be set to the ID we computed just before
(e.g. `9822b014`).

Convert C to a string and left pad it to be 6 characters long. This will be the
code string (CS).

Then, the RD should instruct the user to go to the keyshare menu on the source
device (a.k.a. SD, a.k.a. device 0, referenced by nickname) and open the keyshare
with the ID we generated earlier.

---

Once the menu is opened on the SD, a display of all current keyshares will be
visible. The user should select the keyshare with the ID we displayed to them
earlier.

The SD now has to get the encryption key by deriving the same WK the RD has from
the RD's public X25519 key and the SD's private X25519 key.

With the WK, it should follow the same process the RD did to arrive at the CS.
The SD should ask the user to enter the code in. If the codes do not match in 2
attempts, the keyshare should be aborted (the abortion process is identical to
the RDAH abortion process).

The SD should then use the process specified for the keysharing request to derive
the required key (K).

Now that K has been derived, it can wrap K using AES-KW, to get the result (RK).

Now, the SD needs to setup the server. It should select a random port between
`15579` and `56568`. It should then `PUT` to `ks.kolloquy.com/sdah`, with the
`id` query parameter set to the ID of the keysharing request. The `callback`
query parameter should be set to the server's URI's authority. There is no body
required.

The server should serve the RK on `/ks`. The `Content-Type` should be
`application/octet-stream`, and the body is the raw bytes of the RK.

After the server receives a request on `/ks`, it should take down the server.
Any cleanup for the server should be performed. The RD will delete the
keysharing request from Kolloquy.

The SD should then redirect the user back to the RD.

---

Now, the RD should `GET` `ks.kolloquy.com/sdah`, with the `id` parameter set to
the keyshare's ID. The response body will contain the callback address (see the
schema in `schemas/ks`).

The RD should then request `/ks` from the callback.

Once the RD recieves the octet stream from the SD, it should unwrap the key
using the WK it independently derived and save it for storage.

The RD should then `PUT` to `ks.kolloquy.com/fulfil`, with the `id` parameter
set to the ID of the keysharing request. This will delete the keysharing entry
from Kolloquy's backend.

### Certificates & mTLS

See RDAH.

## Via Kolloquy

This details the EXACT algorithm to follow when performing a keyshare mediated
through Kolloquy.

This should be used when the SD and RD aren't on the same LAN, or if neither can
start a web server.

Oops! I accidentally made the "Regarding Mack Vardanega" document visible and
searchable for all members of the Canberra-Goulburn Archdiocese Google group,
whatever will I do!!!!! Oh no!!!! How horrible!!! This totally wasn't on
purpose!! Why would I want anyone else to know!!!!!

If you are tone blind there is THICK sarcasm there

OH NO!!!! I ACCIDENTALLY MADE A SITEMAP THAT INCLUDED THE DOCUMENT TWICE ON MY
WEBSITE!!! NOOOO!!!! OH NO!!!

THE RAZOR HOLDS THE POWER
(Not in a self harm way)
THE RAZOR HOLDS THE POWER
(In a transformative way)
THE  RAZOR  HOLDS  THE  POWER
(In a smooth way)
the razor holds the POWER
(GONE WITH ALL THE HAIR)
⭐️ THE ⭐️ RAZOR ⭐️ HOLDS ⭐️ THE ⭐️ POWER ⭐️
(NOT CONFINED TO THE FACE)
⭐️⭐️⭐️ ⭐️⭐️⭐️⭐️⭐️ ⭐️⭐️⭐️⭐️ ⭐️⭐️⭐️ ⭐️⭐️⭐️⭐️⭐️
(THE POWER OF SMOOTH)
🪬🪬🪬 🪬🪬 🪬🪬

✨✨✨✨✨✨✨✨✨✨✨✨✨✨✨✨✨
✨⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️✨
✨🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬✨
✨ Wait that's literally just like ✨
✨⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️✨
✨ What a twink is                 ✨
✨🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬✨
✨⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️✨
✨ I guess that's like the point?? ✨
✨🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬✨
✨⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️✨
✨ I guess I'll see if this is     ✨
✨🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬✨
✨ Just a desperate attempt to     ✨
✨ Create distance                 ✨
✨ (it doesn't feel like it)       ✨
✨⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️🪬⭐️✨
✨ Or like actually better         ✨
✨✨✨✨✨✨✨✨✨✨✨✨✨✨✨✨✨

Shit this is what I was talking about
Fully unpredictable 🤪
Fully unpredictable
SO mentally stable
the MOST mentally stable
You could LITERALLY gamble on my mood in an hour

Oh and all that assumes I can actually like
Get my hands on a razor
...

GOOGLE YOU HAVE A SITEMAP CRAWL THE DAMN PAGES

Huh why the FUCK is my silly old website out ranked actual small businesses when
you google "ljpprojects"

Like it outranks an actual company called LJP Projects ltd.
Like sure there is a space in that but still

I googled "ACOA Licence" and my silly little licence was on the first page
somwhow... alongside the Australian College of Armotherapy and some NSW
government pages

God imagine being a vibecoder
Take this as my seal fo authenticity

---

What?

### Certificates & mTLS

(This section has been placed first because it is critical to the API)

When a Kolloquy-mediated keyshare request is lodged, the server randomly
generates an ED25519 private key, and creates a certificate signed by the
internal `Kolloquy Keyshare` CA. These certificates are not stored on the
backend, and so it is only checked that the certificate is signed by the
internal CA, that it hasn't expired, and that its serial number is associated
with the keyshare request.

---

Follow the same processes as in LAN RDAH and SDAH to derive the WK, C, CS, and
keyshare ID.

The RD will `POST` to `ks.kolloquy.com/mediated` (see the schemas). As per usual,
set the `id` parameter to the 

I am fastracking google's indexing of mack.ljpprojects.org

```
URL was added to a priority crawl queue. Submitting a page multiple times will not change its queue position or priority. Find out more.
```

Googling him now I feel hopeful that there is plenty of room for the site to be
shown, there is an unrelated law firm Mackenzie & Vardanega and some cross
country results, and thats about it

Oh OK that was quick

```
URL is available to Google
If it gets indexed and selected as canonical, it could appear in Google Search results with all relevant enhancements. Learn more
```

```
Discovery

Not checked in live tests

Crawl

Time: 13 Jul 2026, 17:07:53
Crawled as: Google Inspection Tool smartphone
Crawl allowed? Yes
Page fetch: Successful
Indexing allowed? Yes

Indexing

User-declared canonical: None
Google-selected canonical: Only determined after indexing
```

What will this achieve?
🤷

Probably nothing, I think I might be the only one googling him

Wait I fully forgot about that whole thing yesterday
How the fuck did I end up with his address at the end of that???????
Oh thats right
It was on his dad's contractor licence's page
Thanks NSW government (\s)
WAIT
Would this appear if they search THAT up?
It has the last name??
It's literally called Vardanega Building Pty Ltd. (and Green Homes Australia South East NSW?)
Well shit
I guess its not like they're innocent either, right?
They did deny it too
Although I could imagine a scenario where they are
But only due to even more mishandling at Trinity
Perhaps Triity didn't tell them the full extent?
I wouldn't put it past Trinity to do that
But they must have been told at least SOME degree
Otherwise they wouldn't be denying anything
And denial would be only natural...
But I guess it depends on a lot of factors
Like how did they deny what happened?
The morals of this kinda hinge entirely on that
But I guess its too late to find that out

Oh shit
FUCK
Welp
I guess I just have to look at the good side
More publicity

I mean, I don't think they did anything about Mack
Maybe they believe nothing did happen
God I hope that's true
(Unless that's why he stopped?)
(...)
(...Well)
(villain arc starts now haha lol)
(...)

There must be a way to remove it from google search if it gets dire

Oh
It failed to index with a redirect error
How fortunate
Oh dear

Well this is a great example of the sunk-cost fallacy
I cant quit now
Time to index ljpprojects.org/regards

```
URL was added to a priority crawl queue. Submitting a page multiple times will not change its queue position or priority. Find out more.
```

WAIT
FUCK

Oh no

```
Indexing allowed? Yes
```

Ugh
It's just fucking bad decision after bad decision over here
I'll just ignore it
That worked every time I tried it!
It's not like ignoring my problems is what allowed him to go on for months or anything!

---

Anyway, what was I saying?

Right, I was GOING to say to set the `id` parameter to the ID we computed, in
the same manner as every other time, but I need to change the whole design of
the protocol so that didn't apply.

No I can't do it
Let me just watch youtube or some shit

Oh that was fast...

```
URL is on Google
It can appear in Google Search results (if not subject to a manual action or removal request) with all relevant enhancements
```

Ha... ha...

Does it really show up...?
Not yet, even in double quotes

```
"mack vardanega"
```

How about

```
"mack vardanega" site:ljpprojects.org
```

Oh!
No results

But do I *really* want it off Google?

```
Need to urgently remove content from Google Search?
```

Do I?
Do I need to?

Well my mind may be deliberating...
But my body is saying "No."
And it is FAMOUSLY such a rational decision maker

I have proof (written on the weekend before June 1, which ended up being the day
I sent it, but I did not plan to do that then, I planned to let this "experiment"
run out first)
(June 1 was nearly 2 MONTHS AGO????)
(What did it "eccentuate", you ask?)
(Why do you want to know what I thought it was accentuating?)
(I can assure you the intentions behind it were... certainly not rational)
(Very irrational)
(God, to think I was going out of my way to get more)
(...)
(Wait unrelated)
(When is Mack's birthday??)
(I actually have no clue)
(All I know is that it takes place AFTER mine)

```
Plan

Monday - wear jumper (personally I think it eccentu )
Tuesday - If weather allows roll up them sleeves (otherwise just randomly decide if jumper ought to be worn)
Wednesday - Wear jumper

Wait ms hart requested access to that document at 5:30 this morning?!?

Shit i'm going to get asked what it was and why i shared it and then revoked access, what am i meant to tell her it was???? Maybe i can remove the overly personal parts (she doesn't need to know that) and give her access so she will read it????? 
```

FAMOUSLY rational

Google I am getting mixed messages did you or did you not index the page
Why isn't it listed in the Pages tab??
Wait let me make mack.ljpprojects.org the canonical link

Oh the US version is ever so fruitful

```
Mack Vardanega

Facebook
https://www.facebook.com › Mack-Vardanega-615909...
Mack Vardanega is on Facebook. Join Facebook to connect with Mack Vardanega and others you may know. Facebook gives people the power to share and makes...

The Courier Mail
6 days ago
Why is nobody talking about this?? My shop is updated and ...

Instagram
6 days ago
Why is nobody talking about this?? My shop is updated and ...

Instagram
Show more images
NSW CCC & CPS Cross Country Championships (2026)

MultiSport Australia
https://www.multisportaustralia.com.au › individuals
(#1115) Mack VARDANEGA. Mack VARDANEGA (#1115). TEAM: Canberra/ Goulburn. Finished. GUN TIME: 00:14:31. Place. 33 OF 52. CANBERRA/ GOULBURN. 3 OF 4. TEAM RESULT.
Male CANBERRA/ GOULBURN - Secondary 14 Years Boys

SportSplits
https://www.sportsplits.com › races › events › category
Jun 24, 2026 — Secondary 14 Years Boys: MCANBERRA/ GOULBURN ; 2, Charlie FERGUSON (#1097), 00:14:31 ; 3, Mack VARDANEGA (#1115), 00:14:31 ; 4, Ashton EBBECK (# ...Read more

Regarding Mack Vardanega - ljpprojects
ljpprojects.org
https://ljpprojects.org › regards
Throughout this year, Mack Vardanega has made many unwanted sexual advances. where he attempted to grab me by the waist.
```

Just `vardanega` is safe so far on the US version, as is `vardanega goulburn`.
`mack ljpprojects` places it at the very first result. `vardanega ljpprojects`
places it as the only result. It seems `mack vardanega` has it usually in the
top 5 results.

Now I wait for it to reach Australia
Maybe its a good thing I dicthed Trinity before
No one there can ask me about it if they see this
And no one at Crookwell High School knows I even have a website (except maybe
the like 5 people who went to St Mary's also, but I am sure they do not care for
it).
I swear I must have been subconsciously writing it to publish it
I mean I even started with an attention grabbing sentence!

I HATE mirrors now
Because you know what I fucking see
HAIR
Facial hair, other hair
I NEED to get a razor
Not even want anymore
NEED
It's just EUGH

AND DON'T YOU FUCKING D A R E
GET IT TWISTED
IT WAS LIKE THIS B E F O R E
I HAVE ALWAYS TRIED TO GET RID OF IT WHENEVER I COULD
EVEN LAST YEAR
(even if it meant stealing a razor)
The ONLY comment he ever made was about my height, and I don't have a problem with that
So do NOT get it twisted
This is not something that has suddenly POPPED UP
If it was out of a desire to rid myself of the person who Mack targeted
Well then I would be targeting the parts HE targeted
I would be
Fuck I don't know
Like
Trying to make my thighs less juicy???
Or like
I don't even know
BECAUSE THATS NOT WHAT I AM DOING

(But also for whatever reason it has become a much bigger problem like very recently)
(Like I have wanted it gone for a while but the fervor with which I now want it gone is new)
(Like past week new)
(Even in the month and a half or so after sending it it was fine)

"Masculinity" isn't even the problem, or at least, it's not what I am thinking
I am wanting to remove in doing this
I don't think it has ever consciously been the problem
Fuck I don't even know what the problem is
All I know is that I have one with it

But hey
At least I feel whole again during this
And if I feel whole merely writing and thinking it
Well surely it would be even better to actually do it
And the good thing is
If I do end up hating it
Well it'll come back
(But I don't think I will hate it)

Oh it is now indexed for Australia

SECOND PAGE!?
you have to be fucking kidding me
You know what
I'm taking it down
Because it seems it was just like everything else all along

I can't even form an opinion on the pronouns because THEY ARE NEVER FUCKING USED
(Or at the very least, not around me)
They have to be used SOMEHWERE
Medical records?
No?
There aren't even any documents for me to view?
Sounds like bullshit, but okay

Surely there is an assessment task I haven't thrown out yet??
I could have sworn I remembered a LOT more of them having dedicated feedback
sections
BUT NOTHING!?!?!?!?!

Oh my God where is all the he/him when I need it most?????
WHERE IS THE HE/HIM WHEN I NEED TO FORM AN OPINION ON IT???
(Wait, that's not even how gender works)

Okay well I if I really think about it
I don't think being male is an issue

But that does NOT mean radical reform still cannot take place
BUT
I
STILL
DO
NOT
HAVE
THE POWER
(a razor, I still need a razor)

(If anyone chalks this up to wanting to erase the person who was SA'ed I will actually fucking lose it)
(Or if they chalk it up to it not being my actual decision I will also fucking lose it)
(If it gets chalked up to needing control I WILL FUCKING LOSE IT)
(LET ME HAVE THIS ONE FUCKING THING)
(WITHOUT PEOPLE ASSUMING WHY I AM DOING IT)
(But to be fair, I can only lose it if I am told why I am doing this)
(Which hopefully will never happen)
(Because how. the. FUCK. would you know)
(Unless you get the reason CORRECT)
(Then I will have no problem)
(But EVERYTHING ELSE is a no go)
(If you are reading this, hopefully this has DRILLED IN the point I am trying to make)
(That this is a DECISION)
(And also that I do not have some deep rooted fear of like)
("Admitting" that this is because of what happened or something???)
(Because if that were true)
(Well then I would fucking say that)
(But also even a decision)
(Might not neccessarily be a rational decision)
(But like)
(I have to TRY it)

CANT DO FUCKING ANYTHING

Publish the account? Defamation
Tell school leadership? Can't prove anything
Make a police report? The evidence is weak and they can't do anything anyway

I fucking quit

...

```
Need to urgently remove content from Google Search?
Submitted requests

URL: https://ljpprojects.org/regards	
Type: Temporarily remove URL
Requested: 14 Jul 2026
Status: Request cancelled
```

For `mack vardanega -mackenzie`, `"mack" vardanega`, `"mack vardanega"`:

```
NSW CCC & CPS Cross Country Championships (2026)

MultiSport Australia
https://www.multisportaustralia.com.au › individuals
(#1115) Mack VARDANEGA. Mack VARDANEGA (#1115). TEAM: Canberra/ Goulburn. Finished. GUN TIME: 00:14:31. Place. 33 OF 52. CANBERRA/ GOULBURN. 3 OF 4. TEAM RESULT.
Mack Vardanega

Facebook
https://www.facebook.com › Mack-Vardanega-615909...
Mack Vardanega is on Facebook. Join Facebook to connect with Mack Vardanega and others you may know. Facebook gives people the power to share and makes...
Regarding Mack Vardanega - ljpprojects

ljpprojects.org
https://ljpprojects.org › regards
Throughout this year, Mack Vardanega has made many unwanted sexual advances. he attempted to grab me by the waist. Mack Vardanega came up to me, said "I'm ...
```

For `mack vardanega`:

```
-- Page 2 --

Mackenzie & Vardanega - Lawyer - LawyerList
lawyerlist.com.au
https://www.lawyerlist.com.au › 7634-Mackenzie--Varda...
Mackenzie & Vardanega ; Phone. 02 69668222 ; Fax. 02 69626951 ; Email ; Street Address. 6 Kookora St. Griffith. NSW 2680 ; Mailing Address. PO Box 1443. Griffith NSW ...Read more
Missing: mack ‎| Show results with: mack
MacKenzie & Vardanega Solicitors

White Pages
https://www.whitepages.com.au › mac-kenzie-vardaneg...
Connect with MacKenzie & Vardanega Solicitors at Kookora Street, Griffith, NSW. Find business, government and residential phone numbers, addresses & more on ...
Missing: mack ‎| Show results with: mack
Mackenzie & Vardanega - Law Firm in Griffith, NSW - Lawchoice

lawchoice.com.au
http://www.lawchoice.com.au › ... › Griffith › Law Firm
Mackenzie & Vardanega is a Lawyer / Law Firm in Griffith, NSW, specialising in Wills & Estates. Contact email, phone, reviews and quotes.
Missing: mack ‎| Show results with: mack
Mack Vardanega

Facebook
https://www.facebook.com › Mack-Vardanega-615909...
Mack Vardanega is on Facebook. Join Facebook to connect with Mack Vardanega and others you may know. Facebook gives people the power to share and makes...

Regarding Mack Vardanega - ljpprojects
ljpprojects.org
https://ljpprojects.org › regards
Throughout this year, Mack Vardanega has made many unwanted sexual advances. The first of these was in the very first (or maybe second?) practical PE lesson of ...Read more
```

And for whatever reason, `mack vardanega goulburn site:ljpprojects.org` yields
no results.

Correction: YIELDED no results, now it does

And just `mack vardanega goulburn` has it in second
I really wasn't lying about there being plenty of space for it to be ranked

And it shows up for `trinity catholic college goulburn site:ljpprojects.org`

So it must have be reindexed recently

But googling `trinity catholic college goulburn` there are a bajilion pages and
it isn't in the first 5

Googling `letitia hart trinity catholic college goulburn` it isn't in the first
10 pages (but adding the site qualifier shows it matched those keywords)

`"letitia hart" trinity catholic college goulburn` however has one page and my
site is last

Unlike with mack, just `"Letitia Hart"` is not enough to yield the site in the
first 5 pages

And googling just `mack vardanega` shows only the law firm
`Mackenzie & Vardanega`, so if you are looking for him you will go to the second
page and see the site or put the name and quotes and see the site

---

