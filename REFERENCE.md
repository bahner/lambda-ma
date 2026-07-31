# lambda-ma Reference

**Version:** 0.1.0
**Status:** Draft

Canonical reference for lambda-ma world protocol behaviour.

---

## Scope boundary

This document defines lambda-ma world/profile behaviour on top of ma-runtime.
It is not the generic runtime specification and is not a global requirement for
all ma worlds.

- Runtime/base protocol specs live in ma-spec.
- This file defines world semantics for the shipped lambda-ma actor set.

---

## Table of contents

1. Routing contract
2. Enter contract
3. Authority model
4. Movement and arrival flows
5. Actor interfaces
6. State keys and authority boundaries
7. Wire/value conventions
8. Build and bootstrap quick reference
9. Cross references

---

## 1. Routing contract

Zion focus shorthand is split by leading colon:

- No leading `:`: avatar-mediated avatar-mediated command.
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
- Room validates request context and ensures the deterministic local avatar.
- If the avatar does not exist yet, room creates it with an explicit
   deterministic fragment; avatar init sends room `:enter` after the avatar is live.
- If the avatar exists, room sends only `:enter-room` to the avatar; the avatar
   sends room `:enter`.
- Room registers avatar entry in room state and sends committed
   `/ma/lambda/ctx/0.0.1` to the avatar. The avatar then persists its room state
   and forwards that ctx to the DID principal.

Compatibility path:

- Root `:enter` still exists and may be used when only runtime target is known.
- For an existing avatar, root asks the avatar to send its current ctx to the
   DID principal. Root must not send messages to rooms.

### 2.2 Enter payload

Room-first enter accepts either no payload or one extensible map named `ctx`.
The payload concept is `ctx` only. Do not introduce a parallel `attrs` map.

For client entry, `ctx.kind` MAY be absent. Absence means the client is not
claiming to be a concrete world object kind yet; the room MUST treat this as a
request to enter as the world's default session kind and send no `:ok` itself.
The room creates or finds the avatar; the avatar performs room entry, receives
committed `/ma/lambda/ctx/0.0.1` context from the room, persists that state, and
forwards the ctx to the DID principal asynchronously. In lambda-ma the default session
kind is an avatar.

Direct non-avatar occupants MUST identify themselves with a strict `ctx` map:

- `kind` (`agent` or `thing`)
- `name`
- `nick`
- `description`

Rules:

1. Missing `ctx.kind` means client/session entry. The client should wait for the
   world to send the committed context; it must not assume the effective kind or
   avatar actor from the enter request alone.
2. `ctx.kind = "agent"` or `"thing"` requires all strict direct occupant keys.
3. Empty direct occupant required values MUST be rejected.
4. Additional keys MAY be present and are forward-compatible extension data.
5. A client that wants a direct thing/agent entry MUST send `ctx.kind`; without
   it, the world assigns the default session kind and reports it in the committed
   context.

### 2.3 Context protocol

Committed lambda-ma context is delivered as `:ctx` with protocol
`/ma/lambda/ctx/0.0.1`.

All actor references crossing actor, client, or runtime message boundaries MUST
be full DIDs or DID-URLs. Runtime-local `#fragment` shorthand may be
canonicalised while reading legacy state, but MUST NOT be sent in messages or
persisted in ctx fields by avatars, agents, clients, or future ctx consumers.

Required context fields:

- `protocol` = `/ma/lambda/ctx/0.0.1`
- `kind` (effective session kind chosen by the world, e.g. `avatar` or `agent`)
- `root`
- `room`
- `nick`

Avatar contexts include `avatar`, the actor to receive avatar-mediated DID principal
commands. Direct agent contexts may set `avatar` to the empty string.

### 2.4 Commit behaviour

Client-side focus/context commit is acknowledgment-driven.

- Enter send does not imply commit.
- Commit occurs only after valid acknowledgment from expected actor path.

---

## 3. Authority model

### 3.1 Rooms

- Room ownership is a bare DID in room `owner` prop.
- Avatar is a delegate for display commands, not the owner identity.
- Direct room RPC uses message sender identity (`msg.from`).

### 3.2 Things and agents

- A movable entity is authoritative for its own `owner` and `parent`.
- `parent` is location/container authority.
- Parent-mediated transfer model applies to `take` and `drop`.
- Rooms currently present agents and things through one local non-avatar
   occupant cache; the `ctx.kind` remains useful protocol context, not a demand
   for separate room-side policy.

Rules:

1. Current parent is the caller that may request transfer.
2. Target parent is payload data for new parent assignment.
3. Current parent and target parent do not perform direct peer negotiation.

---

## 4. Movement and arrival flows

### 4.1 DID enter via room ctx

1. Caller sends `:enter <ctx>` to the target room.
2. Room derives the deterministic avatar fragment from the caller DID.
3. If the avatar already exists, room asks it to enter this room.
4. If the avatar does not exist, room creates it with an explicit deterministic
   fragment; avatar init sends room `:enter` from the live avatar.
5. Room registers entry and sends committed `:ctx` to avatar; avatar persists
   room state and forwards the ctx to DID principal.

### 4.2 Room-to-room movement via exit

1. Avatar sends `:go <direction>` to current room.
2. Room resolves `<direction>` to a first-class exit actor and sends
   `:traverse <ctx>` to that exit. Minimal movement ctx uses `actor`, `kind`,
   `room`, and optional `nick`/`text`.
3. Exit checks source-side state such as lock and target configuration.
4. Exit replies to the source room with `:traversed <ctx>`. If traversal is
   allowed, `ctx.room` is the target room. If traversal is blocked, `ctx.room`
   remains the source room and `ctx.text` explains why.
5. Source room sends `:ctx <ctx>` to `ctx.actor`; it does not commit
   movement state or own departure side effects.
6. The actor validates the ctx, then performs the actual target-room
   `:enter <ctx>`. Target room admission remains target-room authority.
7. Only the target room's committed `:ctx` updates actor/avatar state and is
   forwarded to the DID principal.

### 4.3 Dig/link to existing room

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

## 5. Actor interfaces

All terms are CBOR-style actor terms, typically `:verb` or `[":verb", ...]`.

All Scheme-backed lambda-ma actors inherit the generic actor method below from
`/ma/scheme/actor/0.0.1` before their kind-specific behaviour is loaded.

| Verb | Args | Notes |
| --- | --- | --- |
| `:behaviour` | `[ /ipfs/<cid> ]` | No args returns this actor's current per-entity behaviour reference, if any. With one IPFS reference, the caller must match the actor's `owner` prop; on success this queues a reload of this actor's own extra behaviour layer. |
| `:name` / `:description` / `:kind` | `[text...]` for `:name` and `:description`; none for `:kind` | Generic metadata inspection. With no args, `:name` returns the `name` prop, `:description` returns the `description` prop, and `:kind` returns the runtime kind. With args, `:name` and `:description` are owner-gated setters that persist the joined text and return the new prop value. `:kind` is read-only. |
| `:owner` / `:owner?` | `[printer]` | Generic prop-based owner inspection. `:owner` returns the raw owner DID or `(none)`. `:owner?` returns display text, or sends it to `printer` via `:print` when a target is supplied. Kind-specific actors may override this with stricter policy. |
| `:parent` / `:parent?` | none, or kind-specific `[ctx]` | Generic prop-based parent inspection. `:parent` returns the raw parent actor or `(none)`; `:parent?` returns display text. Movable actors override `:parent <ctx>` as a current-parent-gated parent push: the current parent sends a ctx map naming the new parent in `parent` (or `room` as a movement fallback), the child updates its authoritative parent state, then announces its own ctx to the new parent with `:children <ctx>`. The received ctx is intentionally accepted broadly for now; filtering can be tightened later. |
| `:children` | `[ctx]` | With no args, owner-gated child-cache listing. With one ctx map, registers or updates a child cache entry when `ctx.actor` is a full DID-URL, `ctx` is an actor ctx, and `msg-from` matches `ctx.actor`. The child cache is derived state; the child's own `parent` prop remains authoritative. |
| `:where` / `:where?` / `:here` / `:here?` | none | Aliases for generic parent inspection, so every actor has a minimal location answer even before kind-specific behaviour is loaded. |

### 5.1 root actor

Purpose: deterministic avatar factory.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `[room? nick?]` | Compatibility path when no concrete room target is available. Creates caller avatar if absent, or asks an existing avatar to send its current ctx to the DID principal. Root does not message rooms. |
| `:avatar?` | none | Returns caller avatar, creating if needed in the configured start room. |

### 5.2 avatar actor

Purpose: avatar-mediated command endpoint and context emitter.

Key verbs:

| Verb | Args | Caller constraints | Notes |
| --- | --- | --- | --- |
| `:enter-room` | `<room>` | root or target room | Avatar receives this from root or the target room, sends room `:enter`, and waits for committed room ctx before persisting room state and forwarding `:ctx` to DID principal. |
| `:sync-ctx` | none | root only | Emits current `:ctx` to DID principal without changing avatar state. |
| `:ctx` | `<ctx-map>` | current room, root, or controlling DID | With no args, returns current context to authorised callers. With a movement ctx-map, validates ordinary movement ctx, optionally prints `ctx.text`, then asks `ctx.room` to admit the avatar. The avatar forwards no new `:ctx` to the DID principal until the target room commits one. |
| `:ctx?` | none | DID principal only | Returns context term. |
| `:children` | `[ctx]` | no args: DID principal only; ctx: child actor only | `inventory` storage surface. With child ctx, accepts only sender-matching actor ctx and stores it in the avatar's child cache. With no args, returns the child-cache listing. |
| `:help` | `[topic]` | DID principal only | `help here` asks room `:help`. |
| `:nick` | `[nick]` | DID principal only | No args returns current nick; with args forwards to room. |
| `:make` | `<kind> <init...>` | DID principal only | Requests a new actor of `kind` using `ma-create-actor` with no behaviour override and all args after `kind` joined as the creation payload. The avatar does not inject owner, parent, or room props; the init text owns initial state. `thing` is accepted as shorthand for `/ma/thing/0.0.1`. Creation is queued; the returned DID-URL may not be live until the runtime loads the entity. |
| `:look`/`:l` `:exits?` `:who?` `:did?` `:owner?` `:say` `:emote` `:go` | varies | DID principal only | Delegates to room. |
| `:claim` `:owner` `:dig` `:fill` `:prop` | varies | DID principal only | Delegates to room without owner-authority arguments; rooms recognise the owner DID or deterministic owner avatar from `msg-from`. |
| `:drop-thing` | `<did> <thing> <target-parent> [token] [ctx]` | room caller only | Parent-mediated drop helper; forwards optional DID principal ctx map. |
| `:report-parent` | `<room> <tick> <nonce>` | room caller | Machine presence request; replies with `:parent-report <self> <room> <tick> <nonce>` using the avatar's persisted room. |

### 5.3 room actor

Purpose: local room policy, exits, ownership, occupant presentation.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `<ctx-map>` | Room-first enter endpoint. Avatar ctx maps include full DID/DID-URL actor references and a `did` DID; target rooms create or reuse that DID principal's deterministic local avatar. `agent`/`thing` require ctx required keys. |
| `:enter` | `<avatar-did-url> [old-room-did-url]` | Admit known avatar flow. |
| `:leave` | none | Caller-origin live-presence departure. Removes the caller's local deterministic avatar from this room, but does not change avatar state or client ctx; the saved room remains the next-login return point. |
| `:remove` | `<occupant>` | Owner-gated manual presence cleanup. Resolves an occupant by DID/DID-URL or by a unique current display label; ambiguous labels are rejected. Removes the occupant from room-local presence caches and does not change actor state. The occupant may re-enter later through normal `:enter` flow. |
| `:leave-occupant` | none | Sender-origin cache removal for non-avatar occupants such as agents after actor-owned parent changes. |
| `:look` | `[exit-direction]` | No args prints room text plus `Occupants:`, `Things:`, and `Exits:`. With an exit direction, forwards inspection to the first-class exit actor. |
| `:exits?` `:who?` `:occupants?` `:things?` | none | Local presentation. `exits?` lists directions known to the room; `who?` is people/avatar-oriented; `occupants?` includes avatars plus room-local agents/occupants. |
| `:did?` | `[exit\|thing\|occupant] <name>` | Explicit visible reference lookup. Resolves a visible exit direction, room-local thing alias, or occupant display label to a DID/DID-URL. Without a kind, one unambiguous match is returned; ambiguous names list every matching kind. |
| `:owner?` | `[name]` | With no args, shows room ownership. With a visible exit, thing, or occupant name, resolves the target like `:did?` and asks that actor to print its owner to the requester. |
| `:dids?` | none | Owner-gated full reference listing for visible occupants, room-local things, and exits. |
| `:go` / `:move` | `<direction>` / none | `:go` traverses a named exit. `:move` chooses one currently available room exit for the caller. |
| `:thing` | `<name> [did-or-empty]` | Local occupant alias list/get/set/delete; owner-gated for write. |
| `:take` / `:drop` / `:where?` | `[DID principal?] [token]` | Uses movable actor parent-authority contract. |
| `:claim` / `:owner` / `:prop` | direct args | Room ownership controls write operations; owner authority is checked against `msg-from` or the deterministic owner avatar. |
| `:dig` | direct args | Owner-gated exit creation/linking; newly-created rooms are assigned to the stored owner DID. |
| `:fill` | direct args | Owner-gated exit removal. Removes the direction from the room and asks the exit actor to terminate itself; target rooms are not deleted. |
| `:exit` | `<direction> <verb> [args]` | Owner-gated direction resolver and generic forwarder to the exit actor. Exit semantics live in `exit.ma`. |
| `:child-alive` | `<actor> <kind> <nonce> <direction>` | Child readiness callback; rooms use it for pending new-room dig targets when actor, kind, nonce, and direction match. |
| `:ping` / `:pong` / `:authorise-link` / `:link-authorised` / `:link-denied` | link handshake args | Existing-room link handshake. |
| `:presence-tick` | none | Scheduled room-local occupant sweep. Asks occupants to report their parent and removes occupants that have not reported within the room timeout. |
| `:parent-report` | `<child> <parent> <tick> <nonce>` | Machine reply from a child to the room's `:report-parent` request. A parent different from the room removes the child immediately. |

Room presence cache rules:

1. `occupants` and `avatar-occupants` are room-local derived caches.
2. DID principal-facing `:leave` removes only live room presence. It deliberately leaves
   avatar state and zion `.my.ctx.*` unchanged so the remembered room remains the
   return point on a later login.
3. Owner-facing `:remove <occupant>` is manual cleanup only. It removes cached
   room presence, not actor state or future admission rights. If multiple
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

### 5.4 exit actor

Purpose: first-class inspectable traversal object.

| Verb | Args | Notes |
| --- | --- | --- |
| `:about` | none | Returns name, description, owner, source room, target room, direction, and locked state. |
| `:where?` | none | Returns the source room DID-URL. |
| `:owner` | none | Returns the owner DID or `(none)`. |
| `:report-parent` | `<room> <tick> <nonce>` | Machine presence request; replies to the requesting room with `:parent-report <self> <source-room> <tick> <nonce>`. |
| `:locked?` | none | Returns `true` or `false`. |
| `:lock` / `:unlock` | none | Source-room-only mutation. Avatar/DID principal `lock <direction>` and direct room `:exit <direction> :lock` resolve through the source room. |
| `:message` | `traveller`, `source`, `target`, or `blocked`, plus `text` | Source-room-only traversal-message update. Avatar/DID principal `exit-message <direction> ...` and direct room `:exit <direction> :message ...` resolve through the source room; the exit actor keeps canonical message state. |
| `:traverse` | `<ctx-map>` | Source-room-only traversal request. If unlocked, returns `:traversed <ctx>` to the source room with `ctx.room` set to the target. If locked, returns `:traversed <ctx>` with `ctx.room` still set to the source room and `ctx.text` set to the blocked message. |

Movement ctx is ordinary actor/entity state, not a separate signed or protocol
envelope. Minimal fields for now are `actor`, `kind`, `room`, and optional
`nick`/`text`. The exit may annotate ctx with `exit` and `direction` while
transforming it.

The exit actor is the canonical store for first-class exit metadata and policy.
The room stores topology only: direction to exit actor, and the target room used
to recreate a local exit if the local actor is missing.

### 5.5 Scheme agent parent kind

Kind: `/ma/scheme/agent/0.0.1`, extending `/ma/scheme/state/0.0.1`.

Purpose: reusable autonomous Scheme-agent base behaviour. Concrete agents extend
this kind and inherit owner, parent, recovery, and transfer helpers.

Key helpers and verbs:

| Verb/helper | Args | Notes |
| --- | --- | --- |
| `agent-ctx` | none | Builds `ctx` with `kind=agent`, `name`, `nick`, `description`. |
| `:children` | `[ctx]` | Inherited generic child-cache contract. Agents announce their own ctx to their current parent on lifecycle `:start`, after successful `:take` parent changes, and after committed room ctx movement. |
| `:parent` | `[ctx]` | Current-parent-gated parent push. Accepts a ctx map from the current parent, reads the new parent from `ctx.parent` or `ctx.room`, updates local parent state, persists optional presentation fields from the ctx, and announces to the new parent with `:children <ctx>`. |
| `:about` `:where?` `:owner` | none | Generic state summary. |
| `:exits?` | none | Asks the current parent room for exits and stores the printed reply as `last-message`. |
| `:go` | `<direction>` | Free-agent or owner movement through a named room exit; no exit creation. |
| `:move` | none | Asks the current parent room to choose one available exit at random and traverse it. |
| `:ctx` | `<ctx-map>` | Room-facing movement helper; validates the ctx against the current parent, performs ordinary room-visible `:leave-occupant`, then sends map-shaped agent `:enter` to the target room. With no args, returns current ctx to authorised callers. |
| `:enter-room` | `<target-room-did-url> <source-room-did-url>` | Root/room movement helper retained for direct room-driven entry flows. |
| `:report-parent` | `<room> <tick> <nonce>` | Machine presence request; replies to the requesting room with `:parent-report <self> <parent> <tick> <nonce>`. |
| `:claim` | `<secret>` | Recovery-path ownership claim. |
| `:take` | `<did> <carrier-parent> [ctx]` | Caller must be current parent. |
| `:drop` | `<did> <target-parent> [ctx]` | Caller must be current parent. |

### 5.6 rms actor

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

### 5.7 thing actor

Purpose: movable passive object with owner/parent authority.

| Verb | Args | Notes |
| --- | --- | --- |
| `:about` | none | Name, description, owner, parent summary. |
| `:where?` | none | Current parent. |
| `:owner` | none | Current owner. |
| `:children` | `[ctx]` | Inherited generic child-cache contract. Things announce their own ctx to their current parent on lifecycle `:start` and after successful `:take`/`:drop` parent changes. |
| `:parent` | `[ctx]` | Current-parent-gated parent push. Accepts a ctx map from the current parent, reads the new parent from `ctx.parent` or `ctx.room`, updates local parent state, persists optional presentation fields from the ctx, and announces to the new parent with `:children <ctx>`. |
| `:prop` | `<name\|nick\|description> [value]` | Owner only. Sets an editable presentation prop; no value resets the prop to the kind default. Does not edit `owner` or `parent`. |
| `:set-recovery-secret` | `[text]` | Owner only. |
| `:claim` | `<secret>` | Recovery-path ownership claim. |
| `:report-parent` | `<room> <tick> <nonce>` | Machine presence request; replies to the requesting room with `:parent-report <self> <parent> <tick> <nonce>`. |
| `:take` | `<did> <carrier-parent> [ctx]` | Caller must be current parent; optional ctx map is accepted and persisted. |
| `:drop` | `<did> <target-parent> [ctx]` | Caller must be current parent; optional ctx map is accepted and persisted. |

---

## 6. State keys and authority boundaries

### 6.1 root keys

Root stores no avatar registry. Avatar DID-URLs are derived from the caller DID
by the root/room/avatar actor code with `blake3`, scoped by runtime DID and
caller DID. Runtime validates requested fragments but does not derive these
world-level actor names.

### 6.2 room keys

| Key | Type | Authority |
| --- | --- | --- |
| `owner` | bare DID | authoritative room ownership |
| `name`, `description` | string | authoritative room metadata |
| `exits` | map | direction to exit actor DID-URL |
| `exit-target:<direction>` | DID-URL | target room used for local exit healing |
| `exit-target-name:<direction>` | string | optional deterministic new-room target name |
| `things` map | map | room-local alias map to non-avatar occupant DID-URLs |
| `claim:<actor>` | map | stored enter claim/context |
| `occupants` | list | derived cache (presentation/broadcast), root-fed |
| `avatar-occupants` | list | derived avatar-only presence cache for people-oriented presentation |
| `label:<actor>` | string | derived display cache |
| `presence:tick` | integer | room-local scheduled presence counter |
| `presence:last-report:<actor>` | integer | last tick where actor reported this room as parent |
| `presence:last-request:<actor>` | integer | last tick where room requested parent report |
| `presence:nonce:<actor>` | string | pending parent-report nonce |
| `presence:last-parent:<actor>` | string | last parent reported by actor, for debugging |

### 6.3 avatar keys

| Key | Meaning |
| --- | --- |
| `did` | controlling bare DID |
| `room` | current room DID-URL |
| `nick` | current display nick |

### 6.4 thing/agent keys

| Key | Meaning |
| --- | --- |
| `owner` | owner DID |
| `parent` | current parent DID-URL |
| `name`, `description` | display metadata |
| `recovery-secret` | optional recovery claim secret |

---

## 7. Wire/value conventions

1. Actor message payloads are ma-scheme terms serialised through runtime RPC.
2. Verb dispatch follows `:verb` or tuple/list forms with `:verb` head.
3. Enter `ctx` is an optional map value. Missing `ctx.kind` means client/session
   entry: the client waits for committed context. Direct `agent`/`thing` entry
   MUST carry required string fields listed in section 2.2.
4. DID values crossing actor, zion, or runtime message boundaries MUST be full
   DID or DID-URL values, not runtime-local shorthand. Committed ctx actor
   references MUST be full DID-URLs.
5. Committed client context uses `/ma/lambda/ctx/0.0.1` and includes the
   effective `kind` chosen by the world.
6. Room `:enter` dispatch is kind-driven: absent kind or explicit `avatar`
   ensures the caller's deterministic avatar locally and sends no `:ok`; `thing`
   and `agent` require explicit kind and are admitted into the same room-local
   non-avatar occupant cache for now.
7. Thing transfer validation is strict by default: DID principal MUST be `did:ma:...`;
   non-ctx parent arguments MUST be DID-URLs. Optional
   transfer ctx MUST include non-empty `kind`, `name`, `nick`, and `description`
   fields. Any actor references inside ctx MUST be full DID-URLs.

---

## 8. Build and bootstrap quick reference

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

## 9. Cross references

- Project overview and developer workflow: README.md
- First-run bootstrap walkthrough: HOWTO.md
- Actor protocol detail source: actors/README.md
- Focus routing guardrail for agents: AGENTS.md
