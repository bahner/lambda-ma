; Locked avatar actor.
; Root owns protected state. The user may call exposed command methods only.

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
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
    (list (list :root (root))
          (list :avatar (self))
          (list :nick (nick))
          (list :room (room))
          (list :text text))))

(define (send-ctx text)
  (ma-send! (user) (ctx-term text)))

(define (user? msg) (equal? (msg-from msg) (user)))
(define (root? msg) (same-actor? (msg-from msg) (root)))

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
        (ma-send! (user) (list :print "You are nowhere.")))))

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
    "  exits             list exits\n"
    "  who?              show who is here\n"
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

(set-method! :set-location
  (lambda (args msg)
    (if (root? msg)
        (let ((new-room (car args))
              (text (if (or (null? (cdr args)) (equal? (car (cdr args)) "")) #f (car (cdr args)))))
          (set-prop! "room" new-room)
          (ma-save-state!)
          (send-ctx text))
        #f)))

(set-method! :set-nick
  (lambda (args msg)
    (if (root? msg)
        (begin
          (set-prop! "nick" (car args))
          (ma-save-state!)
          (send-ctx #f))
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
            (let ((new-nick (join-words args)))
              (set-prop! "nick" new-nick)
              (ma-save-state!)
              (ma-send! (root) (list :nick new-nick))
              (ma-reply! msg (list :ok new-nick))))))))

(set-method! :look
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :look '())
        (reply-ok-silent msg)))))

(set-method! :exits
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :exits '())
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

(set-default-method!
  (lambda (verb args msg)
    (require-user msg
      (lambda ()
        (send-room verb args)
        (reply-ok-silent msg)))))
