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
- A client enters a known room directly with `:enter`; the room stores a
  DID-keyed presence record and replies with its committed snapshot.
- Clients route focused room commands directly to the confirmed room DID-URL.
- `msg.from` is the sole authenticated fact. Ctx and revisions are not
  authentication; revisions only order authoritative snapshots and retries.
- Node child records stay keyed by full actor DID-URLs. DID-keyed presence is
  room state, not a child actor.

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

The resolver contract in this profile is deliberately layered. The generic
stdlib lives at the evaluator/compiler level and provides reusable helpers such
as list folding, string checks, and `unique-list`. The lambda-ma runtime domain
lives higher up in the runtime library surface (`runtime.zscheme`): it provides
object-reference helpers such as `resolve-ref`, which flatten attribute-specific
scans like `resolve-name`/`resolve-nick`/`resolve-description` down to a list of
DIDs (deduped by `unique-list`). The avatar layer (`avatar.zscheme`) sits above
that and translates a human word into a single resolution or a caller-visible
ambiguity at the command boundary. It does not invent a new wire protocol; it
uses the runtime's object-reference result shape and forwards object movement
verbs to the runtime actor as ordinary RPC or `:set-parent` traffic.

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
assigns or changes ownership as a side effect — only the explicit `:claim`
verb does that. Its sole gate is a same-room check (`node-same-room-as-
parent?`) when the object's cached parent is a room: it looks up the caller
in that room's cached occupant list (`parent-ctx`'s `"who"` map) and, on a
mismatch, re-announces to the current parent to refresh the cache and
refuses so the caller can simply retry. That cache is not cryptographically
authoritative — `parent-ctx` is unauthenticated data the parent chose to
hand over (see "Parent-ctx caching" below) — but it is the whole requirement
for `:hold`, unlike `:set-parent`, which gates on
`node-transfer-caller-authorised?` alone.

**Parenting is not ownership.** `handle-node-set-parent!`'s only authority
check is `node-transfer-caller-authorised?` (current parent, orphan-owner
recovery, owner delegation, or unowned) — it deliberately does **not** also
require `node-owner-or-unowned?`/current-owner-hood. Whoever currently
holds/carries a thing (i.e. is its parent) may relocate it further —
`drop`/`put` — regardless of who `node-owner` says owns it. You can be
carrying someone else's (or nobody's) property and still put it down or hand
it off; only `:claim`/`:lock`/`:owner`/`:set-recovery-secret` are genuinely
ownership-gated. A prior revision of `handle-node-set-parent!` *did* also
require ownership there ("only owner may set-parent this actor"), which was
a bug: it made `:hold`'s deliberate ownership-blindness pointless, since a
non-owner could pick an owned item up but then could never legally put it
back down. Removed 2026-08-13 — do not re-add an ownership check to
`:set-parent`.

`:drop` is a distinct, room-only capacity pre-check (`handle-room-drop!` in
`actors/room.ma`), sent by the avatar to the room *before* the held item's
own unchanged `:set-parent <room>` call — it never itself relocates
anything. `avatar.zscheme`'s `drop` calls it first and lets a refusal raise
before `:set-parent` is ever sent. `drop` takes an *optional* name, resolved
against both the hand (whatever `.my.ctx.hold` currently holds, using the
ctx cached client-side at `hold`/`take` time) and your own inventory
contents — `resolve-in-given-pool` over `drop-pool`, not the fixed room+
inventory pools `resolve-ref` always searches. With no name it acts on
whatever is in hand, same as before. `put`/`put-in` (arbitrary container
targets) are unaffected and still act on whatever is in hand only; containers
have no equivalent capacity gate yet.

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

`hold` — the client-side confirm/clear half of the resulting `:parent`
proposal (a single-slot "what am I currently the parent of" pointer,
`.my.ctx.hold`/`.my.ctx.hold-pending`/`.my.ctx.hold-then`) is not part of this
repository. It is implemented in `ma-zion`'s `src/inbox_poll.rs`
(`handle_hold_parent_proposal`) and documented in that repository's AGENTS.md
under "Hold — client-side object-transfer state". `avatar.zscheme`'s
`hold`/`take`/`take-from` send `:hold` (implicit target); `drop`/`put` send
`:set-parent`; `recycle-from` sends `:recycle`. None of them confirm the
resulting `:parent` proposal on their own — that is `inbox_poll.rs`'s job.

## Scheme actor

`scheme-actor/` contains the generic Wasm host and common Scheme layers.
`Makefile` builds and publishes them, then substitutes CIDs into
`dist/lambda-ma.yaml`.