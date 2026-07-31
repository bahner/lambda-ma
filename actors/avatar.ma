; Locked avatar actor.
; Root owns protected state. The controlling DID may call exposed command methods only.

; Avatar identity and address helpers.
(define (did) (get-prop "did"))
(define (local-self) (canonical-actor (self)))
(define (local-fragment? actor)
  (and actor (string-prefix? "#" actor)))
(define (qualified-actor actor)
  (if actor (canonical-actor actor) ""))
(define (qualified-ctx-actor? actor)
  (and (non-empty-string? actor)
       (not (local-fragment? actor))))
(define (entity-id) (ma-get-config-key "id"))
(define (valid-did? value)
  (and (string? value)
       (string-prefix? "did:ma:" value)
       (not (string-contains? "#" value))))

; A deterministic avatar may rehydrate its controlling DID from the matching
; DID, but an existing mismatched DID remains denied.
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

; Context terms sent to the controlling did. These must contain fully
; qualified actor references, never runtime-local #fragment shorthand.
(define (ctx-term text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (qualified-actor (root)))
          (list :avatar (qualified-actor (local-self)))
          (list :nick (nick))
          (list :room (qualified-actor (room)))
          (list :text text))))

(define (ctx-term-room r text)
  (list :ctx
    (list (list :protocol LAMBDA_CTX_PROTOCOL)
          (list :kind "avatar")
          (list :root (qualified-actor (root)))
          (list :avatar (qualified-actor (local-self)))
          (list :nick (nick))
          (list :room (qualified-actor r))
          (list :text text))))

(define (start-room) (ma-get-config-key "start"))
(define THING_KIND "/ma/thing/0.0.1")

(define (send-ctx text)
  (ma-send! (did) (ctx-term text)))

(define (did? msg) (ensure-msg-did! msg))
(define (root? msg) (same-actor? (msg-from msg) (root)))
(define (room? msg)
  (let ((current (room)))
    (and current (same-actor? (msg-from msg) current))))

(define (ctx-caller? msg)
  (or (did? msg)
      (root? msg)
      (room? msg)))

; Validate committed context from a room before persisting local avatar state.
(define (ctx-value pairs key)
  (cond ((null? pairs) #f)
        ((and (pair? (car pairs))
              (equal? (car (car pairs)) key)
              (not (null? (cdr (car pairs)))))
         (car (cdr (car pairs))))
        (else (ctx-value (cdr pairs) key))))

(define (avatar-ctx-valid? payload msg)
  (if (pair? payload)
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
             (same-actor? (msg-from msg) target-room)))
      #f))

(define (move-ctx-valid? ctx)
  (let ((actor (ctx-text ctx "actor"))
        (kind (ctx-text ctx "kind"))
        (target-room (ctx-text ctx "room")))
    (and (map? ctx)
         (equal? actor (did))
         (or (equal? kind "avatar")
             (equal? kind "did"))
         (qualified-ctx-actor? target-room))))

(define (set-pending-move! target-room old-room)
  (begin
    (set-prop! "pending-room" (canonical-actor target-room))
    (if old-room
        (set-prop! "pending-old-room" (canonical-actor old-room))
        (del-prop! "pending-old-room"))
    (ma-save-state!)))

(define (clear-pending-move!)
  (begin
    (del-prop! "pending-room")
    (del-prop! "pending-old-room")))

(define (enter-room-authorised? args msg)
  (and (not (null? args))
       (not (null? (cdr args)))
  (ensure-did! (car (cdr args)) (avatar-fragment (car (cdr args))))
       (or (root? msg)
           (same-actor? (msg-from msg) (car args)))))

; DID-command forwarding helpers. Plain avatar commands are translated into
; room RPCs. Only commands whose payload needs the DID prepend it; room
; owner checks recognise direct owner messages and deterministic owner avatars.
(define (require-did msg thunk)
  (if (did? msg)
      (thunk)
      (ma-reply! msg (list :error "avatar command denied"))))

(define (send-room verb args)
  (let ((target (room)))
    (if target
        (ma-send! (canonical-actor target) (cons verb args))
        (let ((start (start-room)))
          (if start
              (ma-send! (did) (ctx-term-room start #f))
              (ma-send! (did) (list :print "You are nowhere.")))))))

(define (send-room-as-did verb args)
  (send-room verb (cons (did) args)))

(define (avatar-look args msg)
  (require-did msg
    (lambda ()
      (send-room :look args)
      (reply-ok-silent msg))))

(define (avatar-say args msg)
  (require-did msg
    (lambda ()
      (send-room :say (list (join-words args)))
      (reply-ok-silent msg))))

(define (send-did-text text)
  (ma-send! (did) (list :print text)))

(define (inventory-map)
  (prop-map "inventory"))

(define (set-inventory-map! m)
  (set-prop-map! "inventory" m)
  (ma-save-state!))

(define (remember-inventory! token actor)
  (if (and (non-empty-string? token) (non-empty-string? actor))
      (set-inventory-map! (map-set (inventory-map) token (canonical-actor actor)))
      #f))

(define (forget-inventory! token)
  (if (non-empty-string? token)
      (set-inventory-map! (map-delete (inventory-map) token))
      #f))

(define (entry-lines xs)
  (cond ((null? xs) "")
        ((null? (cdr xs)) (car xs))
        (else (string-append (car xs) "\n" (entry-lines (cdr xs))))))

(define (inventory-line entry)
  (let ((token (car entry))
        (actor (cdr entry)))
    (if (equal? token actor)
        token
        (string-append token " = " (canonical-actor actor)))))

(define (inventory-lines entries)
  (cond ((null? entries) '())
        (else (cons (inventory-line (car entries)) (inventory-lines (cdr entries))))))

(define (inventory-text)
  (let ((lines (inventory-lines (map->alist (inventory-map)))))
    (if (null? lines)
        "Inventory: empty."
        (string-append "Inventory:\n" (entry-lines lines)))))

(define (make-kind kind)
  (if (equal? kind "thing") THING_KIND kind))

(define (valid-make-kind? kind)
  (and (non-empty-string? kind)
       (string-prefix? "/" kind)))

(define (avatar-make args msg)
  (require-did msg
    (lambda ()
      (cond ((or (null? args) (null? (cdr args)))
             (reply-error msg "Usage: make <kind> <init...>"))
            (else
             (let ((kind (make-kind (car args)))
                   (init (join-words (cdr args))))
               (cond ((not (valid-make-kind? kind))
                      (reply-error msg "make kind must be a protocol id"))
                     ((not (string? init))
                      (reply-error msg "make init must be a string"))
                     (else
                      (let* ((fragment (ma-create-actor kind #f init))
                             (actor (entity-url fragment)))
                       (remember-inventory! actor actor)
                        (send-did-text (string-append "Create requested: " actor "."))
                        (reply-ok-with msg actor))))))))))

(define (reply-ok-silent msg)
  (reply-ok msg))

(define (avatar-help-text)
  (string-append
    "Help\n"
    "  help              show this help\n"
    "  help here         ask this place what is possible here\n"
    "  look              look around\n"
    "  l                 alias for look\n"
    "  look <exit>       inspect an exit\n"
    "  exits?            list exits\n"
    "  who?              show who is here\n"
    "  things?           list local non-avatar occupants\n"
    "  did? [kind] <name> show the DID for a visible occupant, thing, or exit\n"
    "  owner? <name>     show who owns a visible occupant, thing, or exit\n"
    "  make <kind> <init...> create an actor from init text\n"
    "  inventory         show what you are carrying\n"
    "  i                 alias for inventory\n"
    "  take <thing>      ask a local occupant to bind to you\n"
    "  drop <thing>      ask a carried occupant to enter this room\n"
    "  where? <thing>    ask where a local occupant says it is\n"
    "  say <text>        speak here\n"
    "  emote <text>      act here\n"
    "  leave             stop being shown here until you return\n"
    "  go <direction>    move through an exit\n"
    "  claim             claim an unowned room\n"
    "  owner [did]       show or transfer room ownership\n"
    "  dig <dir> [to name] [with code] create an exit\n"
    "  fill <dir>        remove an exit\n"
    "  lock <dir>        lock an exit\n"
    "  unlock <dir>      unlock an exit\n"
    "  exit-message <dir> <traveller|source|target|blocked> <text>\n"
    "  prop <key> [value] set or reset room text\n"
    "  nick [name]       show or set your display name\n"
    "Use :help for the focused actor directly."))

(define (unknown-help-text topic)
  (string-append "No help topic: " topic "\nTry help or help here."))

; Root and room callbacks.
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
                            (null? (cdr (cdr args))))
                                  #f
                          (car (cdr (cdr args))))))
          (if (non-empty-string? requested-nick)
              (begin
                (set-prop! "nick" requested-nick)
                (ma-save-state!))
              #f)
          (ma-send! (canonical-actor target-room) (list :enter (local-self) (canonical-actor old-room) (nick))))
        #f)))

(set-method! :ctx
  (lambda (args msg)
    (if (null? args)
        (if (ctx-caller? msg)
            (ma-reply! msg (list :ok (ctx-term #f)))
            #f)
        (let ((payload (car args)))
          (cond
            ((avatar-ctx-valid? payload msg)
             (begin
               (clear-pending-move!)
               (set-prop! "room" (canonical-actor (ctx-value payload :room)))
               (set-prop! "nick" (ctx-value payload :nick))
               (ma-save-state!)
               (ma-send! (did) (cons :ctx args))))
            ((and (move-ctx-valid? payload) (ctx-caller? msg))
             (let* ((target-room (ctx-text payload "room"))
                    (old-room (room))
                    (text (ctx-text payload "text")))
               (begin
                 (if (non-empty-string? text)
                     (ma-send! (did) (list :print text))
                     #f)
                 (if (same-actor? target-room old-room)
                     #f
                     (begin
                       (set-pending-move! target-room old-room)
                       (ma-send! (did) (list :ctx payload)))))))
            (else #f))))))

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

(set-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args)))))
          (current-room (room)))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (local-self) (if current-room (canonical-actor current-room) "") tick nonce)))))

; DID-facing commands.
(set-method! :help
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (cond ((null? args)
               (begin
                 (send-did-text (avatar-help-text))
                 (reply-ok-silent msg)))
              ((equal? (car args) "here")
               (begin
                 (send-room :help '())
                 (reply-ok-silent msg)))
              (else
               (begin
                 (send-did-text (unknown-help-text (car args)))
                 (reply-ok-silent msg))))))))

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
  avatar-look)

(set-method! :l
  avatar-look)

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

(set-method! :occupants?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :occupants? '())
        (reply-ok-silent msg)))))

(set-method! :things?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :things? '())
        (reply-ok-silent msg)))))

(set-method! :did?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :did? args)
        (reply-ok-silent msg)))))

(set-method! :dids?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :dids? args)
        (reply-ok-silent msg)))))

(set-method! :say
  avatar-say)

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
        (send-room :claim args)
        (reply-ok-silent msg)))))

(set-method! :owner
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :owner args)
        (reply-ok-silent msg)))))

(set-method! :owner?
  (lambda (args msg)
    (if (and (room? msg) (not (null? args)))
        (begin
          (ma-send! (canonical-actor (car args)) (list :print (string-append "Owner: " (did))))
          (reply-ok-silent msg))
        (require-did msg
          (lambda ()
            (send-room :owner? args)
            (reply-ok-silent msg))))))

(set-method! :make
  avatar-make)

(set-method! :inventory
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-did-text (inventory-text))
        (reply-ok-silent msg)))))

(set-method! :i
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-did-text (inventory-text))
        (reply-ok-silent msg)))))

(set-method! :dig
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :dig args)
        (reply-ok-silent msg)))))

(set-method! :fill
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :fill args)
        (reply-ok-silent msg)))))

(set-method! :lock
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            (send-did-text "Usage: lock <direction>")
            (send-room :exit (list (car args) :lock)))
        (reply-ok-silent msg)))))

(set-method! :unlock
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            (send-did-text "Usage: unlock <direction>")
            (send-room :exit (list (car args) :unlock)))
        (reply-ok-silent msg)))))

(set-method! :exit-message
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
            (send-did-text "Usage: exit-message <direction> <traveller|source|target|blocked> <text>")
            (send-room :exit (list (car args) :message (car (cdr args)) (join-words (cdr (cdr args))))))
        (reply-ok-silent msg)))))

(set-method! :prop
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :prop args)
        (reply-ok-silent msg)))))

(set-method! :take
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            #f
            (remember-inventory! (car args) (car args)))
        (send-room-as-did :take args)
        (reply-ok-silent msg)))))

(set-method! :drop
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            #f
            (forget-inventory! (car args)))
        (send-room-as-did :drop args)
        (reply-ok-silent msg)))))

(set-method! :where?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :where? args)
        (reply-ok-silent msg)))))

(set-method! :go
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :go args)
        (reply-ok-silent msg)))))

; Room-mediated drop helper for carried things/agents.
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
          (ma-send! (canonical-actor thing) (list :drop did (canonical-actor target-parent) ctx))
          (ma-send! (canonical-actor thing) (list :drop did (canonical-actor target-parent))))))
        #f)))

; Unknown DID commands are treated as room verbs so room-specific behaviours
; can add commands without changing the avatar proxy.
(set-default-method!
  (lambda (verb args msg)
    (require-did msg
      (lambda ()
        (send-room verb args)
        (reply-ok-silent msg)))))

(define (normalise-command-verb verb)
  (if (symbol? verb)
      (string->symbol (string-downcase (symbol->string verb)))
      verb))

(define (on-message msg)
  (let* ((term (msg-content msg))
         (verb (normalise-command-verb (if (pair? term) (car term) term)))
         (args (if (pair? term) (cdr term) '()))
         (fn (find-method verb)))
    (if fn
        (fn args msg)
        (if *default-method*
            (*default-method* verb args msg)
            (ma-reply! msg (list :error "unknown verb"))))))
