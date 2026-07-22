; Locked avatar actor.
; Root owns protected state. The user may call exposed command methods only.

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define LAMBDA_CTX_PROTOCOL "/ma/lambda/ctx/0.0.1")
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (user) (get-prop "user"))
(define (root)
  (let ((configured (ma-get-config-key "root")))
    (if configured configured (entity-url "root"))))
(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor)) (string-append (runtime) actor) actor))
(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))
(define (room) (get-prop "room"))
(define (nick)
  (let ((value (get-prop "nick")))
    (if value value "avatar")))

(define (ctx-term text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (root))
          (list :avatar (self))
          (list :nick (nick))
          (list :room (room))
          (list :text text))))

(define (ctx-term-room r text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (root))
          (list :avatar (self))
          (list :nick (nick))
          (list :room r)
          (list :text text))))

(define (start-room) (ma-get-config-key "start"))

(define (send-ctx text)
  (ma-send! (user) (ctx-term text)))

(define (user? msg) (equal? (msg-from msg) (user)))
(define (root? msg) (same-actor? (msg-from msg) (root)))
(define (room? msg)
  (let ((current (room)))
    (and current (same-actor? (msg-from msg) current))))
(define (entered-room-caller? new-room msg)
  (and new-room (same-actor? (msg-from msg) new-room)))

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(define (require-user msg thunk)
  (if (user? msg)
      (thunk)
      (ma-reply! msg (list :error "avatar command denied"))))

(define (send-room verb args)
  (let ((target (room)))
    (if target
        (ma-send! target (cons verb args))
        (let ((start (start-room)))
          (if start
              (ma-send! (user) (ctx-term-room start #f))
              (ma-send! (user) (list :print "You are nowhere.")))))))

(define (send-room-as-user verb args)
  (send-room verb (cons (user) args)))

(define (send-user-text text)
  (ma-send! (user) (list :print text)))

(define (reply-ok-silent msg)
  (ma-reply! msg (list :ok "")))

(define (avatar-help-text)
  (string-append
    "Help\n"
    "  help              show this help\n"
    "  help here         ask this place what is possible here\n"
    "  look              look around\n"
    "  exits?            list exits\n"
    "  who?              show who is here\n"
    "  things?           list local non-avatar occupants\n"
    "  take <thing>      ask a local occupant to bind to you\n"
    "  drop <thing>      ask a carried occupant to enter this room\n"
    "  where <thing>     ask where a local occupant says it is\n"
    "  say <text>        speak here\n"
    "  emote <text>      act here\n"
    "  go <direction>    move through an exit\n"
    "  claim             claim an unowned room\n"
    "  owner [did]       show or transfer room ownership\n"
    "  dig <dir> [to name] [with code] create an exit\n"
    "  prop <key> [value] set or reset room text\n"
    "  nick [name]       show or set your display name\n"
    "Use :help for the focused actor directly."))

(define (unknown-help-text topic)
  (string-append "No help topic: " topic "\nTry help or help here."))

(set-method! :entered-room
  (lambda (args msg)
    (let ((new-room (car args))
          (text (if (or (null? (cdr args)) (equal? (car (cdr args)) "")) #f (car (cdr args))))
          (old-room (room)))
            (if (entered-room-caller? new-room msg)
          (begin
            (set-prop! "room" new-room)
            (ma-save-state!)
            (send-ctx text)
            (if (same-actor? new-room old-room)
                #f
                (if old-room (ma-send! old-room (list :leave-avatar (self) new-room)) #f)))
          #f))))

(set-method! :sync-ctx
  (lambda (args msg)
    (if (root? msg)
        (send-ctx #f)
        #f)))

(set-method! :ctx?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (ma-reply! msg (list :ok (ctx-term #f)))))))

(set-method! :print
  (lambda (args msg)
    (ma-send! (user) (list :print (join-words args)))))

(set-method! :here?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (ma-reply! msg (list :ok (room)))))))

(set-method! :help
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (cond ((null? args)
               (begin
                 (send-user-text (avatar-help-text))
                 (ma-reply! msg (list :ok "help"))))
              ((equal? (car args) "here")
               (begin
                 (send-room :help '())
                 (ma-reply! msg (list :ok "help here"))))
              (else
               (begin
                 (send-user-text (unknown-help-text (car args)))
                 (ma-reply! msg (list :ok "help")))))))))

(set-method! :nick
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (if (null? args)
            (ma-reply! msg (list :ok (nick)))
            (begin
              (set-prop! "nick" (join-words args))
              (ma-save-state!)
              (send-ctx #f)
              (send-room :nick args)
              (reply-ok-silent msg)))))))

(set-method! :look
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :look '())
        (reply-ok-silent msg)))))

(set-method! :exits?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :exits? '())
        (reply-ok-silent msg)))))

(set-method! :who?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :who? '())
        (reply-ok-silent msg)))))

(set-method! :say
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :say (list (join-words args)))
        (reply-ok-silent msg)))))

(set-method! :emote
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :emote (list (join-words args)))
        (reply-ok-silent msg)))))

(set-method! :claim
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :claim args)
        (reply-ok-silent msg)))))

(set-method! :owner
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :owner args)
        (reply-ok-silent msg)))))

(set-method! :dig
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :dig args)
        (reply-ok-silent msg)))))

(set-method! :prop
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :prop args)
        (reply-ok-silent msg)))))

(set-method! :go
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :go args)
        (reply-ok-silent msg)))))

(set-method! :drop-thing
  (lambda (args msg)
    (if (room-caller? msg)
        (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
            #f
            (let ((user (car args))
                  (thing (car (cdr args)))
          (target-parent (car (cdr (cdr args))))
          (ctx (if (or (null? (cdr (cdr (cdr args)))) (not (map? (car (cdr (cdr (cdr args))))))) #f (car (cdr (cdr (cdr args)))))))
        (if ctx
          (ma-send! thing (list :drop user target-parent ctx))
          (ma-send! thing (list :drop user target-parent)))))
        #f)))

(set-default-method!
  (lambda (verb args msg)
    (require-user msg
      (lambda ()
        (send-room verb args)
        (reply-ok-silent msg)))))
