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
| root | `:enter?` | Unqualified-entry discovery: always replies with a ctx, `{ parent, rev }`, naming a room to enter. Defaults to the configured `start` room; may consult `#house` internally for a DID-specific answer. |
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

`/ma/node/0.0.1` provides one persisted `children` map keyed by each node's
DID or full actor DID-URL. Rooms filter that map for bare-DID clients, agents,
things, containers, and exits. A bare DID has no identity entity, but remains
a node in the room hierarchy.

Parent changes are target-accepted: a child sends `:parent <ctx>`, the target
confirms `:child <ctx>`, the child commits, then informs its old parent. Each
actor verifies `msg.from` before accepting ctx. Revisions provide ordering and
idempotency, not authority.

Parenting is placement, not ownership. `:set-parent` and `:hold` may change a
movable actor's parent but MUST NOT change its `owner`; an unowned actor remains
unowned after either operation. For an existing movable actor, `:claim` is the
only ownership-changing verb. `:forge` is the sole creation exception: it
initialises the newly created actor's owner from `msg.from`. Owner-gated actions
such as locking a container require a prior claim and never claim implicitly.

## Avatar hand and inventory

An avatar has one client-side hold slot. Its inventory is an ordinary container
created and owned by that avatar; it has no special runtime kind or container
behaviour. When the avatar requests another item while its hand is occupied, it
silently sends the held item its ordinary `:set-parent <inventory>` request.
After Zion receives the item's authoritative departure notice and clears the
hold slot, it sends the queued ordinary `:hold` request for the next item.

`drop [name]` remains independent of this convenience: it resolves against the
held item and the inventory. It sends an already-held item its direct
`:set-parent <room>` request. For an inventory child it starts the ordinary
`:hold` handshake with the room as the client-side follow-up target, so Zion
sends `:set-parent <room>` only after becoming the item's current parent.

## Container locks

A claimed container's owner may call `:lock` and `:unlock` at any time. Calling
`:lock <secret>` locks the container and stores that secret for a non-owner to
use with `:unlock <secret>`. A later `:lock <new-secret>` replaces the secret
immediately, even if the container is already locked. Bare `:lock` leaves an
existing secret unchanged; a container locked without a secret can only be
unlocked by its owner.

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