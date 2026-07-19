; Test fixture: composes helper.ma via a real (ma-include-ipfs #/ipfs/<cid>)
; top-level form to exercise recursive library composition end-to-end
; (ma-scheme-v1.md §11.1) against a real Kubo daemon and a real, compiled
; actor.wasm. The reference below is replaced with the real published CID
; of helper.ma before this file itself is published — see
; rust-ma-runtime's `wasm_repro::dispatch_scheme_actor` test.
;
; Deliberately keeps host-crossing behaviour minimal. Only exercises the
; `:init` signal (host-mechanical payload eval), `:start`/`:shutdown`
; (script-defined, via `on-signal`), `on-message`, and the props primitives.

(ma-include-ipfs #/ipfs/bafkreidv5wwgc5tjjsljrjyxycibdc4pex6peg7evgboy2vj3vfpsimyly)

(define (on-signal term)
  (cond ((equal? term :start) (set-prop! "started" #t))
        ((equal? term :shutdown) (set-prop! "shutdown" #t))))

(define (on-message msg)
  (bump-counter!)
  (set-prop! "last-content-type" (msg-content-type msg))
  (set-prop! "last-verb" (msg-content msg)))
