; ma-scheme state library.
;
; Composed by /ma/scheme/state/0.0.1 above the actor stdlib. This layer may use
; state host functions such as get-prop, set-prop!, and ma-save-state!.

(define raw-set-prop! set-prop!)
(define raw-del-prop! del-prop!)

(define ctx-prop-names '())
(define ctx-prop-dirty '())
(define ctx-prop-notifying? #f)

(define (register-ctx-props! names)
  (set! ctx-prop-names
        (unique-string-entries (list-append ctx-prop-names names))))

(define (ctx-prop? key)
  (member-string? key ctx-prop-names))

(define (ctx-props-changed! keys) #f)

(define (mark-ctx-prop-changed! key old-value new-value)
  (if (or ctx-prop-notifying?
          (not (ctx-prop? key))
          (equal? old-value new-value))
      #f
      (set! ctx-prop-dirty
            (unique-string-entries (list-append ctx-prop-dirty (list key))))))

(define (begin-ctx-prop-changes!)
  (set! ctx-prop-dirty '()))

(define (flush-ctx-prop-changes!)
  (if (null? ctx-prop-dirty)
      #f
      (let ((changed ctx-prop-dirty))
        (begin
          (set! ctx-prop-dirty '())
          (set! ctx-prop-notifying? #t)
          (ctx-props-changed! changed)
          (set! ctx-prop-notifying? #f)))))

(define (set-init-prop! key value)
  (raw-set-prop! key value))

(define (del-init-prop! key)
  (raw-del-prop! key))

(define (set-prop! key value)
  (let ((old-value (get-prop key)))
    (begin
      (raw-set-prop! key value)
      (mark-ctx-prop-changed! key old-value value))))

(define (del-prop! key)
  (let ((old-value (get-prop key)))
    (begin
      (raw-del-prop! key)
      (mark-ctx-prop-changed! key old-value #f))))

(define (prop-map key)
  (let ((value (get-prop key)))
    (if (map? value) value (make-map))))

(define (set-prop-map! key value)
  (set-prop! key value)
  (ma-save-state!))

(define (runtime-started-at)
  (let ((value (ma-get-config-key "started_at")))
    (if value value "")))

(define (scheduled-this-runtime? key)
  (equal? (get-prop key) (runtime-started-at)))

(define (mark-scheduled! key)
  (begin
    (set-prop! key (runtime-started-at))
    (ma-save-state!)))

(define (on-signal term)
  (when (equal? (verb-of term) :shutdown)
    (ma-save-state!)))
