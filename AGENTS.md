# lambda-ma — Agent Notes

`lambda-ma` contains the actor source and bootstrap template for the lambda-ma
world. It is not runtime code; it publishes Scheme actor behaviours and a root
bootstrap YAML for `ma-runtime`.

## Agent rules

- Never modify files outside the current workspace without explicit user approval.
- Commit source files and templates, not generated `dist/`, `scheme-actor/target/`,
  or `scheme-actor/actor.wasm`.
- Use British English for project-owned naming and prose in code, actor verbs,
  docs, templates, and protocol fields. Prefer `behaviour`, `authorise`,
  `authorised`, `authorisation`, `initialise`, `initialised`, `serialise`,
  `colour`, and `licence`. Keep externally mandated API names unchanged, such
  as Rust/serde `Serialize`/`Deserialize`, CSS `color`, canvas `center`, and
  upstream crate or protocol names.

## Focus routing contract with zion

Zion focus shorthand has a strict routing contract:

- Commands without a leading colon are avatar-mediated commands and may be
  sent to the current avatar. Examples: `look`, `say hello`, `go north`,
  `dig east`.
- Commands with a leading colon are direct methods on the focused room/target
  and must not be handled by the avatar. Examples: `:prop name Garden`,
  `:prop description ...`, `:look`.

Actor code must preserve that boundary. Do not add avatar methods just to proxy
colon-prefixed room methods. If a colon-prefixed command fails from zion focus
mode, fix zion's routing or the room actor method, not the avatar.

## RPC replies vs avatar presentation

Actor methods must distinguish RPC replies from user-visible presentation by
semantics, not by caller shape.

Use `ma-reply!` for metadata, introspection, status, configuration, and
getter/setter commands. Examples: `:name`, `:description`, `:kind?`, `:owner`,
`:prop`, `:did?`, `:dids?`, alias/config lookups, and other commands whose
purpose is to return or mutate a value. Target actors must not answer these by
sending `:print`.

Use `:print` or room broadcast only for events caused by avatar commands: actual
in-world verbs such as looking, saying, emoting, entering, leaving, moving,
digging, taking, or dropping. These are presentation or world-event flows, not
plain metadata replies.

Avatar-specific code may turn replies or events into user-facing prints for
plain avatar-mediated UX. That presentation complexity belongs in the avatar or
proxy layer, not as a generic pattern in rooms, things, agents, or exits.

## Scheme actor

The generic scheme actor lives in `scheme-actor/`. `Makefile` builds
`scheme-actor/actor.wasm`, publishes it and `scheme-actor/stdlib.ma`, then
substitutes those CIDs into `dist/lambda-ma.yaml`.

## Documentation contract (lambda-ma profile)

`lambda-ma` is one world/profile on top of `ma-runtime`, not the runtime spec
itself. Keep world semantics documented here, not in `ma-spec`, unless we later
decide to standardize them across multiple worlds.

- `REFERENCE.md` is the canonical protocol reference for lambda-ma world
  behaviour.
- `README.md` and `HOWTO.md` are onboarding/operations docs and should link to
  `REFERENCE.md` for normative behaviour.

When documenting or changing behaviour, keep these contracts aligned:

- Focus routing boundary: plain commands are avatar-mediated; `:`-prefixed
  commands are direct room/target methods.
- Enter flow: room-first when a room target is known.
- Enter verbs: use one room verb `:enter` only (do not reintroduce
  split avatar/DID entry verbs).
- Enter payload naming: one extensible map named `ctx` (not `attrs`). Direct
  non-avatar entry requires fields `kind`, `name`, `nick`, `description`.
- Ctx trust anchor: only `msg.from` is authenticated. Treat every `ctx` field
  as untrusted until validated against `msg.from`. For avatar-sent ctx,
  `ctx.avatar` must equal `msg.from`; then verify `ctx.did` by deriving the
  deterministic avatar fragment for that DID in the sender avatar's runtime.
  Once that derivation matches, `ctx.did` has the same cryptographic authority
  as if the bare DID had been `msg.from`. Use the standard ctx validation helper
  before acting on received ctx.
- Actor references crossing actor, client, or runtime message boundaries must
  be full DIDs or DID-URLs. Do not send runtime-local `#fragment` shorthand in
  messages, ctx fields such as `root`, `avatar`, `room`, or future actor/path
  references. Actors may canonicalise old local shorthand while reading legacy
  state, but sibling privilege is a local policy over full DID-URLs.
- Visible labels, aliases, nicks, and user-facing names are lookup terms only;
  they must never be persistent map keys, authority keys, or actor identity.
  Persistent child, contents, occupant, inventory, and visibility maps must be
  keyed by the canonical full actor DID-URL (`did:ma:...#fragment`).
- Cross-runtime movement must not admit the source-runtime avatar into the
  target room. The target room creates or reuses the target-runtime deterministic
  avatar for the controlling DID, and uses the source avatar only for old-room
  cleanup.
- Enter kind routing: room `:enter` dispatch is kind-driven for ctx payloads.
  Missing kind is room-local default avatar entry: the room creates or finds
  the deterministic avatar, asks an existing avatar to `:enter-room`, and must
  not reply `:ok` itself; `ctx.kind = "avatar"` follows the same room-local
  avatar entry flow; `ctx.kind` of `"thing"` or `"agent"` is categorized by
  room-local policy.
- Root actor boundary: root may create/find an avatar and ask that avatar to
  send its current ctx to the controlling DID, but root must not send messages
  to rooms.
- Avatar placement boundary: do not reintroduce generic avatar setter verbs such
  as `:set-location` or `:set-nick`. Root or the target room may ask an existing
  avatar to enter that room with narrow `:enter-room`; the avatar persists room
  state only after the room sends committed ctx back.
- Authority model: room ownership is by bare DID; avatars are delegates;
  parent authority governs `take`/`drop` flows.
- Parent/child role verbs: method names describe the receiver's role, not the
  sender's role. A child actor asks a candidate or current parent to act as
  parent by sending `<parent>:parent <ctx>`. A parent asks or informs a child by
  sending `<child>:child <ctx>`. For drop/reparenting, a carried actor sends
  `<new-parent>:parent <desired-ctx>`; the new parent confirms with
  `<actor>:child <committed-ctx>`; the actor accepts only a matching ctx where
  `ctx.parent == msg.from`, commits the new parent, then sends a courtesy
  `<old-parent>:parent <departure-or-new-parent-ctx>` so the old parent can
  remove or update the ctx record it holds for that actor. Do not reverse these
  verbs. Parent assignment is idempotent: when a child receives a valid
  `:child <ctx>` naming its already-current parent, it must still send its
  authoritative ctx back to that parent with `:parent`. Treat the repeated
  confirmation as a successful commit; do not suppress the ctx reply merely
  because the parent value did not change.
- Hewitt actor delivery and retries: actors cannot infer whether an earlier
  message or reply was delivered. Lost messages and repeated requests are
  normal protocol conditions, not evidence of misuse. Valid ctx and
  state-setting requests must therefore be idempotent: answer every repetition
  patiently from current authoritative state, including when it only restates
  an already-committed value. Do not reject an authenticated repeat as
  redundant, and do not add delivery tracking, deduplication logs, pending
  commands, or retry counters merely to remember the workflow. A repeated
  request repairs the other actor's knowledge by receiving the same current ctx
  again.
- Container ctx: containers use `/ma/container/0.0.1` and have one specified
  container ctx, including contents. Containers send refreshed container ctx to
  their current parent with `:parent` on contents or presentation changes. Do
  not split container ctx into movable-child vs contents-snapshot forms, and do
  not send parent-facing container ctx with `:ctx`. Avatar inventory is a
  configured container reference in avatar ctx, not a separate inventory kind or
  protocol; the avatar caches only valid, newer container ctx from that
  configured container.
- Actor ctx propagation: whenever an actor changes a field present in its
  parent-facing ctx, such as `parent`, `name`, `nick`, or `description`, it must
  send refreshed ctx to its current parent so the parent's authoritative ctx
  records stay current.
- Functional, stateless ctx flow: actor workflows must pass ctx directly through
  messages and complete from the ctx currently being handled. Do not persist,
  queue, or replay commands or intermediate workflow state such as `pending-take`,
  pending transfer payloads, deferred drops, or resolver results. Persistent
  actor state is reserved for authoritative facts that must survive messages,
  such as ownership, committed parentage, configuration, and accepted ctx
  records. Lifecycle readiness belongs in actor initialisation and protocol
  handshakes, never in a saved user command.
- Resolve once, then use actor identity: `name`, `nick`, aliases, directions,
  and other visible labels may be accepted only as resolver input. A successful
  resolver must immediately produce a canonical full DID or DID-URL. All later
  messages, ctx references, authority checks, membership changes, cleanup, and
  persistent keys must use that DID or DID-URL, never the lookup term.
- Authoritative ctx over maintained lists: actors must not maintain separate
  persistent membership or presentation lists when those lists can be derived
  from authoritative ctx records. Lists such as room occupants, `who`, container
  contents, visible children, inventories, and similar presentation/lookup
  surfaces must be generated on demand from the stored ctx records that already
  define parentage, kind, actor DID-URL, nick/name, and other authority-bearing
  fields. Mutations should add/update the relevant ctx or remove that ctx; they
  must not append/remove bare actor refs in a second list that can drift out of
  sync.
- Avatar inventory lifecycle: avatar ctx carries `inv` as a baton during
  `:go`/`:enter`. Target-runtime avatars adopt that supplied container
  reference when present, and create/reuse a deterministic local
  `/ma/container/0.0.1` only when no inventory has been configured yet. Carried
  actors should be parented to that container, not directly to the avatar.
  `take` uses the inventory container as carrier parent; visible take resolves
  the child from room ctx and asks the child directly, while carried-source take
  asks the explicit source parent. Carried `drop` must follow the parent/child
  ctx algorithm above: the carried actor requests the target parent with
  `:parent <ctx>`, the target parent confirms with `:child <ctx>`, and the old
  inventory container removes or updates its stored ctx record from the actor's
  committed ctx update. Avatar carried `drop` must not invent a new verb, call the current
  room, or use an avatar room-helper. Rooms must not expose `:take` or `:drop`;
  the current room DID-URL may be passed only as target-parent data.
- Transfer strictness (default): thing/agent transfer calls must keep strict
  input validation until explicitly relaxed:
  controlling DID must be `did:ma:...`; non-ctx parent arguments must be DID-URLs.
  Optional transfer `ctx` must contain non-empty `kind`, `name`, `nick`,
  `description`. Any actor references inside ctx must be full DID-URLs.
