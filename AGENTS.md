# lambda-ma - Agent Notes

`lambda-ma` publishes Scheme actor behaviours and a bootstrap template for the
lambda-ma world profile. It is not runtime code.

## Agent rules

- Never modify files outside this workspace without explicit user approval.
- Commit source and template files, never generated `dist/`,
  `scheme-actor/target/`, or `scheme-actor/actor.wasm`.
- Use British English for project-owned names and prose.
- Do not modify `rust-ma-runtime` for world-profile work.

## Direct-DID profile

A bare DID in `msg.from` is the authenticated identity; no identity entity
exists.

- `#root` is the hardcoded local trust anchor. `:ctx?` publishes dynamic full
  DID-URL service references such as `#scheduler` and `#house`.
- `#house` owns world policy for DID ctx and entity ctx. It must not
  impersonate or forward commands for an identity.
- A client enters a known room directly with `:enter`; the room stores a
  DID-keyed presence record and replies with its committed snapshot.
- Clients route focused room commands directly to the confirmed room DID-URL.
- `msg.from` is the sole authenticated fact. Ctx and revisions are not
  authentication; revisions only order authoritative snapshots and retries.
- Node child records stay keyed by full actor DID-URLs. DID-keyed presence is
  room state, not a child actor.

## RPC and events

Use `ma-reply!` for getters, setters, configuration, introspection, metadata,
validation, and errors. Use room broadcasts or `:print` only for in-world
events such as arrival, departure, speech, emotes, movement, and transfer.
An action may return a bare technical `:ok`, but its visible outcome belongs
only on the `:print` event channel.

## Documentation contract

The normative profile is `ma-spec/runtime/ma-lambda-ma-v1.md`. Keep it aligned
with this repository's `REFERENCE.md`, `README.md`, `HOWTO.md`, and actor
sources when changing interoperable behaviour.

## Scheme actor

`scheme-actor/` contains the generic Wasm host and common Scheme layers.
`Makefile` builds and publishes them, then substitutes CIDs into
`dist/lambda-ma.yaml`.