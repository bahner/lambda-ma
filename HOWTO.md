# lambda-ma HOWTO

Lambda-ma is a direct-DID seed world for `ma-runtime`. A user interacts as
their own `did:ma:` identity; rooms store direct DID presence and no identity
entity is created.

## Prerequisites

Install Kubo/IPFS, a current `ma` runtime, Rust, and the Wasm target:

```sh
rustup target add wasm32-unknown-unknown
ma --help
ipfs --version
```

Kubo must be running because the build publishes actor behaviours and the
bootstrap manifest to IPFS.

## Build

From this repository:

```sh
make clean
make
make check
```

This produces `dist/lambda-ma.yaml`. Do not edit that generated file; edit
actor sources, kinds, or `lambda-ma.template.yaml`, then build again.

## Start a world

```sh
ma --bootstrap dist/lambda-ma.yaml
```

Open operator, unlock or create an identity, then discover the runtime. Obtain the
room target from root or known bootstrap state and enter it directly:

```text
@runtime#root:ctx?
@runtime#concourse:enter {name: "Visitor", nick: "Visitor"}
```

Without a known room address, enter root instead — it always redirects to a
room:

```text
@runtime#root:enter
```

Root replies `[:entrance, <room>]`; enter that room with the same traversal. The
client supplies `did` and a fresh `traverseId` for you.

The room's reply is the current DID ctx. Plain focused commands and direct
methods both address that confirmed room; there is no intermediate actor.

The bootstrap `#concourse` is unowned. Claim it before naming, describing, or
digging from it, because room props and `:dig` are owner-only:

```text
@runtime#concourse:claim
@runtime#concourse:prop name Concourse
@runtime#concourse:dig north
```

A room created by `dig` is automatically owned by the digger.

## Lock a container

After claiming a container, its owner may lock it directly. A supplied secret
allows another authenticated DID to unlock it; calling `:lock` with a different
secret replaces the old one immediately.

```text
@runtime#bag:lock supersecret
@runtime#bag:unlock supersecret
```

## Dig an exit

`dig <direction>` creates a passage and, unless `to` names an existing room
DID-URL, a new room. The destination room may name the same passage with a
different word: `back <direction>` sets that word. Without `back`, both ends
use the source direction.

```text
dig down to "Top of a very large tree" back up
dig north to did:ma:<runtime>#library back south
```

The two rooms name the passage independently, so renaming one end never changes
the other. `dig <passage-did-url> <name>` replaces this room's local name for a
passage; the old local name and the passage itself are left untouched except
for the rename:

```text
dig did:ma:<runtime>#passage-… up
```

`exits?` shows each local name and its passage DID-URL when you need the
address. The far room keeps its own word.

Digging an existing direction again is refused and reports the passage DID so
you can rename or `fill` it. Actor DIDs are deterministic per source room and
direction, so a filled direction re-dug later reuses the same passage and room
identity.

## Lock an exit

An exit is one top-level two-ended node; a room references it by a cosmetic
local name in its `exits` map. Its owner or its source room may lock it
directly. The secret contract is the same as for containers: anyone holding the
secret may unlock the exit, and a lock is global — locked is locked for
everyone until it is unlocked again.

```text
lock mirror with supersecret
unlock mirror with supersecret
```

`unlock mirror with supersecret` opens a locked exit for everyone; a following
`go mirror` then walks through. Without the secret the exit blocks the way and
narrates the refusal to the traveller as an in-world `:say` event, using the
`nogo_msg` prop (default `This is not the way.`).

Exit narration props use the LambdaCore names (`leave_msg`, `oleave_msg`,
`arrive_msg`, `oarrive_msg`, `nogo_msg`); the owner or the source room sets
them with `:message <slot> <text>` or `:prop <key> <text>`. The short slot names
are `leave`, `oleave`, `arrive`, `oarrive`, and `nogo`:

```text
prop mirror nogo_msg None shall pass!
```

## Clear out disconnected players

A browser tab that simply closes never announces a departure, so its bare DID
lingers in the room as a tombstone. A roll-call rebuilds the room's children
from whoever actually answers:

```text
@runtime#concourse:roll-call begin 10
```

Every child is asked to re-announce, and ten seconds later the runtime
scheduler ends the roll-call automatically: whoever reported is kept with the
context they just sent, and whoever stayed silent is gone. Omit the seconds to
end it yourself instead:

```text
@runtime#concourse:roll-call begin
@runtime#concourse:roll-call
@runtime#concourse:roll-call end
```

Bare `:roll-call` reports who has answered so far. Only the room's owner or its
current parent may start one. The bootstrap `#concourse` is unowned until
someone claims it, so on a fresh world the owner check fails until you run the
room's `claim` command first. If nobody answers at all the swap is skipped and
the existing children are kept, since that is far more likely to be a transport
failure than a genuinely empty room.

## Profile references

- Normative contract: [ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md)
- Shipped actor API: [REFERENCE.md](REFERENCE.md)
- Actor protocol overview: [actors/README.md](actors/README.md)