# Python actors

`λ-間` is currently bootstrapped entirely from ma-scheme actors in `../`.

Python actors still belong in this repo when a concrete feature needs them, but they need a separate build/publish path:

1. Build each Python actor to Wasm with `extism-py`.
2. Publish the Wasm with IPFS.
3. Add or generate the corresponding kind entry in `lambda-ma.yaml`.
4. Keep host function order aligned with `python-ma-actors` and `ma-runtime`.

The existing Python actor libraries live in `../python-ma-actors` at the workspace level for now. Copying them here should happen when a concrete λ-間 feature uses them, not as a bulk mirror.
