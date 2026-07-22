# λ-間 HOWTO

This guide starts from a fresh machine and ends with a running λ-間 world that
you can enter from zion, edit, fork, and reset.

λ-間 is a seed world for `ma-runtime`. It is similar in spirit to
`lambdamoo-core`: a small functional base that gives a new world rooms, exits,
avatars, and a root actor. You are expected to change it. The shipped world is
not sacred; it is a starting point for your own runtime.

For normative world protocol behavior (routing split, room/enter contract,
actor verbs, ownership and parent-authority rules), use
[REFERENCE.md](REFERENCE.md).

## What you need

You need three things:

- Kubo / IPFS stores actor source, Wasm, runtime manifests, DID documents, and
  room state as content-addressed IPFS/IPLD data.
- `ma` runs your actor runtime, loads the λ-間 bootstrap, and serves zion
  locally.
- λ-間 is the seed actor set and bootstrap YAML for a simple programmable
  world.

Download Kubo/IPFS:

- IPFS Desktop, easiest for new users: <https://docs.ipfs.tech/install/ipfs-desktop/>
- Kubo command-line install: <https://docs.ipfs.tech/install/command-line/>

Download `ma`:

- `ma-runtime` releases: <https://github.com/bahner/ma-runtime/releases>

Kubo must be running before you build or bootstrap λ-間. The `make` process uses
`ipfs add` to publish actor files and Wasm to your local node. The `ma` runtime
also uses Kubo to publish the runtime manifest and DID documents. If Kubo is not
running, the build may fail, and `ma --bootstrap` / `ma --gen-root-cid` cannot
publish the world.

## Install the local build tools

Install Rust and the Wasm target. The λ-間 bootstrap includes a generic
ma-scheme actor compiled to WebAssembly.

```sh
rustup target add wasm32-unknown-unknown
```

Make sure these commands exist:

```sh
ma --help
ipfs --version
```

If you installed IPFS Desktop, the `ipfs` command may already be available. If
not, install Kubo from the command-line link above or add the bundled `ipfs`
binary to your `PATH`.

## Generate `lambda-ma.yaml`

From the `lambda-ma` repository:

```sh
make clean
make
```

This does four things:

1. Builds `scheme-actor/actor.wasm`.
2. Publishes `scheme-actor/actor.wasm` to IPFS.
3. Publishes `scheme-actor/stdlib.ma` and `actors/*.ma` to IPFS.
4. Writes the generated bootstrap file to `dist/lambda-ma.yaml`.

Check that the generated YAML has no unresolved placeholders:

```sh
make check
```

You can inspect the generated file:

```sh
less dist/lambda-ma.yaml
```

Do not edit `dist/lambda-ma.yaml` by hand. Edit `lambda-ma.template.yaml`,
`actors/*.ma`, or `scheme-actor/`, then run `make` again.

## Option A: bootstrap and start `ma` directly

This is the shortest path when you just want to run the world now:

```sh
ma --bootstrap dist/lambda-ma.yaml
```

`--bootstrap` publishes the manifest described by `dist/lambda-ma.yaml`, uses
the resulting root CID for this runtime start, then continues running `ma`.

If you copy `dist/lambda-ma.yaml` somewhere else, pass that path instead:

```sh
ma --bootstrap lambda-ma.yaml
```

The flag is `--bootstrap`, not `--bootstrapo`.

When `ma` is running, open zion from the local runtime:

<http://localhost:5003/zion>

Create or unlock your zion identity, then claim your local runtime if you have
not already done so:

```text
.my.ma:claim
```

Then enter the λ-間 world:

```text
.ma!discover
@ma/config/root: @ma#root
.enter @ma
```

## Option B: generate a reusable root CID

Use this when you want a stable starting point that can be reused without
copying the YAML file around.

```sh
make root-cid
```

This is a convenience wrapper around:

```sh
ma --gen-root-cid dist/lambda-ma.yaml
```

It publishes the λ-間 bootstrap to IPFS, prints one CID, and exits. That CID is
the root of the generated runtime manifest.

Example:

```text
bafyreib...
```

Start `ma` from that root CID:

```sh
ma --root-cid bafyreib...
```

This resets the runtime head to that world root for the start. The runtime then
persists the selected root in its config as it runs. If you later want to reset
back to the same fresh λ-間 starting point, stop `ma` and start it again with the
same root CID:

```sh
ma --root-cid bafyreib...
```

This is useful for experiments. You can build rooms, break things, learn, and
then return to a known clean seed world.

Keep the printed root CID somewhere convenient, for example in a notes file:

```text
lambda-ma fresh root: bafyreib...
```

## Enter and change your first room

After zion has entered the world, you are in `#construct`, the initial room.
Start with:

```text
look
help
help here
```

Claim the room so you can edit it:

```text
claim
```

Set a name and description. Colon-prefixed commands go directly to the focused
room instead of through your avatar:

```text
:prop name The Workshop
:prop description A bright room with cables on the floor and a half-built door in the north wall.
look
```

Create a new room to the north:

```text
dig north to The Gallery
go north
look
```

The new room is owned by you automatically. Change it too:

```text
:prop description A long quiet gallery. Every wall is waiting for a better idea.
```

Move back:

```text
go south
```

Quick regression check for room presence rendering:

```text
look
```

Expected shape in output:

- An `Occupants:` line that includes live room occupants, including avatar
  presence and agents such as `rms` after those agents have entered the room.
- A separate `Things:` line for room-local non-avatar aliases.

Use `who?` when you specifically mean people/avatar presence. Use `occupants?`
when you want the broader room occupant list.

If `look` only shows `Things: none.` or misses expected occupants while actors
are clearly active in the room, the room behavior is stale. Rebuild and
re-bootstrap (see troubleshooting below).

Useful room-building commands:

```text
help                   show avatar/user commands
help here              show what this room says is possible
claim                  claim an unowned room
owner                  show the current room owner
owner did:ma:<target>  transfer the room to another user DID
dig north to Name      create an exit and a new room
:prop name Name        set the focused room name
:prop description ...  set the focused room description
:prop description      reset the focused room description
prop name Name         shorthand for setting the focused room name
```

Digging an existing direction replaces that exit. To link to an existing room
instead of creating a new one, use a room DID-URL or a local fragment from the
same runtime:

```text
dig mirror to @sky#FQWJA5V3
dig mirror to #FQWJA5V3
```

The local `#fragment` form is a local runtime target and stays local through the
handshake. A full `did:ma:...#room` target may point at another runtime. The
target room must run compatible room code and confirm that you own it before the
source room creates the exit. The source room first sends `:ping` to the target
room; after `:pong`, it asks for ownership authorization.

The important rule is simple:

- Commands without a leading colon, such as `look`, `go north`, `say hello`, and
  `dig north`, are avatar-mediated user commands. `prop` is a room-metadata
  shorthand and targets the focused room directly.
- Commands with a leading colon, such as `:prop name ...` and `:help`, are sent
  directly to the focused room.

## Good practice for movable agent init

When you create your own `/ma/scheme/agent/0.0.1` entity, keep init explicit in
the creation-time init code you send when creating the entity (`init:` in
bootstrap YAML, or inline `with (...)` code for creation flows). Do not rely on
hidden helper init functions.

Recommended pattern:

```scheme
(begin
  (set-prop! "name" "My Agent")
  (set-prop! "nick" "myagent")
  (set-prop! "description" "A custom movable agent.")
  (enter (string-append (ma-get-config-key "runtime") "#construct"))
  (ma-save-state!))
```

Why this works well:

- `enter` records the target as pending, sends room `:enter`, and commits the
  new `parent` only after a valid room `:ctx` ack.
- `:enter` with `agent-ctx` informs the room how to present the occupant.
- Full init logic stays in the code users provide at creation time, so users can
  adjust it safely when creating entities.

## Add custom code to one room

Sometimes you want one room to have a special method or two without changing
`actors/room.ma`, without using CRUD, and without registering a new kind. λ-間
supports that from inside the room.

The normal room behaviour still comes from `/ma/room/0.0.1`. Your extra code is
added only to that room.

Go to the room, claim it if you have not already, and open the behaviour editor:

```text
claim
:behaviour!edit
```

That opens the room's current per-entity behaviour source if one is already set,
or a blank editor if the room has no custom code yet. Add Scheme code and press
Publish.

For example:

```scheme
(set-method! :duck
  (lambda (args msg)
    (ma-send! (msg-from msg)
      (list :print "A duck waddles through the room. It looks busy."))))
```

Zion publishes the editor contents as `text/plain`, sends the returned
`/ipfs/<cid>` to the room's `:behaviour` method, and the runtime reloads only
that room. Call the new method directly:

```text
:duck
```

This does not create a new kind. The room is still a normal λ-間 room with
`look`, `say`, `go`, `claim`, `owner`, `dig`, `:prop`, and the rest of the room
behaviour. The `/ipfs/<cid>` is just an extra behaviour layer for that one room,
loaded after the base room code.

Because that extra layer is evaluated after the base room code, it can extend
the room's own help text without changing `actors/room.ma`. Save the original
`room-help-text`, redefine it, and keep using the existing `:help` method:

```scheme
(define base-room-help-text room-help-text)

(define (room-help-text)
  (string-append
    (base-room-help-text)
    "\n  :duck             notice the local duck"))
```

Now both `:help` and `help here` include the local room affordance.

To see the current per-room behaviour reference:

```text
:behaviour
```

You can do the same thing manually: write a `duck.ma` file, publish it with
`ipfs add --quieter duck.ma`, then run `:behaviour /ipfs/<cid>` from the room.
The editor command is just the friendly path over that same mechanism.

You can also compose code files. If `duck-room.ma` contains this:

```scheme
(ma-include-ipfs #/ipfs/bafkreiduck...)

(set-method! :pond
  (lambda (args msg)
    (ma-send! (msg-from msg)
      (list :print "The pond is still. Something under it is considering you."))))
```

then publish `duck-room.ma` and use its CID:

```sh
ipfs add --quieter duck-room.ma
```

```text
:behaviour /ipfs/<duck-room-cid>
```

The include form is named `ma-include-ipfs`, and the method helper is named
`set-method!`.

For tiny experiments, inline code also works:

```text
dig west to Tiny Duck Room with (set-method! :duck (lambda (args msg) (ma-send! (msg-from msg) (list :print "quack"))))
```

Inline `with (...)` code is delivered as creation-time init code. It is useful
for quick live experiments, but for room code you want to keep, publish a file
and use `:behaviour /ipfs/<cid>` after the room exists.

You can also attach code while digging a new room:

```text
dig east to Duck Room with /ipfs/bafkreiduck...
go east
:duck
```

That stores the `/ipfs/<cid>` as the new room's behaviour reference immediately.

One important limitation: `with ...` only applies when a new room is created. If
the target after `to` is an existing room, λ-間 links to that room and ignores
custom code. To add code to an already-existing room, enter it and use
`:behaviour!edit`.

## Modify λ-間 itself

The actor behaviours live in `actors/`:

```text
actors/root.ma
actors/avatar.ma
actors/room.ma
actors/exit.ma
```

To change the default room behaviour, edit `actors/room.ma`. To change movement
or user commands, start with `actors/avatar.ma` and `actors/room.ma`. To change
world entry and zion context, start with `actors/root.ma`.

After editing, regenerate and test the bootstrap:

```sh
make clean
make
make check
```

Then choose a bootstrap workflow again:

```sh
ma --bootstrap dist/lambda-ma.yaml
```

or:

```sh
make root-cid
ma --root-cid <printed-cid>
```

## Publish and share your variant

The source files are the important artefacts:

```text
lambda-ma.template.yaml
actors/*.ma
scheme-actor/src/*.rs
scheme-actor/stdlib.ma
Makefile
```

Do not commit generated output such as `dist/`, `scheme-actor/target/`, or
`scheme-actor/actor.wasm`.

To share a reproducible seed world, share the source branch and the generated
root CID. Someone with Kubo and `ma` can start from that CID directly:

```sh
ma --root-cid <your-root-cid>
```

To let them rebuild it themselves, tell them to run:

```sh
make
make root-cid
```

## Troubleshooting

### `ipfs: command not found`

Install Kubo from <https://docs.ipfs.tech/install/command-line/> or make sure
IPFS Desktop's `ipfs` binary is on your `PATH`.

### `kubo RPC is not reachable for bootstrap`

Start IPFS Desktop or the Kubo daemon, then try again. `ma` needs the local Kubo
RPC API, normally at `http://127.0.0.1:5001`, to publish the bootstrap manifest.

### `make` fails while publishing CIDs

Check that Kubo is running:

```sh
ipfs id
```

If `ipfs id` fails, fix Kubo first. λ-間 cannot generate a complete bootstrap
without publishing actor files to IPFS.

### zion enters but room commands fail

Run:

```text
.ma!discover
@ma/config/root: @ma#root
.enter @ma
```

The `/config/root` value tells zion which runtime actor is the world root. For
λ-間 it should be `@ma#root`.

### I changed actors but the world looks unchanged

Regenerate and re-bootstrap:

```sh
make clean
make
ma --bootstrap dist/lambda-ma.yaml
```

If you are using the root CID workflow, remember that old CIDs are immutable.
After source changes, generate a new root CID:

```sh
make root-cid
ma --root-cid <new-cid>
```
