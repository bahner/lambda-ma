# lambda-ma — Agent Notes

`lambda-ma` contains the actor source and bootstrap template for the lambda-ma
world. It is not runtime code; it publishes Scheme actor behaviours and a root
bootstrap YAML for `ma-runtime`.

## Agent rules

- Never modify files outside the current workspace without explicit user approval.
- Commit source files and templates, not generated `dist/`, `scheme-actor/target/`,
  or `scheme-actor/actor.wasm`.

## Focus routing contract with zion

Zion focus shorthand has a strict routing contract:

- Commands without a leading colon are avatar-mediated user commands and may be
  sent to the current avatar. Examples: `look`, `say hello`, `go north`,
  `dig east`.
- Commands with a leading colon are direct methods on the focused room/target
  and must not be handled by the avatar. Examples: `:prop name Garden`,
  `:prop description ...`, `:look`.

Actor code must preserve that boundary. Do not add avatar methods just to proxy
colon-prefixed room methods. If a colon-prefixed command fails from zion focus
mode, fix zion's routing or the room actor method, not the avatar.

## Scheme actor

The generic scheme actor lives in `scheme-actor/`. `Makefile` builds
`scheme-actor/actor.wasm`, publishes it and `scheme-actor/stdlib.ma`, then
substitutes those CIDs into `dist/lambda-ma.yaml`.

## Documentation contract (lambda-ma profile)

`lambda-ma` is one world/profile on top of `ma-runtime`, not the runtime spec
itself. Keep world semantics documented here, not in `ma-spec`, unless we later
decide to standardize them across multiple worlds.

- `REFERENCE.md` is the canonical protocol reference for lambda-ma world
  behavior.
- `README.md` and `HOWTO.md` are onboarding/operations docs and should link to
  `REFERENCE.md` for normative behavior.

When documenting or changing behavior, keep these contracts aligned:

- Focus routing boundary: plain commands are avatar-mediated; `:`-prefixed
  commands are direct room/target methods.
- Enter flow: room-first when a room target is known.
- Enter verbs: use one room verb `:enter` only (do not reintroduce
  `:enter-avatar`/`:enter-user`).
- Enter payload naming: one extensible map named `ctx` (not `attrs`) with
  required fields `kind`, `name`, `nick`, `description`.
- Enter kind routing: room `:enter` dispatch is kind-driven for ctx payloads.
  Missing kind or `ctx.kind = "avatar"` follows client/avatar entry; `ctx.kind`
  of `"thing"` or `"agent"` is categorized by room-local policy.
- Root actor boundary: root may create/find an avatar and ask that avatar to
  send its current ctx to the user, but root must not send messages to rooms.
- Avatar placement boundary: do not reintroduce generic avatar setter verbs such
  as `:set-location` or `:set-nick`. Room entry may notify an avatar with the
  narrow room-origin `:entered-room` event; nick changes are initiated by the
  avatar and accepted by the room.
- Authority model: room ownership is by user DID; avatars are delegates;
  parent authority governs `take`/`drop` flows.
- Transfer strictness (default): thing/agent transfer calls must keep strict
  input validation until explicitly relaxed:
  user must be `did:ma:...`, parent must be `did:ma:...` or `#fragment`, and
  optional transfer `ctx` must contain non-empty `kind`, `name`, `nick`,
  `description`.
