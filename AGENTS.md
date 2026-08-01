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
- Container ctx: containers use `/ma/container/0.0.1` and send
  `/ma/ctx/container/0.0.1` full contents snapshots to their current parent on
  contents changes. Parents may ignore these notifications. Avatar inventory is
  a configured container reference in avatar ctx, not a separate inventory kind
  or protocol; the avatar caches only valid, newer container ctx from that
  configured container.
- Avatar inventory lifecycle: avatars create/reuse a deterministic local
  `/ma/container/0.0.1` actor as inventory and publish it in avatar ctx as
  `inventory`. Carried actors should be parented to that container, not directly
  to the avatar. `take` routes to the inventory container as carrier parent;
  `drop`/`put` route through the inventory container as the current parent.
- Transfer strictness (default): thing/agent transfer calls must keep strict
  input validation until explicitly relaxed:
  controlling DID must be `did:ma:...`; non-ctx parent arguments must be DID-URLs.
  Optional transfer `ctx` must contain non-empty `kind`, `name`, `nick`,
  `description`. Any actor references inside ctx must be full DID-URLs.
