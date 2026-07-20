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
    (if name name "Construct")))

(define (room-description)
  (let ((description (get-prop "description")))
    (if description description "“This is the Construct. It's our loading program. We can load anything... From clothing to equipment, weapons, training simulations; anything we need.”")))

(define (room-text)
  (string-append (room-name) "\n" (room-description)))

(define (names-of actors)
  (cond ((null? actors) "")
        ((null? (cdr actors)) (speaker-name (car actors)))
        (else (string-append (speaker-name (car actors)) ", " (names-of (cdr actors))))))

(define (exits)
  (let ((xs (get-prop "exits")))
    (if (map? xs) xs (make-map))))

(define (put-exit! direction exit)
  (set-prop! "exits" (map-set (exits) direction exit)))

(define (exit-target direction)
  (let ((exit (map-ref (exits) direction #f)))
    (if exit exit (get-prop (exit-key direction)))))

(define (exit-directions)
  (map-keys (exits)))

(define (exits-text)
  (let ((directions (exit-directions)))
    (if (null? directions)
        "Exits: none."
        (string-append "Exits: " (names-of directions)))))

(define (who-text)
  (let ((actors (occupants)))
    (if (null? actors)
        "Nobody is here."
        (string-append "Here: " (names-of actors)))))

(define (room-help-text)
  (string-append
    (room-name) " help\n"
    "  look              look around\n"
    "  exits             list exits\n"
    "  who?              show who is here\n"
    "  say <text>        speak here\n"
    "  emote <text>      act here\n"
    "  go <direction>    move through an exit\n"
    "  claim             claim this room if it is unowned\n"
    "  owner [did]       show or transfer ownership\n"
    "  dig <dir> [to name] [with code] create an exit\n"
    "  :behaviour /ipfs/<cid> add or replace this room's own code\n"
    "  :prop <key> [value] set or reset room text\n"
    "Commands with : hit this place directly; commands without : go through your avatar."))

(define (avatar-caller? msg)
  (member? (msg-from msg) (occupants)))

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

(define (reply-room-prop-ok msg delegated text)
  (if delegated
      (reply-to-sender msg text)
      (reply-ok msg text)))

(define (reply-room-prop-error msg delegated text)
  (if delegated
      (reply-to-sender msg text)
      (reply-error msg text)))

(define (apply-room-prop! msg key value-args delegated)
  (if (null? value-args)
      (begin
        (del-prop! key)
        (ma-save-state!)
        (reply-room-prop-ok msg delegated (string-append "Reset prop " key ".")))
      (begin
        (set-room-prop! key (join-words value-args))
        (reply-room-prop-ok msg delegated (string-append "Set prop " key ".")))))

(define (handle-room-prop! msg args)
  (let ((delegated (delegated-call? args msg))
        (user (caller-user args msg))
        (prop-args (command-args args msg)))
    (cond ((null? args)
           (reply-room-prop-error msg delegated "Usage: prop <key> [value]"))
          ((null? prop-args)
           (reply-room-prop-error msg delegated "Usage: prop <key> [value]"))
          ((equal? (car prop-args) "")
           (reply-room-prop-error msg delegated "Prop key must be non-empty."))
          ((not (valid-owner? user))
           (reply-room-prop-error msg delegated "Owner must be a non-empty user DID."))
          ((not (owned?))
           (reply-room-prop-error msg delegated "This room is unowned. Claim it before building here."))
          ((not (owner? user))
           (reply-room-prop-error msg delegated "Only this room's owner can set props here."))
          (else
           (apply-room-prop! msg (car prop-args) (cdr prop-args) delegated)))))

(define (handle-room-behaviour! msg args)
  (let ((user (msg-from msg)))
    (cond ((null? args)
           (let ((current (ma-get-config-key "behaviour")))
             (if current
                 (reply-ok msg current)
                 (reply-ok msg "No custom behaviour is set for this room."))))
          ((null? (cdr args))
           (cond ((not (valid-owner? user))
                  (reply-error msg "Owner must be a non-empty user DID."))
                 ((not (owned?))
                  (reply-error msg "This room is unowned. Claim it before editing behaviour."))
                 ((not (owner? user))
                  (reply-error msg "Only this room's owner can edit behaviour."))
                 (else
                  (begin
                    (ma-set-behaviour! (car args))
                    (reply-ok msg "Behaviour update queued.")))))
          (else
           (reply-error msg "Usage: behaviour /ipfs/<cid>")))))

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

(define (pending-link-key direction) (string-append "pending-link:" direction))
(define (pending-link-user-key direction) (string-append "pending-link-user:" direction))
(define (pending-link-requester-key direction) (string-append "pending-link-requester:" direction))

(define (clear-pending-link! direction)
  (begin
    (del-prop! (pending-link-key direction))
    (del-prop! (pending-link-user-key direction))
    (del-prop! (pending-link-requester-key direction))))

(define (create-exit! direction target-room)
  (let* ((exit-fragment (ma-create-actor EXIT_KIND #f (exit-init direction target-room)))
         (exit (entity-url exit-fragment)))
    (set-prop! (exit-key direction) exit)
    (put-exit! direction exit)
    exit))

(define (room-init name owner-did custom-init)
  (string-append
    "(set-prop! \"root\" \"" (root) "\")\n"
    (if name (string-append "(set-prop! \"name\" \"" name "\")\n") "")
    "(set-prop! \"owner\" \"" owner-did "\")\n"
    "(ma-save-state!)\n"
    (if custom-init custom-init "")))

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
  (let ((target-args (take-before "with" (dig-target-args args))))
    (if (null? target-args) #f (join-words target-args))))

(define (take-before marker words)
  (cond ((null? words) '())
        ((equal? (car words) marker) '())
        (else (cons (car words) (take-before marker (cdr words))))))

(define (drop-through marker words)
  (cond ((null? words) '())
        ((equal? (car words) marker) (cdr words))
        (else (drop-through marker (cdr words)))))

(define (dig-custom-init-text args)
  (let ((init-args (drop-through "with" (dig-target-args args))))
    (if (or (null? init-args) (null? (cdr init-args))) #f (join-words init-args))))

(define (dig-custom-behaviour-ref args)
  (let ((code-args (drop-through "with" (dig-target-args args))))
    (if (and (not (null? code-args)) (null? (cdr code-args)))
        (car code-args)
        #f)))

(define (existing-room-target target)
  (cond ((and target (string-prefix? "#" target) (ma-entity-exists? target))
         (string-append (runtime) target))
        ((and target (string-prefix? "did:ma:" target)) target)
        ((and target (ma-entity-exists? target)) target)
        (else #f)))

(define (request-link-authorization! requester user direction target-room)
  (begin
    (ma-send! target-room (list :authorize-link user direction requester))
    (ma-send! requester (list :print (string-append "Checking ownership of " target-room ".")))))

(define (request-existing-link! msg user direction target-room)
  (let ((requester (msg-from msg)))
    (set-prop! (pending-link-key direction) target-room)
    (set-prop! (pending-link-user-key direction) user)
    (set-prop! (pending-link-requester-key direction) requester)
    (ma-save-state!)
    (ma-send! target-room (list :ping user direction requester))
    (reply-to-sender msg (string-append "Checking reachability of " target-room "."))))

(define (pending-link-matches? direction user target-room requester)
  (and (equal? (get-prop (pending-link-key direction)) target-room)
       (equal? (get-prop (pending-link-user-key direction)) user)
       (equal? (get-prop (pending-link-requester-key direction)) requester)))

(define (enter-dig-target! requester target-room)
  (if (member? requester (occupants))
      (ma-send! target-room (list :enter-avatar requester (self)))
      #f))

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
      (ma-send! avatar (list :print (exits-text))))))

(set-method! :exits?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (ma-send! avatar (list :print (exits-text))))))

(set-method! :who?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (ma-send! avatar (list :print (who-text))))))

(set-method! :help
  (lambda (args msg)
    (let ((text (room-help-text)))
      (if (avatar-caller? msg)
          (ma-send! (msg-from msg) (list :print text))
          #f)
      (reply-ok msg text))))

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

(set-method! :behaviour
  (lambda (args msg)
    (handle-room-behaviour! msg args)))

(set-method! :ping
  (lambda (args msg)
    (ma-reply! msg (cons :pong args))))

(set-method! :pong
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((user (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (target-room (msg-from msg)))
          (if (pending-link-matches? direction user target-room requester)
              (request-link-authorization! requester user direction target-room)
              #f)))))

(set-method! :authorize-link
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((user (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (source-room (msg-from msg)))
          (if (owner? user)
              (ma-send! source-room (list :link-authorized user direction requester))
              (ma-send! source-room (list :link-denied user direction requester "You must own both rooms to link them.")))))))

(set-method! :link-denied
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))) (null? (cdr (cdr (cdr args)))))
        #f
        (let ((user (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (reason (car (cdr (cdr (cdr args)))))
              (target-room (msg-from msg)))
          (if (pending-link-matches? direction user target-room requester)
              (begin
                (clear-pending-link! direction)
                (ma-save-state!)
                (ma-send! requester (list :print reason)))
              #f)))))

(set-method! :link-authorized
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((user (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (target-room (msg-from msg)))
          (if (pending-link-matches? direction user target-room requester)
              (cond ((not (owner? user))
                     (begin
                       (clear-pending-link! direction)
                       (ma-save-state!)
                       (ma-send! requester (list :print "You no longer own this room."))))
                    (else
                     (begin
                       (create-exit! direction target-room)
                       (clear-pending-link! direction)
                       (ma-save-state!)
                       (broadcast (string-append user " digs " direction "."))
                       (ma-send! requester (list :print (string-append "You dig " direction " and link to an existing room.")))
                       (enter-dig-target! requester target-room))))
              #f)))))

(set-method! :dig
  (lambda (args msg)
    (let* ((user (caller-user args msg))
           (dig-args (command-args args msg))
           (direction (if (null? dig-args) "out" (car dig-args))))
      (require-valid-owner user msg
        (lambda ()
          (require-owner user msg
            (lambda ()
              (let* ((target (dig-target-text dig-args))
                     (custom-init (dig-custom-init-text dig-args))
                     (custom-behaviour (dig-custom-behaviour-ref dig-args))
                     (existing-room (existing-room-target target)))
                (cond ((and existing-room (or custom-init custom-behaviour))
                       (reply-to-sender msg "Custom room code only applies when digging a new room."))
                      (existing-room
                       (request-existing-link! msg user direction existing-room))
                      (else
                       (let ((target-room (entity-url (ma-create-actor ROOM_KIND custom-behaviour (room-init target user custom-init)))))
                         (create-exit! direction target-room)
                         (ma-save-state!)
                         (broadcast (string-append user " digs " direction "."))
                         (reply-to-sender msg (string-append "You dig " direction " and open a new exit."))
                         (enter-dig-target! (msg-from msg) target-room))))))))))))

(set-method! :go
  (lambda (args msg)
    (let ((avatar (msg-from msg))
          (direction (if (null? args) "out" (car args))))
      (let ((exit (exit-target direction)))
        (if exit
            (ma-send! exit (list :traverse avatar (self)))
            (ma-send! avatar (list :print (string-append "No exit " direction "."))))))))

(set-method! :enter-avatar
  (lambda (args msg)
    (let ((avatar (car args)))
      (ma-send! (root) (list :arrived avatar (self))))))
