; Locked world root actor.
; Root is the hardcoded local trust anchor and publishes dynamic world services.

(define (local-self) (canonical-actor (self)))

(define (root-local-actor? actor)
  (and (is-did-url? actor)
       (equal? (actor-runtime actor) (runtime))))

(define (root-orphanable-ctx? ctx)
  (or (equal? (ctx-text ctx "kind") "thing")
      (equal? (ctx-text ctx "kind") "agent")
      (equal? (ctx-text ctx "kind") "container")))

(define (root-orphan-repair-ctx actor)
  (make-map
    "actor" (canonical-actor actor)
    "kind" "orphan"
    "protocol" "/ma/orphan/0.0.1"
    "parent" (local-self)
    "name" (canonical-actor actor)
    "nick" (canonical-actor actor)
    "description" "An unavailable orphaned actor."))

(define (root-orphan-ctxs)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (reverse acc))
          ((and (root-orphanable-ctx? (car ctxs))
                (entity-live? (ctx-text (car ctxs) "actor")))
           (loop (cdr ctxs) (cons (car ctxs) acc)))
          (else (loop (cdr ctxs) acc)))))

(define (root-service-ref key)
  (let ((value (ma-get-config-key key)))
    (if (is-did-url? value) value #f)))

(define ROOT_SUBSCRIBERS_KEY "subscribers")

(define (root-subscribers)
  (let ((value (get-prop ROOT_SUBSCRIBERS_KEY)))
    (if (map? value) value (make-map))))

(define (remember-root-subscriber! actor)
  (begin
    (set-prop! ROOT_SUBSCRIBERS_KEY
               (map-set (root-subscribers) (canonical-actor actor) #t))
    (ma-save-state!)))

(define (runtime-ctx)
  (let ((root-url (local-self))
        (scheduler-url (root-service-ref "scheduler")))
    (if (and (is-did-url? root-url)
             (is-did-url? scheduler-url))
        (make-map
          "runtime" (runtime)
          "root" root-url
          "scheduler" scheduler-url
          "rev" 1)
        #f)))

(set-rpc-method! :ctx?
  (lambda (args msg)
    (if (null? args)
        (let ((ctx (runtime-ctx)))
          (if ctx
              (reply-ok-with msg ctx)
              (reply-error msg "root service ctx is not configured with full DID-URLs")))
        (reply-error msg "usage: :ctx?"))))

; Root is not a place a traveller can be: it has no children, never commits
; presence, and never answers `:ok`. Every entry is redirected to the configured
; "start" room. A runtime with no start room refuses rather than inventing one.
(set-rpc-method! :enter
  (lambda (args msg)
    (if (or (null? args) (not (map? (car args))))
        (reply-error msg "enter requires a movement ctx")
        (let ((room (root-service-ref "start")))
          (if room
              (ma-reply! msg (list :entrance room))
              (ma-reply! msg (list :deny "root has no start room configured")))))))

(set-internal-rpc-method! :register
  (lambda (args msg)
    (let ((subscriber (msg-from msg))
          (ctx (runtime-ctx)))
      (cond ((not (null? args))
             (reply-error msg "usage: :register"))
            ((not (root-local-actor? subscriber))
             (reply-error msg "root registration requires a local full actor DID-URL"))
            ((not ctx)
             (reply-error msg "root service ctx is not configured with full DID-URLs"))
            (else
             (begin
               (remember-root-subscriber! subscriber)
               (ma-send! (canonical-actor subscriber) (list :ctx ctx))
               (reply-ok msg)))))))

(set-cmd-method! :orphan
  (lambda (args msg)
    (cond ((or (null? args)
               (null? (cdr args))
               (null? (cdr (cdr args)))
               (not (null? (cdr (cdr (cdr args)))))
               (not (equal? (car (cdr args)) "from")))
           (reply-error msg "usage: :orphan <actor> from <parent>"))
          ((not (root-local-actor? (car args)))
           (reply-error msg "orphan actor must belong to this runtime"))
          ((not (is-did-url? (car (cdr (cdr args)))))
           (reply-error msg "orphan parent must be a DID-URL"))
          ((entity-live? (car args))
           (begin
             (ma-send! (canonical-actor (car args))
                       (list :orphan-root (canonical-actor (msg-from msg))
                             (canonical-actor (car (cdr (cdr args))))))
             (reply-ok msg)))
          ((not (same-actor? (msg-from msg) (car (cdr (cdr args)))))
           (reply-error msg "only the named parent may request unavailable orphan repair"))
          (else
           (begin
             (ma-send! (canonical-actor (car (cdr (cdr args))))
                       (list :parent (root-orphan-repair-ctx (car args))))
             (reply-ok msg))))))

(set-rpc-method! :orphans?
  (lambda (args msg)
    (if (null? args)
        (reply-ok-with msg (root-orphan-ctxs))
        (reply-error msg "usage: :orphans?"))))

(define (put-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (map-ref ctx "id" ""))
       (is-did-url? (map-ref ctx "requestor" ""))
       (is-did-url? (map-ref ctx "item" ""))
       (is-did-url? (map-ref ctx "container" ""))))

(define (take-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (map-ref ctx "id" ""))
       (is-did-url? (map-ref ctx "requestor" ""))
       (is-did-url? (map-ref ctx "item" ""))
       (is-did-url? (map-ref ctx "container" ""))))

(define (send-put-event! put-ctx status reason)
  (let ((full-ctx (if reason
                      (map-set (map-set put-ctx "status" status) "reason" reason)
                      (map-set put-ctx "status" status))))
    (ma-send! (canonical-actor (map-ref put-ctx "requestor" ""))
              (list :put-event full-ctx))))

(define (send-take-event! take-ctx status reason)
  (let ((full-ctx (if reason
                      (map-set (map-set take-ctx "status" status) "reason" reason)
                      (map-set take-ctx "status" status))))
    (ma-send! (canonical-actor (map-ref take-ctx "requestor" ""))
              (list :take-event full-ctx))))

; item initiates put: root is a pure relay — spatial ("here") admission is
; enforced peer-to-peer by the container itself via the ctx-passing protocol,
; never by root (see container.ma's node-child-admission-error).
(set-internal-rpc-method! :put-request
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :put-request <put-ctx>"))
          ((not (put-ctx-valid? (car args)))
           (reply-error msg "invalid put-ctx"))
          (else
           (let* ((put-ctx (car args))
                  (item (map-ref put-ctx "item" ""))
                  (container (map-ref put-ctx "container" "")))
             (begin
               (send-put-event! put-ctx "ok" #f)
               ; Container gate: validates locked/full/here and notifies requestor on refusal.
               (ma-send! (canonical-actor container) (list :put put-ctx))
               ; Item executes: drives propose-node-parent! to container.
               (ma-send! (canonical-actor item) (list :put put-ctx))
               (reply-ok-with msg (map-ref put-ctx "id" ""))))))))

; container initiates take: root is a pure relay — spatial ("here") admission
; is enforced peer-to-peer by the container itself, never by root.
(set-internal-rpc-method! :take-request
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :take-request <take-ctx>"))
          ((not (take-ctx-valid? (car args)))
           (reply-error msg "invalid take-ctx"))
          (else
           (let* ((take-ctx (car args))
                  (item (map-ref take-ctx "item" "")))
             (begin
               (send-take-event! take-ctx "ok" #f)
               ; Item decides if requestor may hold it (overridable :take handler).
               (ma-send! (canonical-actor item) (list :take take-ctx))
               (reply-ok-with msg (map-ref take-ctx "id" ""))))))))
