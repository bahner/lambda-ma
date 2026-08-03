; Locked world root / avatar factory actor.
; Root is a known factory only; avatars own ctx, nick, and room state.

(define (local-self) (canonical-actor (self)))

(define (root-local-actor? actor)
  (and (valid-did-url? actor)
       (equal? (actor-runtime actor) (runtime))))

(define (root-orphanable-ctx? ctx)
  (or (equal? (ctx-text ctx "kind") "thing")
      (equal? (ctx-text ctx "kind") "agent")
      (equal? (ctx-text ctx "kind") "container")))

(define (root-orphan-repair-ctx actor)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (canonical-actor actor))
              "kind" "orphan")
            "protocol" "/ma/orphan/0.0.1")
          "parent" (local-self))
        "name" (canonical-actor actor))
      "nick" (canonical-actor actor))
    "description" "An unavailable orphaned actor."))

(define (root-orphan-ctxs)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (reverse acc))
          ((and (root-orphanable-ctx? (car ctxs))
                (entity-live? (ctx-text (car ctxs) "actor")))
           (loop (cdr ctxs) (cons (car ctxs) acc)))
          (else (loop (cdr ctxs) acc)))))

; Entry target resolution. Root only chooses a configured start room; it does
; not infer or create fallback rooms.
(define (configured-start-room)
  (let ((configured (ma-get-config-key "start")))
    (if configured configured (get-prop "start"))))

(define (ensure-start-room)
  (let ((start (configured-start-room)))
    (if (entity-live? start)
        (if (equal? (get-prop "start") start)
            start
            (begin
              (set-prop! "start" start)
              (ma-save-state!)
              start))
        (error "entry start room is not configured"))))

(define (requested-room args)
  (cond ((null? args) #f)
        ((delegated-enter? args)
         (if (or (null? (cdr args)) (equal? (car (cdr args)) "")) #f (car (cdr args))))
        ((equal? (car args) "") #f)
        (else (car args))))

(define (entry-room requested)
  (if (entity-live? requested) (canonical-actor requested) (ensure-start-room)))

(define (requested-nick args)
  (cond ((null? args) #f)
        ((delegated-enter? args)
         (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args)))))
        ((equal? (car args) "") (if (null? (cdr args)) #f (car (cdr args))))
        ((null? (cdr args)) #f)
        (else (car (cdr args)))))

(define (requested-inventory args)
  (let ((inventory (arg-at-or-false args (if (delegated-enter? args) 3 2))))
    (if (valid-did-url? inventory) (canonical-actor inventory) #f)))

(define (delegated-enter? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (entry-did args msg)
  (if (delegated-enter? args) (car args) (msg-from msg)))

; Avatar creation is asynchronous. The init code asks the room to admit the
; avatar; the room later sends committed ctx back to the avatar.
(define (avatar-init did nick room inventory)
  (let ((n (nick-or-default nick))
        (r (local-self))
        (avatar (avatar-for-did did))
        (target-room (canonical-actor room))
        (inv (if (valid-did-url? inventory) (canonical-actor inventory) "")))
    (string-append
      "(set-init-prop! \"did\" \"" did "\")\n"
      "(set-init-prop! \"root\" \"" r "\")\n"
      "(set-init-prop! \"nick\" \"" n "\")\n"
      (if (equal? inv "") "" (string-append "(set-init-prop! \"inventory\" \"" inv "\")\n"))
      "(ma-save-state!)\n"
      "(ma-send! \"" target-room "\" (list :enter \"" avatar "\" #f \"" n "\" \"" inv "\"))\n")))

(define (ensure-avatar did nick room inventory)
  (let ((avatar (avatar-for-did did)))
    (if (entity-live? avatar)
        (begin
          (ma-send! (canonical-actor avatar)
            (list :enter-room (canonical-actor room) did (nick-or-default nick) inventory))
          avatar)
        (entity-url
          (ma-create-actor AVATAR_KIND #f
            (avatar-init did nick room inventory)
            (avatar-fragment did))))))

; Public entry methods.
(set-cmd-method! :enter
  (lambda (args msg)
    (let* ((did (entry-did args msg))
           (room (entry-room (requested-room args)))
           (nick (nick-or-default (requested-nick args)))
          (inventory (requested-inventory args))
          (avatar (ensure-avatar did nick room inventory)))
      (ma-reply! msg (list :ok avatar)))))

(set-rpc-method! :avatar?
  (lambda (args msg)
    (let* ((did (msg-from msg))
           (room (ensure-start-room))
           (avatar (ensure-avatar did #f room #f)))
      (ma-reply! msg (list :ok avatar)))))

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
