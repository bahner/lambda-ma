; Locked world root / placement-registry actor.
; Root owns avatars and protected placement state. Users enter the world here.

(define AVATAR_KIND "/ma/avatar/0.0.1")
(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor)) (string-append (runtime) actor) actor))
(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))

(define (avatar-key user) (string-append "avatar:" user))
(define (avatar-user-key avatar) (string-append "user:" (canonical-actor avatar)))
(define (avatar-room-key avatar) (string-append "room:" (canonical-actor avatar)))
(define (avatar-nick-key avatar) (string-append "nick:" (canonical-actor avatar)))

(define (default-nick) "avatar")

(define (nick-or-default nick)
  (if nick nick (default-nick)))

(define (avatar-nick avatar)
  (let ((nick (get-prop (avatar-nick-key avatar))))
    (if nick nick (default-nick))))

(define (avatar-room avatar fallback)
  (let ((room (get-prop (avatar-room-key avatar))))
    (if room room fallback)))

(define (ctx-term avatar room text)
  (list :ctx
    (list (list :root (self))
          (list :avatar avatar)
          (list :nick (avatar-nick avatar))
          (list :room room)
          (list :text text))))

(define (send-ctx avatar room text)
  (let ((user (get-prop (avatar-user-key avatar)))
        (effective-room (if (entity-live? room) room (configured-start-room))))
    (if user
        (ma-send! user (ctx-term avatar effective-room text))
        #f)))

(define (entity-live? actor)
  (and actor (ma-entity-exists? actor)))

(define (member? x xs)
  (cond ((null? xs) #f)
        ((equal? x (car xs)) #t)
        (else (member? x (cdr xs)))))

  (define (remove-one x xs)
    (cond ((null? xs) '())
      ((equal? x (car xs)) (remove-one x (cdr xs)))
      (else (cons (car xs) (remove-one x (cdr xs))))))

(define (avatars)
  (let ((xs (get-prop "avatars")))
    (if xs xs '())))

(define (add-avatar! avatar)
  (if (member? avatar (avatars))
      #f
      (set-prop! "avatars" (cons avatar (avatars)))))

(define (remove-avatar! avatar)
  (if avatar (set-prop! "avatars" (remove-one avatar (avatars))) #f))

(define (avatars-in-room room)
  (let loop ((xs (avatars)))
    (cond ((null? xs) '())
          ((equal? (get-prop (avatar-room-key (car xs))) room)
           (cons (car xs) (loop (cdr xs))))
          (else (loop (cdr xs))))))

(define (avatar-summaries-in-room room)
  (let loop ((xs (avatars-in-room room)))
    (if (null? xs)
        '()
        (cons (list (car xs) (avatar-nick (car xs))) (loop (cdr xs))))))

(define (send-room-ctx room)
  (ma-send! room (list :ctx :avatars (avatar-summaries-in-room room))))

(define (room-init)
  (string-append "(set-prop! \"root\" \"" (self) "\")"))

(define (exit-init direction target-room)
  (string-append
    "(set-prop! \"root\" \"" (self) "\")\n"
    "(set-prop! \"direction\" \"" direction "\")\n"
    "(set-prop! \"target-room\" \"" target-room "\")"))

(define (avatar-init user nick)
  (string-append
    "(set-prop! \"user\" \"" user "\")\n"
    "(set-prop! \"root\" \"" (self) "\")\n"
    "(set-prop! \"nick\" \"" (nick-or-default nick) "\")"))
; Like avatar-init but also sets room and sends :ctx to the user from init.
; Used when creating a NEW avatar so ctx arrives only after the entity is live,
; eliminating the race between root sending ctx and the entity loading.
(define (avatar-init-room user nick room)
  (let ((n (nick-or-default nick))
        (r (self)))
    (string-append
      "(set-prop! \"user\" \"" user "\")\n"
      "(set-prop! \"root\" \"" r "\")\n"
      "(set-prop! \"nick\" \"" n "\")\n"
      "(set-prop! \"room\" \"" room "\")\n"
      "(ma-send! \"" user "\" (list :ctx (list"
      " (list :root \"" r "\")"
      " (list :avatar (ma-get-config-key \"self\"))"
      " (list :nick \"" n "\")"
      " (list :room \"" room "\")"
      " (list :text \"You arrive.\"))))\n")))
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

(define (entry-exit-key room)
  (string-append "entry-exit:" room))

(define (ensure-entry-exit room)
  (let* ((key (entry-exit-key room))
         (existing (get-prop key)))
    (if existing
        existing
        (let* ((fragment (ma-create-actor EXIT_KIND #f (exit-init "in" room)))
               (exit (entity-url fragment)))
          (set-prop! key exit)
          exit))))

; room: if non-#f and avatar must be created, avatar-init-room is used so the
; new entity sends :ctx itself.  When returning an existing live avatar the
; room arg is ignored; the caller is responsible for sending ctx.
(define (ensure-avatar user nick room)
  (let ((existing (get-prop (avatar-key user))))
    (if (entity-live? existing)
        existing
     (let* ((_ (remove-avatar! existing))
            (init-code (if room (avatar-init-room user nick room) (avatar-init user nick)))
            (fragment (ma-create-actor AVATAR_KIND #f init-code user))
            (avatar (entity-url fragment)))
       (set-prop! (avatar-key user) avatar)
       (set-prop! (avatar-user-key avatar) user)
       (set-prop! (avatar-nick-key avatar) (nick-or-default nick))
       (if room (set-prop! (avatar-room-key avatar) room) #f)
       (add-avatar! avatar)
       (ma-save-state!)
       avatar))))

(define (requested-room args)
  (if (or (null? args) (equal? (car args) "")) #f (car args)))

(define (entry-room requested previous)
  (cond ((entity-live? requested) requested)
        ((entity-live? previous) previous)
        (else (ensure-start-room))))

(define (requested-nick args)
  (cond ((null? args) #f)
        ((equal? (car args) "") (if (null? (cdr args)) #f (car (cdr args))))
        ((null? (cdr args)) #f)
        (else (car (cdr args)))))

(define (effective-nick requested existing-avatar)
  (if requested
      requested
      (if existing-avatar
          (let ((nick (get-prop (avatar-nick-key existing-avatar))))
            (if nick nick (default-nick)))
          (default-nick))))

(define (set-avatar-nick! avatar nick)
  (if nick
      (begin
        (set-prop! (avatar-nick-key avatar) nick)
        (if (entity-live? avatar)
            (ma-send! avatar (list :set-nick nick))
            #f)
        (ma-save-state!))
      #f))

(set-method! :enter
  (lambda (args msg)
    (let* ((user (msg-from msg))
           (existing-avatar (get-prop (avatar-key user)))
           (previous-room (if existing-avatar (get-prop (avatar-room-key existing-avatar)) #f))
           (room (entry-room (requested-room args) previous-room))
           (nick (effective-nick (requested-nick args) existing-avatar))
           ; Check liveness BEFORE ensure-avatar so we know if it's a new creation.
           (was-live (entity-live? existing-avatar))
           (avatar (ensure-avatar user nick room)))
      (set-avatar-nick! avatar nick)
      (ma-reply! msg (list :ok avatar))
      (if was-live
          ; Pre-existing: room will :arrived root which sends ctx
          (ma-send! room (list :enter avatar (self)))
          ; Newly created: avatar init sends ctx; join room directly
          (begin
            (set-prop! (avatar-room-key avatar) room)
            (ma-save-state!)
            (send-room-ctx room)
            (ma-send! room (list :join-avatar avatar #f)))))))

(set-method! :avatar?
  (lambda (args msg)
    (let* ((user (msg-from msg))
       (avatar (ensure-avatar user #f #f)))
     (ma-reply! msg (list :ok avatar)))))

(set-method! :arrived
  ; Fire-and-forget only (sent via ma-send! from room/exit actors); no caller
  ; ever awaits a reply here, so do not ma-reply! (a reply to a non-RPC local
  ; send cannot be routed and only produces "unknown local recipient" noise).
  (lambda (args msg)
    (let ((avatar (car args))
          (room (car (cdr args))))
  (if (same-actor? (msg-from msg) room)
          (let ((user (get-prop (avatar-user-key avatar)))
                (old-room (get-prop (avatar-room-key avatar))))
            (set-prop! (avatar-room-key avatar) room)
            (ma-save-state!)
            (send-ctx avatar room "You arrive.")
            (if (and old-room (not (equal? old-room room)))
                (begin
                  (ma-send! old-room (list :leave-avatar avatar room))
                  (send-room-ctx old-room))
                #f)
            (send-room-ctx room)
            (ma-send! room (list :join-avatar avatar old-room))
            #f)
          #f))))

(set-method! :arrive-user
  ; Fire-and-forget only (sent via ma-send! from room.ma's ctx-map avatar
  ; branch); no caller ever awaits a reply here, so do not ma-reply! (a reply
  ; to a non-RPC local send cannot be routed and only produces "unknown local
  ; recipient" noise, and can stall the sending entity's queue while the
  ; runtime retries resolving a bogus target).
  (lambda (args msg)
    (let* ((user (car args))
           (room (car (cdr args)))
           (nick (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args)))))
           ; Capture liveness BEFORE ensure-avatar: determines who sends ctx.
           (was-live (entity-live? (get-prop (avatar-key user))))
           (avatar (ensure-avatar user nick room)))
      (if (same-actor? (msg-from msg) room)
        (begin
          (set-prop! (avatar-room-key avatar) room)
          (if nick (set-prop! (avatar-nick-key avatar) nick) #f)
          (ma-save-state!)
          ; If avatar was already live, send ctx directly — it's ready.
          ; If newly created, avatar-init-room sends ctx once the entity loads.
          (if was-live (send-ctx avatar room "You arrive.") #f)
          (send-room-ctx room)
          (ma-send! room (list :join-avatar avatar #f))
          #f)
        #f))))

(set-method! :nick
  (lambda (args msg)
    (if (null? args)
        (ma-reply! msg (list :error "nick requires a value"))
        (let ((avatar (canonical-actor (msg-from msg)))
              (nick (car args)))
          (if (get-prop (avatar-user-key avatar))
              (let ((room (avatar-room avatar #f))
                    (user (get-prop (avatar-user-key avatar))))
                (set-prop! (avatar-nick-key avatar) nick)
                (ma-save-state!)
                (ma-send! avatar (list :set-nick nick))
                (if room (send-room-ctx room) #f)
                (ma-reply! msg (list :ok nick)))
              (ma-reply! msg (list :error "nick sender must be an avatar")))))))
