; Locked avatar actor.
; Root owns protected state. The controlling DID may call exposed command methods only.

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define LAMBDA_CTX_PROTOCOL "/ma/lambda/ctx/0.0.1")
(define ENTITY_FRAGMENT_CONTEXT "ma entity-fragment v1")
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (did) (get-prop "did"))
(define (root)
  (let ((configured (ma-get-config-key "root")))
    (if configured configured (entity-url "root"))))
(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor)) (string-append (runtime) actor) actor))
(define (local-fragment? actor)
  (and actor (string-prefix? "#" actor)))
(define (qualified-actor actor)
  (if actor (canonical-actor actor) ""))
(define (qualified-ctx-actor? actor)
  (and (non-empty-string? actor)
       (not (local-fragment? actor))))
(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))
(define (entity-id) (ma-get-config-key "id"))
(define (avatar-fragment did)
  (ma-derived-id ENTITY_FRAGMENT_CONTEXT did 8))
(define (valid-did? value)
  (and (string? value) (string-prefix? "did:ma:" value)))
(define (ensure-did! expected-did expected-id)
  (if (and (valid-did? expected-did)
           (equal? expected-id (entity-id)))
      (let ((current (did)))
        (cond ((not current)
               (begin
                 (set-prop! "did" expected-did)
                 (ma-save-state!)
                 #t))
              ((equal? current expected-did) #t)
              (else #f)))
      #f))
(define (ensure-msg-did! msg)
  (let ((candidate (msg-from msg)))
    (if (and (valid-did? candidate)
             (equal? (avatar-fragment candidate) (entity-id)))
        (let ((current (did)))
          (cond ((not current)
                 (begin
                   (set-prop! "did" candidate)
                   (ma-save-state!)
                   #t))
                ((equal? current candidate) #t)
                (else #f)))
        #f)))
(define (room) (get-prop "room"))
(define (nick)
  (let ((value (get-prop "nick")))
    (if value value "avatar")))

(define (ctx-term text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (qualified-actor (root)))
          (list :avatar (qualified-actor (self)))
          (list :nick (nick))
          (list :room (qualified-actor (room)))
          (list :text text))))

(define (ctx-term-room r text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (qualified-actor (root)))
          (list :avatar (qualified-actor (self)))
          (list :nick (nick))
          (list :room (qualified-actor r))
          (list :text text))))

(define (start-room) (ma-get-config-key "start"))

(define (send-ctx text)
  (ma-send! (did) (ctx-term text)))

(define (did? msg) (ensure-msg-did! msg))
(define (root? msg) (same-actor? (msg-from msg) (root)))
(define (room? msg)
  (let ((current (room)))
    (and current (same-actor? (msg-from msg) current))))

(define (ctx-value pairs key)
  (cond ((null? pairs) #f)
        ((and (pair? (car pairs))
              (equal? (car (car pairs)) key)
              (not (null? (cdr (car pairs)))))
         (car (cdr (car pairs))))
        (else (ctx-value (cdr pairs) key))))

(define (avatar-ctx-valid? payload msg)
  (let ((protocol (ctx-value payload :protocol))
        (kind (ctx-value payload :kind))
        (root (ctx-value payload :root))
        (avatar (ctx-value payload :avatar))
        (target-room (ctx-value payload :room)))
    (and (equal? protocol LAMBDA_CTX_PROTOCOL)
         (equal? kind "avatar")
         (qualified-ctx-actor? root)
         (qualified-ctx-actor? avatar)
         (qualified-ctx-actor? target-room)
         (same-actor? avatar (self))
         (same-actor? (msg-from msg) target-room))))

(define (enter-room-authorised? args msg)
  (and (not (null? args))
       (not (null? (cdr args)))
       (not (null? (cdr (cdr args))))
       (ensure-did! (car (cdr args)) (car (cdr (cdr args))))
       (or (root? msg)
           (same-actor? (msg-from msg) (car args)))))

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(define (require-did msg thunk)
  (if (did? msg)
      (thunk)
      (ma-reply! msg (list :error "avatar command denied"))))

(define (send-room verb args)
  (let ((target (room)))
    (if target
        (ma-send! target (cons verb args))
        (let ((start (start-room)))
          (if start
              (ma-send! (did) (ctx-term-room start #f))
              (ma-send! (did) (list :print "You are nowhere.")))))))

(define (send-room-as-did verb args)
  (send-room verb (cons (did) args)))

(define (send-did-text text)
  (ma-send! (did) (list :print text)))

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

(set-method! :sync-ctx
  (lambda (args msg)
    (if (root? msg)
        (send-ctx #f)
        #f)))

(set-method! :enter-room
  (lambda (args msg)
    (if (and (enter-room-authorised? args msg) (not (null? args)))
        (let ((target-room (car args))
              (old-room (room))
              (requested-nick (if (or (null? (cdr args))
                                      (null? (cdr (cdr args)))
                                      (null? (cdr (cdr (cdr args)))))
                                  #f
                                  (car (cdr (cdr (cdr args)))))))
          (if (non-empty-string? requested-nick)
              (begin
                (set-prop! "nick" requested-nick)
                (ma-save-state!))
              #f)
          (ma-send! target-room (list :enter (self) old-room (nick))))
        #f)))

(set-method! :ctx
  (lambda (args msg)
    (if (null? args)
        #f
        (let ((payload (car args)))
          (if (avatar-ctx-valid? payload msg)
              (begin
                (set-prop! "room" (ctx-value payload :room))
                (set-prop! "nick" (ctx-value payload :nick))
                (ma-save-state!)
                (ma-send! (did) (cons :ctx args)))
              #f)))))

(set-method! :ctx?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (ma-reply! msg (list :ok (ctx-term #f)))))))

(set-method! :print
  (lambda (args msg)
    (ma-send! (did) (list :print (join-words args)))))

(set-method! :here?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (ma-reply! msg (list :ok (room)))))))

(set-method! :help
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (cond ((null? args)
               (begin
                 (send-did-text (avatar-help-text))
                 (ma-reply! msg (list :ok "help"))))
              ((equal? (car args) "here")
               (begin
                 (send-room :help '())
                 (ma-reply! msg (list :ok "help here"))))
              (else
               (begin
                 (send-did-text (unknown-help-text (car args)))
                 (ma-reply! msg (list :ok "help")))))))))

(set-method! :nick
  (lambda (args msg)
    (require-did msg
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
    (require-did msg
      (lambda ()
        (send-room :look '())
        (reply-ok-silent msg)))))

(set-method! :exits?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :exits? '())
        (reply-ok-silent msg)))))

(set-method! :who?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :who? '())
        (reply-ok-silent msg)))))

(set-method! :say
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :say (list (join-words args)))
        (reply-ok-silent msg)))))

(set-method! :emote
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :emote (list (join-words args)))
        (reply-ok-silent msg)))))

(set-method! :claim
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :claim args)
        (reply-ok-silent msg)))))

(set-method! :owner
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :owner args)
        (reply-ok-silent msg)))))

(set-method! :dig
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :dig args)
        (reply-ok-silent msg)))))

(set-method! :prop
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :prop args)
        (reply-ok-silent msg)))))

(set-method! :go
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :go args)
        (reply-ok-silent msg)))))

(set-method! :drop-thing
  (lambda (args msg)
    (if (room-caller? msg)
        (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
            #f
            (let ((did (car args))
                  (thing (car (cdr args)))
          (target-parent (car (cdr (cdr args))))
          (ctx (if (or (null? (cdr (cdr (cdr args)))) (not (map? (car (cdr (cdr (cdr args))))))) #f (car (cdr (cdr (cdr args)))))))
        (if ctx
          (ma-send! thing (list :drop did target-parent ctx))
          (ma-send! thing (list :drop did target-parent)))))
        #f)))

(set-default-method!
  (lambda (verb args msg)
    (require-did msg
      (lambda ()
        (send-room verb args)
        (reply-ok-silent msg)))))
