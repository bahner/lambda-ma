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
