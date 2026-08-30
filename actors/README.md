# Lambda-ma Actor Protocol

Lambda-ma is a direct-DID world profile.

## World actors

- `#root` and `#scheduler` are metanames in this document. Actor messages
  always use their configured full DID-URLs.
- Root is the local trust anchor. Its `:ctx?` reply is the runtime service
  directory and contains full DID-URLs only. `:enter?` gives unqualified-entry
  discovery, always replying with a ctx naming a room to enter. Only `#root`
  is required in a given runtime.
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

A client with no known room address asks `#root :enter?` instead. Root always
replies with a ctx, `{ parent, rev }`, naming a room to enter — the configured
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

An exit accepts lock commands only from its source room; the room owner issues
them through the room's `:exit <direction> <verb>` proxy, which forwards to the
exit actor as the source room. `:lock <secret>` is the same contract as
containers: it locks the exit and stores the optional secret. A traveller
holding the secret may `:unlock <secret>` or pass straight through a locked
exit with `:traverse <ctx> <secret>`; without it, the exit replies with its
blocked message and the traveller stays in the source room.

## Traversal

A DID asks an exit for `:traverse` with transient `{ did, parent }`. The exit
returns `{ did, parent, text, exit, direction }`. The DID enters the returned
parent room with `:enter`.

## Replies and events

Actor inspection and control use RPC replies. World narration uses `:print`:
arrival, departure, speech, emotes, movement, and visible actor transfer are
room broadcasts. A client-side command library may send direct room RPC, but it
does not proxy commands or generate a second narrative for an acknowledged
action.

## Ctx rules

All cross-boundary actor references are full DIDs or DID-URLs. A ctx payload
does not grant authority. A sender must match the relevant DID or DID-URL before
any ctx fields or revisions are considered.