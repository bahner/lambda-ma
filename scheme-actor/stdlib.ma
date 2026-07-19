; ma-scheme stdlib v0.2 (ma-scheme-v1.md §15)
;
; Reference implementation of the non-normative conventions documented in
; ma-scheme-v1.md §15: a default `:shutdown` handler (via `on-signal`) and a
; `set-method!`/`find-method`/`on-message` verb-dispatch table. Conformance
; does NOT require implementing this file — it exists so scripts written
; against different kinds/hosts can look the same, and so a kind author
; doesn't have to hand-roll dispatch boilerplate for the common case.
;
; This is composed into an entity's behaviour via an ordinary
; `(ma-include-ipfs #/ipfs/<cid>)` top-level form (ma-scheme-v1.md §11.1)
; — never baked into the Wasm binary, never a host-level special case.
; Put that form first in a kind's default `EntityNode.behaviour`/template
; to get this for free; a script's own later `(define (on-signal term) ...)`
; or `(define (on-message msg) ...)` simply rebinds the name, overriding
; the default — ordinary lexical scoping, nothing bespoke.
;
; Publish: ipfs add stdlib.ma
; Then reference it from a behaviour source with:
;   (ma-include-ipfs #/ipfs/<cid-printed-above>)

; ── Signal/verb term helpers (ma-scheme-v1.md §6) ──────────────────────────
;
; A dispatch term is either a bare atom (`:start`) or a list whose first
; element is the atom (`[:enter, "ticket-123"]`) — the same shape for both
; `on-signal` and `on-message`.

(define (verb-of term) (if (pair? term) (car term) term))
(define (args-of term) (if (pair? term) (cdr term) '()))

; ── Default signal handler: persist state on :shutdown, nothing more ───────

(define (on-signal term)
  (when (equal? (verb-of term) :shutdown)
    (ma-save-state!)))

; ── Verb dispatch table ─────────────────────────────────────────────────────
;
; A script using this convention writes handlers with `set-method!` instead
; of hand-rolling its own `cond` inside `on-message`:
;
;   (set-method! :look
;     (lambda (args msg)
;       (ma-reply! msg (list :ok (get-prop "description")))))
;
; A script MAY instead define `on-message` directly for full custom dispatch
; (e.g. pattern-matching without verb-based routing at all) — doing so after
; this file is composed in simply overrides the definition below. Likewise a
; script MAY redefine `on-signal` directly (e.g. to add a `cond` branch for
; `:start`) — the new definition rebinds the name, it does not chain to this
; one, so a script that wants the default `:shutdown` behaviour AND handles
; other signals must call `(ma-save-state!)` itself in its own `on-signal`.

(define *methods* '())

(define (set-method! verb fn)
  (set! *methods* (cons (cons verb fn) *methods*)))

(define (find-method verb)
  (let loop ((table *methods*))
    (cond ((null? table) #f)
          ((equal? (car (car table)) verb) (cdr (car table)))
          (else (loop (cdr table))))))

(define (on-message msg)
  (let* ((term (msg-content msg))
         (verb (if (pair? term) (car term) term))
         (args (if (pair? term) (cdr term) '()))
         (fn (find-method verb)))
    (if fn
        (fn args msg)
        (ma-reply! msg (list :error "unknown verb")))))
