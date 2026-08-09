# lambda-ma

Lambda-ma is a small direct-DID actor world for `ma-runtime` and zion. It
publishes Scheme actor behaviours and a bootstrap YAML that builders can fork,
reshape, and extend.

The world has no identity entity. A user acts as their authenticated bare
`did:ma:` DID. Rooms hold DID-keyed presence, while agents, things,
and containers remain ordinary actors with full DID-URLs.

## Contents

```text
actors/root.ma          local trust anchor and runtime service directory
actors/house.ma         DID and actor ctx registry and transition coordinator
actors/room.ma          room presence, exits, presentation, and room policy
actors/exit.ma          direct DID traversal policy
avatar.zscheme          local zion vocabulary and world event presentation
events.zscheme          routing: dispatches incoming events/replies to avatar.zscheme handlers
kinds/                  kinds used by the generated bootstrap
scheme-actor/           Scheme layers and the Wasm actor host
Makefile                publishes sources and generates dist/lambda-ma.yaml
```

Load `avatar.zscheme` then `events.zscheme` explicitly into zion's local
session environment when using this world profile, in that order, since
`events.zscheme` routes to handlers defined in `avatar.zscheme`. Lambda-ma
owns its commands and presentation; zion does not bundle or load either file
automatically.

The bootstrap creates `#root`, `#house`, `#scheduler`, and `#construct`.
`#root :ctx?` exposes the runtime service directory. A bare DID enters a room
directly with `:enter`; the room replies with the committed DID ctx and
publishes it to the full DID-URL in `ctx.house`. House keeps the latest ctx,
including name, nick, description, and parent, indexed by bare DID.

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