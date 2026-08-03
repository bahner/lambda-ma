# lambda-ma Reference

**Version:** 0.1.0
**Status:** Draft

Implementation reference for the shipped lambda-ma actor set.

---

## Scope boundary

This document explains how the shipped lambda-ma actors implement the optional
[lambda-ma world profile](https://github.com/bahner/ma-spec/blob/main/runtime/ma-lambda-ma-v1.md)
on top of ma-runtime. It is not the generic runtime specification and is not a
global requirement for all ma worlds.

- The ma-spec profile is normative for interoperable lambda-ma behaviour.
- This file specifies actor APIs, state keys, bootstrap details, and other
   implementation surfaces for the actor set in this repository.
- Changes to interoperable profile behaviour must keep both documents aligned;
   implementation-only details remain local to this reference.

---

## Table of contents

1. Routing contract
2. Enter contract
3. Ctx and situational maps
4. Authority model
5. Movement and arrival flows
6. Actor interfaces
7. State keys and authority boundaries
8. Wire/value conventions
9. Build and bootstrap quick reference
10. Cross references

---

## 1. Routing contract

Zion focus shorthand is split by leading colon:

- No leading `:`: avatar-mediated command.
- Leading `:`: direct method on focused room/target.

Examples:

- Avatar-mediated: `look`, `say hello`, `go north`, `dig east`.
- Direct: `:help`, `:prop name Garden`, `:thing rms did:ma:...#rms`.

Rules:

1. Actor code MUST preserve this split.
2. Avatar MUST NOT become a proxy layer for colon-prefixed room methods.
3. If a colon command fails in focus mode, fix zion routing or room actor method,
   not avatar proxying.

---

## 2. Enter contract

### 2.1 Room-first target

When a concrete room target is available, enter is room-first.

- Request goes to room actor `:enter`.
- For ordinary client/session entry, the request contains no payload or one nick
   string. The client does not send avatar ctx.
- Room validates the entry intent and ensures the deterministic local avatar.
- If the avatar does not exist yet, room creates it with an explicit
   deterministic fragment; avatar init sends room `:enter` after the avatar is live.
- If the avatar exists, room sends only `:enter-room` to the avatar; the avatar
   sends room `:enter`.
- Room registers avatar entry in room state and sends committed
   `/ma/ctx/avatar/0.0.1` to the avatar. The avatar then persists its room state
   and forwards that ctx to the DID principal.

Compatibility path:

- Root `:enter` still exists and may be used when only runtime target is known.
   Direct avatar entry may include the full inventory DID-URL from the client's
   last accepted avatar ctx so a remote claim/bootstrap entry preserves the
   inventory baton.
- For an existing avatar, root asks the avatar to send its current ctx to the
   DID principal. Root must not send messages to rooms.

### 2.2 Enter payload

Room-first client/session enter accepts either no payload or one nick string.
This is a bodily avatar action intent, not RPC metadata and not client-authored
ctx. The room MUST treat it as a request to enter as the world's default session
kind and send no `:ok` itself. The room creates or finds the avatar; the avatar
performs room entry, receives committed avatar ctx from the room, persists the
committed room, and forwards avatar ctx to the DID principal asynchronously. In
lambda-ma the default session kind is an avatar.

Direct non-avatar occupants identify themselves with a strict situational map:

- `parent`
- `kind` (`agent` or `thing`)
- `protocol`
- `name`
- `nick`
- `description`

Rules:

1. Clients MUST NOT send avatar ctx for session entry; they wait for the avatar
   to send the committed avatar ctx.
2. `ctx.kind = "agent"` or `"thing"` requires all strict direct occupant keys.
3. Empty direct occupant required values MUST be rejected.
4. Additional keys MAY be present and are forward-compatible extension data.

### 2.3 Avatar ctx

Committed avatar situation is delivered as `:ctx` with a flat
`/ma/ctx/avatar/0.0.1` payload. The definition name is not repeated inside the
payload.

All actor references crossing actor, client, or runtime message boundaries MUST
be full DIDs or DID-URLs. Runtime-local `#fragment` shorthand may be
canonicalised while reading legacy state, but MUST NOT be sent in messages or
persisted in ctx fields by avatars, agents, clients, or future ctx consumers.

Required avatar ctx fields:

- `kind` (effective session kind chosen by the world, e.g. `avatar` or `agent`)
- `root`
- `avatar`
- `inv`
- `room`
- `nick`

Avatar ctx includes `avatar`, the actor to receive avatar-mediated DID
principal commands, and `inv`, the avatar's configured inventory container
DID-URL.

### 2.4 Commit behaviour

Client-side focus/avatar ctx commit is acknowledgment-driven.

- Enter send does not imply commit.
- Commit occurs only after valid acknowledgment from expected actor path.

---

## 3. Ctx and situational maps

Ctx is a situational description of an actor or thing. It is not the actor's
private state and not a generic exchange protocol. Lambda-ma currently defines
three ctx shapes: avatar ctx, room ctx, and container ctx.

Actor command flows may carry ordinary situational maps. Those maps describe
where an actor is, what it is called, and how neighbouring actors may present
it, but they are not additional ctx shapes.

### 3.1 Situational map fields

Situational maps commonly contain these fields:

| Key | Type | Meaning |
| --- | --- | --- |
| `parent` | DID-URL | Immediate parent, location, or container actor. |
| `kind` | text | Broad actor category. |
| `protocol` | text | Precise actor behaviour identifier. |

The initial `kind` vocabulary is:

- `thing`
- `agent`
- `avatar`
- `room`
- `container`
- `actor`

`kind` and `protocol` are related but not equivalent. `kind` is a small,
stable, broad category used for initial presentation and local policy.
`protocol` identifies the precise behaviour contract, for example
`/ma/container/0.0.1`.

Recommended presentation hints:

- `name`
- `nick`
- `description`

Presentation hints are self-declared and non-rule-bearing. They let rooms,
parents, and containers populate local presentation without asking the actor
first. They MUST NOT grant authority or encode rule-sensitive state such as hit
points, damage, access rights, or ownership.

Flow-specific attributes MAY be present, including `actor`, `avatar`, `root`,
`room`, `text`, `exit`, and `direction`. Unknown keys MUST NOT cause rejection
by themselves.

### 3.2 Room ctx snapshots

Room ctx is the evented visibility snapshot sent by a room to its present
avatars. It is a full snapshot, not a delta stream. Rooms send a fresh snapshot
whenever visible room state changes, including avatar/agent/thing/exit entry or
removal, display-name changes, and exit topology changes.

Room ctx MUST contain these fields:

| Key | Type | Meaning |
| --- | --- | --- |
| `protocol` | text | The room actor protocol, exactly `/ma/room/0.0.1`. |
| `kind` | text | Exactly `room`. |
| `actor` | DID-URL | The room actor that produced the snapshot. |
| `parent` | DID-URL | The runtime root actor, normally `<runtime DID>#root`. |
| `rev` | integer | Monotonic room-local revision. |
| `name` | text | Room name. |
| `nick` | text | Room display name, normally the same as `name`. |
| `description` | text | Room description. |
| `who` | list | Avatar entries visible in the room. |
| `agents` | list | Agent entries visible in the room. |
| `things` | list | Thing/container entries visible in the room. |
| `exits` | list | Exit entries visible in the room. |

Entries SHOULD include `actor`, `kind`, `protocol`, `name`, `nick`, and
`description` when known. Exit entries SHOULD also include `direction`. Actor
references in room ctx MUST be full DIDs or DID-URLs, never runtime-local
`#fragment` shorthand.

Avatars store only the newest room ctx by `rev`; older or equal revisions are
ignored. `look <name>` is resolved by the avatar from carried inventory first,
then from the stored room ctx. Once resolved, the avatar asks the target actor to
present itself with `:look <viewer-did>`. Rooms do not synchronously resolve
`look <name>` for avatars; bare `look` remains normal room presentation.

Room ctx grants practical lookup and presentation authority only. It is not
authority to mutate the referenced actors; target actors remain responsible for
validating mutating commands.

### 3.2.1 Container ctx

Container ctx is the container kind's single ctx form. It includes both the
container's own presentation fields and its contents cache. Containers send a
fresh container ctx to their current parent with `:parent` whenever that ctx
changes, including parent changes, presentation changes, child admission, child
removal, stale-entry pruning, and explicit reconciliation.

Container ctx MUST contain these fields:

| Key | Type | Meaning |
| --- | --- | --- |
| `protocol` | text | The container actor protocol, exactly `/ma/container/0.0.1`. |
| `kind` | text | Exactly `container`. |
| `actor` | DID-URL | The container actor that produced the snapshot. |
| `parent` | DID-URL | The container's current parent actor. |
| `rev` | integer | Monotonic container-local revision. |
| `name` | text | Container name. |
| `nick` | text | Container display name. |
| `description` | text | Container description. |
| `contents` | map | Child presentation ctx entries keyed by full child DID-URL. |

A parent MAY ignore container ctx. Receiving valid container ctx is not an
obligation to present, index, or recurse into the container. Parents that care,
such as avatars treating one configured container as inventory, store only the
newest revision by `rev`; older or equal revisions are ignored.

Container contents are last-known-good presentation caches built from received
child ctx messages. They are useful for local lookup and display, but
they are not stronger than the child actor's own `parent` state. If a container
contents cache disagrees with a child actor's self-authenticating ctx or own
state, the child actor wins.

All actor references crossing actor, client, or runtime message boundaries MUST
be full DIDs or DID-URLs. Runtime-local `#fragment` shorthand MUST NOT be sent
or persisted in ctx.

### 3.2 Ctx truth and category policy

The actor described by ctx is the only authority that can make that ctx current
for itself.

A ctx announcement is self-authenticating when `msg.from` is the actor described
by the ctx. Receivers may trust that the actor is presenting itself with those
relation and category attributes. A ctx forwarded by any other actor is input to
a decision, not the described actor's current ctx.

`kind` is not capability-bearing by itself. A room, parent, or container MAY use
a self-authenticating `kind` for presentation, but local policy decides what
that category means. Receivers MAY accept, ignore, reinterpret, or reject a ctx
category according to their own rules.

Avatar ctx is authoritative as avatar self-description only when sent by the
avatar actor itself. A bare DID principal sending `kind = "avatar"` is not
equivalent to the deterministic avatar actor sending that ctx.

### 3.3 Actor tree and DID boundary

`/ma/node/0.0.1` is the stateful, opt-in hierarchy base. It extends
`/ma/scheme/state/0.0.1`; pure scheme actors and stateful utility actors do not
join the hierarchy unless their kind extends node. Parenting describes
subordination, not physical containment. A dog may parent a collar, while a
container or room may choose to present its children as contents or occupants.

Only node actors participate in the parent hierarchy. Bare DID principals are
controllers, owners, and authorisation subjects; they are not embodied world
actors and MUST NOT be used as `parent` values. Node child records are keyed by
canonical full actor DID-URL.

Root is the only parentless lambda-ma node. A room's parent is always the
canonical full DID-URL of its runtime root. Rooms MUST reject any other parent,
and non-root nodes MUST reject room ctx as a child. Implementations MUST reject
direct self-parenting. This version does not claim distributed prevention of
longer ancestor cycles.

Embodied user presence is represented by an avatar actor. A user DID may own,
control, or authorise actions, but the avatar is the world body whose ctx can be
admitted by rooms and parents.

### 3.4 Parent change flow

Parent changes are target-accepted and forward-moving:

1. Self has `parent = old_parent`.
2. Self decides to move, or receives an authorised request to move.
3. Self builds ctx with `parent = new_parent` and relevant presentation hints.
4. Self sends `new_parent :parent <ctx>` for admission.
5. `new_parent` rejects, or confirms with `self :child <ctx>`.
6. On acceptance, self commits `parent = new_parent` in its own state/model.
7. Self sends committed ctx to `new_parent`.
8. Self sends a departure or ctx update to `old_parent`.
9. `old_parent` removes the committed ctx it holds for the child. It has no
   ordinary veto after commit.

A parent-mediated transfer MUST retain its child ctx until step 8 reports the
committed new parent. Sending the transfer request is not evidence that it was
delivered or committed, so an implementation MUST NOT eagerly remove the child
or publish a derived view that omits it.

Parent assignment is idempotent. If `new_parent` is already self's committed
parent but sends a valid `:child <ctx>` that differs from self's current
authoritative parent-facing ctx, self commits the confirmed fields and sends its
resulting ctx back with `:parent <ctx>`. If the confirmation already exactly
matches that ctx, self treats the operation as successfully committed and
returns `:ok` without sending another actor message. The actor does not repeat
old-parent cleanup when there was no parent change. A parent still answers every
valid repeated `:parent <ctx>` with `:child <ctx>`, so a lost confirmation can be
repaired while an exact response-to-response exchange terminates.

For ctx kinds with a monotone revision, a confirmation below the actor's current
authoritative revision is a stale acknowledgement. The actor returns `:ok`
without rolling state back, incrementing the revision, or sending another
`:parent`. Delayed confirmations therefore drain instead of starting a new
response-to-response exchange.

This is a consequence of the Hewitt actor delivery model used by lambda-ma.
Actors cannot know whether a previous message or its reply was delivered. A
sender may therefore repeat the same request indefinitely. Receivers MUST treat
valid repeated ctx and state-setting requests as ordinary retries, apply the
same validation, and answer from current authoritative state. If the requested
state is already committed, the receiver replies as though that commit had just
succeeded. It MUST NOT reject the request merely as redundant or assume that a
repeat indicates a protocol error elsewhere.

Implementations MUST NOT make retries reliable by persisting delivery history,
deduplication records, suspended commands, or retry counters. Idempotence comes
from computing the response from current authoritative state. Repeating the
same request therefore produces the same authoritative ctx and can repair a
peer whose earlier request or reply was lost.

Requiring acceptance from both old and new parent is deliberately out of the
ordinary parent-change flow. Protocols MAY define stricter protected flows, but
the base world model is: new parent controls admission, self controls commit,
old parent receives cleanup.

### 3.4.1 Orphan recovery

Root provides a recovery path for movable `thing`, `agent`, and `container`
actors. `room`, `avatar`, and `exit` are never orphanable. Every actor and
parent reference in this flow is a canonical full DID-URL.

- `child-runtime#root :orphan <actor> from <old-parent>` checks whether
   `<actor>` is live in that runtime. For a live actor, root sends it an
   internal request naming the original caller and old parent. The actor itself
   verifies that caller against its persisted owner, then performs the ordinary
   target-accepted parent change to its local root. Root's special authority is
   limited to initiating this request; it never writes a live actor's parent.
- If the actor is unavailable, the named old parent may request a root-signed
   repair ctx. The repair ctx names that actor's own root as its new parent and
   permits the old parent to remove only its matching movable child record.
   This removal is idempotent. It MUST NOT create a root child record or claim
   that the unavailable actor has persisted `parent = #root`.
- An old parent also exposes `:orphan <actor>` for the offline case. Its own
   owner (or deterministic owner avatar) may remove a matching movable child
   record directly, without waiting for the child, its runtime, or its root.
   Repeating an authorised request after removal returns `:ok`.
- Root `:orphans?` returns only its live direct `thing`, `agent`, and
   `container` child ctx records. It is a filtered view of `children`, not a
   separate orphan registry.

### 3.5 Authoritative ctx and derived views

Actor workflows are functional and stateless between messages. An actor handles
the ctx in the current message, sends the next ctx message in the protocol, and
does not persist, queue, or replay the command or an intermediate workflow
payload. In particular, implementations MUST NOT introduce `pending-take`,
pending transfer/drop commands, deferred resolver results, or equivalent saved
workflow state. Readiness is established by actor initialisation and explicit
ctx handshakes, not by storing a user command for later execution.

This does not prohibit authoritative actor state. Ownership, committed
parentage, configuration, and accepted ctx records may be persisted when they
are the durable facts owned by that actor. Such state records what has been
committed; it does not represent a suspended operation.

Stateless does not mean silent on repetition. Every valid retry is handled and
answered from current authoritative state, even when no mutation is needed.

Visible `name`, `nick`, alias, and direction values are resolver inputs and
presentation hints only. Once resolution succeeds, the resolver produces a
canonical full DID or DID-URL and the lookup term is discarded. Subsequent
messages, ctx actor references, authority checks, membership changes, cleanup,
and persistent keys MUST use the resolved DID or DID-URL. Implementations MUST
NOT perform ordinary world mutations against `name` or `nick`.

Parent/child membership, container contents, room occupants, `who`, inventory
views, labels, and similar lookup or presentation surfaces are derived from
authoritative ctx records at use time. Actors must not maintain a second
persistent list of bare actor refs when the list can be generated from ctx; such
lists drift from the authority-bearing data.

Whenever an actor changes any field that appears in its parent-facing ctx, such
as `parent`, `name`, `nick`, or `description`, it sends refreshed ctx to its
current parent with `:parent`. Parents update or remove the stored ctx record;
presentation lists are generated from those ctx records when read. Container
contents are part of container ctx, so contents changes are ordinary container
ctx changes.

A previous parent that receives a self-authenticating ctx where `parent` is no
longer itself MUST treat its local cache entry for that actor as stale.

---

## 4. Authority model

### 4.1 Rooms

- Room ownership is a bare DID in room `owner` prop.
- Avatar is a delegate for display commands, not the owner identity.
- Direct room RPC uses message sender identity (`msg.from`).

### 4.2 Parent and child actors

- A movable actor is authoritative for its own `owner` and `parent`.
- `parent` is the actor's immediate location/container relation.
- Parent changes follow the target-accepted ctx flow in section 3.4.
- Any actor may be both parent and child when it implements the generic
   `:child` parent surface and the movable `:take`/`:drop` child surface.
- Rooms currently present agents and things through one local non-avatar
   occupant cache; the `ctx.kind` remains useful protocol context, not a demand
   for separate room-side policy.

Rules:

1. A parent may request or initiate transfer, but only the child actor commits
   its own parent change.
2. Target parent admission is required before child commit.
3. The old parent receives cleanup after commit and has no ordinary veto.
4. The owner may recover an empty-parent orphan by direct child call, including
   through its local avatar delegate.
5. Current parent and target parent do not perform direct peer negotiation.
6. Visible labels, aliases, nicks, and names are lookup terms only. Persistent
   parent/child/inventory/contents maps are keyed by canonical full actor
   DID-URLs, never by visible aliases.

---

## 5. Movement and arrival flows

### 5.1 DID enter via room

1. Caller sends `:enter` to the target room.
2. Room derives the deterministic avatar fragment from the caller DID.
3. If the avatar already exists, room asks it to enter this room.
4. If the avatar does not exist, room creates it with an explicit deterministic
   fragment; avatar init sends room `:enter` from the live avatar.
5. Room registers entry and sends committed avatar ctx to avatar; avatar
   persists room state and forwards avatar ctx to DID principal.

### 5.2 Room-to-room movement via exit

1. Avatar receives `go <direction>` from its controlling DID.
2. Avatar resolves `<direction>` from cached room ctx and sends a traversal
   command directly to that first-class exit actor.
3. Exit checks source-side state such as lock and target configuration.
4. Exit returns an implementation-local movement result directly to the avatar.
   If traversal is allowed, the result names the target room. If traversal is
   blocked, it names the source room and includes display text.
5. The avatar validates the result, then performs the actual target-room
   `:enter`. Target room admission remains target-room authority.
6. Only the target room's committed avatar ctx updates avatar state and is
   forwarded to the DID principal.

### 5.3 Dig/link to existing room

For named new-room digs such as `dig dør to kjøkken`, the source room derives
deterministic room and exit fragments with `blake3`. The derivation is scoped
by source room, direction, and target name, so repeating the same command in the
same room reuses the same target room and exit instead of creating fresh actors.
Unnamed exploratory digs remain non-deterministic.

New-room digs are two-phase. The source room stores the pending target and a
nonce, creates the room with child-alive metadata in the init payload, and only
creates the exit after receiving `:child-alive <actor> <kind> <nonce> <direction>`
from the expected room. The child-alive metadata is part of the new room's init
payload; after storing it, the room sends the callback to its parent. Actors
without a parent, such as genesis actors, emit no child-alive callback.

For existing-room link targets:

1. Source room stores pending link request.
2. Source room sends `:ping` to target room.
3. After `:pong`, source requests `:authorise-link`.
4. Target room confirms same-owner policy.
5. Source room creates/replaces exit only after explicit authorisation.

---

## 6. Actor interfaces

All terms are CBOR-style actor terms, typically `:verb` or `[":verb", ...]`.

Most Scheme-backed lambda-ma actors inherit identity and metadata methods from
`/ma/scheme/actor/0.0.1`. Node kinds additionally inherit the hierarchy methods
below from `/ma/node/0.0.1`. Kinds may deliberately remove inherited methods;
rooms remove `:name` and `:description` and use `:prop` for metadata changes.

| Verb | Args | Notes |
| --- | --- | --- |
| `:behaviour` | `[ /ipfs/<cid> ]` | No args returns this actor's current per-entity behaviour reference, if any. With one IPFS reference, the caller must match the actor's `owner` prop; on success this queues a reload of this actor's own extra behaviour layer. |
| `:name` / `:description` / `:kind?` | `[text...]` for `:name` and `:description`; none for `:kind?` | Generic metadata inspection. With no args, `:name` returns the `name` prop, `:description` returns the `description` prop, and `:kind?` returns the runtime kind. With args, `:name` and `:description` are owner-gated setters that persist the joined text and return the new prop value. `:kind?` is read-only. |
| `:owner` / `:owner?` | none | Generic prop-based owner inspection. `:owner` returns the raw owner DID or `(none)`. `:owner?` returns display text. Both are dry RPC replies and never send `:print`; avatar/proxy code may present the reply to a user. Kind-specific actors may override this with stricter policy. |
| `:parent` / `:parent?` | `[ctx]` for `:parent`; none for `:parent?` | Node hierarchy methods. A child sends `:parent <ctx>` to request admission or refresh its record. With no ctx, either verb returns the node's current parent. |
| `:child` | `[ctx]` | With no args, lists child ctx according to local policy. With one map, confirms a parent proposal for the receiving node itself. The node validates the named parent and sender, commits its parent, re-announces authoritative ctx, and notifies its previous parent. |
| `:children?` | none | Owner-only node debug query. Returns the complete `children` map keyed by canonical child DID-URL, without kind filtering. |
| `:ctx` | `[map]` | Hidden actor-to-actor situation notification. The generic implementation is a no-op acknowledgement so parents may safely ignore shapes they do not care about. Kind-specific actors override this when they consume room ctx, container ctx, avatar ctx, or implementation-local movement results. |
| `:rpcs?` / `:cmds?` / `:metas?` / `:api?` | none | Generic public API inspection. These introspection verbs are themselves dry public RPCs and appear in `:rpcs?`. `:rpcs?` returns public RPC verbs, `:cmds?` returns avatar and world command verbs, `:metas?` returns explicit relation/repair helpers such as `:parent` or `:child`, and `:api?` returns a grouped view keyed by `:rpcs?`, `:cmds?`, and `:metas?`. Hidden actor-to-actor protocol handlers are not part of this surface. |

### 6.1 root actor

Purpose: deterministic avatar factory.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `[room? nick? inventory?]` | Compatibility path when no concrete room target is available. Creates caller avatar if absent, or asks an existing avatar to send its current ctx to the DID principal. A valid full inventory DID-URL is forwarded to the avatar before it publishes ctx. Root does not message rooms. |
| `:avatar?` | none | Returns caller avatar, creating if needed in the configured start room. |
| `:orphan` | `<actor-did-url> from <old-parent-did-url>` | Recovery request for a local movable actor. A live actor owner-verifies and commits ordinary adoption by root. An unavailable actor can produce only a root-signed old-parent cleanup request; root never forges its persisted parent state. |
| `:orphans?` | none | Returns live direct root child ctx records filtered to `thing`, `agent`, and `container`. |

### 6.2 avatar actor

Purpose: avatar-mediated command endpoint and context emitter.

Key verbs:

| Verb | Args | Caller constraints | Notes |
| --- | --- | --- | --- |
| `:enter-room` | `<room>` | root or target room | Avatar receives this from root or the target room, sends room `:enter`, and waits for committed avatar ctx before persisting room state and forwarding `:ctx` to DID principal. |
| `:ctx` | `<map>` | current room, root, controlling DID, or exit movement result | With no args, returns current avatar ctx to authorised callers. With an implementation-local movement result, validates it, optionally prints its text, then asks the target room to admit the avatar. With room ctx, updates the avatar's room visibility cache. Inventory container ctx is received on `:parent`, not `:ctx`. |
| `:ctx?` | none | DID principal only | Returns context term. |
| `:child` | `[ctx]` | no args: DID principal only; ctx: child actor only | `inventory` storage surface. With one situational map, accepts only sender-matching child data and stores it in the avatar's child cache. With no args, returns the child-cache listing. |
| `:help` | `[topic]` | DID principal only | `help here` asks room `:help`. |
| `:nick` | `[nick]` | DID principal only | No args returns current nick; with args updates avatar nick, emits `:ctx` to the DID principal, and emits `:parent <ctx>` to the current room so the room can update presentation. |
| `:make` | `<kind> <init...>` | DID principal only | Requests a new actor of `kind` using `ma-create-actor` with no behaviour override and all args after `kind` joined as the creation payload. The avatar does not inject owner, parent, or room props; the init text owns initial state. `thing` is accepted as shorthand for `/ma/thing/0.0.1`. Creation is queued; the returned DID-URL may not be live until the runtime loads the entity. |
| `:conjure` | `thing\|container\|agent named <name> [in <parent>]` | DID principal only | Convenience creator for common movable actors. Builds a standard init payload with `name = <name>` and `owner = <controlling did>`. `parent` defaults to the avatar inventory; the optional postfix resolves a carried or room-visible container using the same lookup surface as `look`, or accepts the current room DID-URL. |
| `:owner?` | `[name]` | DID principal or current room | DID calls delegate room ownership inspection to the current room. A room may ask the avatar to present its controlling DID as actor ownership for a named occupant. Avatars do not expose `:owner`; plain `owner` uses default room forwarding. |
| `:did?` / `:dids?` / `:prop` | varies | DID principal only | RPC proxy to the current room for direct metadata lookup or room prop mutation; these are not avatar commands. |
| `:look`/`:l` `:here?` `:exits?` `:who?` `:say` `:emote` `:go` | varies | DID principal only | Avatar-mediated room commands. `here?` reports the avatar's current room DID-URL from its saved context. |
| `:take` | `<thing> [from <parent>]` | DID principal only | Picks something up. If `from` is omitted, the avatar resolves `<thing>` from stored room ctx and asks the child actor directly to move to the inventory container. Explicit `from` may name any parent actor that implements the parent-mediated `:take` contract; carried parent names are resolved through inventory. The avatar must not call room `:take`. Inventory display waits for the child actor's `:child` ctx before showing a DID-URL by name. |
| `:drop` | `<thing>` | DID principal only | Drops a carried actor by resolving `<thing>` from the avatar's inventory cache and starting that actor's existing `:drop` flow. The actor still performs the parent/child ctx handshake: request target parent with `:parent <ctx>`, accept committed `:child <ctx>`, then notify the old parent with `:parent <ctx>`. The avatar must not invent a new verb, call the current room, or use a room-mediated helper; the current room DID-URL is target-parent data only. |
| `:recycle` | `<agent-or-thing>` | DID principal only | Requests permanent removal of an owned carried or visible actor. Carried actors are routed through the inventory container; visible actors are routed through the current room. The parent resolves the lookup term to a canonical actor DID-URL before asking the child to end itself. |
| `:claim` `:dig` `:fill` | varies | DID principal only | Delegates to room without owner-authority arguments; rooms recognise the owner DID from the authoritative avatar child ctx or verify the deterministic owner avatar from `msg-from`. `:owner` is mediated by the avatar's default room forwarding; it is not an avatar method and does not mutate avatar ownership. |
| `:report-parent` | `<room> <tick> <nonce>` | room caller | Machine presence request; replies with `:parent-report <self> <room> <tick> <nonce>` using the avatar's persisted room. |

Avatar ctx carries `inv` as the movement/entry baton. A target-runtime
avatar adopts a supplied inventory container reference when present, and creates
or reuses a deterministic local `/ma/container/0.0.1` only when no inventory has
been configured yet. Carried actors are parented to that inventory container,
not directly to the avatar. `take` asks the current source parent to move the
child to the inventory container. For visible room contents, source lookup comes
from stored room ctx and the avatar asks the child actor directly; rooms do not
implement `:take`. Carried `drop` resolves the child from the avatar's inventory
cache and starts the child's existing `:drop` flow; the actual movement remains
the parent/child ctx handshake described in the transfer model. The avatar's
local inventory display is a last-known-good cache from that container's
container ctx.

For carried `drop`, the avatar must not invent a new drop helper, call the
current room's `:drop`, or use a room-mediated helper. A room DID-URL in the
request is only the target parent value.

### 6.3 room actor

Purpose: local room policy, exits, ownership, occupant presentation.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `<ctx-map>` | Room-first enter endpoint. Avatar ctx maps include full DID/DID-URL actor references and a `did` DID; target rooms create or reuse that DID principal's deterministic local avatar. `agent`/`thing` require ctx required keys. |
| `:enter` | `<avatar-did-url> [old-room-did-url]` | Admit known avatar flow. |
| `:leave` | none | Caller-origin live-presence departure. Removes the caller's local deterministic avatar from this room, but does not change avatar state or client ctx; the saved room remains the next-login return point. |
| `:remove` | `<occupant>` | Owner-gated manual presence cleanup. Resolves an occupant by DID/DID-URL or by a unique current display label; ambiguous labels are rejected. Removes the room's stored ctx claim for that occupant and does not change actor state. The occupant may re-enter later through normal `:enter` flow. |
| `:leave-occupant` | none | Sender-origin ctx-claim removal for non-avatar occupants such as agents after actor-owned parent changes. |
| `:look` | `[exit-direction]` | No args prints room text plus `Occupants:`, `Things:`, and `Exits:`. With an exit direction, forwards inspection to the first-class exit actor. |
| `:exits?` `:who?` `:occupants?` `:things?` | none | Local presentation. `exits?` lists directions known to the room; `who?` includes only avatars; `occupants?` includes avatars and agents; `things?` includes things and containers. |
| `:did?` | `[exit\|thing\|occupant] <name>` | Explicit visible reference lookup. Resolves a visible exit direction, room-local thing alias, or occupant display label to a DID/DID-URL. Without a kind, one unambiguous match is returned; ambiguous names list every matching kind. |
| `:owner?` | `[name]` | With no args, shows room ownership. With a visible exit, thing, or occupant name, resolves the target like `:did?` and asks that actor to print its owner to the requester. Direct calls complete with an RPC reply. Avatar-mediated calls also send presentation to the avatar because the room's RPC reply terminates at the avatar and cannot complete the user's earlier RPC. |
| `:dids?` | none | Owner-gated full reference listing for visible occupants, room-local things, and exits. |
| `:go` / `:move` | `<direction>` / none | `:go` sends avatar ctx through the named exit policy and then asks the selected target room to `:enter`. `:move` chooses one currently available room exit for the caller. |
| `:thing` | `<name> [did-or-empty]` | Owner-gated child ctx lookup/update by presentation name; it does not maintain an alias map. |
| `:recycle` / `:where?` | `[DID principal?] [token]` | Room-local movable actor utilities. `:recycle` is hard removal: the room resolves the visible token or DID-URL, asks the child actor as current parent, and the child calls `ma-end` only after validating the owner DID. Rooms do not expose `:take` or `:drop`; avatar pickup/drop uses room ctx for lookup and then talks to the child or inventory container directly. |
| `:orphan` | `<actor-did-url>` | Owner-gated offline recovery. Idempotently removes only a matching `thing`, `agent`, or `container` child ctx when that child's runtime cannot participate. |
| `:claim` / `:owner` / `:prop` | direct args | Room ownership controls write operations; owner authority is checked against `msg-from` or the deterministic owner avatar. |
| `:dig` | direct args | Owner-gated exit creation/linking; newly-created rooms are assigned to the stored owner DID. |
| `:fill` | direct args | Owner-gated exit removal. Removes the direction from the room and asks the exit actor to terminate itself; target rooms are not deleted. |
| `:exit` | `<direction> <verb> [args]` | Owner-gated direction resolver and generic forwarder to the exit actor. Exit semantics live in `exit.ma`. |

Room callbacks such as `:child-alive`, `:ping`, `:pong`, `:authorise-link`,
`:link-authorised`, `:link-denied`, `:presence-tick`, `:parent-report`,
`:parent`, and `:leave-occupant` are internal protocol handlers. They are
not part of `:api?`.

Room presence rules:

1. Room membership is stored only in the node `children` map. `occupants`,
   broadcast recipients, and room ctx presentation are derived from valid child
   ctx. `occupants` filters by `kind=avatar|agent`, `who` filters by
   `kind=avatar`, and things filter by `kind=thing|container`. There is no
   separate persisted room-local membership, alias, or label list.
2. DID principal-facing `:leave` removes only live room presence. It deliberately leaves
   avatar state and zion `.my.ctx.*` unchanged so the remembered room remains the
   return point on a later login.
3. Owner-facing `:remove <occupant>` is manual cleanup only. It removes the
   room's stored ctx claim, not actor state or future admission rights. If multiple
   occupants share the same nick/display label, the owner must use a DID or
   DID-URL to identify the target.
4. `:did? [exit|thing|occupant] <name>` is an explicit lookup for a visible
   exit, occupant, or thing. Without a kind, one unambiguous match is returned;
   ambiguous names list every matching kind.
5. `:dids?` is owner-gated and lists all visible occupant, thing, and exit
   references for administrative disambiguation.
6. On lifecycle `:start`, a room registers a `#scheduler` interval for
   `:presence-tick`.
7. `#scheduler` later sends `:presence-tick` to the room as an ordinary
   message. On each tick, the room sends `:report-parent <room> <tick> <nonce>` to
   current occupants.
8. Avatars report their current `room`; agents and things report their current
   `parent`.
9. If a child reports a parent other than the room, the room removes that child
   from local occupant caches immediately.
10. If a child does not report for the configured timeout, the room removes it
   from local occupant caches. The child may re-enter later through normal
   `:enter` flow.

### 6.4 exit actor

Purpose: first-class inspectable traversal object.

| Verb | Args | Notes |
| --- | --- | --- |
| `:about` | none | Returns name, description, owner, source room, target room, direction, and locked state. |
| `:where?` | none | Returns the source room DID-URL. |
| `:owner` | none | Returns the owner DID or `(none)`. |
| `:report-parent` | `<room> <tick> <nonce>` | Machine presence request; replies to the requesting room with `:parent-report <self> <parent> <tick> <nonce>`. |
| `:locked?` | none | Returns `true` or `false`. |
| `:lock` / `:unlock` | none | Source-room-only mutation. Avatar/DID principal `lock <direction>` and direct room `:exit <direction> :lock` resolve through the source room. |
| `:message` | `traveller`, `source`, `target`, or `blocked`, plus `text` | Source-room-only travel-message update. Direct room `:exit <direction> :message ...` resolves through the source room; the exit actor keeps canonical message state. |

Exit travel uses avatar ctx passing, not a separate movement protocol. The
moving avatar sends its avatar ctx map to the exit's internal `:ctx` handler;
the exit applies local policy, annotates that same avatar ctx with the selected
`room`, optional display `text`, `exit`, and `direction`, then returns it to the
avatar with `:ctx`. The avatar asks the target room to `:enter` with that ctx;
the target room owns admission and committed avatar ctx.

The exit actor is the canonical store for first-class exit metadata and policy.
The room stores topology only: direction to exit actor, and the target room used
to recreate a local exit if the local actor is missing.

### 6.5 Scheme agent parent kind

Kind: `/ma/scheme/agent/0.0.1`, extending `/ma/node/0.0.1`.

Purpose: reusable autonomous Scheme-agent base behaviour. Concrete agents extend
this kind and inherit node parenting plus agent movement and transfer policy.

Key helpers and verbs:

| Verb/helper | Args | Notes |
| --- | --- | --- |
| `:child` / `:parent` | `[ctx]` | Inherited node ctx-parenting. Agents announce current ctx on lifecycle `:start`, after an accepted parent change, and after committed room movement. Owner recovery is limited to an empty-parent orphan. |
| `:about` `:where?` `:owner` | none | Generic state summary. |
| `:exits?` | none | Asks the current parent room for exits and stores the printed reply as `last-message`. |
| `:go` | `<direction>` | Free-agent or owner movement through a named room exit; no exit creation. |
| `:move` | none | Asks the current parent room to choose one available exit at random and traverse it. |
| `:ctx` | `<map>` | Room-facing movement helper; validates the movement result against the current parent, performs ordinary room-visible `:leave-occupant`, then sends map-shaped agent `:enter` to the target room. With no args, returns current situation to authorised callers. |
| `:enter-room` | `<target-room-did-url> <source-room-did-url>` | Root/room movement helper retained for direct room-driven entry flows. |
| `:report-parent` | `<room> <tick> <nonce>` | Machine presence request; replies to the requesting room with `:parent-report <self> <parent> <tick> <nonce>`. |
| `:claim` | `[secret]` | Ownership claim. If owner is empty and no recovery secret is set, `:claim` with no args claims the thing directly. If a recovery secret exists, the caller must provide the matching secret. |
| `:take` | `<did> <carrier-parent> [ctx]` | Pick-up request. Caller must be current parent; the agent still commits its own parent only after target admission. Owner may recover an empty-parent orphan. |
| `:drop` | `<did> <target-parent> [ctx]` | Drop request. Caller must be current parent; the agent still commits its own parent only after target admission. Owner may recover an empty-parent orphan. |

### 6.6 rms actor

Kind: `/ma/scheme/agent/0.0.1` with rms-specific per-entity behaviour.

Purpose: concrete fortune agent. rms is not a reusable kind; it is one
entity using the generic Scheme-agent parent behaviour plus `actors/rms.ma`.
Its creation-time init code sets defaults (`name = Richard Stallman`,
`nick = rms`), calls `enter` for `#construct`, commits `parent` when the room
acks with `:ctx`, and then registers a caller-owned `#scheduler` job named
`fortune` with `:random 60`. Reloading the actor replaces the same schedule
instead of stacking duplicate jobs.

| Verb | Args | Notes |
| --- | --- | --- |
| `:help` | none | rms command summary and schedule note. |
| `:fortune` | none | Sends `:say <fortune>` to current parent room. |

### 6.7 thing actor

Purpose: movable passive object with owner/parent authority.

| Verb | Args | Notes |
| --- | --- | --- |
| `:about` | none | Name, description, owner, parent summary. |
| `:where?` | none | Current parent. |
| `:owner` | none | Current owner. |
| `:child` / `:parent` | `[ctx]` | Inherited node ctx-parenting. Things announce current ctx on lifecycle `:start` and after accepted `:take`/`:drop` parent changes. |
| `:prop` | `<name\|nick\|description> [value]` | Owner only. Sets an editable presentation prop; no value resets the prop to the kind default. Does not edit `owner` or `parent`. |
| `:set-recovery-secret` | `[text]` | Owner only. |
| `:claim` | `<secret>` | Recovery-path ownership claim. |
| `:report-parent` | `<room> <tick> <nonce>` | Machine presence request; replies to the requesting room with `:parent-report <self> <parent> <tick> <nonce>`. |
| `:take` | `<did> <carrier-parent> [ctx]` | Pick-up request. Caller must be current parent; the thing still commits its own parent only after target admission. Owner may recover only an empty-parent orphan. Optional ctx map may contribute presentation hints. |
| `:drop` | `<did> <target-parent> [ctx]` | Drop request. Caller must be current parent; the thing still commits its own parent only after target admission. Owner may recover only an empty-parent orphan. Optional ctx map may contribute presentation hints. |

### 6.8 container actor

Kind: `/ma/container/0.0.1`.

Purpose: movable passive object that can hold other movable actor situation maps. A
container is a parent actor with `kind=container` and protocol
`/ma/container/0.0.1`; whole-container movement follows the same target-accepted
parent-change flow as things and agents.

The v1 contents model deliberately trusts supplied ctx while the container is
unlocked. Room locality and parent graph proof are not enforced by the container
yet. The active object policy is the lock: locked containers reject contents
listing, putting in, and taking out.

| Verb | Args | Notes |
| --- | --- | --- |
| `:about` | none | Name, description, owner, parent, and locked summary. |
| `:look` | `[target]` | Presentation form for looking at or into the container. With a delegated target from a local actor, prints to that target; otherwise replies with the same text. |
| `:contents?` | none | Lists stored contents when unlocked. If locked, replies with the lock message as an error. |
| `:put-in` | `<ctx-map>` | Admission request for a child ctx when unlocked. Accepted children still become current only when the child self commits `parent` to this container. |
| `:take-from` | `<child>` | Container-content extraction. Removes a child by DID-URL or label and returns the stored ctx when unlocked. Does not pick up or move the child actor itself. |
| `:lock` | `[message]` | Owner or unowned caller. Sets `locked=true`; with text, also updates `locked-message`. |
| `:unlock` | none | Owner or unowned caller. Sets `locked=false` and keeps `locked-message`. |
| `:take` | `<did> <carrier-parent> [ctx]` | Pick-up request for the whole container. Caller must be current parent; the container still commits its own parent only after target admission. Owner may recover only an empty-parent orphan. If the first argument after `<did>` resolves to a stored child, this is the inherited parent-cache transfer helper; an optional `:drop` hint dispatches `:drop` to that child instead of `:take`. |
| `:drop` | `<did> <target-parent> [ctx]` | Drop request for the whole container. Caller must be current parent; the container still commits its own parent only after target admission. Owner may recover only an empty-parent orphan. |
| `:claim` | `[secret]` | Ownership claim. If owner is empty and no recovery secret is set, `:claim` with no args claims the container directly. If a recovery secret exists, the caller must provide the matching secret. |
| `:orphan` | `<actor-did-url>` | Owner-gated offline recovery. Idempotently removes a matching movable child and sends refreshed container ctx to this container's parent. |
| `:child` | `[ctx]` | Node ctx-parenting with container lock policy. No args presents the same `children` map as contents; with ctx, admits or confirms through the node handshake. |

After any contents mutation, the container sends refreshed container ctx to its
current parent with `:parent`. This is a parent notification only; parents that
do not care about container contents may acknowledge and ignore it. An avatar
may designate one container actor as its inventory and use that container ctx as
a local last-known-good inventory cache.

Carried drops rely on the normal parent/child ctx algorithm. The inventory
container is updated from the carried actor's committed ctx notification; do not
add a separate inventory drop verb.

Claim routing note:

- `claim` in avatar focus mode targets the current room.
- To claim a non-room actor by DID-URL, send direct actor RPC such as `@runtime#thing:claim`.
- Focus shorthand may target an actor explicitly with `claim did:ma:...#thing:`; any following argument is passed as the claim secret.

---

## 7. State keys and authority boundaries

The keys below hold durable authoritative facts or explicitly documented
derived operational metadata. They MUST NOT be extended with saved commands,
pending transfer payloads, resolver results, or other suspended workflow state.
Actor-to-actor workflows pass ctx in messages and progress through protocol
handshakes instead.

### 7.1 root keys

Root stores no avatar registry. Avatar DID-URLs are derived from the caller DID
by the root/room/avatar actor code with `blake3`, scoped by runtime DID and
caller DID. Runtime validates requested fragments but does not derive these
world-level actor names.

### 7.2 room keys

| Key | Type | Authority |
| --- | --- | --- |
| `owner` | bare DID | authoritative room ownership |
| `name`, `description` | string | authoritative room metadata |
| `children` | map | canonical child DID-URL to accepted child ctx; sole source for occupants, `who`, things, lookup, and broadcast recipients |
| `exits` | map | direction to exit actor DID-URL |
| `exit-target:<direction>` | DID-URL | target room used for local exit healing |
| `exit-target-name:<direction>` | string | optional deterministic new-room target name |
| `presence:tick` | integer | room-local scheduled presence counter |
| `presence:last-report:<actor>` | integer | last tick where actor reported this room as parent |
| `presence:last-request:<actor>` | integer | last tick where room requested parent report |
| `presence:nonce:<actor>` | string | pending parent-report nonce |
| `presence:last-parent:<actor>` | string | last parent reported by actor, for debugging |

### 7.3 avatar keys

| Key | Meaning |
| --- | --- |
| `did` | controlling bare DID |
| `room` | current room DID-URL |
| `nick` | current display nick |

### 7.4 thing/agent keys

| Key | Meaning |
| --- | --- |
| `owner` | owner DID |
| `parent` | current parent DID-URL |
| `name`, `description` | display metadata |
| `recovery-secret` | optional recovery claim secret |

Containers use the same movable keys, plus:

| Key | Meaning |
| --- | --- |
| `locked` | string boolean, `true` when contents are closed |
| `locked-message` | reusable message returned while locked |
| `children` | contents map keyed by child DID-URL, with ctx-map values |

---

## 8. Wire/value conventions

1. Actor message payloads are ma-scheme terms serialised through runtime RPC.
2. Verb dispatch follows `:verb` or tuple/list forms with `:verb` head.
3. Situational maps commonly use `parent`, `kind`, and `protocol` fields.
   Recommended presentation hints are `name`, `nick`, and `description`.
4. Entry is command traffic. Client/session entry sends no avatar ctx and waits
   for committed avatar ctx. Direct `agent`/`thing` entry MUST carry required
   string fields listed in section 2.2.
5. DID values crossing actor, zion, or runtime message boundaries MUST be full
   DID or DID-URL values, not runtime-local shorthand. Committed ctx actor
   references MUST be full DID-URLs.
6. Committed avatar ctx is the flat `/ma/ctx/avatar/0.0.1` payload, includes
   `inv`, and is the only ctx shape zion uses for focus.
7. Room `:enter` dispatch is kind-driven: absent kind or explicit `avatar`
   ensures the caller's deterministic avatar locally and sends no `:ok`; `thing`
   and `agent` require explicit kind and are admitted into the same room-local
   non-avatar occupant cache for now.
8. Thing transfer validation is strict by default: DID principal MUST be `did:ma:...`;
   non-ctx parent arguments MUST be DID-URLs. Optional
   transfer ctx MUST include non-empty `parent`, `kind`, and `protocol` fields.
   Any actor references inside ctx MUST be full DID-URLs.

---

## 9. Build and bootstrap quick reference

Build world artifacts:

```sh
make clean
make
make check
```

Generate reusable root CID:

```sh
make root-cid
```

Typical zion wiring after runtime is up:

```text
.ma!discover
@ma/config/root: @ma#root
.enter @ma
```

---

## 10. Cross references

- Project overview and developer workflow: README.md
- First-run bootstrap walkthrough: HOWTO.md
- Actor protocol detail source: actors/README.md
- Focus routing guardrail for agents: AGENTS.md
