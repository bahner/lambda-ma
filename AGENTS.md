# lambda-ma - Agent Notes

`lambda-ma` publishes Scheme actor behaviours and a bootstrap template for the
lambda-ma world profile. It is not runtime code.

## Agent rules

- Never modify files outside this workspace without explicit user approval.
- Commit source and template files, never generated `dist/`,
  `scheme-actor/target/`, or `scheme-actor/actor.wasm`.
- Use British English for project-owned names and prose.- Write DRY, KISS code: avoid duplicated logic and prefer the simplest
  implementation that meets the requirement.- Do not modify `rust-ma-runtime` for world-profile work.

## Direct-DID profile

A bare DID in `msg.from` is the authenticated identity; no identity entity
exists.

- `#root` is the hardcoded local trust anchor. `:ctx?` publishes dynamic full
  DID-URL service references such as `#scheduler` and `#house`.
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

Object relocation is one wire verb, `:set-parent <target-parent-did-url>
[ctx]`, sent directly to the object being moved (thing/container/agent alike
— agents are ordinary `/ma/node/0.0.1` nodes for this purpose). It triggers
the same `:parent`/`:child` handshake every actor already has (ma-spec §6);
there is no separate `:take`/`:drop`/`:put-in`/`:take-from`/`:recycle-from`
wire vocabulary. `handle-node-set-parent!` (`scheme-actor/node.ma`) is the one
shared implementation; per-kind files only register it
(`(set-cmd-method! :set-parent handle-node-set-parent!)`).

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

`hold` — the client-side confirm/clear half of `:set-parent` (a single-slot
"what am I currently the parent of" pointer, `.my.ctx.hold`/
`.my.ctx.hold-pending`/`.my.ctx.hold-then`) is not part of this repository. It
is implemented in `ma-zion`'s `src/inbox_poll.rs`
(`handle_hold_parent_proposal`) and documented in that repository's AGENTS.md
under "Hold — client-side object-transfer state". `avatar.zscheme`'s
`hold`/`take`/`drop`/`put`/`take-from`/`recycle-from` only send `:set-parent`
and set the pending pointer; they never confirm on their own.

## Scheme actor

`scheme-actor/` contains the generic Wasm host and common Scheme layers.
`Makefile` builds and publishes them, then substitutes CIDs into
`dist/lambda-ma.yaml`.