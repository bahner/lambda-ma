# lambda-ma Reference

**Version:** 2.0.0
**Status:** Draft

This is the implementation reference for the shipped lambda-ma actors. The
normative interoperability profile is
[ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md).

## Bootstrap

The generated bootstrap contains `#root`, `#scheduler`, and the initial
`#concourse` room. `#root` is the only parentless lambda-ma node.

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
| room | `:narrate <text>` | Owner-only ambient narration: the room itself broadcasts a subject-less in-world event. |
| exit | `:traverse <{ did, parent }>` | Returns `{ did, parent, text, exit, direction }`; client follows with direct room entry. |
| exit | `:to?` | Returns the destination room DID-URL, or `"(none)"` when the exit is unfilled. |
| root | `:ctx?`, `:register` | Returns the runtime service ctx; `:register` subscribes `msg.from` and sends it the current ctx. |
| root | `:enter?` | Unqualified-entry discovery: always replies with a ctx, `{ parent, rev }`, naming a room to enter. Defaults to the configured `start` room. |

Every node answers the standard read-only getters `:name?`, `:nick?`, and
`:description?` (see the ma-spec `ma-actor-basics-v1` recommendation). Rooms
override them so an unnamed room reports `Construct` rather than its own
address.

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

A room parent keeps its `children` map authoritative: every `:parent <ctx>`
from a known child replaces that child's stored key with the announced ctx,
and the room broadcasts a `:child-ctx <ctx>` event to every actor present so
clients can refresh exactly that child's entry. Every `:child <ctx>` ack
carries the room's own ctx as the nested `parent-ctx`; after a wholesale
rebuild (e.g. `roll-call end`) that snapshot delivers the complete visible
actor list to every child in one message.

Parenting is placement, not ownership. `:set-parent` and `:hold` may change a
movable actor's parent but MUST NOT change its `owner`; an unowned actor remains
unowned after either operation. For an existing movable actor, `:claim` is the
only ownership-changing verb. `:forge` is the sole creation exception: it
initialises the newly created actor's `owner` and `parent` from `msg.from`, so a
forged entity is born into its forger's hand (its genesis proposes itself to
the forger). Owner-gated actions such as locking a container require a prior
claim and never claim implicitly.

Two general props modulate relocation and visibility on every node:

- `fixed: true` (set/cleared by the owner via `:prop`) — the node cannot be
  relocated at all. `:hold`, `:set-parent`, `:put`, and the item-side `:take`
  relay all refuse with an error; the node's parent cannot change until the
  owner clears the prop. `:recycle` is a destroy, not a move, and remains
  available to the owner. An unowned node has no owner to defend it, so any
  caller may set its props (mirroring the transfer gate's unowned openness);
  once claimed, only the owner may.
- `visible: false` — the node remains a room child (it still receives
  broadcasts and answers direct RPCs such as `:trigger?`) but is never
  published: the room excludes it from `:look`'s children map, from the
  nested parent-ctx snapshots, and from every broadcast (`:arrive`,
  `:leave`, `:child-ctx`, `:dig`, `:say`, `:emote`), so no client ever
  learns of it. The trigger kind defaults to invisible; absent otherwise.

Newly created entities (forge, dig, healed exits) are born with a
kind-prefixed fragment — `<short-kind>-<random>`, e.g. `thing-7b75…`,
`trigger-0cc8…`, `exit-…`, `room-…` — where the short kind is the protocol's
penultimate segment (`/ma/thing/0.0.1` → `thing`). This groups fresh actors in
`@ma/entities` listings; fragments of already existing entities are never
renamed.

An owner may prepare a consent-based transfer with
`:set-recovery-secret <one-time-secret>`. A later
`:claim <one-time-secret>` from an authenticated bare DID atomically sets that
DID as owner and clears the secret; replay therefore fails. Client commands
such as `give <object> to <person>` may distribute that full claim command in
an ordinary text message, but no runtime actor changes ownership merely because
an offer was sent.

## Ctx

A ctx is a plain map, derived fresh from props whenever it is needed — never a
separately stored structure. Every actor publishes its own ctx; children,
rooms, and clients read it.

### Node ctx — thing / agent / container / trigger

Built from a single declaration (`node-ctx-prop-keys` in
`scheme-actor/node.ma`): the identity fields plus every prop-backed attribute.
`register-ctx-props!` consumes the same declaration, so changing any of the
prop-backed fields via `set-prop!` automatically re-announces the ctx to the
parent (`:parent <ctx>`). Adding a new prop-backed attribute to the declaration
is sufficient — there is no second list to keep in sync.

| Field | Type | Source |
| --- | --- | --- |
| `actor` | text (DID-URL) | the node's own address |
| `kind` | text | `thing` / `agent` / `container` / `trigger` / `exit` / `room` |
| `protocol` | text | `/ma/…/0.0.1` |
| `parent` | text (DID-URL) | the proposed or current parent |
| `name` | text | `node-name` (default: actor) |
| `nick` | text | `node-nick` (default: name) |
| `description` | text | `node-description` (default: `"A node."`) |
| `visible` | text `"true"` / `"false"` | `node-visible` (default `"true"`; trigger defaults `"false"`) |
| `fixed` | boolean | `node-fixed?` — `#t` blocks all relocation |
| `here` | text | current room DID-URL, `""` outside a room |

### Exit ctx

The node ctx plus `extend-node-ctx` in `actors/exit.ma`:

| Field | Type | Source |
| --- | --- | --- |
| `direction` | text | the exit's direction word |
| `target-room` | text | destination room DID-URL |
| `target-name` | text | destination display name |

### Room ctx

`room-ctx` in `actors/room.ma`:

| Field | Type | Meaning |
| --- | --- | --- |
| `protocol` | text | `/ma/room/0.0.1` |
| `kind` | text | `"room"` |
| `actor` | text | the room's DID-URL |
| `parent` | text | the root's DID-URL |
| `rev` | number | ctx revision, bumped on every broadcast |
| `name` / `nick` / `description` | text | room identity |
| `children` | map | `actor → child-ctx`, **visible children only** — `visible: false` children are never published |

### h00man / direct-DID ctx

`did-child-ctx` in `actors/room.ma` — a bare DID presence entry:

| Field | Type | Meaning |
| --- | --- | --- |
| `actor`, `did` | text | the bare DID |
| `parent` | text | the room's DID-URL |
| `kind` | text | `"h00man"` |
| `protocol` | text | `/ma/agent/0.0.1` |
| `name`, `nick`, `description` | text | presence identity |
| `rev` | number | presence revision |

### parent-ctx

Every `:child <ctx>` ack embeds the parent's own ctx as the nested
`parent-ctx` field (`child-ack-ctx` in `scheme-actor/node.ma`) — the room ctx
for rooms, the node ctx otherwise. It is a cached snapshot, never a second
child collection.

### Other small shapes

- `did-exit-ctx` — bare-DID traversal payload: `{ did, parent }`.
- `presentation-entry` — legacy projected entry for `who?`/`things?` replies
  when no stored child ctx exists: `actor`, `kind`, `protocol`, `name`, `nick`,
  `description`.

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

## Exit locks

Exits use the same `:lock <secret>` contract as containers, but only the room
that owns the exit may lock it: the room owner sends `:exit <direction> lock
[secret]` and the room forwards the verb to the exit actor as its source room.
The stored secret lets a traveller unlock the exit with
`:unlock <secret>` or pass straight through a locked exit by supplying it as
the second `:traverse` argument (the avatar's `go <direction> <secret>`).
Without the secret, a locked exit narrates the refusal to the traveller as an
in-world `:say` event (the `locked-msg-text` prop, default
`This is not the way.`), replies with the same text, and keeps the traveller
in the source room.

## Replies and events

Use `ma-reply!` for technical results: queries, metadata, configuration,
structured values, validation, and errors. A room's `:look`, `:who?`,
`:exits?`, and `:things?` responses are reply payloads, not print messages.

Use `:print` only for world events. Rooms broadcast arrival, departure, speech,
emotes, ambient narration, movement, and completed transfer narration to present
bare DIDs. A technical bare `:ok` may acknowledge an action, but it is not
narrative output.

## Build

```sh
make clean
make
make check
```

Generated artefacts live in `dist/` and are not source-controlled.