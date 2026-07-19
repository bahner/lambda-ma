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