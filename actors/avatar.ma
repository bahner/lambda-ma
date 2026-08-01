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

(define (take-source args)
  (if (and (not (null? args))
           (not (null? (cdr args)))
           (equal? (car (cdr args)) "from")
           (not (null? (cdr (cdr args))))
           (null? (cdr (cdr (cdr args)))))
   (let* ((token (car (cdr (cdr args))))
       (entry (inventory-ref token)))
     (if entry (inventory-entry-actor entry) token))
      (room)))

(define (take-token args)
  (if (null? args) #f (car args)))

(define (take-has-source? args)
  (and (not (null? args))
       (not (null? (cdr args)))
       (equal? (car (cdr args)) "from")
       (not (null? (cdr (cdr args))))
       (null? (cdr (cdr (cdr args))))))

(define (take-args-valid? args)
  (or (and (not (null? args)) (null? (cdr args)))
      (and (not (null? args))
           (not (null? (cdr args)))
           (equal? (car (cdr args)) "from")
           (not (null? (cdr (cdr args))))
           (null? (cdr (cdr (cdr args)))))))

(define (send-take args)
  (let ((source (take-source args))
        (token (take-token args)))
  (cond ((and (valid-did-url? token) (not (take-has-source? args)))
     (ma-send! (canonical-actor token) (list :take (did) (avatar-for-did (did)))))
      (source
         (ma-send! (canonical-actor source) (list :take (did) token)))
      (else
       (send-did-text "You are nowhere.")))))

(define (put-args-valid? args)
  (and (not (null? args))
       (not (null? (cdr args)))
       (equal? (car (cdr args)) "in")
       (not (null? (cdr (cdr args))))
       (null? (cdr (cdr (cdr args))))))

(define (put-item-token args)
  (if (null? args) #f (car args)))

(define (put-container-token args)
  (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
      #f
      (car (cdr (cdr args)))))

(define (send-put-via-room args item-ctx)
  (if item-ctx
      (send-room-as-did :put (list-append args (list item-ctx)))
      (send-room-as-did :put args)))

(define (put-carried! did item-actor container-actor item-ctx)
  (ma-send! (canonical-actor item-actor) (list :drop did (canonical-actor container-actor) item-ctx)))

(define (put-visible! did item-actor container-actor item-ctx)
  (begin
    (ma-send! (canonical-actor item-actor) (list :drop did (canonical-actor container-actor) item-ctx))))

(define (send-put args)
  (let* ((item-token (put-item-token args))
         (container-token (put-container-token args))
         (item-entry (inventory-ref item-token))
         (container-entry (inventory-ref container-token))
         (item-actor (if item-entry (inventory-entry-actor item-entry) #f))
         (container-actor (if container-entry (inventory-entry-actor container-entry) #f))
         (item-ctx (if item-entry (cdr item-entry) #f)))
    (cond ((and item-actor container-actor)
           (if (same-actor? item-actor container-actor)
               (send-did-text "You cannot put something inside itself.")
               (if (child-ctx-valid? item-ctx)
                   (put-carried! (did) item-actor container-actor item-ctx)
                   (send-did-text (string-append "Still waiting for carried item details: " item-token)))))
          ((and item-entry (not (child-ctx-valid? item-ctx)))
           (send-did-text (string-append "Still waiting for carried item details: " item-token)))
          (else
           (send-put-via-room args item-ctx)))))

(define (handle-put-thing args msg)
  (if (room? msg)
      (if (or (null? args)
              (null? (cdr args))
              (null? (cdr (cdr args)))
              (null? (cdr (cdr (cdr args)))))
          #f
          (let* ((did (car args))
                 (item-actor (car (cdr args)))
                 (container-actor (car (cdr (cdr args))))
                 (item-ctx (car (cdr (cdr (cdr args)))))
                 (carried-entry (inventory-ref item-actor)))
            (cond ((not (child-ctx-valid? item-ctx)) #f)
                  ((not (valid-did-url? item-actor)) #f)
                  ((not (valid-did-url? container-actor)) #f)
                  ((same-actor? item-actor container-actor)
                   (send-did-text "You cannot put something inside itself."))
                  (carried-entry
                   (put-carried! did item-actor container-actor item-ctx))
                  (else
                   (put-visible! did item-actor container-actor item-ctx)))))
      #f))

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
  (children-map))

(define (set-inventory-map! m)
  (set-children-map! m))

(define (remember-inventory! token actor)
  (if (and (non-empty-string? token) (non-empty-string? actor))
      (let ((ctx (map-set
                   (map-set
                     (map-set
                       (map-set (make-map) "actor" (canonical-actor actor))
                       "kind" "thing")
                     "name" token)
                   "nick" token)))
        (set-inventory-map! (map-set (inventory-map) (canonical-actor actor) ctx)))
      #f))

(define (inventory-label-matches? token label)
  (and (non-empty-string? token)
       (non-empty-string? label)
       (equal? (string-downcase token) (string-downcase label))))

(define (inventory-entry-actor entry)
  (let* ((actor (canonical-actor (car entry)))
         (ctx (cdr entry))
         (ctx-actor (canonical-actor (ctx-text ctx "actor"))))
    (cond ((valid-did-url? ctx-actor) ctx-actor)
          ((valid-did-url? actor) actor)
          (else #f))))

(define (inventory-entry-matches? token entry)
  (let* ((actor (canonical-actor (car entry)))
         (ctx (cdr entry))
         (ctx-actor (canonical-actor (ctx-text ctx "actor"))))
    (or (same-actor? token actor)
        (same-actor? token ctx-actor)
        (inventory-label-matches? token (child-label ctx))
        (inventory-label-matches? token (ctx-text ctx "name"))
        (inventory-label-matches? token (ctx-text ctx "nick")))))

(define (inventory-ref-resolved token)
  (let loop ((entries (map->alist (inventory-map))))
    (cond ((null? entries) #f)
          ((and (inventory-entry-actor (car entries))
                (inventory-entry-matches? token (car entries)))
           (car entries))
          (else (loop (cdr entries))))))

(define (inventory-ref-any token)
  (let loop ((entries (map->alist (inventory-map))))
    (cond ((null? entries) #f)
          ((inventory-entry-matches? token (car entries)) (car entries))
          (else (loop (cdr entries))))))

(define (inventory-ref token)
  (let ((resolved (inventory-ref-resolved token)))
    (if resolved resolved (inventory-ref-any token))))

(define (forget-inventory! actor)
  (if (non-empty-string? actor)
      (set-inventory-map! (map-delete (inventory-map) (canonical-actor actor)))
      #f))

(define (forget-inventory-token! token)
  (let ((entry (inventory-ref-any token)))
    (if entry
        (forget-inventory! (car entry))
        #f)))

(define (forget-inventory-ctx-labels! ctx)
  (begin
    (forget-inventory-token! (ctx-text ctx "actor"))
    (forget-inventory-token! (ctx-text ctx "name"))
    (forget-inventory-token! (ctx-text ctx "nick"))
    (forget-inventory-token! (child-label ctx))))

(define (entry-lines xs)
  (cond ((null? xs) "")
        ((null? (cdr xs)) (car xs))
        (else (string-append (car xs) "\n" (entry-lines (cdr xs))))))

(define (inventory-line entry)
  (let ((actor (car entry))
        (label (child-label (cdr entry))))
    (if (equal? actor label)
        label
        (string-append label " = " (canonical-actor actor)))))

(define (inventory-lines entries)
  (cond ((null? entries) '())
        (else (cons (inventory-line (car entries)) (inventory-lines (cdr entries))))))

(define (inventory-text)
  (let ((lines (inventory-lines (map->alist (inventory-map)))))
    (if (null? lines)
        "Inventory: empty."
        (string-append "Inventory:\n" (entry-lines lines)))))

(define (handle-avatar-children args msg)
  (cond ((null? args)
         (require-did msg
           (lambda ()
             (ma-reply! msg (list :ok (children-text))))))
        ((not (null? (cdr args)))
          (reply-error msg "usage: :child [ctx]"))
        ((not (child-ctx-valid? (car args)))
          (reply-error msg "child ctx must include actor, parent, kind, protocol, name, nick, description"))
        ((not (same-actor? (msg-from msg) (ctx-text (car args) "actor")))
          (reply-error msg "child ctx actor must match sender"))
        ((not (same-actor? (ctx-text (car args) "parent") (local-self)))
         (begin
           (forget-inventory-ctx-labels! (car args))
           (reply-ok msg)))
        (else
         (begin
           (forget-inventory-ctx-labels! (car args))
           (remember-child! (car args))
           (ma-send! (canonical-actor (ctx-text (car args) "actor")) (list :parent (car args)))
           (reply-ok msg)))))

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
    "  here?             show where you are\n"
    "  who?              show who is here\n"
    "  things?           list local non-avatar occupants\n"
    "  did? [kind] <name> show the DID for a visible occupant, thing, or exit\n"
    "  owner? <name>     show who owns a visible occupant, thing, or exit\n"
    "  make <kind> <init...> create an actor from init text\n"
    "  inventory         show what you are carrying\n"
    "  i                 alias for inventory\n"
    "  take <thing> [from <parent>] pick something up\n"
    "  put <thing> in <container> put a thing into a visible container\n"
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
    "  prop <key> [value] set or reset room text\n"
    "  nick [name]       show or set your display name\n"
    "Use :help for the focused actor directly."))

(define (unknown-help-text topic)
  (string-append "No help topic: " topic "\nTry help or help here."))

; Root and room callbacks.
(set-internal-rpc-method! :sync-ctx
  (lambda (args msg)
    (if (root? msg)
        (send-ctx #f)
        #f)))

(set-internal-rpc-method! :enter-room
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

(set-internal-rpc-method! :ctx
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

(set-rpc-method! :ctx?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (ma-reply! msg (list :ok (ctx-term #f)))))))

(set-internal-rpc-method! :print
  (lambda (args msg)
    (ma-send! (did) (list :print (join-words args)))))

(set-internal-rpc-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args)))))
          (current-room (room)))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (local-self) (if current-room (canonical-actor current-room) "") tick nonce)))))

; DID-facing commands.
(unset-method! :owner)

(set-cmd-method! :help
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

(set-cmd-method! :nick
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

(set-cmd-method! :look
  avatar-look)

(set-cmd-method! :l
  avatar-look)

(set-cmd-method! :exits?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :exits? '())
        (reply-ok-silent msg)))))

(set-cmd-method! :who?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :who? '())
        (reply-ok-silent msg)))))

(set-cmd-method! :occupants?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :occupants? '())
        (reply-ok-silent msg)))))

(set-cmd-method! :things?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :things? '())
        (reply-ok-silent msg)))))

(set-cmd-method! :here?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (let ((current (room)))
          (if current
              (send-did-text (string-append "You are in " (qualified-actor current) "."))
              (send-did-text "You are nowhere.")))
        (reply-ok-silent msg)))))

(set-rpc-method! :did?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :did? args)
        (reply-ok-silent msg)))))

(set-rpc-method! :dids?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :dids? args)
        (reply-ok-silent msg)))))

(set-cmd-method! :say
  avatar-say)

(set-cmd-method! :emote
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :emote (list (join-words args)))
        (reply-ok-silent msg)))))

(set-cmd-method! :claim
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :claim args)
        (reply-ok-silent msg)))))

(set-rpc-method! :owner?
  (lambda (args msg)
    (cond ((not (null? args))
           (reply-error msg "usage: :owner?"))
          ((equal? (msg-from msg) (did))
           (reply-ok-with msg (actor-owner)))
          (else
           (reply-error msg "avatar command denied")))))

(set-cmd-method! :make
  avatar-make)

(set-meta-method! :child
  handle-avatar-children)

(set-cmd-method! :inventory
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-did-text (inventory-text))
        (reply-ok-silent msg)))))

(set-cmd-method! :i
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-did-text (inventory-text))
        (reply-ok-silent msg)))))

(set-cmd-method! :dig
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :dig args)
        (reply-ok-silent msg)))))

(set-cmd-method! :fill
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :fill args)
        (reply-ok-silent msg)))))

(set-cmd-method! :lock
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            (send-did-text "Usage: lock <direction>")
            (send-room :exit (list (car args) :lock)))
        (reply-ok-silent msg)))))

(set-cmd-method! :unlock
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            (send-did-text "Usage: unlock <direction>")
            (send-room :exit (list (car args) :unlock)))
        (reply-ok-silent msg)))))

(set-rpc-method! :prop
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :prop args)
        (reply-ok-silent msg)))))

(set-cmd-method! :take
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (not (take-args-valid? args))
            (send-did-text "Usage: take <thing> [from <parent>]")
            (begin
              (if (valid-did-url? (take-token args))
                  #f
                  (remember-inventory! (take-token args) (take-token args)))
              (send-take args)))
        (reply-ok-silent msg)))))

(set-cmd-method! :put
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (not (put-args-valid? args))
            (send-did-text "Usage: put <thing> in <container>")
            (send-put args))
        (reply-ok-silent msg)))))

(set-cmd-method! :drop
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (null? args)
            (send-room-as-did :drop args)
            (let* ((entry (inventory-ref (car args)))
                   (actor (if entry (canonical-actor (car entry)) #f))
                   (ctx (if entry (cdr entry) #f)))
              (if actor
                  (begin
                    (send-room-as-did :drop (list actor ctx)))
                  (send-did-text (string-append "Unknown carried agent or thing: " (car args))))))
        (reply-ok-silent msg)))))

(set-cmd-method! :where?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room :where? args)
        (reply-ok-silent msg)))))

(set-cmd-method! :go
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (send-room-as-did :go args)
        (reply-ok-silent msg)))))

; Room-mediated drop helper for carried things/agents.
(set-internal-rpc-method! :drop-thing
  (lambda (args msg)
    (if (room? msg)
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

(set-internal-rpc-method! :put-thing
  (lambda (args msg)
    (handle-put-thing args msg)))

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
