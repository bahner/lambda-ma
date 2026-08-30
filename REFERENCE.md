# lambda-ma Reference

**Version:** 2.0.0
**Status:** Draft

This is the implementation reference for the shipped lambda-ma actors. The
normative interoperability profile is
[ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md).

## Bootstrap

The generated bootstrap contains `#root`, `#scheduler`, and the initial
`#construct` room. `#root` is the only parentless lambda-ma node.

`#root :ctx?` returns a root-issued service map:

```text
{
  runtime: "did:ma:<runtime>",
  root: "did:ma:<runtime>#root",
  scheduler: "did:ma:<runtime>#scheduler",
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
| root | `:enter?` | Unqualified-entry discovery: always replies with a ctx, `{ parent, rev }`, naming a room to enter. Defaults to the configured `start` room. |

A client accepts entry context only when the reply corresponds to its newest
entry request and originates from the requested room. It never accepts an
unsolicited ctx push as a routing update.

Clients leave a room with argument-free `:leave`; a targeted `:leave <did>` is
not accepted from other principals.

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

An owner may prepare a consent-based transfer with
`:set-recovery-secret <one-time-secret>`. A later
`:claim <one-time-secret>` from an authenticated bare DID atomically sets that
DID as owner and clears the secret; replay therefore fails. Client commands
such as `give <object> to <person>` may distribute that full claim command in
an ordinary text message, but no runtime actor changes ownership merely because
an offer was sent.

## Roll-call

`:roll-call [begin [seconds]|end]` rebuilds a node's `children` map from the
children that actually answer. It exists because a disconnected client leaves
a tombstone that nothing else removes: an avatar is a bare DID with no
identity entity, so it cannot be probed and never announces its own departure
when a browser tab simply closes.

Only the node's owner or its current parent may start a roll-call. Bare `:roll-call`
reports the state of one in progress.

The algorithm is a shadow-map swap:

1. `begin` opens an **empty** shadow map, marks the roll-call active, and sends
   `:roll-call-child` to every current child. The live `children` map is left
   untouched, so a node under roll-call keeps serving its current tree.
1. Children need no roll-call-specific logic. `:roll-call-child` makes a node
   re-announce to its own cached parent with the ordinary `:parent <ctx>`, so
   the pinger's identity is irrelevant and the answer is a plain RPC with no
   parenting side effects.
1. While the roll-call is active, every `remember-child!` writes into both the
   live map and the shadow map, and every `forget-child!` deletes from both.
   Because that is the single choke point for admission, a node that *arrives*
   mid-window is captured too, and one that leaves is not resurrected.
1. `end` replaces `children` with the shadow map wholesale. An absent child is
   simply one that never reappeared, and every survivor carries the ctx it
   reported during the window rather than stale stored state.

`begin <seconds>` registers a one-shot `:in` schedule with the runtime
scheduler, which dispatches the end under its own `<runtime>#scheduler`
fragment; that fragment is therefore accepted as a caller for the `end` phase.
Without `seconds`, the roll-call stays open until its initiator ends it.

An empty shadow map at `end` while the live map is non-empty is treated as a
transport failure rather than an empty node: the swap is skipped and the
children are kept.

## Random values

Scheme actors expose two sibling bounded-integer functions. `(random n)` uses
the actor-local deterministic PRNG seeded from runtime metadata. It is suitable
for world behaviour, but not secrets. `(ma-random n)` requests cryptographic
entropy from the runtime's `ma_random_bytes` host function and returns an
unbiased integer in `[0, n)`. Both require one positive integer argument.

`ma-random` fails when the kind does not grant `ma_random_bytes` or the host
cannot provide secure bytes; it never falls back to the actor-local PRNG.

## Avatar transfer and inventory

An avatar's inventory is an ordinary container created and owned by that
avatar; it has no special runtime kind or container behaviour. Transfer
relationships are authoritative in actor ctx values and the parent/child
handshake. Clients do not maintain a hold slot, pending transfer, queue, or
follow-up state. `hold` and `put` issue ordinary actor requests using the ctx
received from the relevant actor.

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