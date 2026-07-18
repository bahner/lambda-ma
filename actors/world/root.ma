; Locked world root / placement-registry actor.
; Root owns avatars and protected placement state. Users enter the world here.

(define AVATAR_KIND "/ma/avatar/0.0.1")
(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))

(define (avatar-key user) (string-append "avatar:" user))
(define (avatar-user-key avatar) (string-append "user:" avatar))
(define (avatar-room-key avatar) (string-append "room:" avatar))
(define (avatar-nick-key avatar) (string-append "nick:" avatar))

(define (default-nick) "avatar")

(define (nick-or-default nick)
  (if nick nick (default-nick)))

(define (avatar-nick avatar)
  (let ((nick (get-prop (avatar-nick-key avatar))))
    (if nick nick (default-nick))))

(define (avatar-room avatar fallback)
  (let ((room (get-prop (avatar-room-key avatar))))
    (if room room fallback)))

(define (member? x xs)
  (cond ((null? xs) #f)
        ((equal? x (car xs)) #t)
        (else (member? x (cdr xs)))))

(define (avatars)
  (let ((xs (get-prop "avatars")))
    (if xs xs '())))

(define (add-avatar! avatar)
  (if (member? avatar (avatars))
      #f
      (set-prop! "avatars" (cons avatar (avatars)))))

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

(define (ensure-start-room)
  (let ((existing (get-prop "start")))
    (if existing
        existing
        (let ((legacy (get-prop "start-room")))
          (if legacy
              (begin
                (set-prop! "start" legacy)
                legacy)
              (let* ((fragment (ma-create-actor ROOM_KIND #f (room-init)))
                     (room (entity-url fragment)))
                (set-prop! "start" room)
                room))))))

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

(define (ensure-avatar user nick)
  (let ((existing (get-prop (avatar-key user))))
    (if existing
        existing
        (let* ((fragment (ma-create-actor AVATAR_KIND #f (avatar-init user nick)))
               (avatar (entity-url fragment)))
          (set-prop! (avatar-key user) avatar)
          (set-prop! (avatar-user-key avatar) user)
          (set-prop! (avatar-nick-key avatar) (nick-or-default nick))
          (add-avatar! avatar)
          (ma-save-state!)
          avatar))))

(define (send-ctx user avatar room text)
  (ma-send! user
    (list :ctx
      (list (list :root (self))
            (list :avatar avatar)
            (list :nick (avatar-nick avatar))
            (list :room room)
            (list :text text)))))

(define (requested-room args)
  (if (or (null? args) (equal? (car args) ""))
      (ensure-start-room)
      (car args)))

(define (requested-nick args)
  (cond ((null? args) #f)
        ((equal? (car args) "") (if (null? (cdr args)) #f (car (cdr args))))
        ((null? (cdr args)) #f)
        (else (car (cdr args)))))

(define (set-avatar-nick! avatar nick)
  (if nick
      (begin
        (set-prop! (avatar-nick-key avatar) nick)
        (ma-send! avatar (list :set-nick nick))
        (ma-save-state!))
      #f))

(set-method! :enter
  (lambda (args msg)
    (let* ((user (msg-from msg))
           (room (requested-room args))
           (nick (requested-nick args))
       (avatar (ensure-avatar user nick))
       (entry-exit (ensure-entry-exit room)))
      (set-avatar-nick! avatar nick)
     (ma-send! entry-exit (list :traverse avatar (self))))))

(set-method! :avatar?
  (lambda (args msg)
    (let* ((user (msg-from msg))
       (avatar (ensure-avatar user #f)))
     (ma-reply! msg (list :ok avatar)))))

(set-method! :arrived
  (lambda (args msg)
    (let ((avatar (car args))
          (room (car (cdr args))))
      (if (equal? (msg-from msg) room)
          (let ((user (get-prop (avatar-user-key avatar)))
                (old-room (get-prop (avatar-room-key avatar))))
            (set-prop! (avatar-room-key avatar) room)
            (ma-save-state!)
            (ma-send! avatar (list :set-location room))
            (if (and old-room (not (equal? old-room room)))
                (begin
                  (ma-send! old-room (list :leave-avatar avatar room))
                  (send-room-ctx old-room))
                #f)
            (send-room-ctx room)
            (ma-send! room (list :join-avatar avatar old-room))
            (if user
                (send-ctx user avatar room "You arrive.")
                #f)
            (ma-reply! msg (list :ok "arrived")))
          (ma-reply! msg (list :error "arrival sender must be target room"))))))

(set-method! :nick
  (lambda (args msg)
    (if (null? args)
        (ma-reply! msg (list :error "nick requires a value"))
        (let ((avatar (msg-from msg))
              (nick (car args)))
          (if (get-prop (avatar-user-key avatar))
              (let ((room (avatar-room avatar #f))
                    (user (get-prop (avatar-user-key avatar))))
                (set-prop! (avatar-nick-key avatar) nick)
                (ma-save-state!)
                (if room (send-room-ctx room) #f)
                (if user (send-ctx user avatar room #f) #f)
                (ma-reply! msg (list :ok nick)))
              (ma-reply! msg (list :error "nick sender must be an avatar")))))))

(set-method! :avatars
  (lambda (args msg)
    (let ((room (if (null? args) (msg-from msg) (car args))))
      (if (equal? (msg-from msg) room)
          (begin
            (send-room-ctx room)
            (ma-reply! msg (list :ok "avatars")))
          (ma-reply! msg (list :error "avatar context request sender mismatch"))))))
