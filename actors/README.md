# Lambda-ma Actor Protocol

Lambda-ma is a direct-DID world profile.

## World actors

- `#root` and `#scheduler` are metanames in this document. Actor messages
  always use their configured full DID-URLs.
- Root is the local trust anchor. Its `:ctx?` reply is the runtime service
  directory and contains full DID-URLs only. `:enter` on root always replies
  `[:entrance, <room>]`, naming a room to enter; root never admits anyone
  itself. Only `#root` is required in a given runtime.
- Rooms own local bare-DID presence, exits, presentation, and room policy.
- Exits return direct DID traversal results; the DID then enters the target
  room directly.
- Agents, things, and containers are ordinary node actors with full DID-URLs.

## Entry and presence

A bare DID sends `:enter` to a room. The room records DID presence keyed by
that DID, broadcasts an arrival when appropriate, and replies with the
committed `{ parent, name, nick, description, rev }` ctx value. A client accepts
only the reply matching its active request from that addressed room.

DID presence is room-local state. It is not an entity and is not a node child.
Node `children` remains the single actor ctx map keyed by full actor DID-URL.

A client with no known room address sends `:enter` to `#root` instead. Root
always replies `[:entrance, <room>]`, naming a room to enter — the configured
`start` room by default.

## Authority and transfer

`msg.from` authenticates the caller. Visible labels are lookup input only; after
resolution, actor workflows use full DID-URLs. A movable child requests
`:parent <ctx>` from a candidate parent, accepts `:child <ctx>`, commits its
parent, then informs the previous parent. Valid retries are idempotent.

Parentage is placement, not ownership. `:set-parent` and `:hold` never change
an existing actor's owner; `:claim` is the explicit ownership transition.
`:forge` creates the sole exception by setting the new actor's initial owner
and parent to `msg.from`, so a forged entity is born into its forger's hand.
Owner-gated policy such as container locking must require that existing owner
and must not claim implicitly.

## Container locks

Only a claimed container's owner can lock it. `:lock <secret>` also replaces
the optional stored unlock secret, allowing any caller holding that value to
call `:unlock <secret>`. The owner can always call `:unlock` without a secret;
bare `:lock` is idempotent and preserves an existing stored secret.

## Exit locks

An exit is a single top-level node with two hardcoded endpoints, `src` and
`dst`; each room references it by a cosmetic local name in its own `exits` map.
Its owner or its source room may lock it directly with `:lock <secret>`. Any
caller holding that secret, or the runtime global owner key, may `:unlock
<secret>`, which opens the exit for everyone (a lock is global, never per
player). Exits have no owner bypass: the owner and the source room present a
key too, so the gate is testable. A locked passage answers a move with
`[:deny, reason]`; the source room fires the `nogo` event (the `nogo_msg` prop,
default `This is not the way.`), and the traveller stays in the source room.

## Traversal

Walking a passage is `:move` → `:move` → `:enter` → `:ok`. The traveller sends
`:move { direction, traverseId }` to the room, which sends `:move <move-ctx>` to
the passage. The passage routes and authorises: it sends `:move <move-ctx>` to
the destination, follows `[:entrance, <room>]` redirects, and answers the source
room with `{ traverseId, exit, key, entrance }`. The source room then hands the
traveller `{ exit, key }`, and the traveller enters the destination with
`:enter <move-ctx>`. The passage never moves anyone; it orchestrates.

A dig reports its direction in the room's `:dig` broadcast only after the
passage is installed. The digger's own client follows that broadcast by walking
the named direction; other occupants only see the narration.

A room is public unless `public` is `false`. A room created by `:dig` starts
`public: false` (under construction), so travellers need a key until its
builder makes it public. A private room always needs a key for `:enter` — a
room key, a key issued for a live traversal, or the runtime's break-glass
`global_key` — for owner and traveller alike (there is no bypass by owner
identity, so gating is testable). `:dig` mints the new room's admission key and
reports it. Registration and admission are separate: a room's `exits` map
decides only what it shows, never who may enter. The `global_key` is
daemon-local: set it in the runtime's `ma.yaml`, where it is protected from
CRUD, and it is injected into the actor config map read by
`(ma-get-config-key "global_key")`.

A DID that already holds committed presence in the room is re-admitted without
a key: a repeat `:enter` is a presence refresh, not a new admission, and
first-entry keys (such as a traversal grant) are consumed. A DID that is not
already present still needs a key.

## Replies and events

Actor inspection and control use RPC replies. World narration uses `:print`:
arrival, departure, speech, emotes, ambient room narration (`:narrate`), and
movement and visible actor transfer are room broadcasts. A client-side command
library may send direct room RPC, but it does not proxy commands or generate a
second narrative for an acknowledged action.

## Ctx rules

All cross-boundary actor references are full DIDs or DID-URLs. A ctx payload
does not grant authority. A sender must match the relevant DID or DID-URL before
any ctx fields or revisions are considered.