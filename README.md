# λ-間

λ-間 is a small actor space for `ma-runtime` and zion. It is not runtime code; it is a flat set of actors and a generated bootstrap YAML for people who want to clone, fork, and reshape it.

Think of it as our rough equivalent of `lambdamoo-core`: a basic functional
set of world objects that gives a new runtime enough language to become a
place. The point is not to preserve λ-間 exactly as shipped, but to give builders
something useful to extend, replace, and mutate inside their own runtime.

LambdaMOO grew out of the MUD tradition at Xerox PARC in the early 1990s. Its
lasting idea was not just rooms and exits, but a programmable social world: the
database, verbs, objects, and local customs could be changed by the people
living in it. `lambdamoo-core` was the small seed database that made a fresh MOO
bootable and habitable before its own culture took over. λ-間 aims for that same
role in the `ma` stack: enough root, avatar, room, and exit behaviour to start
creating, while staying small enough that a runtime can make it its own.

## What is here

```text
actors/root.ma          deterministic avatar factory
actors/avatar.ma        user command endpoint
actors/room.ma          room policy, occupants, claim/owner, dig/go, exit ownership
actors/exit.ma          traversal between rooms
actors/python/          reserved for Python actors when a concrete feature needs them
kinds/                  kind definitions used by the generated bootstrap
scheme-actor/           generic ma-scheme actor Wasm crate and stdlib
Makefile                publishes actor sources and generates dist/lambda-ma.yaml
```

The generated bootstrap reads its kind registry from `kinds/*.yaml`, then fills in CIDs for the local ma-scheme actor and actor behaviour sources. It currently includes the generic ma-scheme actor kind built from `scheme-actor/`, the λ-間 actor kinds, the scheduler, `#root`, the initial `#construct` room, and the ready Python kind descriptors kept in `kinds/`.

The bundled ma-scheme actor implements the core `random` builtin from
`ma-scheme-v1`: `(random n)` returns a non-cryptographic integer in `[0,n)`.
It is practical randomness for ordinary actor choices such as exits, fortunes,
and dice rolls. It is not security randomness: do not use it for keys, nonces,
capabilities, tokens, proofs, authentication challenges, or anything where
prediction changes authority or privacy. The reference implementation uses a
runtime/entity-seeded 64-bit PRNG.

Python actor libraries are intentionally not bulk-copied yet. Their reusable kind descriptors live in `kinds/`; actor source and build paths should move into `actors/python/` when a concrete λ-間 feature uses them.

For a full first-run guide, including Kubo/IPFS setup, installing `ma`,
generating `dist/lambda-ma.yaml`, bootstrapping a runtime, generating a reusable
root CID, and changing your first rooms, see [HOWTO.md](HOWTO.md).

For the canonical lambda-ma world protocol contract (routing, enter ctx,
authority model, actor verbs, and state-key conventions), see
[REFERENCE.md](REFERENCE.md).

## Build

Kubo/IPFS must be running locally.

The Rust Wasm target must be installed because `make` builds the local scheme actor before publishing the bootstrap inputs:

```sh
rustup target add wasm32-unknown-unknown
```

```sh
make
```

This builds `scheme-actor/actor.wasm`, publishes it together with `scheme-actor/stdlib.ma` and `actors/*.ma` using `ipfs add`, and writes:

```text
dist/lambda-ma.yaml
```

To see the scheme actor, stdlib, and world behaviour CIDs:

```sh
make show-cids
```

To verify that the generated YAML has no unresolved placeholders:

```sh
make check
```

## Bootstrap a runtime

```sh
make root-cid
```

The command prints a runtime root CID. Start `ma` with that CID or save it in the daemon config according to your usual runtime workflow.

`make bootstrap` is kept as an alias for `make root-cid`.

## Wire zion to the world root

`/config/root` is runtime manifest config, not part of this world bootstrap template. Set it once after the runtime is up and zion has discovered `@ma`:

```text
.ma!discover
@ma/config/root: @ma#root
.enter @ma
```

After that, zion routes focus shorthand through the avatar created by `#root`.

## Building rooms

Rooms are owned by user DIDs, not avatars. An avatar is only the user's current
command costume. For normal focus commands the avatar acts as a delegate and
forwards the user's DID to the room; direct room RPCs still use the message
`from` DID as the caller.

```text
help                   show avatar/user commands
help here              ask the current room what is possible here
claim                  claim an unowned room through your avatar
owner                  show the current room owner through your avatar
owner did:ma:<target>  transfer the room to another user DID through your avatar
dig north to Garden    create an exit and a new room owned by you
```

Colon-prefixed methods bypass the avatar and target the focused room directly:

```text
:help
:prop name Biblioteket
:prop description Et stille bibliotek.
:prop description
@ma#construct!help
@ma#construct!prop name Biblioteket
@ma#construct!prop description Et stille bibliotek.
@ma#construct!prop description
```

The last form resets `description` to the default.

Only the current owner may create exits from a room. Newly dug rooms are owned
by the digger automatically, so a builder can give someone a room with `owner`
and that user can then build outward from there. Linking to an already-existing
room is allowed when the target room confirms that the same user owns it too.

## Develop

Edit the `.ma` files or the local scheme actor crate, then run:

```sh
make clean
make
```

Commit the source files and template, not `dist/`, `scheme-actor/target/`, or `scheme-actor/actor.wasm`.
