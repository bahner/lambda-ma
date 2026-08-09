# lambda-ma Reference

**Version:** 2.0.0
**Status:** Draft

This is the implementation reference for the shipped lambda-ma actors. The
normative interoperability profile is
[ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md).

## Bootstrap

The generated bootstrap contains `#root`, `#house`, `#scheduler`, and the
initial `#construct` room. `#root` is the only parentless lambda-ma node.

`#root :ctx?` returns a root-issued service map:

```text
{
  runtime: "did:ma:<runtime>",
  root: "did:ma:<runtime>#root",
  scheduler: "did:ma:<runtime>#scheduler",
  house: "did:ma:<runtime>#house",
  rev: 1
}
```

The root itself is a hardcoded local DID-URL and is not an entry in this map.

## Entry and room API

There is no identity entity. The bare controlling DID is `msg.from`.

| Receiver | Verb | Result |
| --- | --- | --- |
| room | `:enter [claim]` | Commits presence and replies with the DID ctx: `did`, `parent`, `name`, `nick`, `description`, and `rev`. |
| room | `:look`, `:who?`, `:occupants?`, `:things?`, `:exits?` | Direct room presentation. |
| room | `:say`, `:emote` | Broadcasts an in-world event to present DID identities. |
| exit | `:traverse <{ did, parent }>` | Returns `{ did, parent, text, exit, direction }`; client follows with direct room entry. |
| root | `:ctx?`, `:register` | Returns the runtime service ctx; `:register` subscribes `msg.from` and sends it the current ctx. |
| house | `:did-ctx <did> <ctx>` | Internal room publication. Removes the DID from a previous parent, then records the DID ctx. |
| house | `:did-ctx? <did>` | Returns the latest recorded DID ctx for a bare DID; open until an ACL is introduced. |
| house | `:entity-ctx <ctx>` | Records a base actor ctx under the full sender DID-URL. |
| house | `:entity-ctx? <actor-did-url>` | Returns the stored entity ctx. |

A client accepts entry context only when the reply corresponds to its newest
entry request and originates from the requested room. It never accepts an
unsolicited ctx push as a routing update.

After committing entry, a room publishes the ctx through its full
`ctx.house` reference. House indexes it by bare DID. If the registry names a
different previous parent, house sends that parent `:leave <did>` before storing
the new ctx. A room accepts that targeted leave only from exact `ctx.house`;
clients continue to use argument-free `:leave` for themselves.

## Actor hierarchy

`/ma/node/0.0.1` provides one persisted `children` map keyed by full actor
DID-URL. Rooms filter that map for agents, things, containers, and exits. A
room's DID-keyed presence records are separate room state; no identity entity
is created.

Parent changes are target-accepted: a child sends `:parent <ctx>`, the target
confirms `:child <ctx>`, the child commits, then informs its old parent. Each
actor verifies `msg.from` before accepting ctx. Revisions provide ordering and
idempotency, not authority.

## Replies and events

Use `ma-reply!` for technical results: queries, metadata, configuration,
structured values, validation, and errors. A room's `:look`, `:who?`,
`:exits?`, and `:things?` responses are reply payloads, not print messages.

Use `:print` only for world events. Rooms broadcast arrival, departure, speech,
emotes, movement, and completed transfer narration to present bare DIDs. A
technical bare `:ok` may acknowledge an action, but it is not narrative output.

## Build

```sh
make clean
make
make check
```

Generated artefacts live in `dist/` and are not source-controlled.