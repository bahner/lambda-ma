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
- Room validates request context and asks root to register arrival.

Compatibility path:

- Root `:enter` still exists and may be used when only runtime target is known.

### 2.2 Enter payload

Enter payload is one extensible map named `ctx`.

Required keys (all non-empty strings):

- `kind`
- `name`
- `nick`
- `description`

Rules:

1. Missing or empty required keys MUST be rejected.
2. Additional keys MAY be present and are forward-compatible extension data.
3. The payload concept is `ctx` only. Do not introduce a parallel `attrs` map.

### 2.3 Commit behavior

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

### 4.1 User enter via root compatibility path

1. Caller sends `:enter` to root.
2. Root creates/reuses avatar and replies with avatar DID-URL.
3. Root sends avatar to entry room (`:enter <avatar> <old-room?>`).
4. Room admits avatar, updates room-local cache, and avatar receives location.

### 4.2 Room-to-room movement via exit

1. Avatar sends `:go <direction>` to current room.
2. Room sends `:traverse` to exit.
3. Exit sends `:enter` to target room with either
   `<user> <avatar> <old-room> [nick]` or `<avatar> <old-room?>` shape.
4. Target room calls root arrival registration (`:arrive-user` / `:arrived`).
5. Root updates authoritative placement and refreshes room occupant context.

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

Purpose: authoritative avatar/placement registry.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `[room? nick?]` | Compatibility entry path. Creates/reuses avatar and sends to room. |
| `:avatar?` | none | Returns caller avatar, creating if needed. |
| `:arrived` | `<avatar> <room>` | Accepts only if sender is the same room actor. |
| `:arrive-user` | `<user> <room> [nick]` | Arrival registration for user-based room enter flow. |
| `:nick` | `<nick>` | Avatar-originated nick update and room ctx refresh. |

### 5.2 avatar actor

Purpose: user command endpoint and context emitter.

Key verbs:

| Verb | Args | Caller constraints | Notes |
| --- | --- | --- | --- |
| `:set-location` | `<room> [text]` | room or root | Persists room and emits `:ctx` to user. |
| `:set-nick` | `<nick>` | room or root | Persists nick and emits `:ctx`. |
| `:ctx?` | none | user only | Returns context term. |
| `:help` | `[topic]` | user only | `help here` asks room `:help`. |
| `:nick` | `[nick]` | user only | No args returns current nick; with args forwards to room. |
| `:look` `:exits` `:who?` `:say` `:emote` `:go` | varies | user only | Delegates to room. |
| `:claim` `:owner` `:dig` `:prop` | varies | user only | Delegated with prepended user DID. |
| `:drop-thing` | `<user> <thing> <target-parent> [token] [ctx]` | room caller only | Parent-mediated drop helper; forwards optional user ctx map. |

### 5.3 room actor

Purpose: local room policy, exits, ownership, occupant presentation.

Key verbs:

| Verb | Args | Notes |
| --- | --- | --- |
| `:enter` | `<ctx-map>` | Room-first enter endpoint. Requires `ctx` required keys. |
| `:enter` | `<avatar> [old-room]` | Admit known avatar flow. |
| `:enter` | `<user> <avatar> <old-room> [nick]` | Cross-room/cross-runtime-friendly arrival shape. |
| `:join-avatar` / `:leave-avatar` | event args | Root-origin only cache/event updates. |
| `:ctx` | `:avatars <list>` | Root-origin only occupant snapshot refresh. |
| `:look` `:exits` `:who?` `:things?` | none | Local presentation; `:look` prints room text plus `Here:` (avatars + local movable occupants) and `Things:` (non-avatar aliases), while `who?` returns the same presence set. |
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
| `:traverse` | `<avatar> [source-room] [user] [nick]` | Emits movement text and forwards enter event to target room. |

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
`nick = rms`), explicitly sets `parent` with `set-parent!`, sends room `:enter`
with `agent-ctx`, and then registers a caller-owned `#scheduler` job named
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

### 6.1 root authoritative registry

| Key pattern | Meaning |
| --- | --- |
| `avatar:<user>` | user DID to avatar DID-URL |
| `user:<avatar>` | avatar DID-URL to user DID |
| `room:<avatar>` | avatar current room DID-URL |
| `nick:<avatar>` | avatar display nick |
| `avatars` | known avatar DID-URLs |

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
3. Enter `ctx` is a map value and MUST carry required string fields listed in
   section 2.2.
4. DID values crossing zion/runtime boundary SHOULD be full DID or DID-URL
   values, not runtime-local shorthand.
5. Room `:enter` dispatch is kind-driven for ctx payloads: `avatar` goes through
   root arrival registration; `thing` and `agent` are admitted into the same
   room-local non-avatar occupant cache for now.
6. Thing transfer validation is strict by default: user MUST be `did:ma:...`,
   parent refs MUST be `did:ma:...` or `#fragment`, and optional transfer ctx
   MUST include non-empty `kind`, `name`, `nick`, and `description` fields.

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
