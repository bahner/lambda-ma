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

Open zion, unlock or create an identity, then discover the runtime. Obtain the
room target from root or known bootstrap state and enter it directly:

```text
@runtime#root:ctx?
@runtime#construct:enter {name: "Visitor", nick: "Visitor"}
```

Without a known room address, ask root instead — it always answers with a
room to enter:

```text
@runtime#root:enter?
```

The room's reply is the current DID ctx. Plain focused commands and direct
methods both address that confirmed room; there is no intermediate actor.

## Lock a container

After claiming a container, its owner may lock it directly. A supplied secret
allows another authenticated DID to unlock it; calling `:lock` with a different
secret replaces the old one immediately.

```text
@runtime#bag:lock supersecret
@runtime#bag:unlock supersecret
```

## Lock an exit

A room's owner locks its exits through the room; the exit accepts lock commands
from no one else. The secret contract is the same as for containers: anyone
holding it may unlock the exit, and a traveller may pass straight through a
locked exit with `go <direction> <secret>`.

```text
@runtime#room:exit mirror lock supersecret
@runtime#room:exit mirror unlock supersecret
```

`go mirror supersecret` traverses the locked mirror; without the secret the
exit blocks the way and narrates the refusal to the traveller as an in-world
`:say` event. The message defaults to `This is not the way.` and is settable
with the `locked-msg-text` prop (the `:message blocked <text>` slot is a
synonym). Exit props follow the shared `:prop` contract: the source room may
edit them through `:exit`, and the room owner may set them directly with the
ordinary avatar command:

```text
@runtime#room:exit mirror prop locked-msg-text None shall pass!
prop mirror locked-msg-text None shall pass!
```

## Clear out disconnected players

A browser tab that simply closes never announces a departure, so its bare DID
lingers in the room as a tombstone. A roll-call rebuilds the room's children
from whoever actually answers:

```text
@runtime#construct:roll-call begin 10
```

Every child is asked to re-announce, and ten seconds later the runtime
scheduler ends the roll-call automatically: whoever reported is kept with the
context they just sent, and whoever stayed silent is gone. Omit the seconds to
end it yourself instead:

```text
@runtime#construct:roll-call begin
@runtime#construct:roll-call
@runtime#construct:roll-call end
```

Bare `:roll-call` reports who has answered so far. Only the room's owner or its
current parent may start one. The bootstrap `#construct` is unowned until
someone claims it, so on a fresh world the owner check fails until you run the
room's `claim` command first. If nobody answers at all the swap is skipped and
the existing children are kept, since that is far more likely to be a transport
failure than a genuinely empty room.

## Profile references

- Normative contract: [ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md)
- Shipped actor API: [REFERENCE.md](REFERENCE.md)
- Actor protocol overview: [actors/README.md](actors/README.md)