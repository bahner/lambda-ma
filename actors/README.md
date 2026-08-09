# Lambda-ma Actor Protocol

Lambda-ma is a direct-DID world profile.

## World actors

- `#root`, `#house`, and `#scheduler` are metanames in this document. Actor
  messages always use their configured full DID-URLs.
- Root is the local trust anchor. Its `:ctx?` reply is the runtime service
  directory and contains full DID-URLs only.
- House is runtime-agnostic. It stores `did-ctxs` keyed by bare DID and
  `entity-ctxs` keyed by full actor DID-URL.
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

After a room commits entry, it sends `:did-ctx <did> <ctx>` to the full
`ctx.house` DID-URL. House stores the new ctx, then sends `:leave <did>` to a
different previously recorded parent. `:did-ctx? <did>` returns the stored ctx;
the lookup is open until world ACL policy is added. A room accepts targeted
`:leave <did>` only from exact `ctx.house`.

## Authority and transfer

`msg.from` authenticates the caller. Visible labels are lookup input only; after
resolution, actor workflows use full DID-URLs. A movable child requests
`:parent <ctx>` from a candidate parent, accepts `:child <ctx>`, commits its
parent, then informs the previous parent. Valid retries are idempotent.

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