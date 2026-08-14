# lambda-ma HOWTO

Lambda-ma is a direct-DID seed world for `ma-runtime`. A user interacts as
their own `did:ma:` identity; rooms store direct DID presence and no identity
entity is created.

## Prerequisites

Install Kubo/IPFS, a current `ma` runtime, Rust, and the Wasm target:

```sh
rustup target add wasm32-unknown-unknown
ma --help
ipfs --version
```

Kubo must be running because the build publishes actor behaviours and the
bootstrap manifest to IPFS.

## Build

From this repository:

```sh
make clean
make
make check
```

This produces `dist/lambda-ma.yaml`. Do not edit that generated file; edit
actor sources, kinds, or `lambda-ma.template.yaml`, then build again.

## Start a world

```sh
ma --bootstrap dist/lambda-ma.yaml
```

Open zion, unlock or create an identity, then discover the runtime. Obtain the
room target from root or known bootstrap state and enter it directly:

```text
@runtime#root:ctx?
@runtime#construct:enter {name: "Visitor", nick: "Visitor"}
```

Without a known room address, ask root instead — it always answers with a
room to enter:

```text
@runtime#root:enter?
```

The room's reply is the current DID ctx. Plain focused commands and direct
methods both address that confirmed room; there is no intermediate actor.
The room also publishes the committed ctx to house, where it can be queried as
`@runtime#house:did-ctx? did:ma:<identity>` until world ACL policy is enabled.

## Lock a container

After claiming a container, its owner may lock it directly. A supplied secret
allows another authenticated DID to unlock it; calling `:lock` with a different
secret replaces the old one immediately.

```text
@runtime#bag:lock supersecret
@runtime#bag:unlock supersecret
```

## Profile references

- Normative contract: [ma-lambda-ma-v1.md](../ma-spec/runtime/ma-lambda-ma-v1.md)
- Shipped actor API: [REFERENCE.md](REFERENCE.md)
- Actor protocol overview: [actors/README.md](actors/README.md)