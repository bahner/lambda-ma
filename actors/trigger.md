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
- The fragment is ordinary and random, prefixed with its short kind
  (`trigger-…`, like every newly created entity); reconnection and orphanhood
  use the ordinary child lifecycle (`:phone-home` / re-announce).
- The kind is invisible to `who?` / `occupants?` / `things?` / `exits?` —
  those views filter `children` by kind — while the actor still receives every
  room broadcast like any other child. The trigger also defaults to
  `visible: false`, and a child with `visible: false` is never published at
  all: the room excludes it from `:look`'s children map, from the nested
  parent-ctx snapshots, and from every broadcast, so no client ever learns of
  it. The room owner may override with `:prop visible true`, which makes the
  trigger appear in `look`'s "Triggers:" section and in client-side name
  resolution while the kind-filtered views above still exclude it. The owner
  can lock it in place with the general `fixed: true` prop once it is
  deployed.

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

### Codeable Scheme API

The wire verbs are thin wrappers over the same codeable Scheme functions,
which bespoke trigger behaviour can call directly (composed after
`trigger.ma`). Every call funnels through `trigger-count`, so a direct
update evaluates rules exactly like a wire verb:

| Function | Effect |
| --- | --- |
| `(trigger-count <counter> <did> <delta> <msg>)` | Add `delta` (positive or negative) to `counters[<did>][<counter>]`, evaluate rules, record the last updater. |
| `(trigger-set <counter> <did> <value> <msg>)` | Set an absolute value (override a false or unfair value). |
| `(trigger-reset! <counter> <did>)` | Delete / zero the counter. |

`<msg>` is the message that caused the update: its timestamp is "now" and
its sender is the causer when the key is the room's own counter. There is
no wall clock in ma-scheme, so a caller without a message passes `#f`
(a `within` rule needs a real timestamp to compare).

## Automatic counting

The trigger actor is a child of the room, so it already receives every room
broadcast (`:arrive`, `:leave`, `:say`, `:emote`, `:dig`, `:fill`). Each event
with a subject DID increments `counters[<subject-did>][<event-name>]`
automatically — no room-side wiring. Subject-less events (for example
`:narrate` — the trigger's own output channel) are deliberately not counted,
which also prevents a fired narration from re-triggering rules.

Manual `:count`, automatic counting, and direct calls from bespoke trigger
behaviour are the ways a counter changes; all funnel through `trigger-count`.

## Trigger rules

A rule is stored in the `triggers` prop. The wire form is flat and
positional — no rule map crosses the interface:

```
:trigger <id> <counter> [did <did>] [once|count N|every N|within S] <method> <text...>
```

| Field | Meaning |
| --- | --- |
| `id` | Unique rule identifier (required). |
| `<counter>` | Counter name to watch, e.g. `:arrive` or `num_jokes` (required). Automatic counters use the event name verbatim. |
| `did` | Counter scope. Absent = per identity (each DID's own counter, each tracked separately); a DID = that identity only; the room's DID = the room's (general) counter. |
| `once` | Fire on the first qualifying update. |
| `count <N>` | Fire once when the counter reaches `N`. |
| `every <N>` | Fire on every crossing of a new multiple of `N`; an absent condition defaults to `every 1` (fire on every update). |
| `within <S>` | Fire when a *different* DID updates the same counter within `S` seconds of the previous one (simultaneity). |
| `<method>` | The verb sent to the parent node with the rendered text as its argument, e.g. `:narrate`, `:narrate_to` (sent as `:narrate-to`, targeted at the concerning DID), `:lock`. |
| `<text...>` | The words: literal strings and `(...)` expression spans (see Template expressions). |

Counters arrive as symbols (`:arrive`) or strings and are normalised to
strings; method names are normalised (`_` → `-`). Condition counts (`N`,
`S`) may arrive as a number or a numeric string — clients send RPC
arguments as CBOR text, so `every 1` delivers `"1"` — and are normalised
to numbers. The automatic counters are keyed by the room's event names
verbatim (`":arrive"`, `":leave"`, `":say"`, `":emote"`, `":dig"`,
`":fill"`). `:leave` counts like any other event and prunes nothing, so a
departing DID's counters and rule state persist: arrival totals survive a
visit and per-DID rules keep their history across sessions.

| Verb | Effect | Gate |
| --- | --- | --- |
| `:triggers` | Reply with the rule list (data). | owner |
| `:trigger <id> <counter> [did <did>] [condition] <method> <text...>` | Add or replace a rule by `id`. | owner |
| `:trigger <id>` | Remove a rule. | owner |

### The DID the trigger concerns

A trigger always concerns an actor DID — never the room. For per-identity and
explicit-DID rules that is the rule's scope DID. For a rule scoped to the
room's own counter it is the actor whose update caused the fire (the causer).
`narrate-to` targets that DID.

## Template expressions

The `<text...>` is a list of parts: literal strings (carrying their own
spacing and punctuation) and expression forms `(:fn arg ...)`. Parts render
at fire time (spans evaluated against the concerning DID) and are
**concatenated**, so prose like `"…room, " (:did-name) "!"` glues naturally.
The expression vocabulary is closed and side-effect-free (ma-scheme
deliberately has no `eval`, so the grammar is bounded and structurally
validated at setup). Zion never evaluates the text — it only translates the
human prose-with-spans into parts; the actor has the counters.

| Binding | Meaning |
| --- | --- |
| `(did)` | The DID the trigger concerns. |
| `(did-name)` | That DID's `nick` → `name` → `did` (resolved from the cached room snapshot). |
| `(did-attr "<key>")` | Any ctx attribute of that DID, e.g. `(did-attr "description")`; `""` if absent. |
| `(did-counter "<name>")` | `counters[<that DID>][<name>]`, 0 if absent. |
| pure builtins | arithmetic, strings, lists, `if`/`cond`. |

The environment has **no side effects**: no `ma-*` primitives, no
`set-prop!`/`del-prop!`, no `ipfs-*`/`include`. An expression can only compute
text. There is no room binding — the room delivers the text verbatim and does.
Symbol arguments (`:arrive`) read as their string form; glued punctuation
uses `(:string-append …)`.

Example (automatic counters use the room's event names verbatim, so an
arrival is `:arrive`):

```
:trigger enter-1 :arrive once :narrate_to
  "This is the first time you enter this room, " (:did-name) "!"
```

fires on the first arrival and sends to the room:

```
(:narrate-to <did> "This is the first time you enter this room, Lars!")
```

`:trigger` validates the rule immediately: spans are checked against the known
vocabulary so a malformed rule is rejected at setup, not at fire time.
Non-terminating expressions are the owner's responsibility — they are the one
accepted risk, since only the owner creates rules.

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

- Rule removal prunes the rule's trigger state.
- `once` rules prune their own state after firing.
- Counters and trigger state persist across departures: `:leave` counts like
  any other event and prunes nothing, so arrival totals and per-DID rule
  state survive a visit. Stale entries can be removed by hand with
  `:reset <did> <name>` (owner, or the DID itself).
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
