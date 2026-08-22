# lambda-ma - Agent Notes

`lambda-ma` publishes Scheme actor behaviours and a bootstrap template for the
lambda-ma world profile. It is not runtime code.

## Agent rules

- Never modify files outside this workspace without explicit user approval.
- Commit source and template files, never generated `dist/`,
  `scheme-actor/target/`, or `scheme-actor/actor.wasm`.
- Use British English for project-owned names and prose.
- Write DRY, KISS code: avoid duplicated logic and prefer the simplest
  implementation that meets the requirement.
- Do not modify `rust-ma-runtime` for world-profile work.

## Direct-DID profile

A bare DID in `msg.from` is the authenticated identity; no identity entity
exists.

- `#root` is the hardcoded local trust anchor. `:ctx?` publishes dynamic full
  DID-URL service references such as `#scheduler` and `#house`. `:enter?`
  gives unqualified-entry discovery: it always replies with a ctx naming a
  room to enter, defaulting to the configured `start` room. Only `#root` is
  required in a runtime — `#house` is optional and need not exist.
- `#house` owns world policy for DID ctx and entity ctx. It must not
  impersonate or forward commands for an identity.
- A client enters a known room directly with `:enter`; the room stores its
  DID-keyed child ctx and replies with its committed snapshot.
- Clients route focused room commands directly to the confirmed room DID-URL.
- `msg.from` is the sole authenticated fact. Ctx and revisions are not
  authentication; revisions only order authoritative snapshots and retries.
- Every node has exactly one authoritative child-ctx collection: `children`.
  This includes every room node and bare DID. Never add parallel child lists
  split by kind, lifecycle/state, inventory, occupancy, or another category.
  Bare DIDs have no identity entity, but their child ctx is still room state.
  Kind/state/presentation views must be derived by filtering `children`; a
  second source of truth invites divergence in parentage, ctx updates,
  persistence, and removal and is an interoperability risk.

## Props and ctx

Props are the authoritative stored state: `get-prop`/`set-prop!`/`del-prop!`.
Ctx is a derived, lazily-generated map — computed from the current prop values
at the moment it is needed and never stored separately.

`register-ctx-props!` is a **change-detection hook only**. It declares which
prop keys, when changed via `set-prop!`, should trigger `ctx-props-changed!`
so the ctx can be re-generated and broadcast. It does not create a separate
"ctx" storage layer.

Consequences:
- Never cache a ctx and mutate it directly — mutate the underlying props and
  let the ctx be re-derived.
- A function that returns a ctx (e.g. `room-ctx`, `node-ctx`) always calls
  `get-prop` internally; it is always fresh.
- `register-ctx-props!` is not about categorising props; it is about knowing
  when to push a fresh ctx to clients after a message is handled.

## RPC and events

Use `ma-reply!` for getters, setters, configuration, introspection, metadata,
validation, and errors. The runtime is an authoritative actor/data layer; it
never promises a client-side presentation surface. `:print` and other eventy
traffic are a Zion/visualisation concern: they are the client-side stream that a
human workstation consumes to show an in-world event such as arrival,
departure, speech, emotes, movement, and transfer. A runtime actor may answer a
technical RPC with `:ok`, but the user-visible event shape is not part of the
authoritative wire contract owned by the actor library.

## `tell` — generic avatar verb forwarding

`avatar.zscheme`'s `tell <target...> to <verb> [args...]` is a client-side
convenience command, not a new wire verb: it resolves `target` (via the same
`resolve-one` substring matching used by `put`/`claim`/`owner`, so a multi-word
description works, e.g. `tell golden duckie with peacock feathers to quack`)
and forwards `verb`/`args` as an ordinary RPC call via `actor-call`. `to` is a
hard separator, not optional — it is what lets the target description span
multiple words before the verb, exactly like `forge`'s `named`/`in` split.
There is no dedicated ACL capability or handshake for `tell`; the receiving
actor's own `set-cmd-method!`/`reply-error` decides whether to comply or
refuse, same as every other verb.

The resolver contract in this profile is deliberately layered. The zscheme
stdlib provides reusable helpers such as list folding, string checks, and
`unique-list`. The zscheme runtime library lives above it and provides
object-reference helpers such as `resolve-ref`, which flatten attribute-specific
scans like `resolve-name`/`resolve-nick`/`resolve-description` down to a list of
DIDs (deduped by `unique-list`). Its avatar library sits above that and
translates a human word into a single resolution or a caller-visible
ambiguity at the command boundary. It does not invent a new wire protocol; it
uses the runtime's object-reference result shape and forwards object movement
verbs to the runtime actor as ordinary RPC or `:set-parent` traffic.

All ordinary avatar object RPCs use `(command object method . params)`. That
single boundary resolves `object` over room `who`/`agents`/`things`/`exits`
plus inventory contents, accepts exactly one match, and reports no-match or all
ambiguous DID/DID-URL candidates. Do not duplicate resolver logic per verb.
`look <object>` uses the same room-plus-inventory candidate pool, but renders
the resolved child ctx locally rather than calling the target actor.

## Data over presentation (descriptive, not normative)

Actors do not print or format prose for query replies — that is a client's
job (zion or otherwise). An actor's job is to hand back quality data: a plain
ctx-map, built from the same simple DID/text/number attributes the spec
already uses for ctx. A curious teenager should be able to look at any reply
and understand its contents without decoding an ad hoc text format. Nothing
here is a hard limit — but the working expectation is that a typical ctx is
small, roughly on the order of ten keys, not a large or deeply nested
structure. Prefer returning existing ctx shapes (e.g. lists of child ctx's)
over inventing new bespoke reply shapes per verb.

## Documentation contract

The normative profile is `ma-spec/runtime/ma-lambda-ma-v1.md`. Keep it aligned
with this repository's `REFERENCE.md`, `README.md`, `HOWTO.md`, and actor
sources when changing interoperable behaviour.

## Object transfer and ownership

Object relocation is two wire verbs, both driving the same `:parent`/`:child`
handshake every actor already has (ma-spec §6), sent directly to the object
being moved (thing/container/agent alike — agents are ordinary
`/ma/node/0.0.1` nodes for this purpose):

- `:set-parent <target-parent-did-url> [ctx]` — the target is always a
  DID-URL (a room, a container, or another actor's own address). Shared
  implementation: `handle-node-set-parent!` (`scheme-actor/node.ma`);
  per-kind files only register it
  (`(set-cmd-method! :set-parent handle-node-set-parent!)`).
- `:hold` — the target parent is implicit (`msg.from`, a bare avatar DID,
  never a DID-URL) and takes no argument at all; this is the verb `hold`/
  `take`/`take-from` in `avatar.zscheme` actually send. Shared
  implementation: `handle-node-hold!` (`scheme-actor/node.ma`); registered
  on thing/container/agent only, never `room.ma` (rooms are not holdable).

`:hold` is deliberately ownership-blind: anyone (or anything) present in the
same room may hold any item regardless of who owns it, and holding never
assigns or changes ownership as a side effect. For an existing movable node,
only the explicit `:claim` verb may change ownership; `:forge` creates a new
node with `msg.from` as its initial owner. Its sole gate is a same-room check (`node-same-room-as-
parent?`) when the object's cached parent is a room: it looks up the caller
in that room's cached occupant list (`parent-ctx`'s `"who"` map) and, on a
mismatch, re-announces to the current parent to refresh the cache and
refuses so the caller can simply retry. That cache is not cryptographically
authoritative — `parent-ctx` is unauthenticated data the parent chose to
hand over (see "Parent-ctx caching" below) — but it is the whole requirement
for `:hold`, unlike `:set-parent`, which gates on
`node-transfer-caller-authorised?` alone.

**Parenting is not ownership.** Neither `:set-parent` nor `:hold` changes
`owner`. `handle-node-set-parent!`'s only authority check is
`node-transfer-caller-authorised?` (current parent, orphan-owner
recovery, owner delegation, or unowned) — it deliberately does **not** also
require `node-owner-or-unowned?`/current-owner-hood. Whoever currently
holds/carries a thing (i.e. is its parent) may relocate it further —
`drop`/`put` — regardless of who `node-owner` says owns it. You can be
carrying someone else's (or nobody's) property and still put it down or hand
it off. `:lock` and `:set-recovery-secret` require an existing owner; they do
not implicitly claim an unowned node. A prior revision of `handle-node-set-parent!` *did* also
require ownership there ("only owner may set-parent this actor"), which was
a bug: it made `:hold`'s deliberate ownership-blindness pointless, since a
non-owner could pick an owned item up but then could never legally put it
back down. Removed 2026-08-13 — do not re-add an ownership check to
`:set-parent`.

`give` is a client-side consent flow over the existing recovery-secret
contract, not a node `:owner` setter. The giver sets a one-time secret and sends
the intended recipient a plain-text `claim <full-object-did-url> <secret>`
command. Only the recipient's later authenticated `:claim` changes owner, and
successful claim clears the secret. Do not add unilateral ownership transfer
to ordinary movable nodes or automatically execute the offer message.

`:drop` is a distinct, room-only capacity pre-check (`handle-room-drop!` in
`actors/room.ma`), sent by the avatar to the room before object transfer
begins — it never itself relocates anything. Object relocation then uses the
ordinary `:set-parent <room> [ctx]` request, with the actor-provided ctx as
the authority. `put`/`put-in` likewise address the object directly with its
target parent and ctx; neither command requires a client-side hand slot or
queued follow-up.

### Parent-ctx caching

Every `:child <ctx>` message embeds the sender's own self-description as a
nested `"parent-ctx"` field (`child-ack-ctx` in `node.ma`) alongside the
ordinary child-naming fields the handshake already requires — the nested
field is the only place a richer parent ctx (e.g. a room's `who`/`agents`/
`things`) can travel, since the outer ctx's `actor`/`parent` fields must
always name the child and the ctx-issuing parent, never the parent's own
kind/contents. The receiving child caches only the nested map (`parent-ctx`,
`set-parent-ctx!`, `parent-kind` in `node.ma`), cleared (not left stale)
whenever a new parent's ack carries none (e.g. a bare avatar holder). A
parent whose own ctx changes pushes a fresh `:child` to every current child
(`broadcast-ctx-to-children!`, wired into the generic `ctx-props-changed!`
hook), so this cache does not go stale on its own; `room.ma`'s
`broadcast-room-ctx!` reuses it rather than duplicating the loop.

A ctx is heavy enough that every admission path sends exactly one `:child`,
never two: `room.ma`'s `handle-agent-enter!`/`handle-thing-enter!` never
craft their own ack — an unchanged re-entry acks once directly via
`send-fresh-child-ctx!`, and a changed one lets `broadcast-room-ctx!`'s sweep
(which by then already includes the newly claimed child) supply that same
ack as part of notifying every other current child. Never add a second,
hand-rolled `(list :child ctx)` send alongside a `broadcast-room-ctx!`/
`broadcast-ctx-to-children!` call — that duplication is exactly the kind of
thing the DRY rule above exists to catch.

Ownership/claim state (`owner`, `set-owner!`, `recovery-secret`,
`set-recovery-secret!`, `claim-key`, `set-claim!`, `owner-caller?`) and the
`:owner`/`:owner?`/`:set-recovery-secret`/`:claim` handler bodies
(`handle-node-owner`, `handle-node-owner?`, `handle-node-set-recovery-secret!`,
`handle-node-claim!`) live once in `node.ma` too — thing.ma/container.ma/
agent.ma only register them. `room.ma`'s own `:owner`/`:owner?`/`:claim` are
deliberately separate (no recovery-secret, simple first-claim/transfer) and
are not shared with node.ma. Before duplicating any cond-based handler body
across kind files, check whether it belongs in `node.ma` instead — that
duplication is exactly the kind of thing the DRY rule above exists to catch.

Transfer is carried by actor-provided `ctx` values and the ordinary
`:parent`/`:child` handshake. Transfer state is carried by the actor ctx, not
by a client-side queue. Zion only forwards typed events;
the composed zscheme layer acknowledges `:parent` with the received ctx and
issues ordinary `:hold` or `:set-parent` requests. `hold`/`take`/`take-from`,
`drop`/`put`, and `recycle-from` remain zscheme policy and must not gain a
hardcoded Zion dispatch path.

An avatar may request any object transfer directly. The actor ctx and
parent/child handshake carry the authoritative relationship; no local hand
slot or queued replacement is required.

## Scheme actor

`scheme-actor/` contains the generic Wasm host and common Scheme layers.
`Makefile` builds and publishes them, then substitutes CIDs into
`dist/lambda-ma.yaml`.