; Runtime-agnostic world context registry.
; Actor and bare-DID identities are map keys, never duplicated in ctx values.

(define HOUSE_PROTOCOL "/ma/house/0.0.1")
(define ENTITY_CTXS_KEY "entity-ctxs")
(define DID_CTXS_KEY "did-ctxs")

(define (house-map key)
  (let ((value (get-prop key)))
    (if (map? value) value (make-map))))

(define (entity-ctxs) (house-map ENTITY_CTXS_KEY))
(define (did-ctxs) (house-map DID_CTXS_KEY))
(define (entity-ctx actor) (map-ref (entity-ctxs) actor #f))
(define (did-ctx did) (map-ref (did-ctxs) did #f))

(define (ctx-revision-valid? ctx)
  (and (map? ctx) (number? (map-ref ctx "rev" #f))))

(define (did-ctx-valid? did ctx msg)
  (and (valid-did? did)
       (map? ctx)
  (equal? did (ctx-text ctx "did"))
       (valid-did-url? (ctx-text ctx "parent"))
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))
       (ctx-revision-valid? ctx)
       (same-actor? (msg-from msg) (ctx-text ctx "parent"))))

(define (entity-kind-valid? kind)
  (or (equal? kind "actor")
      (equal? kind "room")
      (equal? kind "agent")
      (equal? kind "thing")
      (equal? kind "container")))

(define (entity-ctx-valid? ctx msg)
  (let ((parent (ctx-text ctx "parent")))
    (and (valid-did-url? (msg-from msg))
         (map? ctx)
         (entity-kind-valid? (ctx-text ctx "kind"))
         (non-empty-string? (ctx-text ctx "protocol"))
         (non-empty-string? (ctx-text ctx "name"))
         (non-empty-string? (ctx-text ctx "nick"))
         (non-empty-string? (ctx-text ctx "description"))
         (or (equal? parent "") (valid-did-url? parent))
         (ctx-revision-valid? ctx))))

(define (remember-did-ctx! did ctx)
  (begin
    (set-prop! DID_CTXS_KEY (map-set (did-ctxs) did ctx))
    (ma-save-state!)))

(define (remember-entity-ctx! actor ctx)
  (begin
    (set-prop! ENTITY_CTXS_KEY (map-set (entity-ctxs) actor ctx))
    (ma-save-state!)))

(define (leave-previous-parent! did previous current)
  (if (and (valid-did-url? previous)
           (not (same-actor? previous current)))
      (ma-send! previous (list :leave did))
      #f))

(set-internal-rpc-method! :did-ctx
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (not (null? (cdr (cdr args)))))
        (reply-error msg "usage: :did-ctx <bare-did> <did-ctx>")
        (let* ((did (car args))
               (ctx (car (cdr args)))
               (previous (did-ctx did))
           (previous-parent (if (map? previous) (ctx-text previous "parent") ""))
               (current-parent (ctx-text ctx "parent")))
          (if (did-ctx-valid? did ctx msg)
              (begin
                (remember-did-ctx! did ctx)
                (leave-previous-parent! did previous-parent current-parent)
                (reply-ok msg))
              (reply-error msg "DID ctx must be sent by its exact full parent DID-URL"))))))

(set-rpc-method! :did-ctx?
  (lambda (args msg)
    (if (or (null? args) (not (null? (cdr args))) (not (valid-did? (car args))))
        (reply-error msg "usage: :did-ctx? <bare-did>")
        (let ((ctx (did-ctx (car args))))
          (if ctx (reply-ok-with msg ctx) (reply-error msg "no DID ctx for DID"))))))

(set-internal-rpc-method! :entity-ctx
  (lambda (args msg)
    (if (or (null? args) (not (null? (cdr args))))
        (reply-error msg "usage: :entity-ctx <entity-ctx>")
        (let ((ctx (car args)) (actor (msg-from msg)))
          (if (entity-ctx-valid? ctx msg)
              (begin
                (remember-entity-ctx! actor ctx)
                (reply-ok msg))
              (reply-error msg "entity ctx requires a full actor DID-URL sender"))))))

(set-rpc-method! :entity-ctx?
  (lambda (args msg)
    (if (or (null? args) (not (null? (cdr args))) (not (valid-did-url? (car args))))
        (reply-error msg "usage: :entity-ctx? <actor-did-url>")
        (let ((ctx (entity-ctx (car args))))
          (if ctx (reply-ok-with msg ctx) (reply-error msg "no entity ctx for actor"))))))