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
; Flat ephemeral map of canonical-did-url → ctx rebuilt from :ctx-update
; pushes. Pending-actions tracks in-flight moves until item ctx confirms.

(define ACTOR_REGISTRY_KEY "actor-registry")
(define PENDING_ACTIONS_KEY "pending-actions")
(define PENDING_BY_CHILD_KEY "pending-by-child")

(define (actor-registry)
  (let ((v (get-prop ACTOR_REGISTRY_KEY)))
    (if (map? v) v (make-map))))

(define (registry-put! did-url ctx)
  (set-prop! ACTOR_REGISTRY_KEY
             (map-set (actor-registry) (canonical-actor did-url) ctx)))

(define (registry-lookup did-url)
  (map-ref (actor-registry) (canonical-actor did-url) #f))

(define (pending-actions)
  (let ((v (get-prop PENDING_ACTIONS_KEY)))
    (if (map? v) v (make-map))))

(define (pending-by-child)
  (let ((v (get-prop PENDING_BY_CHILD_KEY)))
    (if (map? v) v (make-map))))

(define (add-pending-action! action-ctx)
  (let ((id (map-ref action-ctx "id" #f))
        (child (map-ref action-ctx "child" #f)))
    (if (and id child)
        (begin
          (set-prop! PENDING_ACTIONS_KEY
                     (map-set (pending-actions) id action-ctx))
          (set-prop! PENDING_BY_CHILD_KEY
                     (map-set (pending-by-child) (canonical-actor child) id)))
        #f)))

(define (remove-pending-action! id child)
  (set-prop! PENDING_ACTIONS_KEY (map-delete (pending-actions) id))
  (set-prop! PENDING_BY_CHILD_KEY (map-delete (pending-by-child) (canonical-actor child))))

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

(define (action-event-verb action)
  (cond ((equal? action "put") :put-event)
        ((equal? action "take") :take-event)
        ((equal? action "drop") :drop-event)
        (else :move-event)))

(define (send-action-event! subject action-ctx status reason)
  (let ((full-ctx (if reason
                      (map-set (map-set action-ctx "status" status) "reason" reason)
                      (map-set action-ctx "status" status))))
    (ma-send! (canonical-actor subject)
              (list (action-event-verb (map-ref action-ctx "action" "")) full-ctx))))

(define (action-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (map-ref ctx "action" ""))
       (non-empty-string? (map-ref ctx "id" ""))
       (valid-did? (map-ref ctx "subject" ""))
       (valid-did-url? (map-ref ctx "child" ""))
       (non-empty-string? (map-ref ctx "parent" ""))))

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

(define (action-subject-can-move? action subject child-did container-did)
  (let ((child-ctx (registry-lookup child-did)))
    (if (not child-ctx)
        #f
        (let ((child-parent (canonical-actor (map-ref child-ctx "parent" ""))))
          (cond ((equal? action "put")
                 (or (actor-effectively-held-by? child-did subject)
                     (equal? child-parent (find-effective-room container-did))))
                ((equal? action "take")
                 (same-actor? child-parent container-did))
                ((equal? action "drop")
                 (actor-effectively-held-by? child-did subject))
                (else #f))))))

(define (check-completed-action! new-ctx)
  (let ((child-did (canonical-actor (ctx-text new-ctx "actor")))
        (new-parent (canonical-actor (map-ref new-ctx "parent" ""))))
    (let ((pending-id (map-ref (pending-by-child) child-did #f)))
      (if pending-id
          (let ((action-ctx (map-ref (pending-actions) pending-id #f)))
            (if (and action-ctx
                     (same-actor? new-parent (map-ref action-ctx "parent" "")))
                (let ((subject (map-ref action-ctx "subject" "")))
                  (remove-pending-action! pending-id child-did)
                  (send-action-event! subject action-ctx "ok" #f))
                #f))
          #f))))

(set-internal-rpc-method! :ctx-update
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :ctx-update <ctx>"))
          ((not (map? (car args)))
           (reply-error msg "ctx-update requires a ctx map"))
          ((not (same-actor? (msg-from msg) (ctx-text (car args) "actor")))
           (reply-error msg "ctx-update must be self-reported"))
          (else
           (let ((ctx (car args)))
             (registry-put! (ctx-text ctx "actor") ctx)
             (check-completed-action! ctx)
             (reply-ok msg))))))

(set-internal-rpc-method! :action
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :action <action-ctx>"))
          ((not (action-ctx-valid? (car args)))
           (reply-error msg "invalid action-ctx"))
          (else
           (let* ((action-ctx (car args))
                  (action (map-ref action-ctx "action" ""))
                  (subject (map-ref action-ctx "subject" ""))
                  (child (map-ref action-ctx "child" ""))
                  (parent (map-ref action-ctx "parent" ""))
                  (id (map-ref action-ctx "id" "")))
             (cond ((not (action-co-located? subject child parent))
                    (begin
                      (send-action-event! subject action-ctx "failed" "not in same room")
                      (reply-error msg "not in same room")))
                   ((not (action-subject-can-move? action subject child parent))
                    (begin
                      (send-action-event! subject action-ctx "failed" "not authorised to move item")
                      (reply-error msg "not authorised to move item")))
                   (else
                    (begin
                      (add-pending-action! action-ctx)
                      (ma-send! (canonical-actor child) (list :set-parent (canonical-actor parent)))
                      (reply-ok-with msg id)))))))))
