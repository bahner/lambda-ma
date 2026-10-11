# lambda-ma

Lambda-ma is a small direct-DID actor world for `ma-runtime` and operator. It
publishes Scheme actor behaviours and a bootstrap YAML that builders can fork,
reshape, and extend.

The world has no identity entity. A user acts as their authenticated bare
`did:ma:` DID. Rooms store every node, including bare DIDs, in one `children`
map; agents, things, and containers use full DID-URLs as their child keys.

## Contents

```text
actors/root.ma          local trust anchor and runtime service directory
actors/room.ma          room presence, exits, presentation, and room policy
actors/exit.ma          direct DID traversal policy
actors/trigger.ma      trigger actor — per-DID counters and narration rules
actors/trigger.md       trigger actor protocol — per-DID counters and narration rules
kinds/                  kinds used by the generated bootstrap
scheme-actor/           Scheme layers and the Wasm actor host
Makefile                publishes sources and generates dist/lambda-ma.yaml
```

The zscheme repository owns the Operator-side `stdlib.zscheme`, `runtime.zscheme`,
`avatar.zscheme`, and `events.zscheme` libraries.
Lambda-ma owns the actor behaviours and wire contract, not duplicate client-library
sources.

The bootstrap creates `#root`, `#scheduler`, and `#concourse`.
`#root :ctx?` exposes the runtime service directory; only `#root` is required
in a given runtime. A bare DID enters a room directly with `:enter`; the room
replies with the committed DID ctx.

A client that has no room address yet asks `#root :enter?`, which always
replies with a ctx naming a room to enter — the configured `start` room by
default.

Claimed containers support owner-controlled locks. An owner may use `:lock` or
`:unlock`; `:lock <secret>` additionally grants anyone with that secret access
to `:unlock <secret>`, until the owner replaces it with another `:lock` call.

An exit is one top-level two-ended node; a room references it by a cosmetic
local name in its own `exits` map. Exits use the same `:lock <secret>` contract
as containers, addressed directly: the owner or the source room locks it, and a
traveller holding the secret presents it with `unlock <direction> with <secret>`
to open the exit (a lock is global: locked is locked, unlocked is unlocked, for
everyone).

Walking an exit, `go <direction>`, runs the `bless → key → enter` protocol: the
exit blesses the destination, which issues a one-time key, and the traveller
enters with it. A free room also admits a bare DID directly; a room with
`free_entry: false` requires a key, so nobody can jump in.

The normative contract is
[ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md). Local actor APIs
and bootstrap details are in [REFERENCE.md](REFERENCE.md); operational setup is
in [HOWTO.md](HOWTO.md).

## Build

Kubo/IPFS and the Rust Wasm target are required:

```sh
rustup target add wasm32-unknown-unknown
make
make check
```

The build publishes the actor inputs with `ipfs add` and writes the generated
bootstrap to `dist/lambda-ma.yaml`. Do not commit generated `dist/` content.
