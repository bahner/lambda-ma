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

Every `:enter` carries a movement ctx; there is no bare or argument-free form.

| Receiver | Verb | Result |
| --- | --- | --- |
| room | `:enter <move-ctx>` | Commits presence when `msg.from` equals `ctx.did` and replies with the DID ctx: `did`, `parent`, `name`, `nick`, `description`, `rev`. A delegated form authorises only and never commits. A public room admits directly; a private room requires a `key` equal to a room key, to `pending[did].key` for the same `traverseId`, or to `global_key`. |
| room | `:move <move-ctx>` | Movement. A walk request from a present traveller is forwarded to the passage; the passage's answer removes the traveller and hands it the move ctx; a delegated `:move` (`exit` names this room) issues a key and commits nothing. |
| room | `:dig <ctx>` | Owner-only. Creates a passage and, when the ctx names no `dst`, a new room. Replies `[:ok, { passage, room?, key? }]`. |
| room | `:exit <name> <passage-did-url>` | Owner-only. Binds a local (cosmetic) name to an existing passage in this room's `exits` map. |
| room | `:look`, `:who?`, `:occupants?`, `:things?`, `:exits?` | Direct room presentation. |
| room | `:say`, `:emote` | Broadcasts an in-world event to present DID identities. |
| room | `:narrate <text>` | Owner-only ambient narration. |
| passage | `:move <move-ctx>` | Routes a movement and relays the destination's answer to `entrance`. Only the `entrance` room may start one. |
| passage | `:to?` | The far end relative to the caller, or `"(none)"`. |
| passage | `:locked?`, `:lock [secret]`, `:unlock [secret]` | Lock state. |
| passage | `:message <slot> <text>` | Sets a narration proposal by short slot. |
| root | `:ctx?`, `:register` | Runtime service ctx and subscriber registration. |
| root | `:enter <move-ctx>` | Redirects: replies `[:entrance, <start-room>]`, or `[:deny, reason]` when no start room is configured. Never `[:ok, …]`. |

Every node answers the standard read-only getters `:name?`, `:nick?`, and
`:description?`. Rooms override them so an unnamed room reports `Construct`.

A client accepts entry context only when the reply corresponds to its newest
entry request and originates from the requested room. It also accepts a `:move`
carrying a live `traverseId`, and `[:narrate, { traverseId, text }]` from a
passage in the current room's `exits` map.

Clients leave a room with argument-free `:leave`.

## Actor hierarchy

`/ma/node/0.0.1` provides one persisted `children` map keyed by each node's
DID or full actor DID-URL. Rooms filter that map for bare-DID clients, agents,
things, and triggers. Exits are not children: an exit is a top-level node with
hardcoded `src`/`dst` endpoints, and each room keeps a separate `exits` map of
local name → exit DID-URL published alongside `children` in its ctx.
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
`trigger-0cc8…`, `passage-…`, `room-…` — where the short kind is the protocol's
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

An exit is a top-level node with two hardcoded endpoints. `extend-node-ctx` in
`actors/passage.ma` adds:

| Field | Type | Source |
| --- | --- | --- |
| `src` | text | one endpoint room's DID-URL |
| `dst` | text | the other endpoint room's DID-URL |

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
| `exits` | map | local name → exit DID-URL; exits are not children and are always listed |

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

- `move-ctx` — traversal payload: `did`, `traverseId`, `name`, `nick`, and the
  optional `key`, `source`, `entrance`.
- `pending[did]` — in-memory entry grant: `traverseId`, `key`, `arrive`,
  `oarrive`.
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

A passage is a single top-level node with ends `entrance` and `exit`; each
endpoint room registers the same object in its own `exits` map under its own
local name. One passage is one object, and therefore one lock — never one per
end, never one per player.

Passages use the same `:lock <secret>` contract as containers, addressed
directly to the passage; its owner or its `entrance` room may lock it. A locked
passage answers a move with `deny` at once, and the `entrance` room fires the
`nogo` event. A lock is global: a traveller holding the secret presents it with
`unlock <name> with <secret>`, which sends `:unlock <secret>` to the passage.
The runtime `global_key` also opens any locked passage.

A locked passage is a door, not a wall. It stops this route only.

## Traversal and entry keys

A walk is `:move` to the room, `:move` to the passage, `:move` to the
destination, then `:enter` by the traveller.

```text
traveller → source room    [:move, { traverseId, direction, key? }]
source room → passage      [:move, { did, traverseId, name, nick, key?, entrance }]
passage → destination      [:move, { …, exit }]
destination → passage      [:move, { traverseId, exit, key }]  |  [ … "deny" … ]
passage → source room      [:move, { traverseId, exit, key }]  |  [ … "deny" … ]
source room → traveller    [:move, { traverseId, exit, key }]  |  [ … "deny" … ]
traveller → exit room      [:enter, { did, traverseId, name, nick, key }]
```

`traverseId` is minted by the traveller, is ephemeral, and identifies one
intended movement. Every hop carries it unchanged and ignores anything else.
`key` is an admission credential, opaque to forwarders and read only by the
destination. `entrance` names the room the traveller leaves; `exit` names the
room it enters.

**Pending state.** A destination that accepts a delegated `:move` records
`pending[did] = { traverseId, key, arrive, oarrive }`, in memory only, and the
traveller's `:enter` consumes it. A passage keeps one in-flight record per
`traverseId` so it can match the destination's answer and bound redirects.

**Registration is not admission.** A room's `exits` map decides only what the
room shows. A passage is two-way when both endpoint rooms have registered it;
the far side does that with `:exit <name> <passage-did-url>`.

**Redirects.** An answer whose `exit` names a different room is a redirect. It
must stay on the destination's own runtime, and a traversal follows at most
three.

The break-glass `global_key` is daemon-local: it is set in the runtime's
`ma.yaml`, is never exposed or writable over CRUD, and is injected into the
actor config map that `(ma-get-config-key "global_key")` reads.

## Narration

Presence narration is proposed by the passage and fired by a room as a world
event to all of its children:

| Field | Audience |
| --- | --- |
| `leave` | the traveller, on departure |
| `oleave` | the others in the `entrance` room |
| `arrive` | the traveller, on arrival |
| `oarrive` | the others in the `exit` room |
| `nogo` | the traveller, on refusal |
| `onogo` | the others in the `entrance` room |

Each recipient renders the traveller-facing text when the subject is itself,
otherwise the others-facing text. The passage's text beats the room default, and
the room may override either. Presence narration is one-way.

The props are `leave_msg`, `oleave_msg`, `arrive_msg`, `oarrive_msg`,
`nogo_msg`, and `onogo_msg`, set with `:message <slot> <text>` or
`:prop <key> <text>`.

Passage narration is free-form text describing the journey itself. The passage
sends it directly to the traveller, carrying the movement id. A client accepts
it only from a passage in its current room's `exits` map with a matching live
`traverseId`.

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