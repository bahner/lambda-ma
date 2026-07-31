# Actor protocol

## Location and occupants

Root is a deterministic avatar factory, not a location registry. It derives a
DID principal's avatar DID-URL from the bare DID via the runtime-scoped entity-fragment
derivation, creates the avatar only if absent, and otherwise only returns the
avatar DID-URL.

Rooms keep a local `occupants` cache for broadcast and room-local presentation.
That cache is derived state, not authority. `parent` alone is not room
presence; a movable actor is present only after it sends the room `:enter`.

Actors do not have to be root-tracked occupants to speak. Any actor that knows
the room DID-URL can send `:say` or `:emote`; the room broadcasts the text to
the current room-local occupants.

## Actor categories

Actors use three starting categories:

- `avatar` — a DID delegate. Avatars represent people and participate in the
   room's social occupant flow.
- `agent` — an autonomous actor with its own will or impulses. A rubber duck
   that moves or quacks on its own is an agent, not room code and not an avatar.
- `thing` — a passive object with state, ownership, and location, but no agenda
   of its own.

Rooms accept the category in `:enter ctx`, but clients may omit `kind` when they
do not know their effective world kind yet. Missing `kind` means session/avatar
entry: the room creates or finds the deterministic avatar and sends no `:ok`
itself. The avatar enters the room, receives committed `/ma/lambda/ctx/0.0.1`
context from the room, persists that state, and forwards the ctx to the DID principal.
Direct `agent` and `thing` entry must provide `kind`; without it, the world
assigns the default session kind and reports that in ctx.

## Free objects and agents

Movable things and agents are free actors. An actor's DID-URL is its identity,
and its own state is authoritative for that actor. Runtime-global placement
actors such as `#house` are intentionally out of scope for this model.

The golden rule is: movable actors know and own their own state.

The minimal structural props for a movable actor are:

- `owner` — DID allowed to perform protected owner operations.
- `parent` — DID-URL of the thing's immediate location/container.

Avatar-mediated creation uses the avatar verb `make <kind> <init...>`. The
avatar calls `ma-create-actor` directly with all arguments after `kind` joined
as the init text; the room is not the factory, and Zion does not own the make
grammar. The init text is authoritative for the new actor's initial props,
including `owner` and `parent`. To create a thing for someone else, set `owner`
to that bare DID in the init text. To create a thing the avatar is holding, set
`parent` to the avatar DID-URL; to create it in the room, set `parent` to the
room DID-URL.

`parent` is location. If a duck is inside a chest, the duck stores the chest as
its parent. If that chest is in a room, the chest stores the room as its parent.
Location is found by walking upward from child to parent until a room or other
world anchor is reached.

Containers, rooms, backpacks, and chests may keep `contents` caches for display
or search, but those caches are derived presentation state only. If a container
claims it contains a thing and the thing's own `parent` disagrees, the thing's
state wins.

Moving a thing means asking the thing to update its `parent`, not editing two
competing container lists. A thing may update its own parent when it moves by
itself. A carrier may ask it to update parent during `take` or `drop`, but the
thing remains the state owner.

Transfer requests may include an optional `ctx` map as a trailing argument
(`:take <did> <carrier-parent> [ctx]`, `:drop <did> <target-parent> [ctx]`).
When provided, that `ctx` is forwarded with the transfer and can be persisted
as claim context by the movable actor.

Agents remain responsible for their own room presence during transfer. After a
successful `:take`, an agent notifies its old room that it left; after a
successful `:drop`, it sends that room `:enter` with its current agent ctx. The
agent commits its new `parent` only after receiving a valid room-origin `:ctx`
for that entry.

Protected operations check caller DID against `owner`:

```scheme
(equal? (msg-from msg) (get-prop "owner"))
```

A thing may also store a recovery/transfer secret, for example
`recovery-secret`. This is not day-to-day authentication. It is an offline
recovery path: if the owner loses their DID, or wants to give the thing away,
the holder of the secret can call a claim verb and the thing can bind `owner` to
the caller DID, then rotate or clear the secret.

Signed owner or parent claims are deferred. They can be useful later as public
proof for external verification, but they are not first-slice authority: the
thing's `owner` prop plus normal DID-authenticated `msg.from` checks are the
practical control boundary.

## Room ownership

Rooms store their owner as a bare DID in the room-local `owner` prop. Avatars
are not owners; room owner checks accept a message from either the owner DID
directly or the deterministic same-runtime avatar for that owner DID. The avatar
fragment is derived with `blake3("lambda-ma avatar v1\n" runtime "\n" did, 8)`.

Protected room commands are ordinary room verbs:

```scheme
(:claim)
(:owner [<new-owner-did>])
(:dig <direction> [to <new-room-name-or-room-target>])
```

No owner-authority argument is accepted. The room derives authority from
`msg-from` and its stored `owner` DID.

`:claim` only succeeds when the room has no owner. `:owner` with no target
prints the owner; with a target DID it transfers ownership and requires the
message to come from the current owner or the current owner's deterministic
avatar. `:dig` requires ownership of the current room and assigns the current
owner DID to any newly-created target room.

Digging an existing direction replaces that exit instead of failing. This lets
room owners rewire mistakes or rebuild a topology without deleting the old exit
first. `fill <direction>` removes the exit from the room and asks the exit actor
to terminate itself; it does not delete the target room.

Named new-room digs are idempotent per source room, direction, and target name.
For example, repeating `dig dør to kjøkken` in the same source room resolves to
the same deterministic target room fragment and the same room-owned exit
fragment. The room derives those fragments with `blake3`; runtime only
validates and creates the requested fragments.

Colon-prefixed methods are not avatar-mediated. Room metadata is a direct room
RPC:

```scheme
(:prop <key> [<value> ...])
```

`:prop` requires the direct caller to be the room owner, sets an arbitrary room
prop to the joined text value, and deletes that prop when no value is supplied.

## Help

`help` is an avatar-mediated avatar-mediated command. It shows the avatar's general command
index: movement, speech, ownership, building, nickname, and `help here`.

`help here` asks the current room/place for its own `:help`. The avatar does not
need to know what is locally possible; room authors can make `:help` describe
the affordances of that specific place.

If the avatar does not know a avatar-mediated command, it forwards that verb and its
arguments to the current room. This lets room-local commands such as `duck` work
through ordinary avatar-mediated input without teaching every avatar each local
method ahead of time.

`:help` is a direct room/place RPC. A room replies with its help text so direct
zion calls complete, and when the caller is a current avatar occupant the room
also sends the text via `:print` for display display.

Exits to already-existing rooms use a room-to-room reachability and ownership
check. The source room stores a pending link request, sends `:ping` to the
target room, then asks the target room to authorise the same bare DID only after
receiving `:pong`. The source only creates the exit if the target room confirms
that DID owns it too. That keeps the invariant simple: no actor creates an exit
to an existing room unless the target room is reachable and ownership of both
rooms can be verified.

Existing-room targets are full DID-URLs. Runtime-local fragments may be
canonicalised while reading legacy state, but actor messages use full DID or
DID-URL values throughout the handshake. Full `did:ma:...#room` targets may
point at another runtime; the same room-to-room ownership handshake must still
succeed before the exit is created.

New-room digs use the lambda-ma child-alive callback because
`ma-create-actor` is queued until the current dispatch completes. The source
room stores a pending room request with a nonce, passes child-alive metadata in the
new room's init payload, and creates the exit only after the new room sends
`:child-alive <actor> <kind> <nonce> <direction>` back from the expected actor
DID-URL. The child-alive metadata is part of the new room's init payload; after
storing it, the room sends the callback to its parent. This prevents exits from
being created without a nonce-bound acknowledgement from the expected child.
Genesis actors or other actors with no parent do not emit a child-alive message.

## Context flow

Zion enters by sending `:enter ctx` to the target room. The room creates or finds
the deterministic local avatar in the background. The avatar owns the client
context it reports to Zion: current root, avatar, room, nick, and optional text.
Zion may cache the room for direct `:` commands, but plain commands are
addressed to the avatar.

New avatar init is push-based: the live avatar sends room `:enter`. For an
existing avatar, the target room sends `:enter-room` to the avatar; the avatar
then sends room `:enter`. The room registers the entry, sends committed `:ctx`
to the avatar, and the avatar persists that room state before forwarding `:ctx`
to the DID principal. Root remains the compatibility path when no room target is known;
root must not send messages to rooms.

Leave event:

```scheme
(:leave)
```

Rooms accept ordinary avatar `:enter` and display `:leave` for deliberate
live-presence departure. Movement cleanup is ctx-driven, not a separate old-room
notification. `:leave` removes the avatar from room presentation without changing the
avatar's stored room or zion's cached context; the same room remains the DID principal's
return point on the next login. DID principal-facing context is sent by avatar.

## Movement flow

External entry is room-first:

1. DID principal asks the target room to enter with `:enter ctx`.
2. Room derives the caller's deterministic avatar URL.
3. Existing avatar: room asks avatar to enter here; avatar sends room `:enter`.
4. New avatar: room creates it with bare DID as fragment hint; avatar init sends
   room `:enter` after the avatar is live.
5. Room registers entry and sends committed `:ctx` to avatar; avatar persists
   room state and forwards the ctx to DID principal.

Room-to-room movement uses the same avatar handshake as external entry. The
source room resolves a direction to a first-class exit actor. The exit
transforms ordinary movement ctx and returns it to the source room; the moving
actor then tries to enter the target room itself. The target room creates or
reuses that DID principal's deterministic local avatar before publishing the new context.

1. Avatar sends `:go <direction>` to its current room.
2. Room sends `:traverse <ctx>` to the exit, with minimal ctx fields `actor`,
   `kind`, `room`, and optional `nick`/`text`.
3. Exit sends `:traversed <ctx>` back to the source room with `ctx.room` set to
   either the target room or the original room if blocked.
4. Source room sends `:ctx <ctx>` to `ctx.actor`.
5. Target rooms ask the deterministic local avatar for `did` to enter the room.

Agent movement is actor-owned and room-visible:

1. The owner, or any caller while the agent is free/unowned, sends `:move` or
   `:go <direction>` to the agent.
2. The agent asks its current parent room to choose an exit for `:move`, or to
   use the named exit for `:go <direction>`.
3. The room sends `:traverse <ctx>` to the exit.
4. The exit returns `:traversed <ctx>` to the source room, and the source room
   sends `:ctx <ctx>` to the agent.
5. The agent sends `:leave-occupant` to the old room, then sends map-shaped
   `:enter` with `agent-ctx` to the target room.
6. The old room broadcasts `<nick> leaves.` and the target room broadcasts
   `<nick> arrives.`; the agent commits its new `parent` only after target-room
   `:ctx`.
