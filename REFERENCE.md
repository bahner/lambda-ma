# lambda-ma Reference

**Version:** 0.1.0
**Status:** Draft

Canonical reference for lambda-ma world protocol behavior.

---

## Scope boundary

This document defines lambda-ma world/profile behavior on top of ma-runtime.
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

- No leading `:`: avatar-mediated user command.
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
- If the avatar does not exist yet, room creates it with the user DID as
   `fragment_hint`; avatar init sends room `:enter` after the avatar is live.
- If the avatar exists, room sends only `:enter-room` to the avatar; the avatar
   sends room `:enter`.
- Room registers avatar entry in room state and sends committed
   `/ma/lambda/ctx/0.0.1` to the avatar. The avatar then persists its room state
   and forwards that ctx to the user.

Compatibility path:

- Root `:enter` still exists and may be used when only runtime target is known.
- For an existing avatar, root asks the avatar to send its current ctx to the
   user. Root must not send messages to rooms.

### 2.2 Enter payload

Room-first enter accepts either no payload or one extensible map named `ctx`.
The payload concept is `ctx` only. Do not introduce a parallel `attrs` map.

For client entry, `ctx.kind` MAY be absent. Absence means the client is not
claiming to be a concrete world object kind yet; the room MUST treat this as a
request to enter as the world's default session kind and send no `:ok` itself.
The room creates or finds the avatar; the avatar performs room entry, receives
committed `/ma/lambda/ctx/0.0.1` context from the room, persists that state, and
forwards the ctx to the user asynchronously. In lambda-ma the default session
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

All actor references in committed ctx MUST be fully qualified DID-URLs. Runtime-
local `#fragment` shorthand is valid only for internal runtime messages, never
for ctx fields persisted by avatars, agents, clients, or future ctx consumers.

Required context fields:

- `protocol` = `/ma/lambda/ctx/0.0.1`
- `kind` (effective session kind chosen by the world, e.g. `avatar` or `agent`)
- `root`
- `room`
- `nick`

Avatar contexts include `avatar`, the actor to receive avatar-mediated user
commands. Direct agent contexts may set `avatar` to the empty string.

### 2.4 Commit behavior

Client-side focus/context commit is acknowledgment-driven.

- Enter send does not imply commit.
- Commit occurs only after valid acknowledgment from expected actor path.

---

## 3. Authority model

### 3.1 Rooms

- Room ownership is a user DID in room `owner` prop.
- Avatar is a delegate for user-facing commands, not the owner identity.
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

### 4.1 User enter via room ctx

1. Caller sends `:enter <ctx>` to the target room.
2. Room derives the deterministic avatar fragment from the caller DID.
3. If the avatar already exists, room asks it to enter this room.
4. If the avatar does not exist, room creates it with the caller DID as
   `fragment_hint`; avatar init sends room `:enter` from the live avatar.
5. Room registers entry and sends committed `:ctx` to avatar; avatar persists
   room state and forwards the ctx to user.

### 4.2 Room-to-room movement via exit

1. Avatar sends `:go <direction>` to current room.
2. Room sends `:traverse` to exit.
3. Exit sends `:enter` to target room with either
   `<user> <avatar-did-url> <old-room-did-url> [nick]` or
   `<avatar-did-url> <old-room-did-url?>` shape.
4. Same-runtime target rooms admit that avatar into their local cache. Cross-
   runtime target rooms create or reuse the deterministic target-runtime avatar
   for `user`; the source avatar is used only to clean up the old room.
5. Target room asks the old room to remove the source avatar with
   `:leave-avatar` when needed.

### 4.3 Dig/link to existing room

For existing-room link targets:

1. Source room stores pending link request.
2. Source room sends `:ping` to target room.
3. After `:pong`, source requests `:authorize-link`.
4. Target room confirms same-owner policy.
5. Source room creates/replaces exit only after explicit authorization.

---

## 5. Actor interfaces

All terms are CBOR-style actor terms, typically `:verb` or `[":verb", ...]`.

### 5.1 root actor

Purpose: deterministic avatar factory.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `[room? nick?]` | Compatibility path when no concrete room target is available. Creates caller avatar if absent, or asks an existing avatar to send its current ctx to the user. Root does not message rooms. |
| `:avatar?` | none | Returns caller avatar, creating if needed in the configured start room. |

### 5.2 avatar actor

Purpose: user command endpoint and context emitter.

Key verbs:

| Verb | Args | Caller constraints | Notes |
| --- | --- | --- | --- |
| `:enter-room` | `<room>` | root or target room | Avatar receives this from root or the target room, sends room `:enter`, and waits for committed room ctx before persisting room state and forwarding `:ctx` to user. |
| `:sync-ctx` | none | root only | Emits current `:ctx` to user without changing avatar state. |
| `:ctx?` | none | user only | Returns context term. |
| `:help` | `[topic]` | user only | `help here` asks room `:help`. |
| `:nick` | `[nick]` | user only | No args returns current nick; with args forwards to room. |
| `:look` `:exits?` `:who?` `:say` `:emote` `:go` | varies | user only | Delegates to room. |
| `:claim` `:owner` `:dig` `:prop` | varies | user only | Delegated with prepended user DID. |
| `:drop-thing` | `<user> <thing> <target-parent> [token] [ctx]` | room caller only | Parent-mediated drop helper; forwards optional user ctx map. |

### 5.3 room actor

Purpose: local room policy, exits, ownership, occupant presentation.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `<ctx-map>` | Room-first enter endpoint. Absent/`avatar` kind ensures the caller's deterministic avatar; `agent`/`thing` require ctx required keys. |
| `:enter` | `<user> <avatar-did-url> <old-room-did-url> [nick]` | Movement arrival shape. Same-runtime avatars are admitted directly; foreign/source-runtime avatars trigger target-runtime local avatar entry for `user` and old-room cleanup for the source avatar. |
| `:enter` | `<avatar-did-url> [old-room-did-url]` | Admit known avatar flow. |
| `:enter` | `<user> <avatar-did-url> <old-room-did-url> [nick]` | Cross-room/cross-runtime-friendly arrival shape. |
| `:leave-avatar` | `<avatar-did-url> <to-room-did-url>` | Target-room-origin cache removal during movement. |
| `:leave-occupant` | none | Sender-origin cache removal for non-avatar occupants such as agents after actor-owned parent changes. |
| `:look` `:exits?` `:who?` `:occupants?` `:things?` | none | Local presentation; `:look` prints room text plus `Occupants:` and `Things:`. `who?` is people/avatar-oriented; `occupants?` includes avatars plus room-local agents/occupants. |
| `:thing` | `<name> [did-or-empty]` | Local occupant alias list/get/set/delete; owner-gated for write. |
| `:take` / `:drop` / `:where` | `[user?] [token]` | Uses movable actor parent-authority contract. |
| `:claim` / `:owner` / `:prop` | delegated or direct shapes | Room ownership controls write operations. |
| `:dig` | delegated or direct shape | Owner-gated exit creation/linking. |
| `:behaviour` | `[ /ipfs/<cid> ]` | Owner-gated behavior update. |
| `:ping` / `:pong` / `:authorize-link` / `:link-authorized` / `:link-denied` | link handshake args | Existing-room link handshake. |

### 5.4 exit actor

Purpose: traversal handoff.

| Verb | Args | Notes |
| --- | --- | --- |
| `:traverse` | `<avatar-did-url> [source-room-did-url] [user] [nick]` | Emits movement text and forwards enter event to target room. |

### 5.5 Scheme agent parent kind

Kind: `/ma/scheme/agent/0.0.1`, extending `/ma/scheme/actor/0.0.1`.

Purpose: reusable autonomous Scheme-agent base behavior. Concrete agents extend
this kind and inherit owner, parent, recovery, and transfer helpers.

Key helpers and verbs:

| Verb/helper | Args | Notes |
| --- | --- | --- |
| `agent-ctx` | none | Builds `ctx` with `kind=agent`, `name`, `nick`, `description`. |
| `:about` `:where` `:owner` | none | Generic state summary. |
| `:claim` | `<secret>` | Recovery-path ownership claim. |
| `:take` | `<user> <carrier-parent> [ctx]` | Caller must be current parent. |
| `:drop` | `<user> <target-parent> [ctx]` | Caller must be current parent. |

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
| `:where` | none | Current parent. |
| `:owner` | none | Current owner. |
| `:set-recovery-secret` | `[text]` | Owner only. |
| `:claim` | `<secret>` | Recovery-path ownership claim. |
| `:take` | `<user> <carrier-parent> [ctx]` | Caller must be current parent; optional ctx map is accepted and persisted. |
| `:drop` | `<user> <target-parent> [ctx]` | Caller must be current parent; optional ctx map is accepted and persisted. |

---

## 6. State keys and authority boundaries

### 6.1 root keys

Root stores no avatar registry. Avatar DID-URLs are derived from the caller DID
using the runtime-scoped `ma-derived-id` primitive and the same entity-fragment
context used by `ma_create_entity` fragment hints.

### 6.2 room keys

| Key | Type | Authority |
| --- | --- | --- |
| `owner` | user DID | authoritative room ownership |
| `name`, `description` | string | authoritative room metadata |
| `exits` map and `exit:<direction>` | map/string | authoritative exit registry |
| `things` map | map | room-local alias map to non-avatar occupant DID-URLs |
| `claim:<actor>` | map | stored enter claim/context |
| `occupants` | list | derived cache (presentation/broadcast), root-fed |
| `label:<actor>` | string | derived display cache |

### 6.3 avatar keys

| Key | Meaning |
| --- | --- |
| `user` | controlling user DID |
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

1. Actor message payloads are ma-scheme terms serialized through runtime RPC.
2. Verb dispatch follows `:verb` or tuple/list forms with `:verb` head.
3. Enter `ctx` is an optional map value. Missing `ctx.kind` means client/session
   entry: the client waits for committed context. Direct `agent`/`thing` entry
   MUST carry required string fields listed in section 2.2.
4. DID values crossing zion/runtime boundary SHOULD be full DID or DID-URL
   values, not runtime-local shorthand. Committed ctx actor references MUST be
   full DID-URLs.
5. Committed client context uses `/ma/lambda/ctx/0.0.1` and includes the
   effective `kind` chosen by the world.
6. Room `:enter` dispatch is kind-driven: absent kind or explicit `avatar`
   ensures the caller's deterministic avatar locally and sends no `:ok`; `thing`
   and `agent` require explicit kind and are admitted into the same room-local
   non-avatar occupant cache for now.
7. Thing transfer validation is strict by default: user MUST be `did:ma:...`;
   non-ctx parent arguments MUST be `did:ma:...` or `#fragment`. Optional
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
