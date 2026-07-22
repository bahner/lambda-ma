; Locked room actor.
; Rooms own exits and local room policy. Avatars act through their current room.

(define AVATAR_KIND "/ma/avatar/0.0.1")
(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")
(define LAMBDA_CTX_PROTOCOL "/ma/lambda/ctx/0.0.1")
(define ENTITY_FRAGMENT_CONTEXT "ma entity-fragment v1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (root)
  (let ((configured (ma-get-config-key "root")))
    (if configured configured (entity-url "root"))))
(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor)) (string-append (runtime) actor) actor))
(define (canonical-entry entry)
  (canonical-actor entry))
(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))
(define (local-actor-ref? actor)
  (and (string? actor)
       (or (string-prefix? "#" actor)
           (string-prefix? (string-append (runtime) "#") actor))))
(define (dead-local-actor? actor)
  (and (local-actor-ref? actor) (not (ma-entity-exists? actor))))
(define (entity-live? actor)
  (and actor (ma-entity-exists? actor)))

(define (member-actor? actor xs)
  (member-entry? actor xs))

(define (unique-actor-entries xs)
  (let loop ((rest xs) (strings '()))
    (cond ((null? rest) (unique-entries strings))
          ((string? (car rest)) (loop (cdr rest) (cons (car rest) strings)))
          (else (loop (cdr rest) strings)))))

(define (occupants)
  (let ((xs (get-prop "occupants")))
    (if xs (unique-actor-entries xs) '())))

(define (add-occupant! actor)
  (if (member-actor? actor (occupants))
      #f
      (begin
        (set-prop! "occupants" (cons (canonical-actor actor) (occupants)))
        #t)))

(define (remove-occupant! actor)
  (set-prop! "occupants" (without-actors (occupants) (list actor))))

(define (add-avatar-presence! avatar)
  (let ((occupant-added (add-occupant! avatar))
        (avatar-added (add-avatar-occupant! avatar)))
    (or occupant-added avatar-added)))

(define (notify-old-room! old-room avatar)
  (if (and old-room (not (same-actor? old-room (self))))
      (ma-send! old-room (list :leave-avatar avatar (self)))
      #f))

(define (label-key actor) (string-append "label:" (canonical-actor actor)))

(define (set-label! actor label)
  (if (non-empty-string? label)
      (set-prop! (label-key actor) label)
      #f))

(define (has-label? actor)
  (let ((label (get-prop (label-key actor))))
    (non-empty-string? label)))

(define (avatar-occupants)
  (let ((xs (get-prop "avatar-occupants")))
    (if xs (unique-actor-entries xs) '())))

(define (add-avatar-occupant! avatar)
  (if (member-actor? avatar (avatar-occupants))
      #f
      (begin
        (set-prop! "avatar-occupants" (cons (canonical-actor avatar) (avatar-occupants)))
        #t)))

(define (remove-avatar-occupant! avatar)
  (set-prop! "avatar-occupants" (without-actors (avatar-occupants) (list avatar))))

(define (without-actors xs drop)
  (cond ((null? xs) '())
        ((member-actor? (car xs) drop)
         (without-actors (cdr xs) drop))
        (else
         (cons (car xs) (without-actors (cdr xs) drop)))))

(define (speaker-name actor)
  (let ((label (get-prop (label-key actor))))
    (if (non-empty-string? label) label actor)))

(define (room-name)
  (let ((name (get-prop "name")))
    (if name name "Construct")))

(define (room-description)
  (let ((description (get-prop "description")))
    (if description description "“This is the Construct. It's our loading program. We can load anything... From clothing to equipment, weapons, training simulations; anything we need.”")))

(define (room-text)
  (string-append
    (room-name) "\n"
    (room-description) "\n"
    (occupants-text) "\n"
    (things-text)))

(define (names-of actors)
  (cond ((null? actors) "")
        ((null? (cdr actors)) (speaker-name (car actors)))
        (else (string-append (speaker-name (car actors)) ", " (names-of (cdr actors))))))

(define (token-list-text label names)
  (if (null? names)
      (string-append label ": none.")
      (string-append label ": " (names-of names))))

(define (things-map) (prop-map "things"))

(define (set-things-map! m) (set-prop-map! "things" m))

(define (claim-key actor)
  (string-append "claim:" (canonical-actor actor)))

(define (set-claim! actor ctx)
  (set-prop! (claim-key actor) ctx)
  (ma-save-state!))

(define (claim-ctx actor)
  (let ((ctx (get-prop (claim-key actor))))
    (if (map? ctx) ctx #f)))

(define (default-nick) "avatar")

(define (nick-or-default nick)
  (if (non-empty-string? nick) nick (default-nick)))

(define (avatar-fragment user)
  (ma-derived-id ENTITY_FRAGMENT_CONTEXT user 8))

(define (avatar-for-user user)
  (entity-url (avatar-fragment user)))

(define (avatar-init user nick room)
  (let ((n (nick-or-default nick))
        (r (root)))
    (string-append
      "(set-prop! \"user\" \"" user "\")\n"
      "(set-prop! \"root\" \"" r "\")\n"
      "(set-prop! \"nick\" \"" n "\")\n"
      "(ma-send! \"" room "\" (list :enter (ma-get-config-key \"self\") #f \"" n "\"))\n")))

(define (ensure-avatar! user nick)
  (let* ((avatar (avatar-for-user user))
         (n (nick-or-default nick)))
    (set-label! avatar n)
    (if (add-avatar-presence! avatar)
      (broadcast (string-append (speaker-name avatar) " arrives."))
      #f)
    (ma-save-state!)
    (if (entity-live? avatar)
        avatar
        (entity-url (ma-create-actor AVATAR_KIND #f (avatar-init user n (self)) user)))))

(define (enter-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (ctx-text ctx "kind"))
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))))

(define (enter-direct-ctx-valid? ctx kind)
  (and (enter-ctx-valid? ctx)
       (equal? (ctx-text ctx "kind") kind)))

(define (direct-room-ctx kind nick text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind kind)
          (list :root (root))
          (list :avatar "")
          (list :nick (if nick nick ""))
          (list :room (self))
          (list :text text))))

(define (avatar-room-ctx avatar nick text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (root))
          (list :avatar (canonical-actor avatar))
          (list :nick (nick-or-default nick))
          (list :room (self))
          (list :text text))))

(define (request-avatar-entry! user nick)
  (let* ((avatar (avatar-for-user user))
         (n (nick-or-default nick)))
    (if (entity-live? avatar)
        (ma-send! avatar (list :enter-room (self)))
        (ma-create-actor AVATAR_KIND #f (avatar-init user n (self)) user))
    avatar))

(define (handle-agent-enter! msg user ctx)
  (let* ((actor (canonical-actor user))
         (nick (ctx-text ctx "nick")))
    (set-claim! actor ctx)
    (set-label! actor nick)
    (if (add-occupant! actor)
      (broadcast (string-append (speaker-name actor) " arrives."))
      #f)
    (ma-save-state!)
    (ma-send! actor (direct-room-ctx "agent" nick "You arrive."))
    (ma-reply! msg (list :ok "entered"))))

(define (handle-thing-enter! msg user ctx name)
  (let* ((actor (canonical-actor user))
         (label (ctx-text ctx "nick"))
         (token (if (non-empty-string? label) label name))
         (bound (thing-ref token)))
    (cond ((not (actor-token-valid? name))
           (reply-error msg "enter requires non-empty name token"))
          ((not (actor-token-valid? token))
           (reply-error msg "enter requires non-empty nick token"))
          ((and bound (not (same-actor? bound actor)))
           (reply-error msg "nick token is already bound to another actor"))
          (else
           (set-claim! actor ctx)
           (set-label! actor label)
           (set-thing! token actor)
           (ma-reply! msg (list :ok "entered"))))))

(define (agent-kind? kind) (equal? kind "agent"))
(define (thing-kind? kind) (equal? kind "thing"))
(define (movable-kind? kind)
  (or (agent-kind? kind) (thing-kind? kind)))

(define (movable-occupant? actor)
  (let ((ctx (claim-ctx actor)))
    (and ctx (movable-kind? (ctx-text ctx "kind")))))

(define (occupant-ref token)
  (let loop ((xs (occupants)))
    (cond ((null? xs) #f)
          ((and (movable-occupant? (car xs)) (equal? (speaker-name (car xs)) token))
           (car xs))
          (else (loop (cdr xs))))))

(define (actor-token-valid? token)
  (non-empty-string? token))

(define (thing-ref token)
  (if (string-prefix? "did:ma:" token)
      token
      (map-ref (things-map) token #f)))

(define (movable-ref token)
  (let ((thing (thing-ref token)))
    (if thing thing (occupant-ref token))))

(define (set-thing! token did)
  (set-things-map! (map-set (things-map) token did)))

(define (remove-thing! token)
  (set-things-map! (map-delete (things-map) token)))

(define (things-text)
  (token-list-text "Things" (map-keys (things-map))))

(define (reconcile-caller-occupant! actor)
  (cond ((member-actor? actor (occupants)) #f)
        ((not (has-label? actor)) #f)
        (else
         (begin
           (add-occupant! actor)
           (ma-save-state!)))))

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
  (let ((avatars (avatar-occupants)))
    (if (null? avatars)
        "Who: none."
        (string-append "Who: " (names-of avatars)))))

(define (thing-token-names)
  (map-keys (things-map)))

(define (occupants-text)
  (let ((actors (occupants))
        (tokens (thing-token-names)))
    (cond ((and (null? actors) (null? tokens))
           "Occupants: none.")
          ((null? actors)
         (string-append "Occupants: " (names-of tokens)))
        ((null? tokens)
         (string-append "Occupants: " (names-of actors)))
        (else
         (string-append "Occupants: " (names-of actors) ", " (names-of tokens))))))

(define (room-help-text)
  (string-append
    (room-name) " help\n"
    "  look              look around\n"
    "  exits?            list exits\n"
    "  who?              show people here\n"
    "  occupants?        show all occupants (avatars + room locals)\n"
    "  things?           list known non-avatar occupants\n"
    "  take <thing>      ask an occupant to bind to you\n"
    "  drop <thing>      ask an occupant to set this room as parent\n"
    "  where <thing>     ask where an occupant says it is\n"
    "  say <text>        speak here\n"
    "  emote <text>      act here\n"
    "  go <direction>    move through an exit\n"
    "  claim             claim this room if it is unowned\n"
    "  owner [did]       show or transfer ownership\n"
    "  dig <dir> [to name] [with code] create an exit\n"
    "  :thing <name> [did] set/list local occupant alias\n"
    "  :behaviour /ipfs/<cid> add or replace this room's own code\n"
    "  :prop <key> [value] set or reset room text\n"
    "Agents and things enter with :enter ctx; their own parent state is the authority.\n"
    "Commands with : hit this place directly; commands without : go through your avatar."))

(define (avatar-caller? msg)
  (member-actor? (msg-from msg) (occupants)))

(define (owner) (get-prop "owner"))
(define (owned?) (if (owner) #t #f))
(define (owner? user)
  (equal? user (owner)))

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

(define (print-and-reply-ok msg text)
  (begin
    (reply-to-sender msg text)
    (reply-ok msg "")))

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

(define (delegated-user-arg? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (local-actor-caller? msg)
  (string-prefix? "#" (msg-from msg)))

(define (delegated-call? args msg)
  (and (delegated-user-arg? args)
       (or (member-actor? (msg-from msg) (occupants))
           (local-actor-caller? msg))))

(define (caller-user args msg)
  (if (delegated-call? args msg) (car args) (msg-from msg)))

(define (command-args args msg)
  (if (delegated-call? args msg) (cdr args) args))

(define (go-delegated-call? args)
  (delegated-user-arg? args))

(define (go-caller-user args msg)
  (if (go-delegated-call? args) (car args) (msg-from msg)))

(define (go-command-args args)
  (if (go-delegated-call? args) (cdr args) args))

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
  (cond ((equal? event :leave-avatar)
         (let ((avatar (car args)))
           (remove-occupant! avatar)
           (remove-avatar-occupant! avatar)
           (ma-save-state!)
           (broadcast (string-append (speaker-name avatar) " leaves."))))
        ((equal? event :leave-occupant)
         (let ((actor (msg-from msg)))
           (remove-occupant! actor)
           (ma-save-state!)
           (broadcast (string-append (speaker-name actor) " leaves."))))
        (else #f)))

(define (broadcast text)
  (let loop ((xs (occupants))
             (changed #f))
    (cond ((null? xs)
           (if changed (ma-save-state!) #f))
          ((dead-local-actor? (car xs))
           (begin
             (remove-occupant! (car xs))
             (remove-avatar-occupant! (car xs))
             (loop (cdr xs) #t)))
          (else
           (begin
             (ma-send! (car xs) (list :print text))
             (loop (cdr xs) changed))))))

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
         target)
        ((and target (string-prefix? "did:ma:" target)) target)
        ((and target (ma-entity-exists? target)) target)
        (else #f)))

(define (request-link-authorization! requester user direction target-room)
  (begin
    (ma-send! target-room (list :authorize-link user direction requester))
    (ma-send! requester (list :print (string-append "Checking ownership of " target-room ".")))))

(define (request-existing-link! msg user direction target-room)
  (let ((requester (canonical-actor (msg-from msg))))
    (set-prop! (pending-link-key direction) target-room)
    (set-prop! (pending-link-user-key direction) user)
    (set-prop! (pending-link-requester-key direction) requester)
    (ma-save-state!)
    (ma-send! target-room (list :ping user direction requester))
    (reply-to-sender msg (string-append "Checking reachability of " target-room "."))))

(define (pending-link-matches? direction user target-room requester)
  (and (same-actor? (get-prop (pending-link-key direction)) target-room)
       (equal? (get-prop (pending-link-user-key direction)) user)
       (same-actor? (get-prop (pending-link-requester-key direction)) requester)))

(define (enter-dig-target! requester user target-room)
  (if (member-actor? requester (occupants))
  (ma-send! target-room (list :enter user (canonical-actor requester) (self) (speaker-name requester)))
      #f))

(set-method! :leave-avatar
  (lambda (args msg)
    (if (and (not (null? args))
             (not (null? (cdr args)))
             (same-actor? (msg-from msg) (car (cdr args))))
        (on-event :leave-avatar args msg)
        #f)))

(set-method! :leave-occupant
  (lambda (args msg)
    (if (member-actor? (msg-from msg) (occupants))
        (on-event :leave-occupant args msg)
        #f)))

(set-method! :look
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (reconcile-caller-occupant! avatar)
      (print-and-reply-ok msg (room-text)))))

(set-method! :exits?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (print-and-reply-ok msg (exits-text)))))

(set-method! :who?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (reconcile-caller-occupant! avatar)
      (print-and-reply-ok msg (who-text)))))

(set-method! :occupants?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (reconcile-caller-occupant! avatar)
      (print-and-reply-ok msg (occupants-text)))))

(set-method! :things?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (print-and-reply-ok msg (things-text)))))

(set-method! :thing
  (lambda (args msg)
    (let ((user (caller-user args msg))
          (thing-args (command-args args msg)))
      (cond ((null? thing-args)
             (reply-ok msg (things-text)))
            ((null? (cdr thing-args))
             (let ((token (car thing-args))
                   (did (thing-ref (car thing-args))))
               (if did
                   (reply-ok msg did)
                   (reply-error msg (string-append "Unknown thing alias: " token)))))
            ((not (owner? user))
             (reply-error msg "Only this room's owner can change thing aliases."))
            ((equal? (car (cdr thing-args)) "")
             (begin
               (remove-thing! (car thing-args))
               (reply-ok msg "thing alias removed")))
            (else
             (begin
               (set-thing! (car thing-args) (car (cdr thing-args)))
               (reply-ok msg "thing alias set")))))))

(set-method! :take
  (lambda (args msg)
    (let* ((user (caller-user args msg))
           (avatar (msg-from msg))
           (take-args (command-args args msg))
           (token (if (null? take-args) #f (car take-args)))
           (actor (if token (movable-ref token) #f)))
      (cond ((not token)
             (reply-to-sender msg "Usage: take <agent-or-thing>"))
            (actor
             (begin
               (ma-send! actor (list :take user avatar (claim-ctx actor)))
               (reply-to-sender msg (string-append "You take " token "."))))
            (else
             (reply-to-sender msg (string-append "Unknown agent or thing: " token)))))))

(set-method! :drop
  (lambda (args msg)
    (let* ((user (caller-user args msg))
           (avatar (msg-from msg))
           (drop-args (command-args args msg))
           (token (if (null? drop-args) #f (car drop-args)))
           (actor (if token (movable-ref token) #f)))
      (cond ((not token)
             (reply-to-sender msg "Usage: drop <agent-or-thing>"))
            (actor
             (begin
               (ma-send! avatar (list :drop-thing user actor (self) token (claim-ctx actor)))
               (reply-to-sender msg (string-append "You drop " token "."))))
            (else
             (reply-to-sender msg (string-append "Unknown agent or thing: " token)))))))

(set-method! :where
  (lambda (args msg)
    (let* ((where-args (command-args args msg))
           (token (if (null? where-args) #f (car where-args)))
           (actor (if token (movable-ref token) #f)))
      (cond ((not token)
             (reply-to-sender msg "Usage: where <agent-or-thing>"))
            (actor
             (ma-send! actor (list :where)))
            (else
             (reply-to-sender msg (string-append "Unknown agent or thing: " token)))))))

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
    (ma-send! (msg-from msg) (cons :pong args))))

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
                       (enter-dig-target! requester user target-room))))
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
                         (enter-dig-target! (msg-from msg) user target-room))))))))))))

(set-method! :go
  (lambda (args msg)
    (let* ((actor (canonical-actor (msg-from msg)))
           (user (go-caller-user args msg))
           (go-args (go-command-args args))
           (direction (if (null? go-args) "out" (car go-args))))
      (let ((exit (exit-target direction)))
        (if exit
            (if (movable-occupant? actor)
                (ma-send! exit (list :traverse-agent actor (self) (speaker-name actor)))
                (ma-send! exit (list :traverse actor (self) user (speaker-name actor))))
            (ma-send! actor (list :print (string-append "No exit " direction "."))))))))

(set-method! :nick
  (lambda (args msg)
    (if (avatar-caller? msg)
        (if (null? args)
            (reply-ok msg (speaker-name (msg-from msg)))
            (let ((new-nick (join-words args))
                  (avatar (msg-from msg)))
              (set-prop! (label-key avatar) new-nick)
              (ma-save-state!)
              (broadcast (string-append (speaker-name avatar) " is now known as " new-nick "."))
              (reply-ok msg new-nick)))
        (reply-error msg "nick sender must be an avatar"))))

(set-method! :enter
  (lambda (args msg)
    (cond
      ((null? args)
       (reply-error msg "avatar entry must come from an avatar actor"))
      ((map? (car args))
       (let* ((user (msg-from msg))
              (ctx (car args))
              (kind (ctx-text ctx "kind"))
              (name (ctx-text ctx "name")))
         (cond
           ((or (not kind) (equal? kind ""))
            (request-avatar-entry! user (ctx-text ctx "nick")))
           ((equal? kind "avatar")
            (request-avatar-entry! user (ctx-text ctx "nick")))
           ((and (agent-kind? kind) (enter-direct-ctx-valid? ctx "agent"))
            (handle-agent-enter! msg user ctx))
           ((agent-kind? kind)
            (reply-error msg "agent enter requires ctx map with kind, name, nick, description"))
           ((and (thing-kind? kind) (enter-direct-ctx-valid? ctx "thing"))
            (handle-thing-enter! msg user ctx name))
           ((thing-kind? kind)
            (reply-error msg "thing enter requires ctx map with kind, name, nick, description"))
           (else
            (reply-error msg "unsupported ctx kind for enter")))))
      ((and (string? (car args))
            (same-actor? (msg-from msg) (car args)))
       (let ((avatar (car args))
             (old-room (if (or (null? (cdr args)) (equal? (car (cdr args)) "")) #f (car (cdr args))))
             (nick (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args))))))
         (set-label! avatar nick)
         (notify-old-room! old-room avatar)
         (if (add-avatar-presence! avatar)
             (broadcast (string-append (speaker-name avatar) " arrives."))
             #f)
         (ma-save-state!)
         (ma-send! avatar (avatar-room-ctx avatar nick "You arrive."))))
      ((and (string? (car args))
            (string-prefix? "did:ma:" (car args))
            (not (null? (cdr args))))
       (let* ((user (car args))
              (rest (cdr args))
              (avatar (if (null? rest) #f (car rest)))
              (old-room (if (or (null? rest) (null? (cdr rest))) #f (car (cdr rest))))
              (nick (if (or (null? rest) (null? (cdr rest)) (null? (cdr (cdr rest)))) #f (car (cdr (cdr rest))))))
         (if (not avatar)
             #f
             (if (local-actor-ref? avatar)
                 (begin
                   (set-label! avatar nick)
                   (notify-old-room! old-room avatar)
                     (if (add-avatar-presence! avatar)
                       (broadcast (string-append user " arrives."))
                       #f)
                   (ma-save-state!)
                   (ma-send! avatar (avatar-room-ctx avatar nick "You arrive.")))
                 (begin
                   (notify-old-room! old-room avatar)
                   (request-avatar-entry! user nick))))))
      (else
       (let* ((avatar (car args))
              (old-room (if (or (null? (cdr args)) (equal? (car (cdr args)) "")) #f (car (cdr args)))))
         (notify-old-room! old-room avatar)
         (if (add-avatar-presence! avatar)
             (broadcast (string-append (speaker-name avatar) " arrives."))
             #f)
         (ma-save-state!)
         #f)))))
