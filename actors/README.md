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

## Context flow

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
Root sends fresh occupant context directly after every movement or nick change;
rooms do not pull avatar context.

## Movement flow

External entry uses the same exit traversal contract as ordinary movement:

1. User asks root to enter with `:enter [nick]`.
2. Root creates/fetches the user's avatar without placing it in a room.
3. Root sends the avatar through the stable entry exit with `:traverse`.
4. The entry exit sends `:enter-avatar` to its target room.
5. The room asks root to register arrival with `:arrived <avatar> <target-room>`.
6. Root updates the authoritative `room:<avatar>` register and pushes zion context.

Room-to-room movement uses the same tail of that flow:

1. Avatar sends `:go <direction>` to its current room.
2. Room sends `:traverse <avatar> <source-room>` to the exit.
3. Exit sends `:enter-avatar <avatar> <exit>` to the target room.
4. Target room asks root to register arrival with `:arrived <avatar> <target-room>`.
5. Root updates the authoritative `room:<avatar>` register.
6. Root pushes `:leave-avatar` + `:ctx` to the old room and `:join-avatar` + `:ctx` to the new room.

## Stationary rubber duck

Rooms can create a local rubber duck with `:duck`. The duck stores only its
room and is not added to root's location registry, so it cannot move between
rooms. The room also stores the duck in its local `ducks` list, so `:look`
can show it as something present in that room. It can still speak through the
room:

```scheme
(:quack) ; sends (:say "kvakk") to the room
```

Because room labels the duck locally, occupants see `rubber duck says: kvakk`
rather than the duck actor DID.

You do not send `:go #room` to a duck. To put a duck somewhere, ask that room
to create it with `:duck`; the created duck belongs to that room until a later
object/mobility model exists.
