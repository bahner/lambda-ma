; Locked room actor.
; Rooms own exits and local room policy. Avatars act through their current room.

(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (root)
  (let ((configured (ma-get-config-key "root")))
    (if configured configured (entity-url "root"))))

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(define (member? x xs)
  (cond ((null? xs) #f)
        ((equal? x (car xs)) #t)
        (else (member? x (cdr xs)))))

(define (occupants)
  (let ((xs (get-prop "occupants")))
    (if xs xs '())))

(define (add-occupant! avatar)
  (if (member? avatar (occupants))
      #f
      (set-prop! "occupants" (cons avatar (occupants)))))

(define (remove-one x xs)
  (cond ((null? xs) '())
        ((equal? x (car xs)) (remove-one x (cdr xs)))
        (else (cons (car xs) (remove-one x (cdr xs))))))

(define (remove-occupant! avatar)
  (set-prop! "occupants" (remove-one avatar (occupants))))

(define (label-key actor) (string-append "label:" actor))

(define (avatar-ref entry)
  (if (pair? entry) (car entry) entry))

(define (avatar-label entry)
  (if (and (pair? entry) (pair? (cdr entry))) (car (cdr entry)) #f))

(define (avatar-refs entries)
  (if (null? entries)
      '()
      (cons (avatar-ref (car entries)) (avatar-refs (cdr entries)))))

(define (store-labels! entries)
  (if (null? entries)
      #f
      (let ((avatar (avatar-ref (car entries)))
            (label (avatar-label (car entries))))
        (if label (set-prop! (label-key avatar) label) #f)
        (store-labels! (cdr entries)))))

(define (speaker-name actor)
  (let ((label (get-prop (label-key actor))))
    (if label label actor)))

(define (room-name)
  (let ((name (get-prop "name")))
    (if name name "A Room")))

(define (room-description)
  (let ((description (get-prop "description")))
    (if description description "You are in a room.")))

(define (room-text)
  (string-append (room-name) "\n" (room-description)))

(define (names-of actors)
  (cond ((null? actors) "")
        ((null? (cdr actors)) (speaker-name (car actors)))
        (else (string-append (speaker-name (car actors)) ", " (names-of (cdr actors))))))

(define (from-root? msg)
  (equal? (msg-from msg) (root)))

(define (owner) (get-prop "owner"))
(define (owned?) (if (owner) #t #f))
(define (owner? user) (equal? user (owner)))

(define (valid-owner? value)
  (and (string? value) (not (equal? value ""))))

(define (set-owner! user)
  (set-prop! "owner" user)
  (ma-save-state!))

(define (set-room-prop! key value)
  (set-prop! key value)
  (ma-save-state!))

(define (reply-to-sender msg text)
  (ma-send! (msg-from msg) (list :print text)))

(define (reply-ok msg text)
  (ma-reply! msg (list :ok text)))

(define (reply-error msg text)
  (ma-reply! msg (list :error text)))

(define (apply-room-prop! msg key value-args)
  (if (null? value-args)
      (begin
        (del-prop! key)
        (ma-save-state!)
        (reply-ok msg (string-append "Reset prop " key ".")))
      (begin
        (set-room-prop! key (join-words value-args))
        (reply-ok msg (string-append "Set prop " key ".")))))

(define (handle-room-prop! msg args)
  (let ((user (msg-from msg)))
    (cond ((null? args)
           (reply-error msg "Usage: prop <key> [value]"))
          ((equal? (car args) "")
           (reply-error msg "Prop key must be non-empty."))
          ((not (valid-owner? user))
           (reply-error msg "Owner must be a non-empty user DID."))
          ((not (owned?))
           (reply-error msg "This room is unowned. Claim it before building here."))
          ((not (owner? user))
           (reply-error msg "Only this room's owner can build exits here."))
          (else
           (apply-room-prop! msg (car args) (cdr args))))))

(define (delegated-call? args msg)
  (and (not (null? args)) (member? (msg-from msg) (occupants))))

(define (caller-user args msg)
  (if (delegated-call? args msg) (car args) (msg-from msg)))

(define (command-args args msg)
  (if (delegated-call? args msg) (cdr args) args))

(define (require-valid-owner user msg thunk)
  (if (valid-owner? user)
      (thunk)
      (reply-to-sender msg "Owner must be a non-empty user DID.")))

(define (require-owner user msg thunk)
  (cond ((not (owned?))
         (reply-to-sender msg "This room is unowned. Claim it before building here."))
        ((owner? user) (thunk))
        (else
         (reply-to-sender msg "Only this room's owner can build exits here."))))

(define (on-event event args msg)
  (cond ((equal? event :join-avatar)
         (let ((avatar (car args)))
           (add-occupant! avatar)
           (ma-save-state!)
           (broadcast (string-append (speaker-name avatar) " arrives."))))
        ((equal? event :leave-avatar)
         (let ((avatar (car args)))
           (remove-occupant! avatar)
           (ma-save-state!)
           (broadcast (string-append (speaker-name avatar) " leaves."))))
        (else #f)))

(define (broadcast text)
  (let loop ((xs (occupants)))
    (if (null? xs)
        #f
        (begin
          (ma-send! (car xs) (list :print text))
          (loop (cdr xs))))))

(define (exit-key direction) (string-append "exit:" direction))

(define (room-init name owner-did)
  (if name
      (string-append
        "(set-prop! \"root\" \"" (root) "\")\n"
        "(set-prop! \"name\" \"" name "\")\n"
        "(set-prop! \"owner\" \"" owner-did "\")\n"
        "(ma-save-state!)")
      (string-append
        "(set-prop! \"root\" \"" (root) "\")\n"
        "(set-prop! \"owner\" \"" owner-did "\")\n"
        "(ma-save-state!)")))

(define (exit-init direction target-room)
  (string-append
    "(set-prop! \"direction\" \"" direction "\")\n"
    "(set-prop! \"target-room\" \"" target-room "\")"))

(define (dig-target-args args)
  (if (null? args)
      '()
      (let ((rest (cdr args)))
        (if (and (not (null? rest)) (equal? (car rest) "to"))
            (cdr rest)
            rest))))

(define (dig-target-text args)
  (let ((target-args (dig-target-args args)))
    (if (null? target-args) #f (join-words target-args))))

(define (existing-room-target target)
  (cond ((equal? target "#construct") (entity-url "construct"))
        (else #f)))

(set-method! :join-avatar
  (lambda (args msg)
    (if (from-root? msg)
        (on-event :join-avatar args msg)
        #f)))

(set-method! :leave-avatar
  (lambda (args msg)
    (if (from-root? msg)
        (on-event :leave-avatar args msg)
        #f)))

(set-method! :ctx
  (lambda (args msg)
    (if (from-root? msg)
        (let ((kind (car args))
              (payload (car (cdr args))))
          (if (equal? kind :avatars)
              (begin
                (set-prop! "occupants" (avatar-refs payload))
                (store-labels! payload)
                (ma-save-state!))
              #f))
        #f)))

(set-method! :look
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (ma-send! avatar (list :print (room-text))))))

(set-method! :exits
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (ma-send! avatar (list :print "Exits are whatever has been dug from here.")))))

(set-method! :say
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args)))
      (broadcast (string-append (speaker-name speaker) " says: " text)))))

(set-method! :emote
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args)))
      (broadcast (string-append (speaker-name speaker) " " text)))))

(set-method! :claim
  (lambda (args msg)
    (let ((user (caller-user args msg)))
      (require-valid-owner user msg
        (lambda ()
          (if (owned?)
              (reply-to-sender msg (string-append "This room is already owned by " (owner) "."))
              (begin
                (set-owner! user)
                (reply-to-sender msg (string-append "You now own " (room-name) ".")))))))))

(set-method! :owner
  (lambda (args msg)
    (let ((user (caller-user args msg))
          (owner-args (command-args args msg)))
      (if (null? owner-args)
          (let ((current-owner (owner)))
            (if current-owner
                (reply-to-sender msg (string-append "Owner: " current-owner))
                (reply-to-sender msg "This room is unowned.")))
          (require-valid-owner user msg
            (lambda ()
              (require-owner user msg
                (lambda ()
                  (let ((new-owner (car owner-args)))
                    (if (valid-owner? new-owner)
                        (begin
                          (set-owner! new-owner)
                          (reply-to-sender msg (string-append "Owner set to " new-owner ".")))
                        (reply-to-sender msg "New owner must be a non-empty user DID.")))))))))))

(set-method! :prop
  (lambda (args msg)
    (handle-room-prop! msg args)))

(set-method! :dig
  (lambda (args msg)
    (let* ((user (caller-user args msg))
           (dig-args (command-args args msg))
           (direction (if (null? dig-args) "out" (car dig-args))))
      (require-valid-owner user msg
        (lambda ()
          (require-owner user msg
            (lambda ()
              (let ((existing-exit (get-prop (exit-key direction))))
                (if existing-exit
                    (reply-to-sender msg (string-append "There is already an exit " direction "."))
                          (let* ((target (dig-target-text dig-args))
                           (existing-room (existing-room-target target)))
                      (if existing-room
                          (reply-to-sender msg "Existing-room links need ownership of both rooms and are not supported yet.")
                          (let* ((target-room (entity-url (ma-create-actor ROOM_KIND #f (room-init target user))))
                                 (exit-fragment (ma-create-actor EXIT_KIND #f (exit-init direction target-room)))
                                 (exit (entity-url exit-fragment)))
                            (set-prop! (exit-key direction) exit)
                            (ma-save-state!)
                            (broadcast (string-append user " digs " direction "."))
                            (reply-to-sender msg (string-append "You dig " direction " and open a new exit."))))))))))))))

(set-method! :go
  (lambda (args msg)
    (let ((avatar (msg-from msg))
          (direction (if (null? args) "out" (car args))))
      (let ((exit (get-prop (exit-key direction))))
        (if exit
            (ma-send! exit (list :traverse avatar (self)))
            (ma-send! avatar (list :print (string-append "No exit " direction "."))))))))

(set-method! :enter-avatar
  (lambda (args msg)
    (let ((avatar (car args)))
      (ma-send! (root) (list :arrived avatar (self))))))
