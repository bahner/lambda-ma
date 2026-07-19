CARGO ?= cargo
WASM_TARGET = target/wasm32-unknown-unknown/release/ma_scheme_actor.wasm
WASM = actor.wasm

# Always invoke cargo — it does its own, correct incremental rebuild
# tracking; `make` has no visibility into the crate's source files, so a
# plain file-based rule here would go stale silently (see .PHONY below).
$(WASM_TARGET): FORCE
	$(CARGO) build --release --target wasm32-unknown-unknown

$(WASM): $(WASM_TARGET)
	cp $< $@

test:
	$(CARGO) test

ipfs: $(WASM)
	ipfs add --quieter $<

stdlib: stdlib.ma
	ipfs add --quieter $< | tee stdlib.cid

clean:
	$(CARGO) clean
	rm -f $(WASM)

FORCE:

.PHONY: test ipfs stdlib clean FORCE
