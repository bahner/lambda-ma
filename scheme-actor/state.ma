; ma-scheme state library.
;
; Composed by /ma/scheme/state/0.0.1 above the actor stdlib. This layer may use
; state host functions such as get-prop, set-prop!, and ma-save-state!.

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
