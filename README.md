# λ-間

λ-間 is a small actor space for `ma-runtime` and zion. It is not runtime code; it is a flat set of actors and a generated bootstrap YAML for people who want to clone, fork, and reshape it.

## What is here

```text
actors/root.ma          placement registry, avatar creation, zion :ctx receipts
actors/avatar.ma        user command endpoint
actors/room.ma          room policy, occupants, dig/go, exit ownership
actors/exit.ma          traversal between rooms
actors/duck.ma          stationary rubber duck that can say kvakk
actors/python/          reserved for Python actors when a concrete feature needs them
Makefile                publishes actor sources and generates dist/lambda-ma.yaml
```

The generated bootstrap currently contains only what this MVP needs: the generic ma-scheme actor kind, a genesis variant, the λ-間 actor kinds, the scheduler, `#root`, and the initial `#construct` room.

Python actors are intentionally not bulk-copied yet. The existing Python actor libraries still live in the workspace-level `python-ma-actors` repo; they should move into `actors/python/` when a concrete λ-間 feature uses them, along with a Makefile path that builds their Wasm and wires their kind CIDs into the generated bootstrap.

## Build

Kubo/IPFS must be running locally.

```sh
make
```

This publishes `actors/*.ma` with `ipfs add` and writes:

```text
dist/lambda-ma.yaml
```

To see the world behaviour CIDs:

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

## Develop

Edit the `.ma` files, then run:

```sh
make clean
make
```

Commit the source files and template, not `dist/`.
