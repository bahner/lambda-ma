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

; ── Small general helpers ───────────────────────────────────────────────────

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(define (non-empty-string? value)
  (and (string? value) (not (equal? value ""))))

(define (ctx-text ctx key)
  (let ((value (map-ref ctx key #f)))
    (if (string? value) value #f)))

(define (canonical-entry entry) entry)

(define (same-entry? a b)
  (equal? (canonical-entry a) (canonical-entry b)))

(define (member-entry? entry xs)
  (cond ((null? xs) #f)
        ((same-entry? entry (car xs)) #t)
        (else (member-entry? entry (cdr xs)))))

(define (unique-entries xs)
  (let loop ((rest xs) (acc '()))
    (cond ((null? rest) acc)
          ((member-entry? (car rest) acc) (loop (cdr rest) acc))
          (else (loop (cdr rest) (cons (canonical-entry (car rest)) acc))))))

(define (prop-map key)
  (let ((value (get-prop key)))
    (if (map? value) value (make-map))))

(define (set-prop-map! key value)
  (set-prop! key value)
  (ma-save-state!))

(define (reply-ok msg text)
  (ma-reply! msg (list :ok text)))

(define (reply-error msg text)
  (ma-reply! msg (list :error text)))

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
(define *default-method* #f)

(define (set-method! verb fn)
  (set! *methods* (cons (cons verb fn) *methods*)))

(define (set-default-method! fn)
  (set! *default-method* fn))

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
      (if *default-method*
        (*default-method* verb args msg)
        (ma-reply! msg (list :error "unknown verb"))))))
