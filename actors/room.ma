; Locked room actor.
; Rooms own exits and local room policy. Avatars act through their current room.

; Kinds and timing constants.
(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")
(define PRESENCE_INTERVAL "30s")
(define PRESENCE_TIMEOUT_TICKS 10)

; Presence caches. `occupants` contains every visible local actor, while
; `avatar-occupants` is the person/avatar subset used by who?.
(define (occupants)
  (let ((xs (get-prop "occupants")))
    (if xs (unique-actor-entries xs) '())))

(define (add-occupant! actor)
  (if (member-entry? actor (occupants))
      #f
      (begin
        (set-prop! "occupants" (cons (canonical-actor actor) (occupants)))
        #t)))

(define (remove-occupant! actor)
  (set-prop! "occupants" (without-entries (occupants) (list actor))))

(define (add-avatar-presence! avatar)
  (let ((occupant-added (add-occupant! avatar))
        (avatar-added (add-avatar-occupant! avatar)))
    (or occupant-added avatar-added)))

(define (label-key actor) (string-append "label:" (canonical-actor actor)))

(define (avatar-did-key actor) (string-append "avatar-did:" (canonical-actor actor)))

(define (set-label! actor label)
  (if (non-empty-string? label)
      (set-prop! (label-key actor) label)
      #f))

(define (set-avatar-did! avatar did)
  (if (valid-did? did)
      (set-prop! (avatar-did-key avatar) did)
      #f))

(define (avatar-did avatar)
  (let ((did (get-prop (avatar-did-key avatar))))
    (if (valid-did? did) did #f)))

(define (has-label? actor)
  (let ((label (get-prop (label-key actor))))
    (non-empty-string? label)))

(define (avatar-occupants)
  (let ((xs (get-prop "avatar-occupants")))
    (if xs (unique-actor-entries xs) '())))

(define (add-avatar-occupant! avatar)
  (if (member-entry? avatar (avatar-occupants))
      #f
      (begin
        (set-prop! "avatar-occupants" (cons (canonical-actor avatar) (avatar-occupants)))
        #t)))

(define (remove-avatar-occupant! avatar)
  (set-prop! "avatar-occupants" (without-entries (avatar-occupants) (list avatar))))

; Presence liveness. Rooms periodically challenge occupants; stale local actors
; are removed before they produce repeated delivery failures.
(define (presence-tick)
  (let ((value (get-prop "presence:tick")))
    (if (number? value) value 0)))

(define (presence-last-report-key actor)
  (string-append "presence:last-report:" (canonical-actor actor)))

(define (presence-last-request-key actor)
  (string-append "presence:last-request:" (canonical-actor actor)))

(define (presence-nonce-key actor)
  (string-append "presence:nonce:" (canonical-actor actor)))

(define (presence-parent-key actor)
  (string-append "presence:last-parent:" (canonical-actor actor)))

(define (presence-last-report actor)
  (let ((value (get-prop (presence-last-report-key actor))))
    (if (number? value) value 0)))

(define (presence-touch! actor tick)
  (begin
    (set-prop! (presence-last-report-key actor) tick)
    (del-prop! (presence-nonce-key actor))))

(define (presence-request! actor tick nonce)
  (begin
    (set-prop! (presence-last-request-key actor) tick)
    (set-prop! (presence-nonce-key actor) nonce)
    (ma-send! (canonical-actor actor) (list :report-parent (canonical-actor (self)) tick nonce))))

(define (presence-nonce-for actor tick)
  (blake3 (string-append "presence" (canonical-actor (self)) (canonical-actor actor) (number->string tick)) 8))

(define (presence-remove! actor)
  (begin
    (remove-occupant! actor)
    (remove-avatar-occupant! actor)
    (del-prop! (presence-last-report-key actor))
    (del-prop! (presence-last-request-key actor))
    (del-prop! (presence-nonce-key actor))
    (del-prop! (presence-parent-key actor))))

(define (presence-timed-out? actor tick)
  (> (- tick (presence-last-report actor)) PRESENCE_TIMEOUT_TICKS))

(define (presence-nonce actor)
  (let ((value (get-prop (presence-nonce-key actor))))
    (if value value "")))

(define (presence-report-valid? actor tick nonce)
  (and (member-entry? actor (occupants))
       (equal? nonce (presence-nonce actor))))

(define (next-presence-tick!)
  (let ((tick (+ 1 (presence-tick))))
    (set-prop! "presence:tick" tick)
    tick))

(define (schedule-presence!)
  (let ((key "schedule:presence:started-at"))
    (if (scheduled-this-runtime? key)
        #f
        (begin
          (mark-scheduled! key)
          (ma-send! (entity-url "scheduler") (list "presence" :interval PRESENCE_INTERVAL :presence-tick))))))

; Presentation helpers.
(define (speaker-name actor)
  (let ((label (get-prop (label-key actor))))
    (if (non-empty-string? label) label actor)))

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

(define (take-carrier-parent take-args avatar)
  (let ((candidate (if (or (null? take-args) (null? (cdr take-args))) #f (car (cdr take-args)))))
    (if (and (non-empty-string? candidate)
             (or (valid-did-url? candidate) (local-actor-ref? candidate)))
        (canonical-actor candidate)
        (canonical-actor avatar))))

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

(define (presentation-entry kind protocol actor name nick description)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set (make-map) "actor" (canonical-actor actor))
            "kind" kind)
          "protocol" protocol)
        "name" name)
      "nick" nick)
    "description" description))

(define (avatar-entry-ctx actor)
  (let ((label (speaker-name actor)))
    (presentation-entry "avatar" AVATAR_KIND actor label label "An avatar.")))

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
  (let ((actor (exit-target direction)))
    (map-set
      (presentation-entry "exit" EXIT_KIND actor direction direction direction)
      "direction" direction)))

(define (avatar-entry-list actors)
  (let loop ((xs actors)
             (acc '()))
    (if (null? xs)
        (reverse acc)
        (loop (cdr xs) (cons (avatar-entry-ctx (car xs)) acc)))))

(define (agent-entry-list actors)
  (let loop ((xs actors)
             (acc '()))
    (cond ((null? xs) (reverse acc))
          ((member-entry? (car xs) (avatar-occupants))
           (loop (cdr xs) acc))
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

(define (room-ctx)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set
                (map-set
                  (map-set
                    (map-set
                      (map-set
                        (map-set
                          (make-map)
                          "protocol" ROOM_KIND)
                        "kind" "room")
                      "actor" (canonical-actor (self)))
                    "parent" (room-ctx-parent))
                  "rev" (room-ctx-rev))
                "name" (room-name))
              "nick" (room-name))
            "description" (room-description))
          "who" (avatar-entry-list (avatar-occupants)))
        "agents" (agent-entry-list (occupants)))
      "things" (thing-entry-list (thing-token-names)))
    "exits" (exit-entry-list (exit-directions))))

(define (send-room-ctx-to! avatar ctx)
  (ma-send! (canonical-actor avatar) (list :ctx ctx)))

(define (send-room-ctx-list! avatars ctx)
  (if (null? avatars)
      #f
      (begin
        (send-room-ctx-to! (car avatars) ctx)
        (send-room-ctx-list! (cdr avatars) ctx))))

(define (broadcast-room-ctx!)
  (begin
    (inc-prop! "ctx:rev" 1)
    (ma-save-state!)
    (send-room-ctx-list! (avatar-occupants) (room-ctx))))

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

; Claim ctx records agent/thing room-local metadata after direct :enter ctx.
(define (claim-key actor)
  (string-append "claim:" (canonical-actor actor)))

(define (set-claim! actor ctx)
  (set-prop! (claim-key actor) ctx)
  (ma-save-state!))

(define (claim-ctx actor)
  (let ((ctx (get-prop (claim-key actor))))
    (if (map? ctx) ctx #f)))

; Entry argument helpers. Historical avatar entry forms are still accepted, but
; new direct entry uses the ctx map path below.
(define (entry-old-room args)
  (empty-string->false (arg-at-or-false args 1)))

(define (entry-nick args)
  (arg-at-or-false args 2))

(define (entry-did args)
  (arg-at-or-false args 0))

(define (entry-avatar args)
  (arg-at-or-false args 1))

(define (entry-did-old-room args)
  (arg-at-or-false args 2))

(define (entry-did-nick args)
  (arg-at-or-false args 3))

(define (entry-did-inventory args)
  (arg-at-or-false args 4))

(define (entry-avatar-did args)
  (arg-at-or-false args 4))

(define (entry-inventory args)
  (arg-at-or-false args 3))

(define (named-room-fragment direction target-name)
  (blake3 (string-append "lambda-ma room v1\n" (canonical-actor (self)) "\n" direction "\n" target-name) 8))

(define (exit-fragment direction)
  (blake3 (string-append "lambda-ma exit v1\n" (canonical-actor (self)) "\n" direction) 8))

; Avatar creation is asynchronous: init asks this room to admit the avatar, and
; the room later sends committed ctx back to the avatar.
(define (avatar-init did nick room)
  (let ((n (nick-or-default nick))
        (r (root))
        (avatar (avatar-for-did did))
        (target-room (canonical-actor room)))
    (string-append
      "(set-prop! \"did\" \"" did "\")\n"
      "(set-prop! \"root\" \"" r "\")\n"
      "(set-prop! \"nick\" \"" n "\")\n"
      "(ma-save-state!)\n"
      "(ma-send! \"" target-room "\" (list :enter \"" avatar "\" #f \"" n "\"))\n")))

(define (avatar-init-with-inventory did nick room inventory)
  (let ((n (nick-or-default nick))
        (r (root))
        (avatar (avatar-for-did did))
        (target-room (canonical-actor room))
        (inv (if (valid-did-url? inventory) (canonical-actor inventory) "")))
    (string-append
      "(set-prop! \"did\" \"" did "\")\n"
      "(set-prop! \"root\" \"" r "\")\n"
      "(set-prop! \"nick\" \"" n "\")\n"
      "(set-prop! \"inventory\" \"" inv "\")\n"
      "(ma-save-state!)\n"
      "(ma-send! \"" target-room "\" (list :enter \"" avatar "\" #f \"" n "\" \"" inv "\"))\n")))

(define (ensure-avatar! did nick)
  (let* ((avatar (avatar-for-did did))
         (n (nick-or-default nick)))
    (set-label! avatar n)
    (if (add-avatar-presence! avatar)
        (begin
          (presence-touch! avatar (presence-tick))
          (broadcast-except avatar (string-append (speaker-name avatar) " arrives.")))
        #f)
    (ma-save-state!)
    (if (entity-live? avatar)
        avatar
      (entity-url (ma-create-actor AVATAR_KIND #f (avatar-init did n (canonical-actor (self))) (avatar-fragment did))))))

; Entry ctx builders and validators. Committed ctx actor refs must be full
; DID-URLs, never runtime-local #fragment shorthand.
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
          (list :avatar "")
          (list :nick (if nick nick ""))
          (list :room (canonical-actor (self)))
          (list :text text))))

(define (avatar-room-ctx avatar nick text)
  (list :ctx
    (list (list :kind "avatar")
          (list :root (canonical-actor (root)))
          (list :avatar (canonical-actor avatar))
          (list :nick (nick-or-default nick))
          (list :room (canonical-actor (self)))
          (list :text text))))

(define (avatar-room-ctx-with-inventory avatar nick text inventory)
  (list :ctx
    (list (list :kind "avatar")
          (list :root (canonical-actor (root)))
          (list :avatar (canonical-actor avatar))
          (list :inv (if (valid-did-url? inventory) (canonical-actor inventory) ""))
          (list :nick (nick-or-default nick))
          (list :room (canonical-actor (self)))
          (list :text text))))

(define (request-avatar-entry! did nick inventory)
  (let* ((avatar (avatar-for-did did))
         (n (nick-or-default nick)))
    (if (entity-live? avatar)
      (ma-send! (canonical-actor avatar) (list :enter-room (canonical-actor (self)) did n inventory))
      (ma-create-actor AVATAR_KIND #f (avatar-init-with-inventory did n (canonical-actor (self)) inventory) (avatar-fragment did)))
    avatar))

(define (expected-avatar? did avatar)
  (same-actor? avatar (avatar-for-did did)))

; Avatar entry flows. Same-runtime expected avatars can be admitted directly;
; foreign or stale source avatars cause the target room to create/reuse its
; deterministic local avatar for the did.
(define (handle-avatar-arrival! did avatar old-room nick inventory)
  (if (and (local-actor-ref? avatar)
           (entity-live? avatar)
           (expected-avatar? did avatar))
  (ma-send! (canonical-actor avatar) (list :enter-room (canonical-actor (self)) did (nick-or-default nick) inventory))
  (request-avatar-entry! did nick inventory)))

(define (announce-avatar-presence! avatar)
  (if (add-avatar-presence! avatar)
      (broadcast-except avatar (string-append (speaker-name avatar) " arrives."))
      #f))

(define (commit-avatar-entry! avatar old-room nick inventory)
  (begin
    (set-label! avatar nick)
    (announce-avatar-presence! avatar)
    (presence-touch! avatar (presence-tick))
    (ma-save-state!)
    (ma-send! (canonical-actor avatar) (avatar-room-ctx-with-inventory avatar nick (arrival-text) inventory))
    (broadcast-room-ctx!)))

(define (record-avatar-presence! avatar old-room)
  (begin
    (announce-avatar-presence! avatar)
    (presence-touch! avatar (presence-tick))
    (ma-save-state!)
    #f))

(define (enter-empty? args)
  (null? args))

(define (direct-did-enter-args? args msg)
  (and (valid-did? (msg-from msg))
       (or (null? args)
           (and (string? (car args))
                (or (null? (cdr args))
                    (and (string? (cadr args))
                         (null? (cddr args))))))))

(define (direct-did-enter-nick args)
  (if (null? args) #f (car args)))

(define (direct-did-enter-inventory args)
  (if (or (null? args) (null? (cdr args))) #f (cadr args)))

(define (enter-ctx-args? args)
  (and (not (null? args)) (map? (car args))))

(define (self-avatar-entry? args msg)
  (and (not (null? args))
       (string? (car args))
  (local-actor-ref? (car args))
       (same-actor? (msg-from msg) (car args))))

(define (did-avatar-entry? args)
  (and (not (null? args))
  (valid-did? (car args))
       (not (null? (cdr args)))))

(define (enter-avatar-kind? kind)
  (or (not kind) (equal? kind "") (equal? kind "avatar")))

; Direct non-avatar entry is kind-driven. Agents are visible occupants; things
; are token-bound room locals whose own parent state remains authoritative.
(define (handle-enter-ctx! msg ctx)
  (let* ((actor (ctx-text ctx "actor"))
         (ctxdid (ctx-did ctx))
         (did (if (valid-did? ctxdid) ctxdid (if (valid-did? actor) actor (msg-from msg))))
         (kind (ctx-text ctx "kind"))
         (name (ctx-text ctx "name")))
    (cond
      ((and (enter-avatar-kind? kind) (ctx-sender-valid? ctx msg))
        (request-avatar-entry! did (ctx-text ctx "nick") (ctx-text ctx "inv")))
      ((enter-avatar-kind? kind)
       (reply-error msg "avatar enter ctx must come from the deterministic avatar for its DID"))
      ((and (agent-kind? kind) (enter-direct-ctx-valid? ctx "agent"))
       (handle-agent-enter! msg did ctx))
      ((agent-kind? kind)
       (reply-error msg "agent enter requires ctx map with kind, name, nick, description"))
      ((and (thing-kind? kind) (enter-direct-ctx-valid? ctx "thing"))
       (handle-thing-enter! msg did ctx name))
      ((thing-kind? kind)
       (reply-error msg "thing enter requires ctx map with kind, name, nick, description"))
      (else
       (reply-error msg "unsupported ctx kind for enter")))))

(define (handle-self-avatar-entry! args)
  (let ((avatar (car args))
        (old-room (entry-old-room args))
    (nick (entry-nick args))
    (inventory (entry-inventory args))
    (did (entry-avatar-did args)))
    (begin
      (set-avatar-did! avatar did)
      (commit-avatar-entry! avatar old-room nick inventory))))

(define (handle-did-avatar-entry! args)
  (let* ((did (entry-did args))
         (avatar (entry-avatar args))
         (old-room (entry-did-old-room args))
       (nick (entry-did-nick args))
       (inventory (entry-did-inventory args)))
    (if (not avatar)
        #f
      (handle-avatar-arrival! did avatar old-room nick inventory))))

(define (handle-legacy-avatar-entry! args)
  (let* ((avatar (car args))
         (old-room (entry-old-room args)))
    (if (local-actor-ref? avatar)
        (record-avatar-presence! avatar old-room)
        #f)))

(define (handle-agent-enter! msg did ctx)
  (let* ((actor (canonical-actor did))
         (nick (ctx-text ctx "nick"))
         (was-known (member-entry? actor (occupants)))
         (same-claim (equal? (claim-ctx actor) ctx))
         (same-label (equal? (speaker-name actor) nick)))
    (if (and was-known same-claim same-label)
        (reply-ok msg)
        (begin
          (set-claim! actor ctx)
          (set-label! actor nick)
          (if (add-occupant! actor)
              (begin
                (presence-touch! actor (presence-tick))
                (broadcast-except actor (string-append (speaker-name actor) " arrives.")))
              #f)
          (ma-save-state!)
          (ma-send! (canonical-actor actor) (list :child ctx))
          (ma-send! (canonical-actor actor) (direct-room-ctx "agent" nick (arrival-text)))
          (broadcast-room-ctx!)
          (ma-reply! msg (list :ok "entered"))))))

(define (handle-thing-enter! msg did ctx name)
  (let* ((actor (canonical-actor did))
         (label (ctx-text ctx "nick"))
         (token (if (non-empty-string? label) label name))
         (bound (thing-ref token))
         (same-claim (equal? (claim-ctx actor) ctx))
         (same-label (equal? (speaker-name actor) label)))
    (cond ((not (actor-token-valid? name))
           (reply-error msg "enter requires non-empty name token"))
          ((not (actor-token-valid? token))
           (reply-error msg "enter requires non-empty nick token"))
          ((and bound (not (same-actor? bound actor)))
           (reply-error msg "nick token is already bound to another actor"))
          ((and (same-actor? bound actor) same-claim same-label)
           (reply-ok msg))
          (else
           (set-claim! actor ctx)
           (set-label! actor label)
           (set-thing! token actor)
           (ma-send! (canonical-actor actor) (list :child ctx))
           (broadcast-room-ctx!)
           (ma-reply! msg (list :ok "entered"))))))

(define (child-parent-target ctx)
  (let ((parent (ctx-text ctx "parent")))
    (if (non-empty-string? parent)
        parent
        (ctx-text ctx "room"))))

(define (child-announcement-valid? ctx msg)
  (and (actor-ctx? ctx msg)
       (or (same-actor? (child-parent-target ctx) (self))
           (not (same-actor? (child-parent-target ctx) (self))))))

(define (handle-avatar-parent! msg ctx)
  (let* ((actor (ctx-text ctx "actor"))
         (old-name (speaker-name actor))
         (new-nick (ctx-text ctx "nick"))
         (was-present (member-entry? actor (occupants))))
    (cond ((not (and (enter-direct-ctx-valid? ctx "avatar")
                     (actor-ctx? ctx msg)))
           (reply-error msg "avatar parent ctx must include actor, parent, kind, protocol, name, nick, description and match sender"))
          ((not (same-actor? (child-parent-target ctx) (self)))
           (begin
             (remove-occupant! actor)
             (remove-avatar-occupant! actor)
             (ma-save-state!)
             (broadcast-room-ctx!)
             (reply-ok msg)))
          (else
           (begin
             (set-label! actor new-nick)
             (add-avatar-presence! actor)
             (presence-touch! actor (presence-tick))
             (ma-save-state!)
             (broadcast-room-ctx!)
             (if (and was-present
                      (non-empty-string? old-name)
                      (non-empty-string? new-nick)
                      (not (equal? old-name new-nick)))
                 (broadcast (string-append old-name " is now known as " new-nick "."))
                 #f)
             (reply-ok msg))))))

(define (handle-child-announcement! msg ctx)
  (let ((kind (ctx-text ctx "kind"))
        (actor (ctx-text ctx "actor"))
        (name (ctx-text ctx "name")))
    (cond ((not (child-announcement-valid? ctx msg))
           (reply-error msg "child ctx actor must match sender"))
          ((not (same-actor? (child-parent-target ctx) (self)))
           (begin
             (remove-occupant! actor)
             (remove-avatar-occupant! actor)
             (remove-thing! name)
             (ma-save-state!)
             (broadcast-room-ctx!)
             (reply-ok msg)))
          ((and (agent-kind? kind) (enter-direct-ctx-valid? ctx "agent"))
           (handle-agent-enter! msg actor ctx))
          ((and (or (thing-kind? kind) (container-kind? kind)) (enter-ctx-valid? ctx))
           (handle-thing-enter! msg actor ctx name))
          (else
           (reply-error msg "child ctx must include actor, parent, kind, protocol, name, nick, description")))))

(define (agent-kind? kind) (equal? kind "agent"))
(define (thing-kind? kind) (equal? kind "thing"))
(define (container-kind? kind) (equal? kind "container"))
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
  (if (string-prefix? "did:ma:" token)
      token
      (map-ref (things-map) token #f)))

(define (movable-ref token)
  (let ((thing (thing-ref token)))
    (if thing thing (occupant-ref token))))

(define (remove-movable! token actor)
  (begin
    (if (same-actor? (thing-ref token) actor)
        (remove-thing! token)
        #f)
    (remove-occupant! actor)
    (remove-avatar-occupant! actor)
    (broadcast-room-ctx!)
    (ma-save-state!)))

(define (set-thing! token did)
  (set-things-map! (map-set (things-map) token did)))

(define (remove-thing! token)
  (set-things-map! (map-delete (things-map) token)))

(define (things-text)
  (token-list-text "Things" (map-keys (things-map))))

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
  (cond ((member-entry? actor (occupants)) #f)
        ((not (has-label? actor)) #f)
        (else
         (begin
           (add-occupant! actor)
           (presence-touch! actor (presence-tick))
           (ma-save-state!)))))

; Exit state and traversal helpers.
(define (exits)
  (let ((xs (get-prop "exits")))
    (if (map? xs) xs (make-map))))

(define (put-exit! direction exit)
  (set-prop! "exits" (map-set (exits) direction exit)))

(define (remove-exit! direction)
  (begin
    (set-prop! "exits" (map-delete (exits) direction))
    (del-prop! (exit-target-key direction))
    (del-prop! (exit-target-name-key direction))))

(define (exit-target direction)
  (map-ref (exits) direction #f))

(define (exit-room-target direction)
  (get-prop (exit-target-key direction)))

(define (heal-local-exit! direction exit)
  (let ((target-room (exit-room-target direction)))
    (if (and (dead-local-actor? exit) target-room)
        (let ((fragment (exit-fragment direction)))
          (ma-create-actor EXIT_KIND #f (exit-init direction target-room) fragment)
          (let ((healed (entity-url fragment)))
            (put-exit! direction healed)
            (ma-save-state!)
            healed))
        #f)))

(define (exit-directions)
  (map-keys (exits)))

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

(define (movement-kind actor)
  (let ((claim (claim-ctx actor)))
    (cond ((map? claim) (ctx-text claim "kind"))
          (else "actor"))))

(define (avatar-exit-ctx actor did)
  (let* ((actor-ref (if did did (canonical-actor actor)))
         (base-ctx (map-set (make-map) "actor" actor-ref))
         (avatar-ctx (if did
                         (map-set
                           (map-set base-ctx "did" did)
                           "avatar" (canonical-actor actor))
                         base-ctx))
         (ctx (map-set
                (map-set avatar-ctx "kind" (if did "avatar" (movement-kind actor)))
                "room" (canonical-actor (self)))))
    (let ((with-nick (if (has-label? actor)
                         (map-set ctx "nick" (speaker-name actor))
                         ctx)))
      (if did
          (map-set with-nick "avatar" (canonical-actor actor))
          with-nick))))

(define (look-ctx msg)
  (let* ((caller (msg-from msg))
         (caller-ref (canonical-actor caller))
         (ctx (map-set
                (map-set
                  (map-set (make-map) "actor" caller-ref)
                  "kind" (if (valid-did? caller-ref) "did" "avatar"))
                "room" (canonical-actor (self)))))
    (if (valid-did? caller-ref)
        ctx
        (map-set ctx "avatar" caller-ref))))

(define (send-exit-ctx! actor did direction exit)
  (let* ((healed-exit (heal-local-exit! direction exit))
         (active-exit (if healed-exit healed-exit exit)))
    (ma-send! (canonical-actor active-exit) (list :ctx (avatar-exit-ctx actor did)))))

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

      ; Room-facing text surfaces.
(define (thing-token-names)
  (map-keys (things-map)))

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
    "  who?              show people here\n"
    "  occupants?        show all occupants (avatars, agents, DIDs)\n"
    "  things?           list known things\n"
    "  did? [kind] <name> show the DID for a visible occupant, thing, or exit\n"
    "  owner? <name>     show who owns a visible occupant, thing, or exit\n"
    "  take <thing>      ask an agent or thing to bind to you\n"
    "  drop <thing>      ask an occupant to set this room as parent\n"
    "  recycle <thing>   remove an owned agent or thing from here\n"
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
    "Commands with : hit this place directly; commands without : go through your avatar."))

(define (avatar-caller? msg)
  (member-entry? (msg-from msg) (occupants)))

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
    (cond ((valid-did? from) from)
          ((avatar-did from) (avatar-did from))
          ((and (not (null? args))
                (valid-did? (car args))
                (did-avatar? (car args) from))
           (car args))
          (else #f))))

(define (set-room-prop! key value)
  (set-prop! key value)
  (ma-save-state!))

(define (reply-to-sender msg text)
  (ma-send! (canonical-actor (msg-from msg)) (list :print text)))

(define (print-and-reply-ok msg text)
  (begin
    (reply-to-sender msg text)
    (reply-ok msg)))

(define (reply-command-ok msg delegated text)
  (if delegated
      (reply-to-sender msg text)
  (reply-ok-with msg text)))

(define (reply-command-error msg delegated text)
  (if delegated
      (reply-to-sender msg text)
      (reply-error msg text)))

(define (reply-room-prop-ok msg delegated text)
  (if delegated
      (reply-to-sender msg text)
  (reply-ok-with msg text)))

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
  (let ((mediated (avatar-caller? msg))
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

; DID-context helpers for movement and parent-authority flows. Owner checks do
; not use this shape; they compare msg-from with the owner or owner avatar.
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

(define (go-delegated-call? args msg)
  (delegated-call? args msg))

(define (go-caller-did args msg)
  (cond ((go-delegated-call? args msg) (car args))
        ((valid-did? (msg-from msg)) (msg-from msg))
        (else #f)))

(define (go-command-args args msg)
  (if (go-delegated-call? args msg) (cdr args) args))

(define (go-caller-actor args msg)
  (cond ((go-delegated-call? args msg) (canonical-actor (msg-from msg)))
        ((valid-did? (msg-from msg)) (avatar-for-did (msg-from msg)))
        (else (canonical-actor (msg-from msg)))))

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
      (reply-to-sender msg "Owner must be a DID.")))

; Exit building keeps its historical messages; ownership transfer uses the
; narrower helper below so :owner does not mention building exits.
(define (require-owner msg thunk)
  (cond ((not (owned?))
         (reply-to-sender msg "This room is unowned. Claim it before building here."))
   ((not (valid-owner? (owner)))
    (reply-to-sender msg "Owner must be a DID."))
   ((owner-message? msg) (thunk))
        (else
         (reply-to-sender msg "Only this room's owner can build exits here."))))

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
           (presence-remove! actor)
           (ma-save-state!)
           (broadcast (string-append (speaker-name actor) " leaves."))))
        (else #f)))

(define (leave-candidate msg)
  (let ((caller (canonical-actor (msg-from msg))))
    (cond ((member-entry? caller (occupants)) caller)
          ((valid-did? caller) (avatar-for-did caller))
          (else caller))))

(define (handle-leave! msg)
  (let ((actor (leave-candidate msg)))
    (if (member-entry? actor (occupants))
        (begin
          (presence-remove! actor)
          (ma-save-state!)
          (broadcast (string-append (speaker-name actor) " leaves."))
          (reply-ok msg))
        (reply-ok msg))))

(define (remove-candidate token)
  (cond ((not (string? token)) #f)
        ((and (valid-did? token) (member-entry? (avatar-for-did token) (occupants)))
         (avatar-for-did token))
        ((member-entry? token (occupants)) (canonical-actor token))
    (else (unique-visible-occupant-ref token))))

(define (handle-remove! msg args)
  (let ((remove-args args))
    (cond ((null? remove-args)
           (reply-error msg "Usage: remove <occupant>"))
          ((not (owned?))
           (reply-error msg "This room is unowned. Claim it before removing occupants."))
          ((not (valid-owner? (owner)))
           (reply-error msg "Owner must be a DID."))
          ((not (owner-message? msg))
           (reply-error msg "Only this room's owner can remove occupants."))
          (else
           (let* ((target (join-words remove-args))
                  (actor (remove-candidate target)))
             (cond ((equal? actor :ambiguous)
                    (reply-error msg (string-append "Ambiguous occupant nick: " target ". Use a DID or DID-URL.")))
                   (actor
                 (let ((name (speaker-name actor)))
                   (presence-remove! actor)
                   (ma-save-state!)
                   (broadcast (string-append name " leaves."))
                   (reply-ok-with msg (string-append "Removed " name " from this room."))))
                   (else
                    (reply-error msg (string-append "No such occupant: " target)))))))))

(define (presence-sweep! tick)
  (let loop ((xs (occupants))
             (changed #f))
    (cond ((null? xs) changed)
          ((dead-local-actor? (car xs))
           (begin
             (presence-remove! (car xs))
             (loop (cdr xs) #t)))
          ((presence-timed-out? (car xs) tick)
           (begin
             (presence-remove! (car xs))
             (loop (cdr xs) #t)))
          (else
           (begin
             (presence-request! (car xs) tick (presence-nonce-for (car xs) tick))
             (loop (cdr xs) #t))))))

(define (broadcast text)
  (let loop ((xs (occupants))
             (changed #f))
    (cond ((null? xs)
           (if changed (ma-save-state!) #f))
          ((dead-local-actor? (car xs))
           (begin
             (presence-remove! (car xs))
             (loop (cdr xs) #t)))
          (else
           (begin
             (ma-send! (canonical-actor (car xs)) (list :print text))
             (loop (cdr xs) changed))))))

(define (broadcast-except excluded text)
  (let loop ((xs (occupants))
             (changed #f))
    (cond ((null? xs)
           (if changed (ma-save-state!) #f))
          ((dead-local-actor? (car xs))
           (begin
             (presence-remove! (car xs))
             (loop (cdr xs) #t)))
          ((same-actor? (car xs) excluded)
           (loop (cdr xs) changed))
          (else
           (begin
             (ma-send! (canonical-actor (car xs)) (list :print text))
             (loop (cdr xs) changed))))))

; Exit build/link state. Existing-room links handshake across both rooms;
; new-room digs wait for a child-alive callback before installing the exit.
(define (exit-target-key direction) (string-append "exit-target:" direction))
(define (exit-target-name-key direction) (string-append "exit-target-name:" direction))

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

(define (remember-exit-target! direction target-room target-name)
  (begin
    (set-prop! (exit-target-key direction) (canonical-actor target-room))
    (if target-name
        (set-prop! (exit-target-name-key direction) target-name)
        (del-prop! (exit-target-name-key direction)))))

(define (remembered-new-room-target direction target-name)
  (if (and target-name (equal? (get-prop (exit-target-name-key direction)) target-name))
      (get-prop (exit-target-key direction))
      #f))

(define (create-exit! direction target-room target-name)
  (let* ((exit-fragment (ma-create-actor EXIT_KIND #f (exit-init direction target-room) (exit-fragment direction)))
         (exit (entity-url exit-fragment)))
    (put-exit! direction exit)
    (remember-exit-target! direction target-room target-name)
    (broadcast-room-ctx!)
    exit))

(define (room-init name owner-did custom-init ready-init)
  (string-append
    "(set-prop! \"root\" \"" (root) "\")\n"
    (if name (string-append "(set-prop! \"name\" \"" name "\")\n") "")
    "(set-prop! \"owner\" \"" owner-did "\")\n"
    "(ma-save-state!)\n"
    (if custom-init custom-init "")
    (if ready-init ready-init "")))

(define (child-alive-init nonce direction)
  (string-append
    "(set-prop! \"child-alive-nonce\" \"" nonce "\")\n"
    "(set-prop! \"child-alive-direction\" \"" direction "\")\n"
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

(define (exit-init direction target-room)
  (string-append
    "(set-prop! \"direction\" \"" direction "\")\n"
    (if (owner) (string-append "(set-prop! \"owner\" \"" (owner) "\")\n") "")
    "(set-prop! \"source-room\" \"" (canonical-actor (self)) "\")\n"
    "(set-prop! \"target-room\" \"" (canonical-actor target-room) "\")\n"
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
  (cond ((and target (local-actor-ref? target) (ma-entity-exists? (canonical-actor target)))
         (canonical-actor target))
        ((and target (valid-did-url? target)) target)
        ((and target (ma-entity-exists? target)) target)
        (else #f)))

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
    (reply-to-sender msg (string-append "Checking reachability of " target-room "."))))

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
                (broadcast (string-append did " digs " direction "."))
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

; After a successful dig, the avatar that requested it may immediately enter
; the newly linked target if it is still present in the source room.
(define (enter-dig-target! requester did target-room)
  (if (member-entry? requester (occupants))
  (ma-send! (canonical-actor target-room) (list :enter did (canonical-actor requester) (canonical-actor (self)) (speaker-name requester)))
      #f))

; Start a new-room dig and persist pending state until the child-alive callback
; arrives from the freshly created room.
(define (request-new-room! msg did direction target custom-init custom-behaviour)
  (let* ((target-fragment (if (and target (not custom-init) (not custom-behaviour))
                              (named-room-fragment direction target)
                              #f))
         (requester (canonical-actor (msg-from msg)))
         (nonce (pending-new-room-nonce direction requester did target))
         (target-room (entity-url (ma-create-actor ROOM_KIND
                                                   custom-behaviour
                                                   (room-init target did custom-init (child-alive-init nonce direction))
                                                   target-fragment))))
    (remember-pending-new-room! direction target-room requester did target nonce)
    (reply-to-sender msg (string-append "Digging " direction "..."))))

; ── Presence and presentation methods ─────────────────────────────────────

(set-internal-rpc-method! :leave-occupant
  (lambda (args msg)
    (if (member-entry? (msg-from msg) (occupants))
        (on-event :leave-occupant args msg)
        #f)))

(set-cmd-method! :leave
  (lambda (args msg)
    (handle-leave! msg)))

(set-cmd-method! :remove
  (lambda (args msg)
    (handle-remove! msg args)))

(set-cmd-method! :look
  (lambda (args msg)
    (let ((avatar (msg-from msg))
          (look-args (command-args args msg)))
      (reconcile-caller-occupant! avatar)
      (if (null? look-args)
          (print-and-reply-ok msg (room-text))
          (print-and-reply-ok msg "Use your avatar to inspect visible things.")))))

(unset-method! :name)
(unset-method! :description)

(set-cmd-method! :exits?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (print-and-reply-ok msg (exits-text)))))

(set-cmd-method! :who?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (reconcile-caller-occupant! avatar)
      (print-and-reply-ok msg (who-text)))))

(set-cmd-method! :occupants?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (reconcile-caller-occupant! avatar)
      (print-and-reply-ok msg (occupants-text)))))

(set-cmd-method! :things?
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (print-and-reply-ok msg (things-text)))))

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
               (reply-ok-with msg "thing alias removed")))
            (else
             (begin
               (set-thing! (car thing-args) (car (cdr thing-args)))
               (reply-ok-with msg "thing alias set")))))))

(set-cmd-method! :recycle
  (lambda (args msg)
    (let* ((did (caller-did args msg))
           (recycle-args (command-args args msg))
           (token (if (null? recycle-args) #f (car recycle-args)))
           (actor (if token (if (valid-did-url? token) token (movable-ref token)) #f)))
      (cond ((not token)
             (reply-to-sender msg "Usage: recycle <agent-or-thing>"))
            (actor
             (begin
               (ma-send! (canonical-actor actor) (list :recycle did))
               (reply-to-sender msg (string-append "You recycle " token "."))))
            (else
             (reply-to-sender msg (string-append "Unknown agent or thing: " token)))))))

(set-cmd-method! :put
  (lambda (args msg)
    (let* ((did (caller-did args msg))
           (avatar (msg-from msg))
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
               (ma-send! (canonical-actor item-actor) (list :drop did (canonical-actor container-actor) item-ctx))
               (reply-to-sender msg (string-append "You try to put " item-token " in " container-token "."))))))))

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
      (if (avatar-caller? msg)
          (ma-send! (canonical-actor (msg-from msg)) (list :print text))
          #f)
        (reply-ok-with msg text))))

(set-cmd-method! :say
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args)))
      (broadcast (string-append (speaker-name speaker) " says: " text)))))

(set-cmd-method! :emote
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args)))
      (broadcast (string-append (speaker-name speaker) " " text)))))

; ── Ownership and room mutation methods ───────────────────────────────────

(set-cmd-method! :claim
  (lambda (args msg)
    (let ((did (claim-owner-did args msg)))
      (if (valid-owner? did)
          (if (owned?)
              (print-and-reply-ok msg (string-append "This room is already owned by " (owner) "."))
              (begin
                (set-owner! did)
                (print-and-reply-ok msg (string-append "You now own " (room-name) "."))))
          (reply-error msg "Owner must be a DID.")))))

(set-rpc-method! :owner
  (lambda (args msg)
    (let ((owner-args args))
      (cond ((null? owner-args)
             (let ((current-owner (owner)))
               (if current-owner
                   (reply-ok-with msg current-owner)
                   (reply-ok-with msg "(none)"))))
            ((not (owned?))
             (reply-error msg "This room is unowned. Claim it before transferring ownership."))
            ((not (valid-owner? (owner)))
             (reply-error msg "Owner must be a DID."))
            ((not (owner-message? msg))
             (reply-error msg "Only this room's owner can transfer ownership."))
            ((not (valid-owner? (car owner-args)))
             (reply-error msg "New owner must be a DID."))
            (else
             (let ((new-owner (car owner-args)))
               (set-owner! new-owner)
               (reply-ok-with msg (string-append "Owner set to " new-owner "."))))))))

(set-rpc-method! :prop
  (lambda (args msg)
    (handle-room-prop! msg args)))

; ── Link handshake and scheduled presence callbacks ───────────────────────

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

(set-internal-rpc-method! :presence-tick
  (lambda (args msg)
    (let ((tick (next-presence-tick!)))
      (if (presence-sweep! tick)
          (ma-save-state!)
          #f))))

(set-internal-rpc-method! :parent-report
  (lambda (args msg)
    (if (or (null? args)
            (null? (cdr args))
            (null? (cdr (cdr args)))
            (null? (cdr (cdr (cdr args)))))
        #f
        (let ((actor (canonical-actor (car args)))
              (parent (car (cdr args)))
              (tick (car (cdr (cdr args))))
              (nonce (car (cdr (cdr (cdr args))))))
          (if (and (same-actor? actor (msg-from msg))
                   (presence-report-valid? actor tick nonce))
              (begin
                (set-prop! (presence-parent-key actor) parent)
                (if (same-actor? parent (self))
                    (presence-touch! actor tick)
                    (presence-remove! actor))
                (ma-save-state!))
              #f)))))

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
                       (broadcast (string-append did " digs " direction "."))
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
                       (reply-to-sender msg "Custom room code only applies when digging a new room."))
                      (existing-room
                       (request-existing-link! msg did direction existing-room))
                      (remembered-room
                       (begin
                         (reply-to-sender msg (string-append "Exit " direction " already leads to " target "."))
                         (enter-dig-target! (msg-from msg) did remembered-room)))
                      (else
                       (request-new-room! msg did direction target custom-init custom-behaviour)))))))))))

(set-cmd-method! :fill
  (lambda (args msg)
    (let* ((did (owner))
           (fill-args args))
      (if (null? fill-args)
          (reply-to-sender msg "Usage: fill <direction>")
          (require-valid-owner did msg
            (lambda ()
              (require-owner msg
                (lambda ()
                  (let* ((direction (car fill-args))
                         (exit (exit-target direction)))
                    (if exit
                        (begin
                          (ma-send! (canonical-actor exit) (list :fill))
                          (remove-exit! direction)
                          (ma-save-state!)
                          (broadcast-room-ctx!)
                          (broadcast (string-append did " fills " direction "."))
                          (reply-ok msg))
                        (reply-to-sender msg (string-append "No exit " direction "."))))))))))))

(set-cmd-method! :move
  (lambda (args msg)
    (if (reject-foreign-delegated-go? args msg)
      (reply-ok msg)
        (let* ((actor (go-caller-actor args msg))
               (did (go-caller-did args msg))
               (direction (random-exit-direction)))
          (if direction
              (begin
                (send-exit-ctx! actor did direction (exit-target direction))
                (reply-ok msg))
              (begin
                (ma-send! (canonical-actor actor) (list :print "No exits."))
                (reply-ok msg)))))))

(set-meta-method! :child
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :child <ctx>"))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :child <ctx>"))
          (else
           (handle-child-announcement! msg (car args))))))

(set-meta-method! :parent
  (lambda (args msg)
    (cond ((null? args)
           (reply-ok-with msg (actor-parent)))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :parent [ctx]"))
          ((not (equal? (ctx-text (car args) "kind") "avatar"))
           (handle-child-announcement! msg (car args)))
          (else
           (handle-avatar-parent! msg (car args))))))

(set-cmd-method! :enter
  (lambda (args msg)
    (cond
      ((direct-did-enter-args? args msg)
       (request-avatar-entry! (msg-from msg) (direct-did-enter-nick args) (direct-did-enter-inventory args)))
      ((enter-ctx-args? args)
       (handle-enter-ctx! msg (car args)))
      ((self-avatar-entry? args msg)
       (handle-self-avatar-entry! args))
      ((did-avatar-entry? args)
       (handle-did-avatar-entry! args))
      (else
       (handle-legacy-avatar-entry! args)))))

; Lifecycle signals from the runtime.
(define (on-signal term)
  (cond ((or (equal? (verb-of term) :init)
             (equal? (verb-of term) :start))
         (schedule-presence!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
