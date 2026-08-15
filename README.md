# lambda-ma

Lambda-ma is a small direct-DID actor world for `ma-runtime` and zion. It
publishes Scheme actor behaviours and a bootstrap YAML that builders can fork,
reshape, and extend.

The world has no identity entity. A user acts as their authenticated bare
`did:ma:` DID. Rooms store every node, including bare DIDs, in one `children`
map; agents, things, and containers use full DID-URLs as their child keys.

## Contents

```text
actors/root.ma          local trust anchor and runtime service directory
actors/house.ma         DID and actor ctx registry and transition coordinator
actors/room.ma          room presence, exits, presentation, and room policy
actors/exit.ma          direct DID traversal policy
kinds/                  kinds used by the generated bootstrap
scheme-actor/           Scheme layers and the Wasm actor host
Makefile                publishes sources and generates dist/lambda-ma.yaml
```

The zscheme repository owns the Zion-side `stdlib.zscheme`, `runtime.zscheme`,
`avatar.zscheme`, and `events.zscheme` libraries. Its combined `.my.z.scheme`
source provides the ordinary client vocabulary for this profile. Lambda-ma
owns the actor behaviours and wire contract, not duplicate client-library
sources.

The bootstrap creates `#root`, `#house`, `#scheduler`, and `#construct`.
`#root :ctx?` exposes the runtime service directory. Only `#root` is required
in a given runtime — `#house` is an optional convenience and a runtime need
not have one. A bare DID enters a room directly with `:enter`; the room
replies with the committed DID ctx and, if a `#house` is configured, publishes
it to the full DID-URL in `ctx.house`. House keeps the latest ctx, including
name, nick, description, and parent, indexed by bare DID.

A client that has no room address yet asks `#root :enter?`, which always
replies with a ctx naming a room to enter — the configured `start` room by
default, or a richer DID-specific answer if root chooses to consult `#house`
internally.

Claimed containers support owner-controlled locks. An owner may use `:lock` or
`:unlock`; `:lock <secret>` additionally grants anyone with that secret access
to `:unlock <secret>`, until the owner replaces it with another `:lock` call.

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