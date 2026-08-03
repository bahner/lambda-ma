; Locked avatar actor.
; Root owns protected state. The controlling DID may call exposed command methods only.

; Avatar identity and address helpers.
(define ROOM_KIND "/ma/room/0.0.1")
(define CONTAINER_KIND "/ma/container/0.0.1")
(define INVENTORY_LABEL "Inventory")
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
(define (inventory-actor)
  (let ((actor (get-prop "inventory")))
    (if (and actor (not (equal? actor ""))) (canonical-actor actor) #f)))

(define (inventory-fragment did)
  (blake3 (string-append "lambda-ma inventory v1\n" (runtime) "\n" did) 8))

(define (inventory-for-did did)
  (entity-url (inventory-fragment did)))

(define (inventory-init owner-did avatar)
  (string-append
    "(set-init-prop! \"owner\" \"" owner-did "\")\n"
    "(set-init-prop! \"parent\" \"" (canonical-actor avatar) "\")\n"
    "(set-init-prop! \"name\" \"" INVENTORY_LABEL "\")\n"
    "(set-init-prop! \"nick\" \"inventory\")\n"
    "(set-init-prop! \"description\" \"A personal inventory container.\")\n"
    "(ma-save-state!)\n"))

(define (set-inventory-actor! actor)
  (begin
    (set-prop! "inventory" (canonical-actor actor))
    (ma-save-state!)))

(define (inventory-parent-ctx actor)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (canonical-actor actor))
              "kind" "container")
            "protocol" CONTAINER_KIND)
          "parent" (qualified-actor (local-self)))
        "name" INVENTORY_LABEL)
      "nick" "inventory")
    "description" "A personal inventory container."))

(define (adopt-inventory! actor)
  (if (valid-did-url? actor)
      (let ((current (inventory-actor))
            (adopted (canonical-actor actor)))
        (begin
          (if (and current (not (same-actor? current adopted)))
              (begin
                (set-prop! "inventory-ctx" ""))
              #f)
          (set-inventory-actor! adopted)
            (ma-send! adopted (list :child (inventory-parent-ctx adopted)))))
      #f))

(define (ensure-configured-inventory! owner-did actor)
  (if (and actor
           (same-actor? actor (inventory-for-did owner-did))
           (dead-local-actor? actor))
      (ma-create-actor CONTAINER_KIND #f (inventory-init owner-did (local-self)) (inventory-fragment owner-did))
      #f)
  actor)

(define (ensure-inventory-ref!)
  (let ((owner-did (did)))
    (if (valid-did? owner-did)
        (let ((configured (inventory-actor)))
          (if configured
              (ensure-configured-inventory! owner-did configured)
              (let ((actor (inventory-for-did owner-did)))
                (begin
                  (set-inventory-actor! actor)
                  (if (entity-live? actor)
                      #f
                      (ma-create-actor CONTAINER_KIND #f (inventory-init owner-did (local-self)) (inventory-fragment owner-did)))
                  actor))))
        #f)))

(define (ensure-inventory!)
  (let ((owner-did (did)))
    (if (valid-did? owner-did)
        (let ((configured (inventory-actor)))
          (if configured
              (ensure-configured-inventory! owner-did configured)
              (let ((actor (inventory-for-did owner-did)))
                (begin
                  (set-inventory-actor! actor)
                  (if (entity-live? actor)
                      #f
                      (ma-create-actor CONTAINER_KIND #f (inventory-init owner-did (local-self)) (inventory-fragment owner-did)))
                  actor))))
        #f)))

(define (inventory-parent)
  (let ((actor (ensure-inventory!)))
    (if actor actor (local-self))))

(define (nick)
  (let ((value (get-prop "nick")))
    (if value value "avatar")))

; Context terms sent to the controlling did. These must contain fully
; qualified actor references, never runtime-local #fragment shorthand.
(define (ctx-term text)
  (let ((inventory (ensure-inventory-ref!)))
    (list :ctx
      (list (list :kind "avatar")
            (list :did (did))
            (list :root (qualified-actor (root)))
            (list :avatar (qualified-actor (local-self)))
            (list :inv (qualified-actor inventory))
            (list :nick (nick))
            (list :room (qualified-actor (room)))
            (list :text text)))))

(define (ctx-term-room r text)
  (let ((inventory (ensure-inventory-ref!)))
    (list :ctx
      (list (list :kind "avatar")
            (list :did (did))
            (list :root (qualified-actor (root)))
            (list :avatar (qualified-actor (local-self)))
            (list :inv (qualified-actor inventory))
            (list :nick (nick))
            (list :room (qualified-actor r))
            (list :text text)))))

(define (avatar-ctx-map r text)
  (let ((inventory (ensure-inventory-ref!)))
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set
                (map-set
                  (make-map)
                  "kind" "avatar")
                "did" (did))
              "root" (qualified-actor (root)))
            "avatar" (qualified-actor (local-self)))
          "inv" (qualified-actor inventory))
        "nick" (nick))
      "room" (qualified-actor r))))

(define (avatar-child-ctx)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (local-self))
              "kind" "avatar")
            "protocol" "/ma/avatar/0.0.1")
          "parent" (qualified-actor (room)))
        "name" (nick))
      "nick" (nick))
    "description" "An avatar."))

(define (notify-old-room-after-commit! old-room)
  (if (and (non-empty-string? old-room)
           (not (same-actor? old-room (room))))
  (ma-send! (canonical-actor old-room) (list :parent (avatar-child-ctx)))
      #f))

(define (start-room) (ma-get-config-key "start"))
(define THING_KIND "/ma/thing/0.0.1")
(define AGENT_KIND "/ma/scheme/agent/0.0.1")
(define CONJURE_KIND_MAP
  (list
    (list "thing" THING_KIND)
    (list "container" CONTAINER_KIND)
    (list "agent" AGENT_KIND)))

(define (send-ctx text)
  (begin
    (ensure-inventory!)
    (ma-send! (did) (ctx-term text))
    (if (room)
      (ma-send! (canonical-actor (room)) (list :parent (avatar-child-ctx)))
        #f)))

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
      (let ((kind (ctx-value payload :kind))
            (root (ctx-value payload :root))
            (avatar (ctx-value payload :avatar))
            (target-room (ctx-value payload :room)))
        (and (equal? kind "avatar")
             (qualified-ctx-actor? root)
             (qualified-ctx-actor? avatar)
             (qualified-ctx-actor? target-room)
             (same-actor? avatar (self))
             (same-actor? (msg-from msg) target-room)))
      #f))

(define (room-ctx-valid? payload msg)
  (and (map? payload)
       (equal? (ctx-text payload "protocol") ROOM_KIND)
       (equal? (ctx-text payload "kind") "room")
       (qualified-ctx-actor? (ctx-text payload "actor"))
       (qualified-ctx-actor? (ctx-text payload "parent"))
       (number? (map-ref payload "rev" #f))
       (same-actor? (ctx-text payload "actor") (msg-from msg))
       (room? msg)))

(define (stored-room-ctx)
  (let ((ctx (get-prop "room-ctx")))
    (if (map? ctx) ctx #f)))

(define (stored-room-ctx-rev)
  (let ((ctx (stored-room-ctx)))
    (if ctx (map-ref ctx "rev" 0) 0)))

(define (remember-room-ctx! ctx)
  (let ((rev (map-ref ctx "rev" 0)))
    (if (> rev (stored-room-ctx-rev))
        (begin
          (set-prop! "room-ctx" ctx)
          (ma-save-state!))
        #f)))

(define (container-ctx-valid? payload msg)
  (and (map? payload)
       (equal? (ctx-text payload "protocol") CONTAINER_KIND)
       (equal? (ctx-text payload "kind") "container")
       (qualified-ctx-actor? (ctx-text payload "actor"))
       (qualified-ctx-actor? (ctx-text payload "parent"))
       (number? (map-ref payload "rev" #f))
       (same-actor? (ctx-text payload "actor") (msg-from msg))
       (same-actor? (ctx-text payload "parent") (local-self))
       (inventory-actor)
       (same-actor? (ctx-text payload "actor") (inventory-actor))))

(define (stored-inventory-ctx)
  (let ((ctx (get-prop "inventory-ctx")))
    (if (map? ctx) ctx #f)))

(define (stored-inventory-ctx-rev)
  (let ((ctx (stored-inventory-ctx)))
    (if ctx (map-ref ctx "rev" 0) 0)))

(define (remember-inventory-ctx! ctx)
  (let ((rev (map-ref ctx "rev" 0)))
    (if (> rev (stored-inventory-ctx-rev))
        (begin
          (set-prop! "inventory-ctx" ctx)
          (ma-save-state!)
          #t)
        #f)))

(define (exit-avatar-ctx-valid? payload msg)
  (and (map? payload)
       (equal? (ctx-text payload "kind") "avatar")
       (equal? (ctx-text payload "did") (did))
       (same-actor? (ctx-text payload "avatar") (local-self))
       (qualified-ctx-actor? (ctx-text payload "room"))
      (exit-ctx-caller? payload msg)))

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

(define (claim-target-token args)
  (if (null? args) #f (car args)))

(define (claim-target-token? token)
  (and (string? token)
       (> (string-length token) 1)
       (equal? (substring token (- (string-length token) 1) (string-length token)) ":")))

(define (claim-token-strip-colon token)
  (if (claim-target-token? token)
      (substring token 0 (- (string-length token) 1))
      token))

; Resolve a claim target the same way take/drop resolve a token: a literal
; DID-URL, an inventory item, or a visible room entry (agent, thing, exit).
(define (claim-resolve-actor token)
  (cond ((valid-did-url? token) (canonical-actor token))
        ((inventory-ref token) (inventory-entry-actor (inventory-ref token)))
        (else
         (let ((room-entry (room-ctx-ref token)))
           (cond ((equal? room-entry :ambiguous) :ambiguous)
                 ((and (map? room-entry) (valid-did-url? (ctx-text room-entry "actor")))
                  (canonical-actor (ctx-text room-entry "actor")))
                 (else #f))))))

(define (claim-target-ref token)
  (if token (claim-resolve-actor (claim-token-strip-colon token)) #f))

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

(define (take-source-entry args)
  (if (take-has-source? args)
      (let ((token (car (cdr (cdr args)))))
        (cond ((inventory-ref token) (inventory-ref token))
              (else
               (let ((entry (room-ctx-ref token)))
                 (cond ((and (map? entry)
                             (valid-did-url? (ctx-text entry "actor")))
                        (cons (canonical-actor (ctx-text entry "actor")) entry))
                       ((valid-did-url? token)
                        (cons (canonical-actor token) (make-map)))
                       (else #f))))))
      #f))

(define (take-source-ref source-entry)
  (if source-entry (inventory-entry-actor source-entry) #f))

(define (container-content-ref contents token)
  (let loop ((entries (map->alist contents)))
    (cond ((null? entries) #f)
          ((and (inventory-entry-actor (car entries))
                (inventory-entry-matches? token (car entries)))
           (car entries))
          (else (loop (cdr entries))))))

(define (take-child-ref source-entry token)
  (cond ((valid-did-url? token) (canonical-actor token))
        ((and source-entry
              (map? (cdr source-entry))
              (map? (map-ref (cdr source-entry) "contents" #f)))
         (let ((entry (container-content-ref (map-ref (cdr source-entry) "contents") token)))
           (if entry (inventory-entry-actor entry) #f)))
        (else #f)))

(define (send-take args)
  (let* ((token (take-token args))
         (source-entry (take-source-entry args))
      (source (if (take-has-source? args)
        (take-source-ref source-entry)
        (room)))
         (child (take-child-ref source-entry token))
         (carrier (inventory-parent)))
  (cond ((and (valid-did-url? token) (not (take-has-source? args)))
    (ma-send! (canonical-actor token) (list :take (did) carrier)))
      ((and source child (take-has-source? args))
       (ma-send! (canonical-actor source)
                 (list :take (did) (canonical-actor child) carrier)))
      ((and source (take-has-source? args))
       (send-did-text (string-append "Unknown child in " (car (cdr (cdr args))) ": " token)))
      (source
      (if (take-has-source? args)
        #f
         (let ((entry (room-ctx-ref token)))
           (cond ((equal? entry :ambiguous)
                  (send-did-text (string-append "Ambiguous visible agent or thing: " token)))
                 (entry
                  (ma-send! (canonical-actor (ctx-text entry "actor")) (list :take (did) carrier entry)))
                 (else
                  (send-did-text (string-append "Unknown visible agent or thing: " token)))))))
                ((take-has-source? args)
                 (send-did-text (string-append "Unknown carried or visible parent: " (car (cdr (cdr args))))))
      (else
       (send-did-text "You are nowhere.")))))

(define (send-recycle args)
  (let* ((token (take-token args))
         (entry (if token (inventory-ref token) #f))
         (actor (if entry (canonical-actor (car entry)) #f)))
    (cond (entry
           (ma-send! (canonical-actor (inventory-parent)) (list :recycle-from (did) actor)))
          ((room)
           (send-room :recycle (list (did) token)))
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
  (let ((inventory (inventory-parent)))
    (if inventory
        (ma-send! (canonical-actor inventory) (list :take did (canonical-actor item-actor) (canonical-actor container-actor) :drop))
        (ma-send! (canonical-actor item-actor) (list :drop did (canonical-actor container-actor) item-ctx)))))

(define (put-visible! did item-actor container-actor item-ctx)
  (begin
    (ma-send! (canonical-actor item-actor) (list :drop did (canonical-actor container-actor) item-ctx))))

(define (visible-container-ref token)
  (let ((entry (room-ctx-ref token)))
    (if (and (map? entry)
             (valid-did-url? (ctx-text entry "actor")))
        entry
        #f)))

(define (send-put args)
  (let* ((item-token (put-item-token args))
         (container-token (put-container-token args))
         (item-entry (inventory-ref item-token))
         (container-entry (inventory-ref container-token))
         (visible-container (visible-container-ref container-token))
         (item-actor (if item-entry (inventory-entry-actor item-entry) #f))
         (container-actor
           (cond (container-entry (inventory-entry-actor container-entry))
                 ((valid-did-url? container-token) (canonical-actor container-token))
                 (visible-container (canonical-actor (ctx-text visible-container "actor")))
                 (else #f)))
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

(define (look-token args)
  (if (null? args) #f (join-words args)))

(define (look-carried-text ctx)
  (let ((label (child-label ctx))
        (description (ctx-text ctx "description")))
    (if (non-empty-string? description)
        (string-append label "\n" description)
        label)))

(define (look-carried-entry entry)
  (let ((actor (inventory-entry-actor entry))
        (ctx (cdr entry)))
    (if actor
        (ma-send! (canonical-actor actor) (list :look (did)))
        (send-did-text (look-carried-text ctx)))))

(define (room-ctx-entry-matches? token entry)
  (and (map? entry)
       (or (inventory-label-matches? token (ctx-text entry "actor"))
           (inventory-label-matches? token (ctx-text entry "nick"))
           (inventory-label-matches? token (ctx-text entry "direction")))))

(define (room-ctx-same-entry? a b)
  (and (map? a)
       (map? b)
       (same-actor? (ctx-text a "actor") (ctx-text b "actor"))))

(define (room-ctx-merge-found current candidate)
  (cond ((not candidate) current)
        ((not current) candidate)
        ((equal? current :ambiguous) :ambiguous)
        ((room-ctx-same-entry? current candidate) current)
        (else :ambiguous)))

(define (room-ctx-find-entry-in token entries)
  (let loop ((rest entries)
             (found #f))
    (cond ((null? rest) found)
          ((equal? found :ambiguous) :ambiguous)
          ((room-ctx-entry-matches? token (car rest))
           (loop (cdr rest) (room-ctx-merge-found found (car rest))))
          (else (loop (cdr rest) found)))))

(define (room-ctx-find-entry token ctx keys)
  (if (null? keys)
      #f
      (let ((found (room-ctx-find-entry-in token (map-ref ctx (car keys) '()))))
        (cond ((equal? found :ambiguous) :ambiguous)
              (found
               (room-ctx-merge-found found (room-ctx-find-entry token ctx (cdr keys))))
              (else (room-ctx-find-entry token ctx (cdr keys)))))))

(define (room-ctx-ref token)
  (let ((ctx (stored-room-ctx)))
    (if ctx
        (room-ctx-find-entry token ctx (list "things" "agents" "who" "exits"))
        #f)))

(define (room-ctx-exit-ref token)
  (let ((ctx (stored-room-ctx)))
    (if ctx
        (let ((entry (room-ctx-find-entry token ctx (list "exits"))))
          (if (and entry
                   (equal? (ctx-text entry "kind") "exit")
                   (valid-did-url? (ctx-text entry "actor")))
              entry
              #f))
        #f)))

(define (room-ctx-exit-actor? actor)
  (let ((entry (room-ctx-exit-ref actor)))
    (and entry (same-actor? actor (ctx-text entry "actor")))))

(define (exit-ctx-caller? ctx msg)
  (let ((exit (ctx-text ctx "exit")))
    (and (non-empty-string? exit)
         (same-actor? exit (msg-from msg))
         (room-ctx-exit-actor? exit))))

(define (avatar-go! args msg)
  (let ((direction (if (null? args) #f (car args))))
    (cond ((not direction)
           (send-did-text "Usage: go <direction>"))
          ((get-prop "pending-room")
           (send-did-text "Movement already pending."))
          (else
           (let ((entry (room-ctx-exit-ref direction)))
             (if entry
                 (ma-send! (canonical-actor (ctx-text entry "actor")) (list :ctx (avatar-ctx-map (room) #f)))
                 (send-did-text (string-append "No exit " direction "."))))))))

(define (look-room-ctx-entry entry token)
  (let ((actor (ctx-text entry "actor")))
    (if (valid-did-url? actor)
        (ma-send! (canonical-actor actor) (list :look (did)))
        (send-did-text (string-append "You cannot inspect " token " yet.")))))

(define (avatar-look-text)
  (string-append (nick) "\nAn avatar."))

(define (present-avatar-look! target msg)
  (let ((recipient (presentation-avatar-target target msg)))
    (if recipient
      (begin
        (ma-send! recipient (list :print (avatar-look-text)))
        (reply-ok msg))
      #f)))

(define (avatar-look args msg)
  (if (and (not (null? args)) (present-avatar-look! (car args) msg))
      #f
      (require-did msg
        (lambda ()
          (let* ((token (look-token args))
                 (entry (if token (inventory-ref token) #f)))
            (cond (entry (look-carried-entry entry))
                  (token
                   (let ((visible (room-ctx-ref token)))
                     (cond ((equal? visible :ambiguous)
                        (send-did-text (string-append "Ambiguous visible agent or thing: " token)))
                         (visible
                        (look-room-ctx-entry visible token))
                         (else
                        (send-did-text (string-append "You do not see " token " here."))))))
                  (else (send-room :look args))))
          (reply-ok-silent msg)))))

(define (avatar-say args msg)
  (require-did msg
    (lambda ()
      (send-room :say (list (join-words args)))
      (reply-ok-silent msg))))

(define (send-did-text text)
  (ma-send! (did) (list :print text)))

(define (inventory-map)
  (let* ((ctx (stored-inventory-ctx))
         (contents (if ctx (map-ref ctx "contents" #f) #f)))
    (if (map? contents) contents (make-map))))

(define (inventory-label-matches? token label)
  (and (non-empty-string? token)
       (non-empty-string? label)
       (or (equal? (string-downcase token) (string-downcase label))
           (string-list-member? (string-downcase token) (string-split (string-downcase label) " ")))))

(define (string-list-member? token xs)
  (cond ((null? xs) #f)
        ((equal? token (car xs)) #t)
        (else (string-list-member? token (cdr xs)))))

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

(define (inventory-ref token)
  (inventory-ref-resolved token))

(define (inventory-lookup-did-url token)
  (cond ((valid-did-url? token) (canonical-actor token))
        (else
         (let ((entry (inventory-ref token)))
           (if entry (inventory-entry-actor entry) #f)))))

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
        ((not (child-ctx-self-authentic? (car args) msg))
          (reply-error msg "child ctx actor must match sender"))
        ((not (same-actor? (ctx-text (car args) "parent") (local-self)))
         (begin
           (forget-child! (ctx-text (car args) "actor"))
           (reply-ok msg)))
        (else
         (begin
           (remember-child! (car args))
           (ma-send! (canonical-actor (ctx-text (car args) "actor")) (list :child (car args)))
           (reply-ok msg)))))

(define (make-kind kind)
  (if (equal? kind "thing") THING_KIND kind))

(define (valid-make-kind? kind)
  (and (non-empty-string? kind)
       (string-prefix? "/" kind)))

(define (conjure-kind-from-map entries kind)
  (cond ((null? entries) #f)
        ((and (pair? (car entries))
              (not (null? (cdr (car entries))))
              (equal? (car (car entries)) kind))
         (car (cdr (car entries))))
        (else (conjure-kind-from-map (cdr entries) kind))))

(define (conjure-kind kind)
  (conjure-kind-from-map CONJURE_KIND_MAP kind))

(define (conjure-in-split args)
  (if (null? args)
      #f
      (let ((later (conjure-in-split (cdr args))))
        (cond (later
               (list (cons (car args) (car later)) (car (cdr later))))
              ((and (equal? (car args) "in") (not (null? (cdr args))))
               (list '() (cdr args)))
              (else #f)))))

(define (conjure-parent-ref token)
  (let ((carried (inventory-ref token))
        (visible (room-ctx-ref token))
        (current-room (room)))
    (cond ((and carried (equal? (ctx-text (cdr carried) "kind") "container"))
           (inventory-entry-actor carried))
          ((equal? visible :ambiguous) :ambiguous)
          ((and (map? visible)
                (equal? (ctx-text visible "kind") "container")
                (valid-did-url? (ctx-text visible "actor")))
           (canonical-actor (ctx-text visible "actor")))
          ((and current-room (same-actor? token current-room))
           (canonical-actor current-room))
          (else #f))))

(define (conjure-init name parent)
  (let ((creator (did))
        (qualified-parent (qualified-actor parent)))
    (if (and (non-empty-string? creator)
             (non-empty-string? qualified-parent)
             (non-empty-string? name))
        (string-append
          "(set-init-prop! \"name\" \"" name "\")\n"
          "(set-init-prop! \"owner\" \"" creator "\")\n"
          "(set-init-prop! \"parent\" \"" qualified-parent "\")\n"
          "(ma-save-state!)\n")
        #f)))

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
                        (send-did-text (string-append "Create requested: " actor "."))
                        (reply-ok-with msg actor))))))))))

(define (avatar-conjure args msg)
  (require-did msg
    (lambda ()
      (cond ((or (null? args)
                 (null? (cdr args))
                 (null? (cdr (cdr args)))
                 (not (equal? (car (cdr args)) "named")))
               (reply-error msg "Usage: conjure thing|container|agent named <name> [in <parent>]"))
            (else
                  (let* ((kind (conjure-kind (car args)))
                    (name-and-parent (conjure-in-split (cdr (cdr args))))
                    (name-args (if name-and-parent
                    (car name-and-parent)
                    (cdr (cdr args))))
                    (parent-args (if name-and-parent
                      (car (cdr name-and-parent))
                      '()))
                    (name (join-words name-args))
                    (parent-token (if (null? parent-args) #f (join-words parent-args))))
               (cond ((not kind)
                      (reply-error msg "conjure kind must be thing, container, or agent"))
                     ((not (non-empty-string? name))
                      (reply-error msg "conjure name must be non-empty"))
                     (else
                 (let ((parent (if parent-token
                         (conjure-parent-ref parent-token)
                         (ensure-inventory!))))
                   (cond ((equal? parent :ambiguous)
                     (reply-error msg (string-append "Ambiguous visible parent: " parent-token)))
                    ((not (valid-did-url? parent))
                     (reply-error msg (string-append "Unknown visible container or room: "
                                (if parent-token parent-token "inventory"))))
                    (else
                     (let* ((init (conjure-init name parent))
                                   (fragment (ma-create-actor kind #f init #f))
                                   (actor (entity-url fragment)))
                       (send-did-text (string-append "Conjure requested: " actor "."))
                       (reply-ok-with msg actor)))))))))))))

(define (reply-ok-silent msg)
  (reply-ok msg))

(define (avatar-help-text)
  (string-append
    "Help\n"
    "  help              show this help\n"
    "  help here         ask this place what is possible here\n"
    "  look              look around\n"
    "  l                 alias for look\n"
    "  look <thing|exit> inspect a carried thing or visible exit\n"
    "  exits?            list exits\n"
    "  here?             show where you are\n"
    "  who?              show avatars here\n"
    "  things?           list known things\n"
    "  did? [kind] <name> show the DID for a visible occupant, thing, or exit\n"
    "  owner? <name>     show who owns a visible occupant, thing, or exit\n"
    "  make <kind> <init...> create an actor from init text\n"
    "  conjure thing|container|agent named <name> [in <parent>] create with standard init\n"
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
    "  claim <thing> [secret] claim an inventory item, visible actor, or DID-URL\n"
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

(define (avatar-arg-at args index)
  (cond ((null? args) #f)
        ((= index 0) (car args))
        (else (avatar-arg-at (cdr args) (- index 1)))))

(set-internal-rpc-method! :enter-room
  (lambda (args msg)
    (if (and (enter-room-authorised? args msg) (not (null? args)))
        (let ((target-room (car args))
              (old-room (room))
              (requested-nick (avatar-arg-at args 2))
              (requested-inventory (avatar-arg-at args 3)))
          (if (non-empty-string? requested-nick)
              (begin
                (set-prop! "nick" requested-nick)
                (ma-save-state!))
              #f)
          (if (valid-did-url? requested-inventory)
              (adopt-inventory! requested-inventory)
              #f)
          (ma-send! (canonical-actor target-room)
                    (list :enter
                          (local-self)
                          (canonical-actor old-room)
                          (nick)
                            (qualified-actor (inventory-actor))
                            (did))))
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
             (let ((old-room (room)))
               (clear-pending-move!)
               (set-prop! "room" (canonical-actor (ctx-value payload :room)))
               (set-prop! "nick" (ctx-value payload :nick))
               (ma-save-state!)
                 (if (valid-did-url? (ctx-value payload :inv))
                   (adopt-inventory! (ctx-value payload :inv))
                   (ensure-inventory!))
               (ma-send! (did)
                         (ctx-term-room (ctx-value payload :room)
                                        (ctx-value payload :text)))
               (notify-old-room-after-commit! old-room)))
            ((room-ctx-valid? payload msg)
             (remember-room-ctx! payload))
            ((exit-avatar-ctx-valid? payload msg)
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
                       (ma-send! (canonical-actor target-room) (list :enter payload)))))))
            (else #f))))))

(set-rpc-method! :ctx?
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (ensure-inventory!)
        (ma-reply! msg (list :ok (ctx-term #f)))))))

(set-internal-rpc-method! :print
  (lambda (args msg)
    (ma-send! (did) (list :print (join-words args)))))

(set-internal-rpc-method! :delivery-failed
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)))
        #f
        (ma-send! (did)
                  (list :print
                        (string-append "Could not deliver to "
                                       (car args)
                                       ": "
                                       (car (cdr args))))))))

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
        (let* ((token (claim-target-token args))
               (target (if token (claim-target-ref token) #f))
               (claim-args (if (and target (not (equal? target :ambiguous))) (cdr args) args)))
          (cond ((equal? target :ambiguous)
                 (reply-error msg (string-append "Ambiguous visible agent or thing: " token)))
                ((and (claim-target-token? token) (not target))
                 (reply-error msg "claim target must be a known actor, inventory item, or visible name"))
                (target
                 (begin
                   (ma-send! target (cons :claim claim-args))
                   (reply-ok-silent msg)))
                (else
                 (begin
                   (send-room :claim claim-args)
                   (reply-ok-silent msg)))))))))

(set-cmd-method! :owner?
  (lambda (args msg)
    (if (and (room? msg) (not (null? args)))
        (begin
          (ma-send! (canonical-actor (car args)) (list :print (string-append "Owner: " (did))))
          (reply-ok-silent msg))
        (require-did msg
          (lambda ()
            (send-room :owner? args)
            (reply-ok-silent msg))))))

(set-cmd-method! :make
  avatar-make)

(set-cmd-method! :conjure
  avatar-conjure)

(set-meta-method! :child
  handle-avatar-children)

(set-meta-method! :parent
  (lambda (args msg)
    (cond ((or (null? args) (not (null? (cdr args))))
           (reply-error msg "usage: :parent <ctx>"))
          ((container-ctx-valid? (car args) msg)
           (begin
             (if (remember-inventory-ctx! (car args))
                 (ma-send! (did) (ctx-term #f))
                 #f)
             (reply-ok msg)))
          (else
           (reply-error msg "parent ctx must come from configured inventory container")))))

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
          (send-take args))
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
            (send-did-text "Usage: drop <thing>")
            (let* ((token (join-words args))
                   (entry (inventory-ref token))
                   (actor (if entry
                              (inventory-entry-actor entry)
                              (if (valid-did-url? token) (canonical-actor token) #f))))
              (if actor
                  (if (and entry (inventory-parent))
                    (ma-send! (canonical-actor (inventory-parent))
                        (list :take
                            (did)
                            (canonical-actor actor)
                            (canonical-actor (room))
                            :drop))
                    (ma-send! (canonical-actor actor)
                        (list :drop (did) (canonical-actor (room)))))
                  (send-did-text (string-append "Unknown carried agent or thing: " token)))))
        (reply-ok-silent msg)))))

(set-cmd-method! :recycle
  (lambda (args msg)
    (require-did msg
      (lambda ()
        (if (or (null? args) (not (null? (cdr args))))
            (send-did-text "Usage: recycle <agent-or-thing>")
            (send-recycle args))
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
        (avatar-go! args msg)
        (reply-ok-silent msg)))))

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

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (ensure-inventory!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
