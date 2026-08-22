; Locked world root actor.
; Root is the hardcoded local trust anchor and publishes dynamic world services.

(define (local-self) (canonical-actor (self)))

(define (root-local-actor? actor)
  (and (valid-did-url? actor)
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
    (if (valid-did-url? value) value #f)))

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
        (house-url (root-service-ref "house"))
        (scheduler-url (root-service-ref "scheduler")))
    (if (and (valid-did-url? root-url)
             (valid-did-url? house-url)
             (valid-did-url? scheduler-url))
        (make-map
          "runtime" (runtime)
          "root" root-url
          "house" house-url
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

; Unqualified entry always gets a ctx from root, even if it is a dumb default
; (the configured "start" room). A richer root may consult #house internally
; and return a DID-specific ctx instead; that is an implementation detail
; invisible to the caller.
(set-rpc-method! :enter?
  (lambda (args msg)
    (if (not (null? args))
        (reply-error msg "usage: :enter?")
        (let ((room (root-service-ref "start")))
          (if room
              (reply-ok-with msg (make-map "parent" room "rev" 1))
              (reply-error msg "root has no start room configured"))))))

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
          ((not (valid-did-url? (car (cdr (cdr args)))))
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

; ── Actor spatial registry ─────────────────────────────────────────────────
; Flat ephemeral map keyed by canonical msg.from. Last write wins; no side effects.

(define ACTOR_REGISTRY_KEY "actor-registry")

(define (actor-registry)
  (let ((v (get-prop ACTOR_REGISTRY_KEY)))
    (if (map? v) v (make-map))))

(define (registry-put! did-url ctx)
  (set-prop! ACTOR_REGISTRY_KEY
             (map-set (actor-registry) (canonical-actor did-url) ctx)))

(define (registry-lookup did-url)
  (map-ref (actor-registry) (canonical-actor did-url) #f))

; Walk registry parent chain until a room-kind node is found.
(define (find-effective-room did-url)
  (let loop ((current (canonical-actor did-url)) (depth 0))
    (if (> depth 10)
        #f
        (let ((ctx (registry-lookup current)))
          (cond ((not ctx) #f)
                ((equal? (ctx-text ctx "kind") "room")
                 (if (entity-live? current) current #f))
                (else (loop (canonical-actor (ctx-text ctx "parent")) (+ depth 1))))))))

(define (put-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (map-ref ctx "id" ""))
       (valid-did? (map-ref ctx "requestor" ""))
       (valid-did-url? (map-ref ctx "item" ""))
       (valid-did-url? (map-ref ctx "container" ""))))

(define (take-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (map-ref ctx "id" ""))
       (valid-did? (map-ref ctx "requestor" ""))
       (valid-did-url? (map-ref ctx "item" ""))
       (valid-did-url? (map-ref ctx "container" ""))))

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

(define (action-co-located? subject child-did container-did)
  (let ((subject-room (find-effective-room subject))
        (child-room (find-effective-room child-did))
        (container-room (find-effective-room container-did)))
    (and subject-room child-room container-room
         (equal? subject-room container-room)
         (equal? subject-room child-room))))

; Walk parent chain, skipping containers, to check if subject is the holder.
(define (actor-effectively-held-by? did subject)
  (let loop ((current did) (depth 0))
    (if (> depth 10)
        #f
        (let ((ctx (registry-lookup current)))
          (if (not ctx)
              #f
              (let ((kind (ctx-text ctx "kind"))
                    (parent (canonical-actor (ctx-text ctx "parent" ""))))
                (cond ((equal? kind "room") #f)
                      ((same-actor? parent subject) #t)
                      (else (loop parent (+ depth 1))))))))))

(set-internal-rpc-method! :ctx
  (lambda (args msg)
    (when (and (not (null? args)) (map? (car args)))
      (registry-put! (msg-from msg) (car args)))))

; item initiates put: validates caller, root validates spatial, then gates container and drives item
(set-internal-rpc-method! :put-request
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :put-request <put-ctx>"))
          ((not (put-ctx-valid? (car args)))
           (reply-error msg "invalid put-ctx"))
          (else
           (let* ((put-ctx (car args))
                  (requestor (map-ref put-ctx "requestor" ""))
                  (item (map-ref put-ctx "item" ""))
                  (container (map-ref put-ctx "container" "")))
             (cond ((not (action-co-located? requestor item container))
                    (begin
                      (send-put-event! put-ctx "failed" "not in same room")
                      (reply-error msg "not in same room")))
                   (else
                    (begin
                      (send-put-event! put-ctx "ok" #f)
                      ; Container gate: validates locked/full and notifies requestor on refusal.
                      (ma-send! (canonical-actor container) (list :put put-ctx))
                      ; Item executes: drives propose-node-parent! to container.
                      (ma-send! (canonical-actor item) (list :put put-ctx))
                      (reply-ok-with msg (map-ref put-ctx "id" ""))))))))))

; container initiates take: validates locally, root validates spatial, then drives item
(set-internal-rpc-method! :take-request
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :take-request <take-ctx>"))
          ((not (take-ctx-valid? (car args)))
           (reply-error msg "invalid take-ctx"))
          (else
           (let* ((take-ctx (car args))
                  (requestor (map-ref take-ctx "requestor" ""))
                  (item (map-ref take-ctx "item" ""))
                  (container (map-ref take-ctx "container" "")))
             (cond ((not (action-co-located? requestor item container))
                    (begin
                      (send-take-event! take-ctx "failed" "not in same room")
                      (reply-error msg "not in same room")))
                   (else
                    (begin
                      (send-take-event! take-ctx "ok" #f)
                      ; Item decides if requestor may hold it (overridable :take handler).
                      (ma-send! (canonical-actor item) (list :take take-ctx))
                      (reply-ok-with msg (map-ref take-ctx "id" ""))))))))))
