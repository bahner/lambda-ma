; Locked room actor.
; Rooms own exits, bare-DID presence, and local policy for movable actors.

; Kinds.
(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")

; Every room resident lives in the inherited children map. Bare direct DIDs
; are ordinary child keys alongside actor DID-URLs; views derive from ctxs.
(define (did-ctx did)
  (let ((ctx (child-ctx did)))
    (if (and (map? ctx) (valid-did? (ctx-text ctx "actor"))) ctx #f)))

(define (did-occupants)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (reverse acc))
          ((valid-did? (ctx-text (car ctxs) "actor"))
           (loop (cdr ctxs) (cons (ctx-text (car ctxs) "actor") acc)))
          (else (loop (cdr ctxs) acc)))))

; Normalise direct-entry ctxs without introducing a second store.
(define (set-did-ctx! did ctx)
  (begin
    (remember-child!
      (map-set
        (map-set
          (map-set ctx "actor" did)
          "kind" "agent")
        "protocol" "/ma/agent/0.0.1"))
    (ma-save-state!)))
(define (claims-map) (children-map))

(define (set-claims-map! claims) (set-children-map! claims))

(define (claim-key actor)
  (canonical-actor actor))

(define (room-claim? ctx)
  (and (actor-ctx-shape? ctx)
       (same-actor? (child-parent-target ctx) (self))))

(define (claim-ctx actor)
  (let ((ctx (child-ctx actor)))
    (if (map? ctx) ctx #f)))

(define (set-claim! actor ctx)
  (begin
    (remember-child! ctx)
    (ma-save-state!)))

(define (remove-claim! actor)
  (forget-child! actor))

(define (node-child-orphaned! ctx)
  (begin
    (remove-claim! (ctx-text ctx "actor"))
    (ma-save-state!)
    (broadcast-room-ctx!)))

(define (claim-actors-by-kind kind)
  (let loop ((ctxs (child-ctxs-by-kind kind)) (acc '()))
    (cond ((null? ctxs) (reverse acc))
          ((room-claim? (car ctxs))
           (loop (cdr ctxs)
                 (cons (canonical-actor (ctx-text (car ctxs) "actor")) acc)))
          (else
           (loop (cdr ctxs) acc)))))

(define (occupant-kind? kind)
  (equal? kind "agent"))

(define (occupants)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (unique-actor-entries (reverse acc)))
          ((and (room-claim? (car ctxs))
                (occupant-kind? (ctx-text (car ctxs) "kind")))
           (loop (cdr ctxs)
                 (cons (canonical-actor (ctx-text (car ctxs) "actor")) acc)))
          (else
           (loop (cdr ctxs) acc)))))

(define (set-label! actor label)
  (let ((ctx (child-ctx actor)))
    (if (and (map? ctx) (non-empty-string? label))
        (remember-child! (map-set ctx "nick" label))
        #f)))

(define (has-label? actor)
  (let ((ctx (child-ctx actor)))
    (and (map? ctx) (non-empty-string? (ctx-text ctx "nick")))))

; Presentation helpers.
(define (speaker-name actor)
  (let ((ctx (child-ctx actor)))
    (if (map? ctx) (child-label ctx) actor)))

(define (visible-occupant-matches token)
  (let loop ((xs (occupants)))
    (cond ((null? xs) '())
          ((equal? (speaker-name (car xs)) token)
           (cons (car xs) (loop (cdr xs))))
          (else (loop (cdr xs))))))

(define (unique-visible-occupant-ref token)
  (let ((matches (visible-occupant-matches token)))
    (cond ((null? matches) #f)
          ((null? (cdr matches)) (car matches))
          (else :ambiguous))))

(define (visible-child-matches token)
  (let loop ((ctxs (child-ctxs)) (matches '()))
    (cond ((null? ctxs) (reverse matches))
          ((equal? (child-label (car ctxs)) token)
           (loop (cdr ctxs) (cons (ctx-text (car ctxs) "actor") matches)))
          (else (loop (cdr ctxs) matches)))))

(define (unique-visible-child-ref token)
  (let ((matches (visible-child-matches token)))
    (cond ((null? matches) #f)
          ((null? (cdr matches)) (car matches))
          (else :ambiguous))))

(define (take-carrier-parent take-args carrier)
  (let ((candidate (if (or (null? take-args) (null? (cdr take-args))) #f (car (cdr take-args)))))
    (if (and (non-empty-string? candidate)
             (or (valid-did-url? candidate) (local-actor-ref? candidate)))
        (canonical-actor candidate)
        (canonical-actor carrier))))

(define (visible-ref token)
  (let ((matches (did-matches token)))
    (cond ((null? matches) #f)
          ((null? (cdr matches)) (car matches))
          (else :ambiguous))))

(define (entry-lines xs)
  (cond ((null? xs) "")
        ((null? (cdr xs)) (car xs))
        (else (string-append (car xs) "\n" (entry-lines (cdr xs))))))

(define (occupant-did-lines)
  (let loop ((xs (occupants)))
    (if (null? xs)
        '()
        (cons (string-append (speaker-name (car xs)) " = " (canonical-actor (car xs)))
              (loop (cdr xs))))))

(define (thing-did-lines)
  (let loop ((tokens (thing-token-names)))
    (if (null? tokens)
        '()
        (cons (string-append (car tokens) " = " (canonical-actor (thing-ref (car tokens))))
              (loop (cdr tokens))))))

(define (exit-did-lines)
  (let loop ((directions (exit-directions)))
    (if (null? directions)
        '()
        (cons (string-append (car directions) " = " (canonical-actor (exit-target (car directions))))
              (loop (cdr directions))))))

(define (exit-did-matches token)
  (let ((exit (exit-target token)))
    (if exit (list exit) '())))

(define (thing-did-matches token)
  (let ((thing (thing-ref token)))
    (if thing (list thing) '())))

(define (occupant-did-matches token)
  (visible-occupant-matches token))

(define (did-matches token)
  (list-append
    (list-append (exit-did-matches token) (thing-did-matches token))
    (occupant-did-matches token)))

(define (qualified-did-line kind token actor)
  (string-append kind " " token " = " (canonical-actor actor)))

(define (exit-did-match-lines token)
  (let ((exit (exit-target token)))
    (if exit (list (qualified-did-line "exit" token exit)) '())))

(define (thing-did-match-lines token)
  (let ((thing (thing-ref token)))
    (if thing (list (qualified-did-line "thing" token thing)) '())))

(define (occupant-did-match-lines token)
  (let loop ((xs (visible-occupant-matches token)))
    (if (null? xs)
        '()
        (cons (qualified-did-line "occupant" token (car xs))
              (loop (cdr xs))))))

(define (did-match-lines token)
  (list-append
    (list-append (exit-did-match-lines token) (thing-did-match-lines token))
    (occupant-did-match-lines token)))

(define (room-name)
  (let ((name (get-prop "name")))
    (if name name "Construct")))

(define (room-nick)
  (let ((nick (get-prop "nick")))
    (if nick nick (room-name))))

(define (room-description)
  (let ((description (get-prop "description")))
    (if description description "“This is the Construct. It's our loading program. We can load anything... From clothing to equipment, weapons, training simulations; anything we need.”")))

(define (room-text)
  (string-append
    (room-name) "\n"
    (room-description) "\n"
    (occupants-text) "\n"
    (things-text) "\n"
    (exits-text)))

(define (arrival-text) (room-name))

(define (room-ctx-rev)
  (let ((value (get-prop "ctx:rev")))
    (if (number? value) value 0)))

(define (room-ctx-parent)
  (canonical-actor (root)))

(register-ctx-props! (list "ctx:rev" "children"))

(define (presentation-entry kind protocol actor name nick description)
  (make-map
    "actor" (canonical-actor actor)
    "kind" kind
    "protocol" protocol
    "name" name
    "nick" nick
    "description" description))

(define (claim-or-fallback-entry token actor kind protocol)
  (let ((ctx (claim-ctx actor)))
    (if (actor-ctx-shape? ctx)
        ctx
        (presentation-entry kind protocol actor token token token))))

(define (agent-entry-ctx actor)
  (claim-or-fallback-entry (speaker-name actor) actor "agent" "/ma/agent/0.0.1"))

(define (thing-entry-ctx token)
  (let* ((actor (thing-ref token))
         (ctx (claim-ctx actor)))
    (if (actor-ctx-shape? ctx)
        ctx
        (presentation-entry "thing" "/ma/thing/0.0.1" actor token token token))))

(define (exit-entry-ctx direction)
  (exit-ctx direction))

(define (agent-entry-list actors)
  (let loop ((xs actors)
             (acc '()))
    (cond ((null? xs) (reverse acc))
          (else
           (loop (cdr xs) (cons (agent-entry-ctx (car xs)) acc))))))

(define (thing-entry-list tokens)
  (let loop ((xs tokens)
             (acc '()))
    (if (null? xs)
        (reverse acc)
        (loop (cdr xs) (cons (thing-entry-ctx (car xs)) acc)))))

(define (exit-entry-list directions)
  (let loop ((xs directions)
             (acc '()))
    (if (null? xs)
        (reverse acc)
        (loop (cdr xs) (cons (exit-entry-ctx (car xs)) acc)))))

(define (did-entry-map dids)
  (let loop ((xs dids)
             (entries (make-map)))
    (if (null? xs)
        entries
        (loop (cdr xs) (map-set entries (car xs) (did-ctx (car xs)))))))

(define (room-ctx)
  (make-map
    "protocol" ROOM_KIND
    "kind" "room"
    "actor" (canonical-actor (self))
    "parent" (room-ctx-parent)
    "rev" (room-ctx-rev)
    "name" (room-name)
    "nick" (room-nick)
    "description" (room-description)
    "who" (did-entry-map (did-occupants))
    "agents" (agent-entry-list (occupants))
    "things" (thing-entry-list (thing-token-names))
    "exits" (exit-entry-list (exit-directions))))

(define (node-ctx-for-parent target-parent)
  (map-set (room-ctx) "parent" (canonical-actor target-parent)))

; Every current child, including a bare-DID client, gets a fresh :child so its
; cached parent-ctx never goes stale.
(define (broadcast-room-ctx!)
  (begin
    (set-prop! "ctx:rev" (+ (room-ctx-rev) 1))
    (ma-save-state!)
    (broadcast-ctx-to-children!)))

(define (names-of actors)
  (cond ((null? actors) "")
        ((null? (cdr actors)) (speaker-name (car actors)))
        (else (string-append (speaker-name (car actors)) ", " (names-of (cdr actors))))))

(define (token-list-text label names)
  (if (null? names)
      (string-append label ": none.")
      (string-append label ": " (names-of names))))

; Entry helpers for direct bare-DID and actor ctx admission.

(define (enter-ctx-args? args)
  (and (not (null? args)) (map? (car args))))

(define (enter-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (ctx-text ctx "kind"))
       (non-empty-string? (ctx-text ctx "parent"))
       (non-empty-string? (ctx-text ctx "protocol"))
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))))

(define (enter-direct-ctx-valid? ctx kind)
  (and (enter-ctx-valid? ctx)
       (equal? (ctx-text ctx "kind") kind)))

(define (direct-room-ctx kind nick text)
  (list :ctx
    (list (list :kind kind)
          (list :root (canonical-actor (root)))
          (list :nick (if nick nick ""))
          (list :room (canonical-actor (self)))
          (list :text text))))

(define (did-name-from-entry args did)
  (let ((ctx (if (enter-ctx-args? args) (car args) #f)))
    (if (and ctx (non-empty-string? (ctx-text ctx "name")))
        (ctx-text ctx "name")
        did)))

(define (did-nick-from-entry args did)
  (let ((ctx (if (enter-ctx-args? args) (car args) #f)))
    (cond ((and ctx (non-empty-string? (ctx-text ctx "nick")))
           (ctx-text ctx "nick"))
          ((and (not ctx) (not (null? args)) (string? (car args)))
           (car args))
          (else did))))

(define (next-did-revision did)
  (+ 1 (let ((previous (did-ctx did)))
         (if (and (map? previous) (number? (map-ref previous "rev" #f)))
             (map-ref previous "rev" #f)
             0))))

(define (did-child-ctx did name nick)
  (make-map "actor" did
            "did" did
            "parent" (canonical-actor (self))
            "kind" "agent"
            "protocol" "/ma/agent/0.0.1"
            "name" name
            "nick" nick
            "description" "A direct DID presence."
            "rev" (next-did-revision did)))

(define (publish-did-ctx! did ctx)
  (let ((runtime-ctx (stored-runtime-ctx)))
    (if runtime-ctx
        (ma-send! (ctx-text runtime-ctx "house") (list :did-ctx did ctx))
        #f)))

(define (commit-did-entry! msg args)
  (let* ((did (msg-from msg))
         (name (did-name-from-entry args did))
         (nick (did-nick-from-entry args did))
         (was-present (member-string? did (did-occupants)))
         (ctx (did-child-ctx did name nick)))
          (remember-child! ctx)
          (ma-save-state!)
    (if was-present #f (broadcast-term-except did (list :arrive ctx)))
    (broadcast-room-ctx!)
    (publish-did-ctx! did ctx)
    (ma-reply! msg (list :ok ctx))))

(define (handle-enter-ctx! msg ctx)
  (let* ((actor (ctx-text ctx "actor"))
         (ctxdid (ctx-did ctx))
         (did (if (valid-did? ctxdid) ctxdid (if (valid-did? actor) actor (msg-from msg))))
         (kind (ctx-text ctx "kind"))
         (name (ctx-text ctx "name")))
    (cond ((and (agent-kind? kind) (enter-direct-ctx-valid? ctx "agent"))
           (handle-agent-enter! msg did ctx))
          ((and (thing-kind? kind) (enter-direct-ctx-valid? ctx "thing"))
           (handle-thing-enter! msg did ctx name))
          (else (reply-error msg "enter ctx requires an agent or thing ctx")))))

; Both branches ack with exactly one :child, via the shared send-fresh-child-ctx!
; (node.ma) so every ack carries the same wrapped parent-ctx, whether it's a
; plain re-confirmation or reached as part of broadcast-room-ctx!'s sweep below.
(define (handle-agent-enter! msg did ctx)
  (let* ((actor (canonical-actor did))
         (nick (ctx-text ctx "nick"))
         (was-known (member-entry? actor (occupants)))
         (same-claim (equal? (claim-ctx actor) ctx))
         (same-label (equal? (speaker-name actor) nick)))
    (if (and was-known same-claim same-label)
        (begin
          (send-fresh-child-ctx! actor)
          (reply-ok msg))
        (begin
          (set-claim! actor ctx)
          (set-label! actor nick)
          (if was-known
              #f
              (broadcast-term-except actor (list :arrive ctx)))
          (ma-save-state!)
          (ma-send! (canonical-actor actor) (direct-room-ctx "agent" nick (arrival-text)))
          (broadcast-room-ctx!)
          (ma-reply! msg (list :ok "entered"))))))

(define (handle-thing-enter! msg did ctx name)
  (let* ((actor (canonical-actor did))
         (label (ctx-text ctx "nick"))
         (token (if (non-empty-string? label) label name))
         (bound (thing-ref token))
         (was-known (map? (claim-ctx actor)))
         (same-claim (equal? (claim-ctx actor) ctx))
         (same-label (equal? (speaker-name actor) label)))
    (cond ((not (actor-token-valid? name))
           (reply-error msg "enter requires non-empty name token"))
          ((not (actor-token-valid? token))
           (reply-error msg "enter requires non-empty nick token"))
          ((and bound (not (same-actor? bound actor)))
           (reply-error msg "nick token is already bound to another actor"))
          ((and (same-actor? bound actor) same-claim same-label)
           (begin
             (send-fresh-child-ctx! actor)
             (reply-ok msg)))
          (else
           (set-claim! actor ctx)
           (set-label! actor label)
           (set-thing! token actor)
           (if was-known #f (broadcast-term (list :arrive ctx)))
           (broadcast-room-ctx!)
           (ma-reply! msg (list :ok "entered"))))))

(define (child-parent-target ctx)
  (let ((parent (ctx-text ctx "parent")))
    (if (non-empty-string? parent)
        parent
        (ctx-text ctx "room"))))

(define (child-announcement-valid? ctx msg)
  (and (or (actor-ctx? ctx msg)
           (and (child-departure-ctx? ctx) (ctx-sender-valid? ctx msg)))
       (or (same-actor? (child-parent-target ctx) (self))
           (not (same-actor? (child-parent-target ctx) (self))))))

(define (handle-child-announcement! msg ctx)
  (let ((kind (ctx-text ctx "kind"))
        (actor (ctx-text ctx "actor"))
        (name (ctx-text ctx "name")))
    (cond ((not (child-announcement-valid? ctx msg))
           (reply-error msg "child ctx actor must match sender"))
          ((not (same-actor? (child-parent-target ctx) (self)))
           (begin
             (remove-claim! actor)
             (remove-thing-actor! actor)
             (ma-save-state!)
             (broadcast-term (list :leave ctx))
             (broadcast-room-ctx!)
             (reply-ok msg)))
          ((and (agent-kind? kind) (enter-direct-ctx-valid? ctx "agent"))
           (handle-agent-enter! msg actor ctx))
          ((and (or (thing-kind? kind) (container-kind? kind)) (enter-ctx-valid? ctx))
           (handle-thing-enter! msg actor ctx name))
          ((and (exit-kind? kind) (enter-ctx-valid? ctx))
           (handle-node-parent (list ctx) msg))
          (else
           (reply-error msg "child ctx must include actor, parent, kind, protocol, name, nick, description")))))

(define (agent-kind? kind) (equal? kind "agent"))
(define (thing-kind? kind) (equal? kind "thing"))
(define (container-kind? kind) (equal? kind "container"))
(define (exit-kind? kind) (equal? kind "exit"))
(define (movable-kind? kind)
  (or (agent-kind? kind) (thing-kind? kind) (container-kind? kind)))

; Movable occupant lookup: local token aliases first, then visible agent labels.
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
  (if (valid-did-url? token)
      (let ((ctx (child-ctx token)))
        (if (and (map? ctx)
                 (or (thing-kind? (ctx-text ctx "kind"))
                     (container-kind? (ctx-text ctx "kind"))))
            (canonical-actor token)
            #f))
      (let loop ((ctxs (list-append (child-ctxs-by-kind "thing")
                                    (child-ctxs-by-kind "container"))))
        (cond ((null? ctxs) #f)
              ((child-token-matches? token (car ctxs))
               (canonical-actor (ctx-text (car ctxs) "actor")))
              (else (loop (cdr ctxs)))))))

(define (movable-ref token)
  (let ((thing (thing-ref token)))
    (if thing thing (occupant-ref token))))

(define (remove-movable! token actor)
  (begin
    (if (same-actor? (thing-ref token) actor)
        (remove-thing! token)
        #f)
    (remove-claim! actor)
    (broadcast-room-ctx!)
    (ma-save-state!)))

(define (set-thing! token did)
  (let ((existing (child-ctx did)))
    (remember-child!
      (if (map? existing)
          (map-set existing "nick" token)
          (presentation-entry "thing" "/ma/thing/0.0.1" did token token token)))))

(define (remove-thing! token)
  (let ((actor (thing-ref token)))
    (if actor (forget-child! actor) #f)))

(define (remove-thing-actor! actor)
  (forget-child! actor))

(define (things-text)
  (token-list-text "Things" (thing-token-names)))

(define (dids-text)
  (let ((lines (list-append (list-append (occupant-did-lines) (thing-did-lines)) (exit-did-lines))))
    (if (null? lines)
        "DIDs: none."
        (string-append "DIDs:\n" (entry-lines lines)))))

(define (handle-dids! msg args)
  (let ((mediated #f))
    (cond ((not (owned?))
           (reply-command-error msg mediated "This room is unowned. Claim it before listing DIDs."))
          ((not (valid-owner? (owner)))
           (reply-command-error msg mediated "Owner must be a DID."))
          ((not (owner-message? msg))
           (reply-command-error msg mediated "Only this room's owner can list visible DIDs."))
          (else
           (reply-command-ok msg mediated (dids-text))))))

(define (handle-did! msg args)
  (let ((mediated #f)
        (did-args args))
   (cond ((null? did-args)
        (reply-command-error msg mediated "Usage: did? [exit|thing|occupant] <name>"))
       ((or (equal? (car did-args) "exit")
          (equal? (car did-args) "thing")
          (equal? (car did-args) "occupant"))
        (if (null? (cdr did-args))
          (reply-command-error msg mediated "Usage: did? [exit|thing|occupant] <name>")
          (let* ((kind (car did-args))
               (token (join-words (cdr did-args)))
               (lines (cond ((equal? kind "exit") (exit-did-match-lines token))
                        ((equal? kind "thing") (thing-did-match-lines token))
                        (else (occupant-did-match-lines token)))))
            (if (null? lines)
              (reply-command-error msg mediated (string-append "No " kind ": " token))
              (reply-command-ok msg mediated (entry-lines lines))))))
       (else
        (let* ((token (join-words did-args))
            (matches (did-matches token))
            (lines (did-match-lines token)))
         (cond ((null? matches)
              (reply-command-error msg mediated (string-append "No visible occupant, thing, or exit: " token)))
             ((null? (cdr matches))
              (reply-command-ok msg mediated (string-append token " = " (canonical-actor (car matches)))))
             (else
              (reply-command-ok msg mediated (string-append "Ambiguous name: " token "\n" (entry-lines lines))))))))))

(define (handle-owner-query! msg args)
  (let ((mediated #f)
        (owner-args args))
    (if (null? owner-args)
        (let ((current-owner (owner)))
          (if current-owner
              (reply-command-ok msg mediated (string-append "Owner: " current-owner))
              (reply-command-ok msg mediated "This room is unowned.")))
        (let* ((token (join-words owner-args))
               (matches (did-matches token))
               (lines (did-match-lines token)))
          (cond ((null? matches)
                 (reply-command-error msg mediated (string-append "No visible occupant, thing, or exit: " token)))
                ((null? (cdr matches))
                 (begin
                   (ma-send! (canonical-actor (car matches)) (list :owner? (canonical-actor (msg-from msg)) token))
                   (reply-ok msg)))
                (else
                 (reply-command-ok msg mediated (string-append "Ambiguous name: " token "\n" (entry-lines lines)))))))))

(define (reconcile-caller-occupant! actor)
  (member-entry? actor (occupants)))

; Exit state and traversal helpers.
(define (exit-ctxs) (child-ctxs-by-kind "exit"))

; Rooms published before child records were consolidated stored an exit map.
; Preserve those exits by deriving equivalent child ctxs during the next load.
(define (exit-target-key direction) (string-append "exit-target:" direction))
(define (exit-target-name-key direction) (string-append "exit-target-name:" direction))

(define (legacy-exits)
  (let ((value (get-prop "exits")))
    (if (map? value) value (make-map))))

(define (legacy-exit-ctx direction actor)
  (make-map "actor" (canonical-actor actor)
            "parent" (canonical-actor (self))
            "kind" "exit"
            "protocol" EXIT_KIND
            "name" direction
            "nick" direction
            "description" direction
            "direction" direction
            "target-room" (get-prop (exit-target-key direction))
            "target-name" (get-prop (exit-target-name-key direction))))

(define (migrate-legacy-exits!)
  (let loop ((directions (map-keys (legacy-exits))))
    (if (null? directions)
        #f
        (let ((direction (car directions)))
          (if (exit-ctx direction)
              #f
              (let ((actor (map-ref (legacy-exits) direction #f)))
                (if actor (remember-child! (legacy-exit-ctx direction actor)) #f)))
          (loop (cdr directions))))))

(define (exit-ctx direction)
  (let loop ((ctxs (exit-ctxs)))
    (cond ((null? ctxs) #f)
          ((equal? (ctx-text (car ctxs) "direction") direction) (car ctxs))
          (else (loop (cdr ctxs))))))

(define (exit-target direction)
  (let ((ctx (exit-ctx direction)))
    (if ctx (ctx-text ctx "actor") #f)))

(define (exit-room-target direction)
  (let ((ctx (exit-ctx direction)))
    (if ctx (ctx-text ctx "target-room") #f)))

; Paired with exit-fragment: same input + "\ndst" suffix.
(define (room-fragment direction)
  (blake3 (string-append "lambda-ma exit v2\n" (canonical-actor (self)) "\n" direction "\ndst") 8))

(define (exit-fragment direction)
  (blake3 (string-append "lambda-ma exit v2\n" (canonical-actor (self)) "\n" direction) 8))

(define (heal-local-exit! direction exit)
  (let ((target-room (exit-room-target direction)))
    (if (and (dead-local-actor? exit) target-room)
        (let ((fragment (exit-fragment direction)))
          (let ((healed (ma-create-actor EXIT_KIND #f (exit-init direction target-room) fragment)))
            (remember-child!
              (map-set (exit-ctx direction) "actor" healed))
            (ma-save-state!)
            healed))
        #f)))

(define (exit-directions)
  (let loop ((ctxs (exit-ctxs)) (directions '()))
    (if (null? ctxs)
        (reverse directions)
        (loop (cdr ctxs) (cons (ctx-text (car ctxs) "direction") directions)))))

(define (known-exit? actor)
  (let loop ((directions (exit-directions)))
    (cond ((null? directions) #f)
          ((same-actor? actor (exit-target (car directions))) #t)
          (else (loop (cdr directions))))))

(define (random-exit-direction)
  (let ((directions (exit-directions)))
    (if (null? directions)
        #f
        (list-ref-at directions (random (list-length directions))))))


(define (did-exit-ctx did)
  (make-map "did" did "parent" (canonical-actor (self))))

(define (send-did-traverse! did direction exit)
  (let* ((healed-exit (heal-local-exit! direction exit))
         (active-exit (if healed-exit healed-exit exit)))
    (ma-send! (canonical-actor active-exit) (list :traverse (did-exit-ctx did)))))

(define (exits-text)
  (let ((directions (exit-directions)))
    (if (null? directions)
        "Exits: none."
        (string-append "Exits: " (names-of directions)))))

(define (who-text)
  (let ((dids (did-occupants)))
    (if (null? dids)
        "Who: none."
        (string-append "Who: " (names-of dids)))))

      ; Room-facing text surfaces.
(define (thing-token-names)
  (let loop ((ctxs (list-append (child-ctxs-by-kind "thing")
                                (child-ctxs-by-kind "container")))
             (acc '()))
    (if (null? ctxs)
        (reverse acc)
        (loop (cdr ctxs) (cons (child-label (car ctxs)) acc)))))

(define (occupants-text)
  (let ((actors (occupants)))
    (if (null? actors)
        "Occupants: none."
        (string-append "Occupants: " (names-of actors)))))

(define (room-help-text)
  (string-append
    (room-name) " help\n"
    "  look              look around\n"
    "  exits?            list exits\n"
    "  who?              show bare DIDs here\n"
    "  occupants?        show agents here\n"
    "  things?           list known things\n"
    "  did? [kind] <name> show the DID for a visible occupant, thing, or exit\n"
    "  owner? <name>     show who owns a visible occupant, thing, or exit\n"
    "  take <thing>      move something here into your inventory\n"
    "  drop [<thing>]    drop what you hold, or a named inventory item, here\n"
    "  put <item...> in <container...> place something inside a container\n"
    "  forge <kind> named <name...> [in <target...>] create something\n"
    "  tell <target...> to <verb> [args...] ask something to perform a verb\n"
    "  recycle <thing>  remove an owned agent, thing, or container\n"
    "  where? <thing>    ask where an occupant says it is\n"
    "  say <text>        speak here\n"
    "  emote <text>      act here\n"
    "  look <exit>       inspect an exit\n"
    "  :leave           stop being shown here until you return\n"
    "  :remove <who>     owner removes an occupant by unique nick or DID\n"
    "  move              move through one available exit\n"
    "  claim             claim this room if it is unowned\n"
    "  owner [did]       show or transfer ownership\n"
    "  dig <dir> [to name] [with code] create an exit\n"
    "  fill <dir>        remove an exit\n"
    "  :exit <dir> <verb> [args] forward a command to an exit\n"
    "  :dids?            owner lists occupants, things, and exits with DIDs\n"
    "  :thing <name> [did] set/list local occupant alias\n"
    "  :behaviour /ipfs/<cid> add or replace this actor's own code\n"
    "  :prop <key> [value] set or reset room text\n"
    "Agents and things enter with :enter ctx; their own parent state is the authority.\n"
    "Commands with and without : target this place directly."))

(define (did-caller? msg)
  (and (valid-did? (msg-from msg))
  (member-string? (msg-from msg) (did-occupants))))

; Ownership and room text mutation.
(define (owner) (get-prop "owner"))
(define (owned?) (if (owner) #t #f))
(define (owner? did)
  (equal? did (owner)))

(define (valid-owner? value)
  (valid-did? value))

(define (owner-message? msg)
  (msg-from-owner? (owner) msg))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (claim-owner-did args msg)
  (let ((from (msg-from msg)))
    (if (valid-did? from) from #f)))

(define (set-room-prop! key value)
  (set-node-prop! key value))

(define (reply-to-sender msg text)
  (ma-send! (canonical-actor (msg-from msg)) (list :print text)))

(define (reply-text-ok msg text)
  (reply-ok-with msg text))

(define (reply-command-ok msg delegated text)
  (begin
    (if delegated (reply-to-sender msg text) #f)
    (reply-ok-with msg text)))

(define (reply-command-error msg delegated text)
  (begin
    (if delegated (reply-to-sender msg text) #f)
    (reply-error msg text)))

(define (reply-room-prop-ok msg delegated text)
  (reply-command-ok msg delegated text))

(define (reply-room-prop-error msg delegated text)
  (reply-command-error msg delegated text))

(define (apply-room-prop! msg key value-args delegated)
  (if (null? value-args)
      (begin
        (set-room-prop! key "")
        (reply-room-prop-ok msg delegated (string-append "Reset prop " key ".")))
      (begin
        (set-room-prop! key (join-words value-args))
        (reply-room-prop-ok msg delegated (string-append "Set prop " key ".")))))

(define (handle-room-prop! msg args)
  (let ((mediated #f)
        (prop-args args))
    (cond ((null? args)
           (reply-room-prop-error msg mediated "Usage: prop <key> [value]"))
          ((null? prop-args)
           (reply-room-prop-error msg mediated "Usage: prop <key> [value]"))
          ((equal? (car prop-args) "")
           (reply-room-prop-error msg mediated "Prop key must be non-empty."))
          ((not (owned?))
           (reply-room-prop-error msg mediated "This room is unowned. Claim it before building here."))
          ((not (valid-owner? (owner)))
           (reply-room-prop-error msg mediated "Owner must be a DID."))
          ((not (owner-message? msg))
           (reply-room-prop-error msg mediated "Only this room's owner can set props here."))
          (else
           (apply-room-prop! msg (car prop-args) (cdr prop-args) mediated)))))

; DID-context helpers for movement and parent-authority flows.
(define (delegated-did-arg? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (local-actor-caller? msg)
  (local-actor-ref? (msg-from msg)))

(define (delegated-call? args msg)
  (and (delegated-did-arg? args)
      (or (member-entry? (msg-from msg) (occupants))
           (local-actor-caller? msg))))

(define (caller-did args msg)
  (if (delegated-call? args msg) (car args) (msg-from msg)))

(define (command-args args msg)
  (if (delegated-call? args msg) (cdr args) args))

(define (recycle-caller-did args msg)
  (if (valid-did? (msg-from msg)) (msg-from msg) #f))

(define (recycle-command-args args msg) args)

(define (go-delegated-call? args msg)
  (delegated-call? args msg))

(define (go-caller-did args msg)
  (if (valid-did? (msg-from msg)) (msg-from msg) #f))

(define (go-command-args args msg)
  (if (go-delegated-call? args msg) (cdr args) args))


(define (put-args-valid? args)
  (and (not (null? args))
       (not (null? (cdr args)))
       (equal? (car (cdr args)) "in")
       (not (null? (cdr (cdr args))))
       (or (null? (cdr (cdr (cdr args))))
           (and (map? (car (cdr (cdr (cdr args)))))
                (null? (cdr (cdr (cdr (cdr args)))))))))

(define (put-item-token args)
  (if (null? args) #f (car args)))

(define (put-container-token args)
  (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
      #f
      (car (cdr (cdr args)))))

(define (put-supplied-ctx args)
  (if (or (null? args)
          (null? (cdr args))
          (null? (cdr (cdr args)))
          (null? (cdr (cdr (cdr args)))))
      #f
      (car (cdr (cdr (cdr args))))))

(define (put-visible-error msg token)
  (reply-to-sender msg (string-append "You cannot see " token ".")))

(define (reject-foreign-delegated-go? args msg)
  (and (delegated-did-arg? args)
       (not (go-delegated-call? args msg))))

(define (require-valid-owner did msg thunk)
  (if (valid-owner? did)
      (thunk)
      (reply-error msg "Owner must be a DID.")))

; Exit building keeps its historical messages; ownership transfer uses the
; narrower helper below so :owner does not mention building exits.
(define (require-owner msg thunk)
  (cond ((not (owned?))
         (reply-error msg "This room is unowned. Claim it before building here."))
   ((not (valid-owner? (owner)))
    (reply-error msg "Owner must be a DID."))
   ((owner-message? msg) (thunk))
        (else
         (reply-error msg "Only this room's owner can build exits here."))))

(define (require-owner-transfer msg thunk)
  (cond ((not (owned?))
         (reply-to-sender msg "This room is unowned. Claim it before transferring ownership."))
   ((not (valid-owner? (owner)))
    (reply-to-sender msg "Owner must be a DID."))
   ((owner-message? msg) (thunk))
        (else
         (reply-to-sender msg "Only this room's owner can transfer ownership."))))

; Presence and room event handlers.
(define (on-event event args msg)
  (cond ((equal? event :leave-occupant)
         (let ((actor (msg-from msg)))
           (let ((ctx (claim-ctx actor)))
           (forget-child! actor)
           (ma-save-state!)
           (if (map? ctx) (broadcast-term (list :leave ctx)) #f))))
        (else #f)))


(define (house-caller? msg)
  (let ((runtime-ctx (stored-runtime-ctx)))
    (and runtime-ctx
         (same-actor? (msg-from msg) (ctx-text runtime-ctx "house")))))

(define (leave-candidate args msg)
  (cond ((house-caller? msg)
         (if (and (not (null? args))
                  (null? (cdr args))
                  (valid-did? (car args)))
             (car args)
             #f))
        ((null? args) (canonical-actor (msg-from msg)))
        (else #f)))

(define (handle-leave! msg args)
  (let ((actor (leave-candidate args msg)))
    (if actor
        (if (and (valid-did? actor) (member-string? actor (did-occupants)))
            (begin
              (let ((ctx (did-ctx actor)))
              (forget-child! actor)
              (ma-save-state!)
              (if (map? ctx) (broadcast-term (list :leave ctx)) #f))
              (reply-ok msg))
            (reply-ok msg))
        (reply-error msg "usage: :leave [bare-did]"))))

(define (remove-candidate token)
  (cond ((not (string? token)) #f)
    ((and (or (valid-did? token) (valid-did-url? token))
      (map? (child-ctx token)))
     (canonical-actor token))
    (else (unique-visible-child-ref token))))

(define (handle-remove! msg args)
  (let ((remove-args args))
    (cond ((null? remove-args)
           (reply-error msg "Usage: remove <child>"))
          ((not (owned?))
           (reply-error msg "This room is unowned. Claim it before removing children."))
          ((not (valid-owner? (owner)))
           (reply-error msg "Owner must be a DID."))
          ((not (owner-message? msg))
           (reply-error msg "Only this room's owner can remove children."))
          (else
           (let* ((target (join-words remove-args))
                  (actor (remove-candidate target)))
             (cond ((equal? actor :ambiguous)
                    (reply-error msg (string-append "Ambiguous child nick: " target ". Use a DID or DID-URL.")))
                   (actor
                 (let ((name (speaker-name actor)))
                   (let ((ctx (child-ctx actor)))
                   (forget-child! actor)
                   (ma-save-state!)
                   (broadcast-room-ctx!)
                   (if (map? ctx) (broadcast-term (list :leave ctx)) #f)
                   (ma-send! actor (list :report-parent (canonical-actor (self)))))
                   (reply-ok-with msg (string-append "Removed " name " from this room; asked it to reannounce."))))
                   (else
                    (reply-error msg (string-append "No such child: " target)))))))))

(define (broadcast-term term)
  (let loop ((xs (did-occupants)))
    (cond ((null? xs)
           #f)
          (else
           (begin
             (ma-send! (car xs) term)
             (loop (cdr xs)))))))

(define (broadcast-term-except excluded term)
  (let loop ((xs (did-occupants)))
    (cond ((null? xs)
           #f)
          ((same-actor? (car xs) excluded)
           (loop (cdr xs)))
          (else
           (begin
             (ma-send! (car xs) term)
             (loop (cdr xs)))))))

(define (event-subject-ctx actor)
  (if (valid-did? actor)
      (did-ctx actor)
      (claim-ctx actor)))

; Exit build/link state. Existing-room links handshake across both rooms;
; new-room digs wait for a child-alive callback before installing the exit.
(define (pending-link-key direction) (string-append "pending-link:" direction))
(define (pending-link-did-key direction) (string-append "pending-link-did:" direction))
(define (pending-link-requester-key direction) (string-append "pending-link-requester:" direction))

(define (pending-new-room-key direction) (string-append "pending-new-room:" direction))
(define (pending-new-room-did-key direction) (string-append "pending-new-room-did:" direction))
(define (pending-new-room-requester-key direction) (string-append "pending-new-room-requester:" direction))
(define (pending-new-room-target-name-key direction) (string-append "pending-new-room-target-name:" direction))
(define (pending-new-room-nonce-key direction) (string-append "pending-new-room-nonce:" direction))

(define (clear-pending-link! direction)
  (begin
    (del-prop! (pending-link-key direction))
    (del-prop! (pending-link-did-key direction))
    (del-prop! (pending-link-requester-key direction))))

(define (clear-pending-new-room! direction)
  (begin
    (del-prop! (pending-new-room-key direction))
    (del-prop! (pending-new-room-did-key direction))
    (del-prop! (pending-new-room-requester-key direction))
    (del-prop! (pending-new-room-target-name-key direction))
    (del-prop! (pending-new-room-nonce-key direction))))

(define (remembered-new-room-target direction target-name)
  (let ((ctx (exit-ctx direction)))
    (if (and ctx
             (or (not target-name)
                 (equal? (ctx-text ctx "target-name") target-name)))
        (ctx-text ctx "target-room")
        #f)))

(define (create-exit! direction target-room target-name)
  (let ((exit (ma-create-actor EXIT_KIND #f (exit-init direction target-room target-name) (exit-fragment direction))))
    (ma-send! exit (list :report-parent (canonical-actor (self))))
    exit))

(define (room-init name owner-did custom-init ready-init)
  (string-append
    "(set-init-prop! \"root\" \"" (root) "\")\n"
    (if name (string-append "(set-init-prop! \"name\" \"" name "\")\n") "")
    "(set-init-prop! \"owner\" \"" owner-did "\")\n"
    "(ma-save-state!)\n"
    (if custom-init custom-init "")
    (if ready-init ready-init "")))

(define (child-alive-init nonce direction)
  (string-append
    "(set-init-prop! \"child-alive-nonce\" \"" nonce "\")\n"
    "(set-init-prop! \"child-alive-direction\" \"" direction "\")\n"
    "(ma-save-state!)\n"
    "(notify-child-alive!)\n"))

(define (notify-child-alive!)
  (let ((parent (config-actor (ma-get-config-key "parent")))
        (nonce (get-prop "child-alive-nonce"))
        (direction (get-prop "child-alive-direction")))
    (if (and parent
             (non-empty-string? nonce)
             (non-empty-string? direction))
        (ma-send! parent (list :child-alive (config-actor (self)) ROOM_KIND nonce direction))
        #f)))

(define (exit-init direction target-room target-name)
  (string-append
    "(set-init-prop! \"direction\" \"" direction "\")\n"
    (if (owner) (string-append "(set-init-prop! \"owner\" \"" (owner) "\")\n") "")
    "(set-init-prop! \"parent\" \"" (canonical-actor (self)) "\")\n"
    "(set-init-prop! \"target-room\" \"" (canonical-actor target-room) "\")\n"
    (if target-name (string-append "(set-init-prop! \"target-name\" \"" target-name "\")\n") "")
    "(ma-save-state!)\n"))

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
  (if (and target (valid-did-url? target)) target #f))

(define (request-link-authorisation! requester did direction target-room)
  (begin
    (ma-send! (canonical-actor target-room) (list :authorise-link did direction (canonical-actor requester)))
    (ma-send! (canonical-actor requester) (list :print (string-append "Checking ownership of " target-room ".")))))

(define (request-existing-link! msg did direction target-room)
  (let ((requester (canonical-actor (msg-from msg))))
    (set-prop! (pending-link-key direction) (canonical-actor target-room))
    (set-prop! (pending-link-did-key direction) did)
    (set-prop! (pending-link-requester-key direction) requester)
    (ma-save-state!)
    (ma-send! (canonical-actor target-room) (list :ping did direction requester))
    (reply-to-sender msg (string-append "Checking reachability of " target-room "."))
    (reply-ok msg)))

(define (pending-link-matches? direction did target-room requester)
  (and (same-actor? (get-prop (pending-link-key direction)) target-room)
       (equal? (get-prop (pending-link-did-key direction)) did)
       (same-actor? (get-prop (pending-link-requester-key direction)) requester)))

(define (pending-new-room-nonce direction requester did target-name)
  (blake3 (string-append "lambda-ma pending room v1\n"
                         (canonical-actor (self)) "\n"
                         direction "\n"
                         (canonical-actor requester) "\n"
                         did "\n"
                         (if target-name target-name "")) 16))

(define (remember-pending-new-room! direction target-room requester did target-name nonce)
  (begin
    (set-prop! (pending-new-room-key direction) (canonical-actor target-room))
    (set-prop! (pending-new-room-did-key direction) did)
    (set-prop! (pending-new-room-requester-key direction) (canonical-actor requester))
    (if target-name
        (set-prop! (pending-new-room-target-name-key direction) target-name)
        (del-prop! (pending-new-room-target-name-key direction)))
    (set-prop! (pending-new-room-nonce-key direction) nonce)
    (ma-save-state!)))

(define (pending-new-room-matches? direction nonce target-room)
  (and (same-actor? (get-prop (pending-new-room-key direction)) target-room)
       (equal? (get-prop (pending-new-room-nonce-key direction)) nonce)))

; New-room readiness callback. The child room proves it is the expected actor
; before the source room installs an exit to it.
(define (handle-child-alive! msg args)
  (if (or (null? args)
          (null? (cdr args))
          (null? (cdr (cdr args)))
          (null? (cdr (cdr (cdr args)))))
      #f
      (let* ((actor (car args))
             (kind (car (cdr args)))
             (nonce (car (cdr (cdr args))))
             (direction (car (cdr (cdr (cdr args)))))
             (target-room (msg-from msg)))
        (if (and (equal? kind ROOM_KIND)
                 (same-actor? actor target-room)
                 (pending-new-room-matches? direction nonce target-room))
            (let ((requester (get-prop (pending-new-room-requester-key direction)))
                  (did (get-prop (pending-new-room-did-key direction)))
                  (target-name (get-prop (pending-new-room-target-name-key direction))))
              (begin
                (create-exit! direction target-room target-name)
                (clear-pending-new-room! direction)
                (ma-save-state!)
                (let ((ctx (did-ctx did)))
                  (if (map? ctx) (broadcast-term (list :dig ctx direction)) #f))
                (enter-dig-target! requester did target-room)))
            #f))))

(define (delivery-failed-ping? term)
  (and (pair? term)
       (equal? (car term) :ping)
       (pair? (cdr term))
       (pair? (cdr (cdr term)))
       (pair? (cdr (cdr (cdr term))))))

(define (ping-did term) (car (cdr term)))
(define (ping-direction term) (car (cdr (cdr term))))
(define (ping-requester term) (car (cdr (cdr (cdr term)))))

; After a successful dig, move the requester into the target room.
; Full bare DIDs from claim-ctx are used throughout — msg-from of ma-send! is
; always a DID-URL so the bare-DID `:enter` path is unreachable from here.
(define (enter-dig-target! requester did target-room)
  (if (member-entry? requester (occupants))
    (let* ((ctx (claim-ctx requester))
           (entry-ctx (if (map? ctx)
                          (map-set ctx "parent" (canonical-actor target-room))
                          #f)))
      (when entry-ctx
        (let ((leave-ctx ctx))
          (forget-child! requester)
          (ma-save-state!)
          (when (map? leave-ctx) (broadcast-term (list :leave leave-ctx)))
          (broadcast-room-ctx!))
        (ma-send! (canonical-actor target-room) (list :enter entry-ctx))))
    #f))

; Start a new-room dig and persist pending state until the child-alive callback
; arrives from the freshly created room.
(define (request-new-room! msg did direction target custom-init custom-behaviour)
  (let* ((target-fragment (room-fragment direction))
         (requester (canonical-actor (msg-from msg)))
         (nonce (pending-new-room-nonce direction requester did target))
         (target-room (ma-create-actor ROOM_KIND
                                       custom-behaviour
                                       (room-init target did custom-init (child-alive-init nonce direction))
                                       target-fragment)))
    (remember-pending-new-room! direction target-room requester did target nonce)
    (reply-to-sender msg (string-append "Digging " direction "..."))
    (reply-ok msg)))

; ── Presence and presentation methods ─────────────────────────────────────

(set-internal-rpc-method! :leave-occupant
  (lambda (args msg)
    (if (member-entry? (msg-from msg) (occupants))
        (on-event :leave-occupant args msg)
        #f)))

(set-cmd-method! :leave
  (lambda (args msg)
    (handle-leave! msg args)))

(set-cmd-method! :remove
  (lambda (args msg)
    (handle-remove! msg args)))

(set-cmd-method! :look
  (lambda (args msg)
    (let ((look-args (command-args args msg)))
      (if (null? look-args)
          (reply-ok-with msg (room-ctx))
          (reply-error msg "look does not accept arguments")))))

(unset-method! :name)
(unset-method! :description)

(set-cmd-method! :exits?
  (lambda (args msg)
    (reply-ok-with msg (exit-entry-list (exit-directions)))))

(set-cmd-method! :who?
  (lambda (args msg)
    (reply-ok-with msg (did-entry-map (did-occupants)))))

(set-cmd-method! :occupants?
  (lambda (args msg)
    (reply-ok-with msg (agent-entry-list (occupants)))))

(set-cmd-method! :things?
  (lambda (args msg)
    (reply-ok-with msg (thing-entry-list (thing-token-names)))))

(set-rpc-method! :dids?
  (lambda (args msg)
    (handle-dids! msg args)))

(set-rpc-method! :did?
  (lambda (args msg)
    (handle-did! msg args)))

(set-rpc-method! :owner?
  (lambda (args msg)
    (handle-owner-query! msg args)))

; ── Room-local occupant commands ──────────────────────────────────────────

(set-cmd-method! :thing
  (lambda (args msg)
    (let ((thing-args args))
      (cond ((null? thing-args)
           (reply-ok-with msg (things-text)))
            ((null? (cdr thing-args))
             (let ((token (car thing-args))
                   (did (thing-ref (car thing-args))))
               (if did
               (reply-ok-with msg did)
                   (reply-error msg (string-append "Unknown thing alias: " token)))))
            ((not (owner-message? msg))
             (reply-error msg "Only this room's owner can change thing aliases."))
            ((equal? (car (cdr thing-args)) "")
             (begin
               (remove-thing! (car thing-args))
               (ma-save-state!)
               (broadcast-room-ctx!)
               (reply-ok-with msg "thing alias removed")))
            (else
             (begin
               (set-thing! (car thing-args) (car (cdr thing-args)))
               (ma-save-state!)
               (broadcast-room-ctx!)
               (reply-ok-with msg "thing alias set")))))))

(set-cmd-method! :recycle
  (lambda (args msg)
    (let* ((mediated #f)
           (did (recycle-caller-did args msg))
           (recycle-args (recycle-command-args args msg))
           (token (if (null? recycle-args) #f (car recycle-args)))
           (actor (if token (if (valid-did-url? token) token (movable-ref token)) #f)))
      (cond ((not did)
             (reply-command-error msg mediated "Could not determine caller DID for recycle."))
            ((not token)
             (reply-command-error msg mediated "Usage: recycle <agent-or-thing-or-container>"))
            (actor
             (begin
               (ma-send! (canonical-actor actor) (list :recycle did))
               (reply-command-ok msg mediated (string-append "Recycle requested for " token "."))))
            (else
             (reply-command-error msg mediated (string-append "Unknown agent, thing, or container: " token)))))))

(set-cmd-method! :put
  (lambda (args msg)
    (let* ((did (caller-did args msg))
           (put-args (command-args args msg))
           (item-token (put-item-token put-args))
           (container-token (put-container-token put-args))
           (supplied-ctx (put-supplied-ctx put-args))
           (item-actor (if supplied-ctx
                           (ctx-text supplied-ctx "actor")
                           (if item-token (movable-ref item-token) #f)))
           (container-actor (if container-token (movable-ref container-token) #f))
           (item-ctx (if supplied-ctx supplied-ctx (if item-actor (claim-ctx item-actor) #f))))
      (cond ((not (put-args-valid? put-args))
             (reply-to-sender msg "Usage: put <agent-or-thing> in <container>"))
            ((not item-actor)
              (put-visible-error msg item-token))
            ((not container-actor)
              (put-visible-error msg container-token))
            ((same-actor? item-actor container-actor)
             (reply-to-sender msg "You cannot put something inside itself."))
            ((not (child-ctx-valid? item-ctx))
             (reply-to-sender msg (string-append "Missing details for agent or thing: " item-token)))
            (else
             (begin
               (ma-send! (canonical-actor item-actor) (list :set-parent did (canonical-actor container-actor) item-ctx))
               (reply-to-sender msg (string-append "You try to put " item-token " in " container-token "."))))))))

; :drop is a capacity pre-check only, sent by the avatar to the room before
; the held item's own (unchanged) :set-parent - it never itself relocates
; anything or changes room state.
(set-rpc-method! :drop
  (lambda (args msg)
    (cond ((not (null? args))
           (reply-error msg "usage: :drop"))
          ((>= (node-children-count) (node-max-children))
           (reply-error msg "drop refused: room is full"))
          (else
           (reply-ok msg)))))

(set-cmd-method! :where?
  (lambda (args msg)
    (let* ((where-args (command-args args msg))
           (token (if (null? where-args) #f (car where-args)))
           (actor (if token (movable-ref token) #f)))
      (cond ((not token)
             (reply-to-sender msg "Usage: where? <agent-or-thing>"))
            (actor
             (ma-send! (canonical-actor actor) (list :where?)))
            (else
             (reply-to-sender msg (string-append "Unknown agent or thing: " token)))))))

(define (normalise-exit-verb verb)
  (cond ((symbol? verb) verb)
        ((and (string? verb) (string-prefix? ":" verb)) (string->symbol verb))
        ((string? verb) (string->symbol (string-append ":" verb)))
        (else verb)))

(define (proxy-exit-command! msg direction term)
  (require-valid-owner (owner) msg
    (lambda ()
      (require-owner msg
        (lambda ()
          (let ((exit (exit-target direction)))
            (if exit
                (begin
                  (ma-send! (canonical-actor exit) term)
                  (reply-to-sender msg "Exit command queued."))
                (reply-to-sender msg (string-append "No exit " direction ".")))))))))

(set-rpc-method! :exit
  (lambda (args msg)
    (let ((exit-args (command-args args msg)))
      (if (or (null? exit-args) (null? (cdr exit-args)))
          (reply-to-sender msg "Usage: exit <direction> <verb> [args]")
          (let ((direction (car exit-args))
                (verb (normalise-exit-verb (car (cdr exit-args))))
                (verb-args (cdr (cdr exit-args))))
            (proxy-exit-command! msg direction (cons verb verb-args)))))))

(set-cmd-method! :help
  (lambda (args msg)
    (let ((text (room-help-text)))
      (reply-ok-with msg text))))

(set-cmd-method! :say
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args))
          (ctx (event-subject-ctx (msg-from msg))))
      (if (map? ctx)
          (begin
            (broadcast-term (list :say ctx text))
            (reply-ok msg))
          (reply-error msg "speaker has no event ctx")))))

(set-cmd-method! :emote
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args))
          (ctx (event-subject-ctx (msg-from msg))))
      (if (map? ctx)
          (begin
            (broadcast-term (list :emote ctx text))
            (reply-ok msg))
          (reply-error msg "speaker has no event ctx")))))

; ── Ownership and room mutation methods ───────────────────────────────────

(set-cmd-method! :claim
  (lambda (args msg)
    (let ((did (claim-owner-did args msg)))
      (if (valid-owner? did)
          (if (owned?)
              (reply-user-ok msg did (string-append "This room is already owned by " (owner) "."))
              (begin
                (set-owner! did)
                (reply-user-ok msg did (string-append "You now own " (room-name) "."))))
          (reply-user-error msg did "Owner must be a DID.")))))

(set-rpc-method! :owner
  (lambda (args msg)
    (let ((owner-args args)
          (mediated #f))
      (cond ((null? owner-args)
             (let ((current-owner (owner)))
               (if current-owner
                   (reply-command-ok msg mediated current-owner)
                   (reply-command-ok msg mediated "(none)"))))
            ((not (owned?))
             (reply-command-error msg mediated "This room is unowned. Claim it before transferring ownership."))
            ((not (valid-owner? (owner)))
             (reply-command-error msg mediated "Owner must be a DID."))
            ((not (owner-message? msg))
             (reply-command-error msg mediated "Only this room's owner can transfer ownership."))
            ((not (valid-owner? (car owner-args)))
             (reply-command-error msg mediated "New owner must be a DID."))
            (else
             (let ((new-owner (car owner-args)))
               (set-owner! new-owner)
               (reply-command-ok msg mediated (string-append "Owner set to " new-owner "."))))))))

(set-rpc-method! :prop
  (lambda (args msg)
    (handle-room-prop! msg args)))

; ── Link handshake callbacks ──────────────────────────────────────────────

(set-internal-rpc-method! :ping
  (lambda (args msg)
    (ma-send! (canonical-actor (msg-from msg)) (cons :pong args))))

(set-internal-rpc-method! :pong
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((did (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (target-room (msg-from msg)))
          (if (pending-link-matches? direction did target-room requester)
              (request-link-authorisation! requester did direction target-room)
              #f)))))

(set-internal-rpc-method! :authorise-link
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((did (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (source-room (msg-from msg)))
          (if (owner? did)
              (ma-send! (canonical-actor source-room) (list :link-authorised did direction (canonical-actor requester)))
              (ma-send! (canonical-actor source-room) (list :link-denied did direction (canonical-actor requester) "You must own both rooms to link them.")))))))

(set-internal-rpc-method! :link-denied
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))) (null? (cdr (cdr (cdr args)))))
        #f
        (let ((did (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (reason (car (cdr (cdr (cdr args)))))
              (target-room (msg-from msg)))
          (if (pending-link-matches? direction did target-room requester)
              (begin
                (clear-pending-link! direction)
                (ma-save-state!)
                (ma-send! (canonical-actor requester) (list :print reason)))
              #f)))))

(set-internal-rpc-method! :link-authorised
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((did (car args))
              (direction (car (cdr args)))
              (requester (car (cdr (cdr args))))
              (target-room (msg-from msg)))
          (if (pending-link-matches? direction did target-room requester)
              (cond ((not (owner? did))
                     (begin
                       (clear-pending-link! direction)
                       (ma-save-state!)
                       (ma-send! (canonical-actor requester) (list :print "You no longer own this room."))))
                    (else
                     (begin
                       (create-exit! direction target-room #f)
                       (clear-pending-link! direction)
                       (ma-save-state!)
                       (let ((ctx (did-ctx did)))
                         (if (map? ctx) (broadcast-term (list :dig ctx direction)) #f))
                       (enter-dig-target! requester did target-room))))
              #f)))))

(set-internal-rpc-method! :delivery-failed
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args))))
        #f
        (let ((target-room (car args))
              (reason (car (cdr args)))
              (term (car (cdr (cdr args)))))
          (if (and (delivery-failed-ping? term)
                   (pending-link-matches? (ping-direction term) (ping-did term) target-room (ping-requester term)))
              (begin
                (clear-pending-link! (ping-direction term))
                (ma-save-state!)
                (ma-send! (canonical-actor (ping-requester term))
                          (list :print (string-append "Could not reach " target-room ": " reason))))
              #f)))))

(set-internal-rpc-method! :child-alive
  (lambda (args msg)
    (handle-child-alive! msg args)))

; ── Building, movement, and direct entry methods ──────────────────────────

(set-cmd-method! :dig
  (lambda (args msg)
    (let* ((did (owner))
           (dig-args args)
           (direction (if (null? dig-args) "out" (car dig-args))))
      (require-valid-owner did msg
        (lambda ()
          (require-owner msg
            (lambda ()
              (let* ((target (dig-target-text dig-args))
                     (custom-init (dig-custom-init-text dig-args))
                     (custom-behaviour (dig-custom-behaviour-ref dig-args))
                     (existing-room (existing-room-target target))
                     (remembered-room (if (or existing-room custom-init custom-behaviour)
                                          #f
                                          (remembered-new-room-target direction target))))
                (cond ((and existing-room (or custom-init custom-behaviour))
                       (reply-error msg "Custom room code only applies when digging a new room."))
                      (existing-room
                       (request-existing-link! msg did direction existing-room))
                      (remembered-room
                       (begin
                         (reply-to-sender msg (string-append "Exit " direction " already leads to " target "."))
                         (enter-dig-target! (msg-from msg) did remembered-room)
                         (reply-ok msg)))
                      (else
                       (request-new-room! msg did direction target custom-init custom-behaviour)))))))))))

(set-cmd-method! :fill
  (lambda (args msg)
    (let* ((did (owner))
           (fill-args args))
      (if (null? fill-args)
          (reply-error msg "Usage: fill <direction>")
          (require-valid-owner did msg
            (lambda ()
              (require-owner msg
                (lambda ()
                  (let* ((direction (car fill-args))
                         (exit (exit-target direction)))
                    (if exit
                        (begin
                          (ma-send! (canonical-actor exit) (list :fill))
                          (forget-child! exit)
                          (ma-save-state!)
                          (broadcast-room-ctx!)
                          (let ((ctx (did-ctx did)))
                            (if (map? ctx) (broadcast-term (list :fill ctx direction)) #f))
                          (reply-ok msg))
                        (reply-error msg (string-append "No exit " direction "."))))))))))))

(set-cmd-method! :move
  (lambda (args msg)
    (if (reject-foreign-delegated-go? args msg)
      (reply-ok msg)
        (let* ((did (go-caller-did args msg))
               (direction (random-exit-direction)))
          (if (and did direction)
              (begin
                (send-did-traverse! did direction (exit-target direction))
                (reply-ok msg))
              (begin
                (reply-error msg "No exits or direct bare DID." )
                (reply-ok msg)))))))

(set-meta-method! :child
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :child <ctx>"))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :child <ctx>"))
          ((same-actor? (ctx-text (car args) "actor") (self))
           (handle-node-child args msg))
          (else
           (handle-child-announcement! msg (car args))))))

(set-meta-method! :parent
  (lambda (args msg)
    (cond ((null? args)
          (reply-ok-with msg (room-ctx-parent)))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :parent [ctx]"))
          ((node-root-orphan-ctx? (car args) msg)
           (handle-node-parent args msg))
          (else
           (handle-child-announcement! msg (car args))))))

(set-cmd-method! :orphan handle-node-orphan!)

(set-cmd-method! :enter
  (lambda (args msg)
    (if (valid-did? (msg-from msg))
        (commit-did-entry! msg args)
        (if (enter-ctx-args? args)
            (handle-enter-ctx! msg (car args))
            (reply-error msg "room entry requires a controlling DID")))))

; Lifecycle signals from the runtime.
(define (on-signal term)
  (cond ((or (equal? (verb-of term) :init)
             (equal? (verb-of term) :start))
         (begin
           (migrate-legacy-exits!)
           (ma-save-state!)
           (propose-node-parent! (room-ctx-parent))))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
