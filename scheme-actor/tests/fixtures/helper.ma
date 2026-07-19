; Test fixture: a small helper composed into main.ma via a #/ipfs/<cid>
; directive, to exercise real recursive behaviour-directive expansion
; (ma-runtime-v1.md §14.2.2) end-to-end against a real Kubo daemon.
;
; Deliberately does not call any ma-*-prefixed host-crossing primitive, so
; this helper stays focused on core builtins and the unprefixed props
; primitives (§9), which are pure in-guest state.

(define (bump-counter!)
  (inc-prop! "counter" 1))
