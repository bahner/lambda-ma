; Locked avatar actor.
; Root owns protected state. The user may call exposed command methods only.

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (user) (get-prop "user"))
(define (root)
  (let ((configured (ma-get-config-key "root")))
    (if configured configured (entity-url "root"))))
(define (room) (get-prop "room"))
(define (nick)
  (let ((value (get-prop "nick")))
    (if value value "avatar")))

(define (user? msg) (equal? (msg-from msg) (user)))
(define (root? msg) (equal? (msg-from msg) (root)))

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

(set-method! :set-location
  (lambda (args msg)
    (if (root? msg)
        (begin
          (set-prop! "room" (car args))
          (ma-save-state!))
        #f)))

(set-method! :set-nick
  (lambda (args msg)
    (if (root? msg)
        (begin
          (set-prop! "nick" (car args))
          (ma-save-state!))
        #f)))

(set-method! :print
  (lambda (args msg)
    (ma-send! (user) (list :print (join-words args)))))

(set-method! :here?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (ma-reply! msg (list :ok (room)))))))

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
        (ma-reply! msg (list :ok "looking"))))))

(set-method! :exits
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :exits '())
        (ma-reply! msg (list :ok "checking exits"))))))

(set-method! :exits?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :exits? '())
        (ma-reply! msg (list :ok "checking exits"))))))

(set-method! :who?
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :who? '())
        (ma-reply! msg (list :ok "checking who"))))))

(set-method! :say
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :say (list (join-words args)))
        (ma-reply! msg (list :ok "said"))))))

(set-method! :emote
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :emote (list (join-words args)))
        (ma-reply! msg (list :ok "emoted"))))))

(set-method! :claim
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :claim args)
        (ma-reply! msg (list :ok "claiming"))))))

(set-method! :owner
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :owner args)
        (ma-reply! msg (list :ok "owner"))))))

(set-method! :dig
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room-as-user :dig args)
        (ma-reply! msg (list :ok "digging"))))))

(set-method! :go
  (lambda (args msg)
    (require-user msg
      (lambda ()
        (send-room :go args)
        (ma-reply! msg (list :ok "going"))))))
