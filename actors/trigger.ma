; Trigger actor behaviour.
; Composed after node.ma: stdlib → actor → state → node → trigger.
;
; Per-DID counters and event-reactive narration. The actor is a child of a
; room, receives the room's broadcasts, and evaluates declarative rules on
; every counter update. When a rule fires it asks the room to narrate
; (:narrate / :narrate-to); the room owns its voice, so the trigger actor is
; a sensor, never a mouth. Only the room's designated trigger may narrate.
;
; Random rules are the counter-space counterpart of the runtime
; scheduler's :random delay: `:trigger <id> <counter> random <N> <method>
; <text...>` fires at a random one of the next N updates of the counter, so
; a rule does not trigger identically on every update. The actor has no
; clock, so the randomness is drawn from the update stream itself — no
; #scheduler round-trip is involved.

(define TRIGGER_KIND "/ma/trigger/0.0.1")

(define (node-kind) "trigger")
(define (node-protocol) TRIGGER_KIND)

; The trigger is a hidden sensor: invisible in room views and name resolution
; by default, discoverable only via the room's :trigger?. The owner may
; override with `:prop visible true`, which surfaces it in look's "Triggers:"
; section and client-side name resolution.
(define (node-visible)
  (let ((value (get-prop "visible")))
    (if (non-empty-string? value) value "false")))

; ── Counter store ───────────────────────────────────────────────────────────
; counters[did][name] = int. The DID is always the map key, never a stored
; field. The room dimension is implicit — the parent is the room — so the
; room's own DID is the natural general (non-personal) counter key.

(define (counters)
  (let ((value (get-prop "counters")))
    (if (map? value) value (make-map))))

(define (did-counters did)
  (let ((value (map-ref (counters) did #f)))
    (if (map? value) value (make-map))))

(define (counter-value did name)
  (let ((value (map-ref (did-counters did) name #f)))
    (if (number? value) value 0)))

(define (set-counter! did name value)
  (set-prop! "counters"
    (map-set (counters) did
      (map-set (did-counters did) name value))))

(define (del-counter! did name)
  (begin
    (set-prop! "counters"
      (map-set (counters) did
        (map-delete (did-counters did) name)))
    (ma-save-state!)))

; counter-last[name] = { did, at } — the last updater per counter name,
; used by the `within` condition (a different DID updating within a window).
(define (counter-last)
  (let ((value (get-prop "counter-last")))
    (if (map? value) value (make-map))))

(define (set-counter-last! name did at)
  (set-prop! "counter-last"
    (map-set (counter-last) name
      (make-map "did" did "at" at))))

; ── Trigger rules and state ─────────────────────────────────────────────────

; Rules are validated at setup (valid-rule?), but a rule that reaches state
; with a non-atom symbol — a leftover from an older behaviour version, say —
; must not brick the actor: every counter update re-saves the whole props
; table via ma-save-state!, and :triggers replies with the raw list. Prune
; invalid rules on every read and write the cleaned list back, so the first
; save after a poisoned load succeeds and the actor heals itself.
(define (trigger-rules)
  (let ((value (get-prop "triggers")))
    (let ((clean (if (or (null? value) (pair? value))
                     (prune-invalid-rules value)
                     '())))
      (set-prop! "triggers" clean)
      clean)))

(define (trigger-state)
  (let ((value (get-prop "trigger-state")))
    (if (map? value) value (make-map))))

(define (set-trigger-state! rule-id by-did)
  (set-prop! "trigger-state" (map-set (trigger-state) rule-id by-did)))

(define (rule-state rule-id did)
  (let ((by-did (map-ref (trigger-state) rule-id #f)))
    (if (map? by-did) (map-ref by-did did #f) #f)))

; ── Gates ───────────────────────────────────────────────────────────────────
; The trigger actor is forged by the room owner, so its own owner prop is the
; room owner at forge time.

(define (room-did) (node-parent))

(define (room-present? did)
  (and (non-empty-string? did) (parent-has-child? did)))

; :count/:reset accept the DID itself, the room owner, or — when the key is
; the room's own DID (shared room state) — any child present in the room.
(define (count-caller-authorised? did msg)
  (or (equal? (msg-from msg) did)
      (owner-caller? msg)
      (and (equal? did (room-did)) (room-present? (msg-from msg)))))

; ── Public counter API ─────────────────────────────────────────────────────
; The codeable counter functions. Bespoke trigger behaviour and the wire
; verbs share one update path, so every change — manual or automatic —
; evaluates the rules watching that counter. `msg` is the message that
; caused the update: its timestamp is "now" and its sender is the causer
; when the key is the room's own counter. There is no wall clock, so a
; caller without a message passes #f (a `within` rule needs a real
; timestamp to compare).

(define (trigger-count counter did delta msg)
  (let ((at (if msg (msg-created-at msg) 0))
        (causer (if msg (msg-from msg) did)))
    (begin
      (set-counter! did counter (+ (counter-value did counter) delta))
      (evaluate-triggers! counter did at causer)
      (set-counter-last! counter did at)
      (ma-save-state!))))

(define (trigger-set counter did value msg)
  (trigger-count counter did (- value (counter-value did counter)) msg))

(define (trigger-reset! counter did)
  (del-counter! did counter))

(define (evaluate-triggers! name key-did at causer)
  (let loop ((rules (trigger-rules)))
    (if (null? rules)
        #f
        (begin
          (evaluate-rule! (car rules) name key-did at causer)
          (loop (cdr rules))))))

(define (evaluate-rule! rule name key-did at causer)
  (let* ((rule-on (map-ref rule "on" #f))
         (rule-did (map-ref rule "did" #f))
         (applies (and (equal? rule-on name)
                       (or (not rule-did) (equal? rule-did key-did)))))
    (if applies
        (if (map-has-key? rule "within")
            (evaluate-within! rule key-did at causer)
            (evaluate-count-rule! rule key-did at causer))
        #f)))

; The DID the trigger concerns is the key that changed, except for the room's
; own key, where it is the actor whose update caused the fire (the causer).
(define (concerned-did key-did causer)
  (if (equal? key-did (room-did)) causer key-did))

(define (evaluate-count-rule! rule key-did at causer)
  (let* ((name (map-ref rule "on" #f))
         (rule-id (map-ref rule "id" #f))
         (value (counter-value key-did name))
         (concern (concerned-did key-did causer))
         (state (rule-state rule-id key-did)))
    (cond ((map-has-key? rule "every")
           (evaluate-every-rule! rule rule-id key-did value concern state))
          ((map-has-key? rule "once")
           (evaluate-threshold-rule! rule rule-id key-did value at concern state 1))
          ((map-has-key? rule "count")
           (evaluate-threshold-rule! rule rule-id key-did value at concern state
                                     (map-ref rule "count" 1)))
          ((map-has-key? rule "random")
           (evaluate-random-rule! rule rule-id key-did value concern state))
          (else #f))))

; once / count N: fire once when the counter first crosses the threshold.
(define (evaluate-threshold-rule! rule rule-id key-did value at concern state threshold)
  (let ((fired (if (map? state) (map-ref state "fired-at" #f) #f)))
    (if (and (not fired) (number? threshold) (>= value threshold))
        (begin
          (record-rule-state! rule-id key-did value at #f)
          (fire-rule! rule concern))
        (begin
          (record-rule-state! rule-id key-did value fired #f)
          #f))))

; every N: fire on each crossing of a new multiple of N, tracked as the next
; target so jumps in the counter need no modulo builtin.
(define (evaluate-every-rule! rule rule-id key-did value concern state)
  (let* ((n (map-ref rule "every" 1))
         (stored-next (if (map? state) (map-ref state "next" #f) #f))
         (next (if (number? stored-next) stored-next n)))
    (if (and (number? n) (>= value next))
        (begin
          (record-rule-state! rule-id key-did value #f (advance-next next value n))
          (fire-rule! rule concern))
        (begin
          (record-rule-state! rule-id key-did value #f next)
          #f))))

(define (advance-next next value n)
  (let loop ((target next))
    (if (> target value) target (loop (+ target n)))))

; random N: fire at a random one of the next N updates of the counter — the
; counter-space counterpart of the runtime scheduler's :random delay, which
; this actor cannot use: there is no clock here, so the randomness is drawn
; from the update stream itself. The stored next target is rolled from 1..N
; when the rule first sees the counter and re-rolled from the current value
; after every fire; a jump past the target fires once and re-rolls, exactly
; like every N.
(define (random-gap n)
  (+ 1 (random (if (and (number? n) (> n 0)) n 1))))

(define (evaluate-random-rule! rule rule-id key-did value concern state)
  (let* ((n (map-ref rule "random" 1))
         (stored-next (if (map? state) (map-ref state "next" #f) #f))
         (next (if (number? stored-next) stored-next (random-gap n))))
    (if (>= value next)
        (begin
          (record-rule-state! rule-id key-did value #f (+ value (random-gap n)))
          (fire-rule! rule concern))
        (begin
          (record-rule-state! rule-id key-did value #f next)
          #f))))

(define (record-rule-state! rule-id did value fired-at next)
  (set-trigger-state! rule-id
    (map-set (map-ref (trigger-state) rule-id (make-map)) did
      (make-map "last-value" value "fired-at" fired-at "next" next))))

(define (evaluate-within! rule key-did at causer)
  (let* ((name (map-ref rule "on" #f))
         (last (map-ref (counter-last) name #f))
         (last-did (if (map? last) (map-ref last "did" #f) #f))
         (last-at (if (map? last) (map-ref last "at" #f) #f))
         (seconds (map-ref rule "within" 0))
         (concern (concerned-did key-did causer)))
    (if (and last-did
             (not (equal? last-did key-did))
             (number? last-at)
             (number? seconds)
             (<= (- at last-at) seconds))
        (fire-rule! rule concern)
        #f)))

(define (fire-rule! rule concern)
  (let* ((method (normalise-method (map-ref rule "method" #f)))
         (text (map-ref rule "text" '()))
         (bindings (make-bindings concern)))
    (if (equal? method :narrate-to)
        (ma-send! (node-parent)
          (if (null? text)
              (list :narrate-to concern)
              (list :narrate-to concern (render-words text bindings))))
        (ma-send! (node-parent)
          (if (null? text)
              (list method)
              (list method (render-words text bindings)))))))

; ── Template expressions ────────────────────────────────────────────────────
; The action text is either a plain string or a list of parts: literal strings
; and expression forms `(:fn arg ...)`. Expressions are evaluated in a closed,
; side-effect-free vocabulary; no eval is available in ma-scheme, so the
; grammar is deliberately bounded. Malformed rules are rejected at setup.

; The action text is a list of parts: literal strings (carrying their own
; spacing and punctuation) and expression forms. Parts render at fire time
; (spans evaluated against the bindings) and are concatenated, so prose like
; "…room, " (:did-name) "!" glues naturally.
(define (render-words words bindings)
  (cond ((null? words) "")
        ((null? (cdr words)) (render-word (car words) bindings))
        (else (string-append (render-word (car words) bindings)
                             (render-words (cdr words) bindings)))))

(define (render-word word bindings)
  (to-text (cond ((string? word) word)
                 ((number? word) word)
                 ((symbol? word) (symbol->string word))
                 ((pair? word) (eval-expr word bindings))
                 (else ""))))

(define (form-head-symbol head)
  (if (symbol? head) head (string->symbol head)))

(define (eval-expr form bindings)
  (if (not (and (pair? form) (or (symbol? (car form)) (string? (car form)))))
      ""
      (let ((fn (form-head-symbol (car form)))
            (args (cdr form)))
        (cond
          ((equal? fn :did) (binding-get bindings "did"))
          ((equal? fn :did-name) (binding-get bindings "name"))
          ((equal? fn :did-attr) (template-call1 binding-attr args bindings))
          ((equal? fn :did-counter) (template-call1 binding-counter args bindings))
          ((equal? fn :+) (template-arith + args bindings))
          ((equal? fn :-) (template-arith - args bindings))
          ((equal? fn :*) (template-arith * args bindings))
          ((equal? fn :/) (template-arith / args bindings))
          ((equal? fn :mod) (template-arith mod args bindings))
          ((equal? fn :=) (template-cmp = args bindings))
          ((equal? fn :<) (template-cmp < args bindings))
          ((equal? fn :>) (template-cmp > args bindings))
          ((equal? fn :<=) (template-cmp <= args bindings))
          ((equal? fn :>=) (template-cmp >= args bindings))
          ((equal? fn :and) (template-and args bindings))
          ((equal? fn :or) (template-or args bindings))
          ((equal? fn :not) (not (truthy (template-arg1 args bindings))))
          ((equal? fn :if) (template-if args bindings))
          ((equal? fn :string-append) (template-string-append args bindings))
          ((equal? fn :string-length) (string-length (to-text (template-arg1 args bindings))))
          ((equal? fn :number->string) (number->string (template-arg1 args bindings)))
          ((equal? fn :string->number) (string->number (to-text (template-arg1 args bindings))))
          ((equal? fn :string-upcase) (string-upcase (to-text (template-arg1 args bindings))))
          ((equal? fn :string-downcase) (string-downcase (to-text (template-arg1 args bindings))))
          (else "")))))

(define (template-call1 proc args bindings)
  (if (pair? args)
      (proc (to-text (eval-arg (car args) bindings)) bindings)
      ""))

(define (template-arith proc args bindings)
  (if (pair? args)
      (template-arith-fold proc (eval-arg (car args) bindings) (cdr args) bindings)
      ""))

(define (template-arith-fold proc acc args bindings)
  (if (null? args)
      acc
      (template-arith-fold proc (proc acc (eval-arg (car args) bindings)) (cdr args) bindings)))

(define (template-cmp proc args bindings)
  (if (or (not (pair? args)) (not (pair? (cdr args))))
      ""
      (proc (eval-arg (car args) bindings) (eval-arg (cadr args) bindings))))

(define (template-and args bindings)
  (cond ((null? args) #t)
        ((null? (cdr args)) (truthy (eval-arg (car args) bindings)))
        (else (if (truthy (eval-arg (car args) bindings))
                  (template-and (cdr args) bindings)
                  #f))))

(define (template-or args bindings)
  (cond ((null? args) #f)
        ((null? (cdr args)) (truthy (eval-arg (car args) bindings)))
        (else (if (truthy (eval-arg (car args) bindings))
                  #t
                  (template-or (cdr args) bindings)))))

(define (template-if args bindings)
  (if (not (pair? args))
      ""
      (if (truthy (eval-arg (car args) bindings))
          (if (pair? (cdr args))
              (eval-arg (cadr args) bindings)
              "")
          (if (pair? (cddr args))
              (eval-arg (caddr args) bindings)
              ""))))

(define (template-string-append args bindings)
  (cond ((null? args) "")
        ((null? (cdr args)) (to-text (eval-arg (car args) bindings)))
        (else (string-append (to-text (eval-arg (car args) bindings))
                             (template-string-append (cdr args) bindings)))))

(define (template-arg1 args bindings)
  (if (pair? args) (eval-arg (car args) bindings) ""))

(define (eval-arg arg bindings)
  (cond ((string? arg) arg)
        ((number? arg) arg)
        ((boolean? arg) arg)
        ((symbol? arg) (symbol->string arg))
        ((pair? arg) (eval-expr arg bindings))
        (else "")))

(define (to-text value)
  (cond ((string? value) value)
        ((number? value) (number->string value))
        ((equal? value #t) "true")
        (else "")))

(define (truthy value)
  (not (equal? value #f)))

; ── Bindings for the concerning DID ─────────────────────────────────────────
; The concerning DID's name and attributes come from the cached room snapshot
; (parent-ctx); that snapshot is unauthenticated presentation data, never the
; authoritative child state.

(define (child-ctx-of did)
  (let ((parent (parent-ctx)))
    (if (map? parent)
        (map-ref (map-ref parent "children" (make-map)) did #f)
        #f)))

(define (resolve-name did)
  (let ((ctx (child-ctx-of did)))
    (cond ((and (map? ctx) (non-empty-string? (ctx-text ctx "nick"))) (ctx-text ctx "nick"))
          ((and (map? ctx) (non-empty-string? (ctx-text ctx "name"))) (ctx-text ctx "name"))
          (else did))))

(define (make-bindings did)
  (let ((ctx (child-ctx-of did)))
    (make-map "did" did
              "name" (resolve-name did)
              "attrs" (if (map? ctx) ctx (make-map))
              "counters" (did-counters did))))

(define (binding-get bindings key)
  (map-ref bindings key #f))

(define (binding-attr key bindings)
  (let ((value (map-ref (map-ref bindings "attrs" (make-map)) key #f)))
    (if (string? value) value "")))

(define (binding-counter name bindings)
  (let ((value (map-ref (map-ref bindings "counters" (make-map)) name #f)))
    (if (number? value) value 0)))

; ── Rule validation ─────────────────────────────────────────────────────────

(define (rule-condition-count rule)
  (+ (if (map-has-key? rule "once") 1 0)
     (if (map-has-key? rule "count") 1 0)
     (if (map-has-key? rule "every") 1 0)
     (if (map-has-key? rule "within") 1 0)
     (if (map-has-key? rule "random") 1 0)))

(define (valid-method? method)
  (and (not (equal? method #f))
       (or (string? method)
           (and (symbol? method) (atom-symbol? method)))
       (not (equal? (word-as-string method) ""))))

(define (valid-rule? rule)
  (and (map? rule)
       (non-empty-string? (map-ref rule "id" #f))
       (= (rule-condition-count rule) 1)
       (valid-method? (map-ref rule "method" #f))
       (valid-text? (map-ref rule "text" '()))
       ; Every rule watches a counter name; a random condition also needs a
       ; positive max gap. A counterless random rule left by an older
       ; behaviour version fails here and is pruned on the next read.
       (non-empty-string? (map-ref rule "on" #f))
       (or (not (map-has-key? rule "random"))
           (and (number? (map-ref rule "random" #f))
                (> (map-ref rule "random" #f) 0)))))

(define (valid-text? words)
  (or (null? words)
      (and (valid-part? (car words)) (valid-text? (cdr words)))))

(define (valid-parts? parts)
  (or (null? parts)
      (and (valid-part? (car parts)) (valid-parts? (cdr parts)))))

; A symbol that starts with ':' (a verb atom) — the only symbol kind with a
; CBOR representation (§6). Rules are persisted via ma-save-state!, so a bare
; symbol such as `every` or `lock` can never be stored.
(define (atom-symbol? value)
  (and (symbol? value)
       (let ((s (symbol->string value)))
         (and (not (equal? s ""))
              (equal? (substring s 0 1) ":")))))

(define (valid-part? part)
  (cond ((string? part) #t)
        ((number? part) #t)
        ((symbol? part) (atom-symbol? part))
        ((pair? part) (valid-expr? part))
        (else #f)))

(define (valid-expr? form)
  (and (pair? form)
       (or (symbol? (car form)) (string? (car form)))
       (known-fn? (form-head-symbol (car form)))
       (valid-args? (cdr form))))

(define (valid-args? args)
  (or (null? args)
      (and (valid-part? (car args)) (valid-args? (cdr args)))))

(define (known-fn? fn)
  (let loop ((fns '(:did :did-name :did-attr :did-counter
                      :+ :- :* :/ :mod
                      := :< :> :<= :>=
                      :and :or :not :if
                      :string-append :string-length
                      :number->string :string->number
                      :string-upcase :string-downcase)))
    (cond ((null? fns) #f)
          ((equal? fn (car fns)) #t)
          (else (loop (cdr fns))))))

; Drop rules that can never be re-saved or fired — a rule whose method is a
; non-atom symbol, or whose parts are not CBOR-encodable. Idempotent: a clean
; list passes through unchanged, so this is safe to run on every read.
(define (prune-invalid-rules rules)
  (cond ((null? rules) '())
        ((valid-rule? (car rules))
         (cons (car rules) (prune-invalid-rules (cdr rules))))
        (else (prune-invalid-rules (cdr rules)))))

; ── Rule management ─────────────────────────────────────────────────────────

(define (upsert-rule rules rule)
  (cons rule (without-rule-id rules (map-ref rule "id" #f))))

; ── Flat rule parsing and name normalisation ────────────────────────────────
; The wire form is positional: :trigger <id> <counter> [condition] <method>
; <text...>. Counters arrive as symbols (:arrive) or strings ("num_jokes")
; and are normalised to strings; methods arrive as :narrate_to and are sent to
; the parent as :narrate-to (underscores become hyphens).

(define (word-as-string word)
  (if (symbol? word) (symbol->string word) word))

(define (word=? word text)
  (equal? (word-as-string word) text))

(define (normalise-name name)
  (if (symbol? name) (symbol->string name) name))

(define (replace-chars s from to)
  (let ((len (string-length s)))
    (let loop ((i 0) (acc ""))
      (if (>= i len)
          acc
          (let ((c (substring s i (+ i 1))))
            (loop (+ i 1) (string-append acc (if (equal? c from) to c))))))))

(define (normalise-method method)
  (cond ((symbol? method)
         (string->symbol (replace-chars (symbol->string method) "_" "-")))
        ((string? method)
         (string->symbol (replace-chars method "_" "-")))
        ; A number or other junk in the method position (a malformed rule)
        ; must yield #f so valid-rule? rejects it instead of crashing the
        ; string helpers above.
        (else #f)))

; Normalise a condition count that arrives as a number or a numeric string
; (clients send RPC arguments as CBOR text, so `every 1` delivers "1").
(define (as-number value)
  (cond ((number? value) value)
        ((string? value) (string->number value))
        (else #f)))

(define (number-or-string-number? value)
  (number? (as-number value)))

; Condition detection: once, or count/every/within/random with a number,
; or none (defaults to every 1 — fire on every update).
(define (flat-condition tail)
  (if (pair? tail)
      (let ((word (car tail)))
        (cond ((word=? word "once") (list "once" #t (cdr tail)))
              ((and (pair? (cdr tail))
                    (number-or-string-number? (cadr tail))
                    (or (word=? word "count")
                        (word=? word "every")
                        (word=? word "within")
                        (word=? word "random")))
               (list (word-as-string word) (as-number (cadr tail)) (cddr tail)))
              (else #f)))
      #f))

(define (parse-flat-rule args)
  (let* ((id (arg-at-or-false args 0))
         (counter (normalise-name (arg-at-or-false args 1)))
         (tail (cddr args)))
    (parse-counter-rule id counter tail)))

; Counter rules: :trigger <id> <counter> [did <did>] [once|count N|every N|
; within S|random N] <method> <text...>.
(define (parse-counter-rule id counter tail)
  (let* ((did (if (and (pair? tail) (word=? (car tail) "did") (pair? (cdr tail)))
                  (normalise-name (cadr tail))
                  #f))
         (tail (if did (cddr tail) tail))
         (cond-result (flat-condition tail))
         (cond-key (if cond-result (car cond-result) "every"))
         (cond-value (if cond-result (cadr cond-result) 1))
         (tail (if cond-result (caddr cond-result) tail))
         (method (normalise-method (arg-at-or-false tail 0)))
         (text (if (pair? tail) (cdr tail) '())))
    (let ((rule (make-map "id" id
                          "on" counter
                          "method" method
                          "text" text)))
      (let ((rule (map-set rule cond-key cond-value)))
        (if did (map-set rule "did" did) rule)))))

(define (without-rule-id rules id)
  (cond ((null? rules) '())
        ((equal? (map-ref (car rules) "id" #f) id) (cdr rules))
        (else (cons (car rules) (without-rule-id (cdr rules) id)))))

(define (remove-rule rules id)
  (cond ((null? rules) '())
        ((equal? (map-ref (car rules) "id" #f) id) (cdr rules))
        (else (cons (car rules) (remove-rule (cdr rules) id)))))

; ── Automatic counting (room broadcasts) ────────────────────────────────────
; The trigger actor is a child of the room, so every room event reaches it
; here. Events with a subject count under that DID. Subject-less events
; (:narrate — the trigger's own output channel) are deliberately not counted,
; which also prevents a fired narration from re-triggering rules.

(define (count-event! name ctx msg)
  (if (map? ctx)
      (let ((subject (event-subject ctx)))
        (if (non-empty-string? subject)
            (trigger-count name subject 1 msg)
            #f))
      #f))

(define (event-subject ctx)
  (if (map? ctx)
      (let ((actor (ctx-text ctx "actor")))
        (if (non-empty-string? actor) actor (ctx-text ctx "did")))
      #f))

(define (count-broadcast! name args msg)
  (if (pair? args)
      (count-event! name (car args) msg)
      #f))

(set-cmd-method! :say
  (lambda (args msg) (count-broadcast! ":say" args msg)))

(set-cmd-method! :emote
  (lambda (args msg) (count-broadcast! ":emote" args msg)))

(set-cmd-method! :arrive
  (lambda (args msg) (count-broadcast! ":arrive" args msg)))

(set-cmd-method! :dig
  (lambda (args msg) (count-broadcast! ":dig" args msg)))

(set-cmd-method! :fill
  (lambda (args msg) (count-broadcast! ":fill" args msg)))

; :leave counts like any other event and prunes nothing: per-DID counters are
; this actor's history, not presence state, so arrival totals survive a
; departure and "welcome back, N visits" rules keep working across sessions.
; trigger-state persists too, so a `once` rule fires once ever per DID, not
; once per visit.
(set-cmd-method! :leave
  (lambda (args msg) (count-broadcast! ":leave" args msg)))

; Generic event notification from a non-broadcasting parent (a chest, an exit,
; a thing): the parent forwards (:event <name> <subject-did>) and the actor
; counts it like any other event.
(set-cmd-method! :event
  (lambda (args msg)
    (let ((name (normalise-name (arg-at-or-false args 0)))
          (subject (arg-at-or-false args 1)))
      (if (and (non-empty-string? name) (non-empty-string? subject))
          (trigger-count name subject 1 msg)
          #f))))

; ── Counter verbs ───────────────────────────────────────────────────────────

(set-cmd-method! :count
  (lambda (args msg)
    (let ((did (arg-at-or-false args 0))
          (name (normalise-name (arg-at-or-false args 1)))
          (delta (as-number (arg-at-or-false args 2))))
      (cond ((not (and (string? did) (string? name) (number? delta)))
             (reply-error msg "usage: :count <did> <name> <delta>"))
            ((not (count-caller-authorised? did msg))
             (reply-error msg "not authorised to count that DID"))
            (else
             (trigger-count name did delta msg)
             (reply-ok msg))))))

(set-cmd-method! :set
  (lambda (args msg)
    (let ((did (arg-at-or-false args 0))
          (name (normalise-name (arg-at-or-false args 1)))
          (value (as-number (arg-at-or-false args 2))))
      (cond ((not (and (string? did) (string? name) (number? value)))
             (reply-error msg "usage: :set <did> <name> <value>"))
            ((not (owner-caller? msg))
             (reply-error msg "only this room's owner may set counter values"))
            (else
             (trigger-set name did value msg)
             (reply-ok msg))))))

(set-cmd-method! :reset
  (lambda (args msg)
    (let ((did (arg-at-or-false args 0))
          (name (normalise-name (arg-at-or-false args 1))))
      (cond ((not (and (string? did) (string? name)))
             (reply-error msg "usage: :reset <did> <name>"))
            ((not (count-caller-authorised? did msg))
             (reply-error msg "not authorised to reset that DID"))
            (else
             (trigger-reset! name did)
             (reply-ok msg))))))

(define (counter-query-authorised? msg)
  (or (room-present? (msg-from msg)) (owner-caller? msg)))

(set-rpc-method! :value
  (lambda (args msg)
    (let ((did (arg-at-or-false args 0))
          (name (normalise-name (arg-at-or-false args 1))))
      (cond ((not (counter-query-authorised? msg))
             (reply-error msg "not in room"))
            ((not (and (string? did) (string? name)))
             (reply-error msg "usage: :value <did> <name>"))
            (else
             (reply-ok-with msg (counter-value did name)))))))

(set-rpc-method! :counters
  (lambda (args msg)
    (if (counter-query-authorised? msg)
        (if (pair? args)
            (reply-ok-with msg (did-counters (car args)))
            (reply-ok-with msg (counters)))
        (reply-error msg "not in room"))))

; ── Trigger verbs ───────────────────────────────────────────────────────────

(set-rpc-method! :triggers
  (lambda (args msg)
    (if (owner-caller? msg)
        (reply-ok-with msg (trigger-rules))
        (reply-error msg "only this room's owner may list triggers"))))

(set-rpc-method! :trigger
  (lambda (args msg)
    (cond ((not (owner-caller? msg))
           (reply-error msg "only this room's owner may edit triggers"))
          ((and (pair? args) (string? (car args)) (null? (cdr args)))
           (begin
             (set-prop! "triggers" (remove-rule (trigger-rules) (car args)))
             (set-prop! "trigger-state" (map-delete (trigger-state) (car args)))
             (ma-save-state!)
             (reply-ok msg)))
          ((< (list-length args) 3)
           (reply-error msg "usage: :trigger <id> <counter> [once|count N|every N|within S|random N] <method> <text...>"))
          (else
           (let ((rule (parse-flat-rule args)))
             (if (valid-rule? rule)
                 (begin
                   (set-prop! "triggers" (upsert-rule (trigger-rules) rule))
                   (ma-save-state!)
                   (reply-ok msg))
                 (reply-error msg "invalid trigger rule")))))))

; ── Object relocation and ownership ─────────────────────────────────────────
; The trigger is an ordinary movable node: forge births it into the forger's
; hand, and the owner places it in the room with the shared :set-parent/:hold
; flow (trigger.md). Every body below is the shared node.ma handler; this kind
; file only registers them.
(set-rpc-method! :owner handle-node-owner)
(set-rpc-method! :owner? handle-node-owner?)
(set-rpc-method! :set-recovery-secret handle-node-set-recovery-secret!)
(set-cmd-method! :claim handle-node-claim!)
(set-rpc-method! :put handle-node-put!)
(set-cmd-method! :take handle-node-take-relay!)
(set-cmd-method! :set-parent handle-node-set-parent!)

; :hold implicitly targets the caller (msg-from) as new parent - no argument.
; Shared body lives in node.ma as handle-node-hold!.
(set-cmd-method! :hold handle-node-hold!)

(define (recycle! msg)
  (let ((old-parent (node-parent)))
    (begin
      (set-node-parent! "")
      (if (non-empty-string? old-parent)
          (ma-send! (canonical-actor old-parent) (list :parent (node-ctx-for-parent "")))
          #f)
      (ma-end))))

(set-cmd-method! :recycle
  (lambda (args msg)
    (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((not (null? rest))
             (reply-error msg "usage: :recycle <did>"))
            ((not (node-recycle-caller-authorised? did msg))
             (reply-error msg "only owner may recycle this trigger"))
            (else
             (recycle! msg))))))
