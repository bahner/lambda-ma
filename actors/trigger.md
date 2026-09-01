# Trigger actor protocol — `/ma/trigger/0.0.1`

Per-identity counters and event-reactive narration. One actor per room
listens to the room's event stream, keeps per-DID counters, evaluates
declarative rules on every counter update, and asks the room to narrate when a
rule fires. The room owns its voice; the trigger actor is a sensor, never a
mouth.

## Kind

```yaml
# kinds/ma-scheme-trigger.yaml
protocol: /ma/trigger/0.0.1
extends: /ma/node/0.0.1
behaviour:
  "/": __TRIGGER_BEHAVIOUR_CID__
```

No counter or narrator kind exists; a future split into a counter base kind is
possible without a protocol change and is deliberately not done now (KISS).

## Deployment and lifecycle

- One actor per room, forged by the room owner (`forge /ma/trigger/0.0.1 named
  <name>`) and placed in the room with the ordinary `:set-parent` / `:hold`
  flow (forge always births into the forger's hand, never into a room).
- The room owner then designates it with the room prop
  `:prop trigger <trigger-did-url>` — a single full DID-URL. This is the trust
  anchor: only the designated trigger may narrate, so arbitrary trigger-kind
  actors cannot be forged and given a voice in a room.
- The fragment is ordinary and random; reconnection and orphanhood use the
  ordinary child lifecycle (`:phone-home` / re-announce).
- The kind is invisible to `who?` / `occupants?` / `things?` / `exits?` —
  those views filter `children` by kind — while the actor still receives every
  room broadcast like any other child.

## State

All state is authoritative props on the trigger actor, persisted with
`ma-save-state!`.

```
prop "counters"      → { <did>: { <name>: <int> } }        ; DID is the map key, never a field
prop "counter-last"  → { <name>: { did, at } }             ; last updater per counter (within)
prop "triggers"      → [ rule, … ]
prop "trigger-state" → { <trigger-id>: { <did>: { last-value, fired-at } } }
```

- The room dimension is implicit — it is the trigger actor's parent — so the
  structure is portable.
- The DID is always the key: every actor has a DID-URL, and the room is an
  actor, so `counters[<room-did>][name]` is the natural general (non-personal)
  counter. No reserved sentinel, no second structure.
- Arbitrary non-per-DID values belong in plain state, not counters.
- Time is `msg-created-at` — every message carries its timestamp; there is no
  wall clock.

## Counter verbs

| Verb | Effect | Gate |
| --- | --- | --- |
| `:count <did> <name> <delta>` | Add `delta` (positive or negative) to `counters[did][name]`, creating it as 0 first. | caller is `did`, or the room owner; for the room's own DID: any child present in the room, or the owner |
| `:set <did> <name> <value>` | Set an absolute value (override a false or unfair value). | room owner only |
| `:reset <did> <name>` | Delete / zero the counter. | caller is `did`, or the room owner; room DID: any child present, or the owner |
| `:value <did> <name>` | Reply with the current integer. | any child of the room |
| `:counters [<did>]` | Reply with the whole map `{ <did>: { <name>: <int> } }`; optional `<did>` filters. | any child of the room |

The room owner is verified by asking the parent room `:owner?`; the answer is
cached in the trigger actor's state.

## Automatic counting

The trigger actor is a child of the room, so it already receives every room
broadcast (`:arrive`, `:leave`, `:say`, `:emote`, `:dig`, `:fill`). Each event
with a subject DID increments `counters[<subject-did>][<event-name>]`
automatically — no room-side wiring. Subject-less events (for example
`:narrate`) count under the room's own DID, so every event has a subject.

Manual `:count` and automatic counting are the two ways a counter changes;
both funnel into the same update path.

## Trigger rules

A rule is a declarative map in the `triggers` prop:

```
{ id, on <counter-name>, [did], <condition>, <narrate|narrate-to> <text> }
```

| Field | Meaning |
| --- | --- |
| `id` | Unique rule identifier (required). |
| `on` | Counter name to watch, e.g. `":say"`, `"num_jokes"` (required). Automatic counters use the event name verbatim. |
| `did` | Counter scope. Absent = per identity (each DID's own counter, each tracked separately); a DID = that identity only; the room's DID = the room's (general) counter. |
| `once` | Fire on the first qualifying update. |
| `count <N>` | Fire once when the counter reaches `N`. |
| `every <N>` | Fire on every crossing of a new multiple of `N`. |
| `within <S>` | Fire when a *different* DID updates the same counter within `S` seconds of the previous one (simultaneity). |
| `narrate <text>` | Ask the room to broadcast the text. |
| `narrate-to <text>` | Ask the room to send the text to the DID the trigger concerns. |

Exactly one condition field and exactly one action field are present. The
action value is a plain string or a parts list (see Template expressions).
The automatic counters are keyed by the room's event names verbatim
(`":say"`, `":arrive"`, `":leave"`, `":emote"`, `":dig"`, `":fill"`);
`":leave"` prunes the departing DID instead of counting it.

| Verb | Effect | Gate |
| --- | --- | --- |
| `:triggers` | Reply with the rule list (data). | owner |
| `:trigger <rule-map>` | Add or replace a rule by `id`. | owner |
| `:trigger <id>` | Remove a rule. | owner |

### The DID the trigger concerns

A trigger always concerns an actor DID — never the room. For per-identity and
explicit-DID rules that is the rule's scope DID. For a rule scoped to the
room's own counter it is the actor whose update caused the fire (the causer).
`narrate-to` targets that DID.

## Template expressions

The action is either a plain string or a list of parts: literal strings and
expression forms `(:fn arg ...)` evaluated at fire time in a closed,
side-effect-free vocabulary (ma-scheme deliberately has no `eval`, so the
grammar is bounded and structurally validated at setup). The zscheme avatar
layer owns any human-facing prose syntax with parenthesised spans and
translates it to this structured form; the actor stores and renders data, not
text that must be parsed.

| Binding | Meaning |
| --- | --- |
| `(did)` | The DID the trigger concerns. |
| `(did-name)` | That DID's `nick` → `name` → `did` (resolved from the cached room snapshot). |
| `(did-attr "<key>")` | Any ctx attribute of that DID, e.g. `(did-attr "description")`; `""` if absent. |
| `(did-counter "<name>")` | `counters[<that DID>][<name>]`, 0 if absent. |
| pure builtins | arithmetic, strings, lists, `if`/`cond`. |

The environment has **no side effects**: no `ma-*` primitives, no
`set-prop!`/`del-prop!`, no `ipfs-*`/`include`. An expression can only compute
text. There is no room binding — the room may deliver the text verbatim and
does.

Example (structured parts; automatic counters use the room's event names
verbatim, so an arrival is `":arrive"`):

```
{ id: "farvel", on: ":arrive", every: 10,
  narrate-to: ["Kjære ", (:did-name), ", du har vært her ",
               (:did-counter ":arrive"), " ganger. Husk å si pent farvel når du drar."] }
```

`:trigger <rule>` validates the rule immediately: spans are parsed and
dry-run-evaluated so a malformed template is rejected at setup, not at fire
time. Non-terminating expressions are the owner's responsibility — they are
the one accepted risk, since only the owner creates rules.

## Evaluation

Every counter update — automatic or manual — runs `evaluate-triggers!` for
rules watching that counter name, in rule order. "Now" is the `msg-created-at`
of the message that caused the update. Bookkeeping is keyed per
`(trigger, did)` so each identity is treated separately:

- `once` / `count N`: fire when the counter first crosses the threshold and
  `fired-at` is absent.
- `every N`: fire when `last-value // N < value // N`.
- `within S`: fire when a different DID updated the same counter within `S`
  seconds (via `counter-last`), then update `counter-last`.

Trigger-generated narration is marked with the `trigger-notifying?` guard
(mirroring `ctx-prop-notifying?` in `state.ma`) so it does not re-evaluate
triggers.

## Room narration verbs

The trigger actor never sends events to occupants as itself; all actions go to
the parent room, which delivers:

| Verb | Delivery |
| --- | --- |
| `:narrate <words...>` | `(join-words words)` → broadcast `(:narrate <text>)` to every child. |
| `:narrate-to <did> <words...>` | First argument is the target DID, the rest are joined → send `(:narrate <text>)` to that one child. |

Both are subject-less ambient events (`(:narrate <text>)`, rendered verbatim
by zscheme's `event-narrate`), so no actor name ever appears. The `:narrate`
gate accepts: the room owner, the runtime `#scheduler`, or the designated
trigger — the actor named by the room's `trigger` prop, provided it is still
present as a child of the room. The scheduler entry also makes time-based
narration possible without any runtime change.

## Garbage collection

- `:leave` events prune the leaving DID's counters and trigger state.
- Rule removal prunes the rule's trigger state.
- `once` rules prune their own state after firing.
- Long-lived DIDs accumulate one entry per counter name; when that becomes a
  real problem the counter encoding can be made denser without a protocol
  change.

## Deliberately out of scope

- Arbitrary non-per-DID values: plain state, not counters.
- Time-based scheduling and randomness: the runtime `#scheduler` + room
  `:narrate`, not counter triggers.
- Non-narration actions (unlocking, moving, sending verbs): later, and any
  action still flows through the room so the room's gates stay the single
  authority.
- Exits: same engine later, fed by explicit forwards (exits do not broadcast).
