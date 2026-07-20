# Actor protocol

## Location and occupants

Root is the authoritative location registry:

- `avatar:<user>` -> avatar DID-URL
- `user:<avatar>` -> user DID
- `room:<avatar>` -> current room DID-URL
- `nick:<avatar>` -> non-unique display name
- `avatars` -> known avatar DID-URLs

Rooms keep a local `occupants` cache for broadcast and room-local presentation. That cache is derived state, not authority.

Actors do not have to be root-tracked occupants to speak. Any actor that knows
the room DID-URL can send `:say` or `:emote`; the room broadcasts the text to
the current root-tracked occupants.

## Room ownership

Rooms store their owner as a user DID in the room-local `owner` prop. Avatars
are not owners; for user-facing commands they act as delegates and prepend their
stored user DID before forwarding to the room. Direct room RPCs use `msg-from`
as the caller identity.

Protected room commands accept both shapes:

```scheme
(:claim)
(:owner [<new-owner-did>])
(:dig <direction> [to <new-room-name-or-room-target>])

(:claim <user-did>)
(:owner <user-did> [<new-owner-did>])
(:dig <user-did> <direction> [to <new-room-name-or-room-target>])
```

The second shape is accepted only from known room occupants, which are avatar
actors maintained by root context.

`:claim` only succeeds when the room has no owner. `:owner` with no target
prints the owner; with a target DID it transfers ownership and requires the
caller to be the current owner. `:dig` requires ownership of the current room
and assigns the digger's user DID to any newly-created target room.

Digging an existing direction replaces that exit instead of failing. This lets
room owners rewire mistakes or rebuild a topology without deleting the old exit
first.

Colon-prefixed methods are not avatar-mediated. Room metadata is a direct room
RPC:

```scheme
(:prop <key> [<value> ...])
```

`:prop` requires the direct caller to be the room owner, sets an arbitrary room
prop to the joined text value, and deletes that prop when no value is supplied.

## Help

`help` is an avatar-mediated user command. It shows the avatar's general command
index: movement, speech, ownership, building, nickname, and `help here`.

`help here` asks the current room/place for its own `:help`. The avatar does not
need to know what is locally possible; room authors can make `:help` describe
the affordances of that specific place.

If the avatar does not know a user command, it forwards that verb and its
arguments to the current room. This lets room-local commands such as `duck` work
through ordinary avatar-mediated input without teaching every avatar each local
method ahead of time.

`:help` is a direct room/place RPC. A room replies with its help text so direct
zion calls complete, and when the caller is a current avatar occupant the room
also sends the text via `:print` for user-facing display.

Exits to already-existing rooms use a room-to-room reachability and ownership
check. The source room stores a pending link request, sends `:ping` to the
target room, then asks the target room to authorize the same user DID only after
receiving `:pong`. The source only creates the exit if the target room confirms
that user owns it too. That keeps the invariant simple: no actor creates an exit
to an existing room unless the target room is reachable and ownership of both
rooms can be verified.

Existing-room targets may be full DID-URLs or local runtime fragments. A local
fragment such as `#garden` is checked against this runtime and kept as a local
target throughout the handshake. Full `did:ma:...#room` targets may point at
another runtime; the same room-to-room ownership handshake must still succeed
before the exit is created.

## Context flow

Zion's normal contact point is the active avatar. Root creates or finds that
avatar and owns the authoritative placement registry, but avatar owns the client
context it reports to Zion: current root, avatar, room, nick, and optional text.
Root corrects avatar placement with `:set-location`; avatar persists it and
pushes `:ctx` to the user. Zion may cache the room for direct `:` commands, but
plain commands are addressed to the avatar.

Normal flow is push-based: whenever root registers that someone enters or
leaves a room, root sends the affected room an event and then sends a fresh
context snapshot.

Entry event:

```scheme
(:join-avatar <avatar> <from-room-or-#f>)
```

Leave event:

```scheme
(:leave-avatar <avatar> <to-room>)
```

Occupant context snapshot:

```scheme
(:ctx :avatars ((<avatar> <nick>) ...))
```

Rooms also accept the older `(<avatar> ...)` shape as a repair/backwards-
compatibility input, but root now sends nick-bearing entries.

Room accepts `:join-avatar`, `:leave-avatar`, and `:ctx` only from root.
Root sends fresh occupant context to rooms after movement or nick change; rooms
do not pull avatar context. User-facing context is sent by avatar.

## Movement flow

External entry uses the same exit traversal contract as ordinary movement:

1. User asks root to enter with `:enter [nick]`.
2. Root creates/fetches the user's avatar and replies with the avatar URL.
3. Root sends the avatar through the stable entry exit with `:traverse`.
4. The entry exit sends `:enter-avatar` to its target room.
5. The room asks root to register arrival with `:arrived <avatar> <target-room>`.
6. Root updates the authoritative `room:<avatar>` register and sends
	`:set-location` to avatar.
7. Avatar persists the room and pushes zion context to the user.

Room-to-room movement uses the same tail of that flow inside one runtime. When
the exit crosses to another runtime, the source avatar carries the user DID and
nick through the exit; the target runtime root creates or reuses that user's
local avatar before publishing the new context.

1. Avatar sends `:go <direction>` to its current room.
2. Room sends `:traverse <avatar> <source-room> <user> <nick>` to the exit.
3. Exit sends `:enter-user <user> <avatar> <exit> <nick>` to the target room.
4. Target room asks root to register arrival with `:arrive-user <user> <target-room> <nick>`.
5. Root creates or reuses the local avatar, updates its authoritative
	`room:<avatar>` register, and sends fresh client context.
6. Root pushes `:leave-avatar` + `:ctx` to the old room and `:join-avatar` + `:ctx` to the new room.
