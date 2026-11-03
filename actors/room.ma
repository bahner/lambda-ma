; Locked room actor.
; Rooms own exits, bare-DID presence, and local policy for movable actors.

; Kinds.
(define ROOM_KIND "/ma/room/0.0.1")
(define PASSAGE_KIND "/ma/passage/0.0.1")
(define (node-kind) "room")
(define (node-protocol) ROOM_KIND)

; Direct DID presence is stored in the same child collection as actor children.
; Every room resident lives in the inherited children map. A bare DID's child
; ctx is an ordinary child key alongside actor DID-URLs; views derive from
; ctxs. Every actor has a DID-URL address — there is no bare-DID-only subset.

; A room claim is any child ctx whose parent resolves to this room.
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

; Kind lists are the single place presence kinds are classified. h00man is
; the direct-DID human presence kind; runtime agents keep "agent". `who?` is
; the h00man-only view — the people one can meaningfully talk to.
(define occupant-kinds (list "agent" "h00man"))
(define person-kinds (list "h00man"))

(define (occupant-kind? kind) (member-string? kind occupant-kinds))
(define (person-kind? kind) (member-string? kind person-kinds))

; A child with `visible: false` remains a room child (it still receives
; broadcasts and answers direct RPCs like :trigger?) but is never published:
; :look's children map, the nested parent-ctx snapshots, and every broadcast
; (:arrive/:leave/:child-ctx/:dig/:say/:emote) exclude it, and the movable
; lookup pool never resolves it. No client ever learns of a hidden child.
(define (room-child-visible? ctx)
  (not (equal? (ctx-text ctx "visible") "false")))

(define (occupants-by kind-pred)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (unique-actor-entries (reverse acc)))
          ((and (room-child-visible? (car ctxs))
                (room-claim? (car ctxs))
                (kind-pred (ctx-text (car ctxs) "kind")))
           (loop (cdr ctxs)
                 (cons (canonical-actor (ctx-text (car ctxs) "actor")) acc)))
          (else
           (loop (cdr ctxs) acc)))))

(define (occupants) (occupants-by occupant-kind?))
(define (who) (occupants-by person-kind?))

(define (set-label! actor label)
  (let ((ctx (child-ctx actor)))
    (if (and (map? ctx) (non-empty-string? label))
        (remember-child! (map-set ctx "nick" label))
        #f)))

; Labels live on child ctx so display and matching share one naming source.
; Presentation helpers.
(define (speaker-name actor)
  (let ((ctx (child-ctx actor)))
    (if (map? ctx) (child-label ctx) actor)))

(define (visible-occupant-matches token)
  (let loop ((xs (occupants)))
    (cond ((null? xs) '())
          ((casefold-text-match? (speaker-name (car xs)) token)
           (cons (car xs) (loop (cdr xs))))
          (else (loop (cdr xs))))))

(define (visible-child-matches token)
  (let loop ((ctxs (child-ctxs)) (matches '()))
    (cond ((null? ctxs) (reverse matches))
          ((casefold-text-match? (child-label (car ctxs)) token)
           (loop (cdr ctxs) (cons (ctx-text (car ctxs) "actor") matches)))
          (else (loop (cdr ctxs) matches)))))

(define (unique-visible-child-ref token)
  (let ((matches (visible-child-matches token)))
    (cond ((null? matches) #f)
          ((null? (cdr matches)) (car matches))
          (else :ambiguous))))

; Container take defaults to the carrier unless a valid parent override is supplied.
(define (take-carrier-parent take-args carrier)
  (let ((candidate (if (or (null? take-args) (null? (cdr take-args))) #f (car (cdr take-args)))))
    (if (and (non-empty-string? candidate)
             (or (is-did-url? candidate) (local-actor-ref? candidate)))
        (canonical-actor candidate)
        (canonical-actor carrier))))

; Human token lookup crosses exits, things, containers, and occupants.
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
    (token-list-text "Things" (thing-token-names)) "\n"
    (exits-text)))

(define (arrival-text) (room-name))

(define (room-ctx-rev)
  (let ((value (get-prop "ctx:rev")))
    (if (number? value) value 0)))

(define (room-ctx-parent)
  (canonical-actor (root)))

(register-ctx-props! (list "ctx:rev" "children" "exits"))

; Older presentation queries still use projected entries; room-ctx does not.
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

(define (exit-entry-ctx name)
  (make-map "actor" (exit-target name) "kind" "passage" "protocol" PASSAGE_KIND
            "name" name "nick" name "description" name))

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

; Public visible children are the authoritative KV map, keyed by actor DID.
; Children with `visible: false` are never published — not in :look's children
; map, not in the nested parent-ctx snapshots, not in any broadcast. They stay
; room children (broadcasts and direct RPCs still reach them) but no client
; ever learns of them: hidden means unpublished, never merely unrendered.
(define (visible-ctx)
  (let loop ((entries (map->alist (children-map))) (acc (make-map)))
    (cond ((null? entries) acc)
          ((room-child-visible? (cdr (car entries)))
           (loop (cdr entries) (map-set acc (car (car entries)) (cdr (car entries)))))
          (else (loop (cdr entries) acc)))))

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
    "children" (visible-ctx)
    "exits" (exits-map)))

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

; Bare-DID entry synthesises the minimal public child ctx owned by this room.
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
  (+ 1 (let ((previous (claim-ctx did)))
         (if (and (map? previous) (number? (map-ref previous "rev" #f)))
             (map-ref previous "rev" #f)
             0))))

(define (did-child-ctx did name nick)
  (make-map "actor" did
            "did" did
            "parent" (canonical-actor (self))
            "kind" "h00man"
            "protocol" "/ma/agent/0.0.1"
            "name" name
            "nick" nick
            "description" "A direct DID presence."
            "rev" (next-did-revision did)))

; ── Traversal (exit-routed movement) ─────────────────────────────────────
; Memory-only: traveller DID -> the authorisation this room issued for one
; traversal. In-flight grants need not survive reload/reboot.
(define move-grants (make-map))

; Memory-only: traverseId -> traveller DID, for a walk this room is driving.
(define move-owners (make-map))

(define (room-deny-text)
  (let ((t (get-prop "nogo_msg")))
    (if (non-empty-string? t) t "The room refuses you entry.")))

; A room is public (a bare DID may `:enter`) unless `public` is present and
; falsy. A private room admits a traversal grant key or the runtime's global
; (owner) key.
(define (public?)
  (let ((value (get-prop "public")))
    (cond ((not (has-prop? "public")) #t)
          ((equal? value #f) #f)
          ((equal? value "false") #f)
          (else #t))))

; The runtime-wide break-glass key. It admits the admin to any private room and
; opens any locked exit. It is daemon-local: set `global_key` in the runtime's
; `ma.yaml`; it is never exposed or writable over CRUD. Shared by every room
; and exit on this runtime. Absent or empty disables it.
(define (global-key)
  (let ((k (ma-get-config-key "global_key")))
    (if (non-empty-string? k) k #f)))

; A fresh 6-digit lock key for a newly dug passage.
(define (generate-lock-key)
  (number->string (ma-random 1000000)))

; Commit a direct DID entry without creating a separate identity actor.
(define (commit-did-entry! msg args)
  (let* ((did (msg-from msg))
         (name (did-name-from-entry args did))
         (nick (did-nick-from-entry args did))
         (was-present (member-entry? did (occupants)))
         (ctx (did-child-ctx did name nick)))
          (remember-child! ctx)
          (ma-save-state!)
    (if was-present #f (broadcast-term-except did (list :arrive ctx)))
    (broadcast-room-ctx!)
    (ma-reply! msg (list :ok ctx))))

; A DID already holding a place here needs no entry key again. Its presence is
; the claim: a repeat entry (`.ma` republish, reconnect) refreshes the same
; child ctx rather than admitting a new traveller, and first-entry keys such as
; a traversal grant are consumed, so they may no longer exist. This is the
; idempotent repeat required by "Direct entry and admission".
(define (present-entry? actor)
  (let ((ctx (claim-ctx actor)))
    (and (map? ctx) (room-claim? ctx))))

; The traveller of a movement ctx, with msg.from as the last resort.
(define (move-traveller ctx msg)
  (let ((did (ctx-text ctx "did")))
    (if (non-empty-string? did)
        did
        (let ((actor (ctx-text ctx "actor")))
          (if (non-empty-string? actor) actor (msg-from msg))))))

(define (self-entry? ctx msg)
  (let ((t (move-traveller ctx msg)))
    (or (same-actor? (msg-from msg) t) (equal? (msg-from msg) t))))

; A delegate (an exit) asks this room to authorise one traveller. The room never
; commits presence here: it answers with the key the traveller must present.
(define (handle-delegated-enter! msg ctx)
  (let* ((did (ctx-text ctx "did"))
         (tid (ctx-text ctx "traverseId"))
         (exit (canonical-actor (msg-from msg)))
         (issued (number->string (ma-random 1000000000000))))
    (if (and (non-empty-string? did) (non-empty-string? tid))
        (begin
          (set! move-grants
            (map-set move-grants did
              (make-map "traverseId" tid
                        "key" issued
                        "arrive" (let ((t (ctx-text ctx "arrive")))
                                   (if (non-empty-string? t) t "You arrive."))
                        "oarrive" (let ((t (ctx-text ctx "oarrive")))
                                    (if (non-empty-string? t) t "arrives.")))))
          (ma-send! exit (list :move
                               (make-map "traverseId" tid "exit" (canonical-actor (self)) "key" issued)))
          (reply-ok msg))
        (ma-send! exit (list :move
                             (make-map "traverseId" (if (non-empty-string? tid) tid "")
                                       "exit" (canonical-actor (self))
                                       "deny" "enter requires did and traverseId"))))))

; Commit the arrival the room already authorised. The narration is the room's
; own choice: the exit's proposal, or the room default.
(define (commit-move-entry! msg grant ctx)
  (let* ((did (msg-from msg))
         (name (did-name-from-entry (list ctx) did))
         (nick (did-nick-from-entry (list ctx) did))
         (was-present (member-entry? did (occupants)))
         (child (did-child-ctx did name nick))
         (arrive  (let ((t (ctx-text grant "arrive")))  (if (non-empty-string? t) t "You arrive.")))
         (oarrive (let ((t (ctx-text grant "oarrive"))) (if (non-empty-string? t) t (string-append name " arrives.")))))
    (remember-child! child)
    (ma-save-state!)
    (if was-present #f
        (broadcast-term-except did (list :arrive (map-set child "narrate" oarrive))))
    (broadcast-room-ctx!)
    (ma-send! (canonical-actor did) (list :event :narrate arrive))
    (ma-reply! msg (list :ok child))))

; The traveller presents the key this room issued for its own traversal.
(define (handle-move-enter! msg ctx)
  (let* ((did (msg-from msg))
         (tid (ctx-text ctx "traverseId"))
         (key (ctx-text ctx "key"))
         (grant (map-ref move-grants did #f)))
    (cond ((and (map? grant) (equal? (ctx-text grant "traverseId") tid))
           (set! move-grants (map-delete move-grants did))
           (commit-move-entry! msg grant ctx))
          ((and (global-key) (equal? key (global-key)))
           (commit-did-entry! msg (list ctx)))
          ((public?)
           (commit-did-entry! msg (list ctx)))
          (else
           (reply-error msg "invalid or missing entry key")))))

; Actor entries must carry their own ctx; room only admits or refuses it.
(define (handle-enter-ctx! msg ctx)
  (let* ((actor (ctx-text ctx "actor"))
         (did (if (non-empty-string? actor) actor (msg-from msg)))
         (kind (ctx-text ctx "kind"))
         (name (ctx-text ctx "name")))
    (cond ((and (agent-kind? kind) (enter-direct-ctx-valid? ctx "agent"))
           (handle-agent-enter! msg did ctx))
          ((and (person-kind? kind) (enter-direct-ctx-valid? ctx "h00man"))
           (handle-agent-enter! msg did ctx))
          ((and (thing-kind? kind) (enter-direct-ctx-valid? ctx "thing"))
           (handle-thing-enter! msg did ctx name))
          (else (reply-error msg "enter ctx requires an agent, h00man, or thing ctx")))))

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
          ; Unchanged re-entry skips remember-child!, so note the report here.
          (roll-call-note-child! ctx)
          (send-fresh-child-ctx! actor)
          (reply-ok msg))
        (begin
          (set-claim! actor ctx)
          (set-label! actor nick)
          (if was-known
              #f
              (if (room-child-visible? ctx)
                  (broadcast-term-except actor (list :arrive ctx))
                  #f))
          (ma-save-state!)
          (broadcast-room-ctx!)
          (ma-send! actor (list :here (self)))
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
             ; Unchanged re-entry skips remember-child!, so note the report here.
             (roll-call-note-child! ctx)
             (send-fresh-child-ctx! actor)
             (reply-ok msg)))
          (else
           (set-claim! actor ctx)
           (set-label! actor label)
           (set-thing! token actor)
           (if was-known
               #f
               (if (room-child-visible? ctx)
                   (broadcast-term-except actor (list :arrive ctx))
                   #f))
           (broadcast-room-ctx!)
           (ma-send! actor (list :here (self)))
           (ma-reply! msg (list :ok "entered"))))))

(define (child-parent-target ctx)
  (ctx-text ctx "parent"))

; Shape and authenticity checks stay near the dispatcher that trusts them.
(define (child-announcement-valid? ctx msg)
  (and (or (actor-ctx? ctx msg)
           (and (child-departure-ctx? ctx) (ctx-sender-valid? ctx msg)))
             ))

; Child announcements are the room's single admission/departure boundary.
; The child's own ctx is the authority: :parent simply replaces the stored
; children-map key with the announced ctx. There is deliberately no re-entry
; ceremony here — no token binding, no label/claim comparison, no kind-specific
; admission rules — so a changed name/nick lands in the map immediately and a
; roll-call answer is always re-noted. Views (look, things?, who?) derive
; lazily from the children map. Exits keep the shared node handler, which
; admits them without arrival events.
(define (handle-child-announcement! msg ctx)
  (let ((kind (ctx-text ctx "kind"))
        (actor (ctx-text ctx "actor")))
    (cond ((not (child-announcement-valid? ctx msg))
           (reply-error msg "child ctx actor must match sender"))
          ((not (same-actor? (child-parent-target ctx) (self)))
           (begin
             (forget-child! actor)
             (ma-save-state!)
             (if (room-child-visible? ctx)
                 (broadcast-term (list :leave ctx))
                 #f)
             (broadcast-room-ctx!)
             (reply-ok msg)))
          ((not (room-admissible-kind? kind))
           (reply-error msg "child ctx must include actor, parent, kind, protocol, name, nick, description"))
          ((equal? (child-ctx actor) ctx)
           ; Unchanged announcement (prop refresh, roll-call answer): the
           ; replace is a no-op, so just ack the child directly.
           (begin
             (roll-call-note-child! ctx)
             (send-fresh-child-ctx! actor)
             (reply-ok msg)))
          ((child-ctx actor)
           ; Known child re-announcing a changed ctx: replace the stored key
           ; and tell every actor in the room so clients can refresh exactly
           ; that child's entry without a full room snapshot.
           (begin
             (remember-child! ctx)
             (ma-save-state!)
             (if (room-child-visible? ctx)
                 (broadcast-term (list :child-ctx ctx))
                 #f)
             (broadcast-room-ctx!)
             (reply-ok msg)))
          (else
           ; First announcement: admit and announce the arrival.
           (begin
             (remember-child! ctx)
             (ma-save-state!)
             (if (room-child-visible? ctx)
                 (broadcast-term-except actor (list :arrive ctx))
                 #f)
             (broadcast-room-ctx!)
             (ma-send! actor (list :here (self)))
             (reply-ok msg))))))

(define (agent-kind? kind) (equal? kind "agent"))
(define (thing-kind? kind) (equal? kind "thing"))
(define (container-kind? kind) (equal? kind "container"))
; A h00man is a movable thing like any other — one shared object namespace.
(define movable-kinds (list "agent" "thing" "container" "h00man"))
(define (movable-kind? kind) (member-string? kind movable-kinds))

; The room admits a trigger as a child (so it receives the room's broadcasts
; and can be designated via the `trigger` prop) but deliberately does NOT make
; it a movable kind: who?/occupants?/things?/exits? derive from
; movable-children, so the trigger stays out of the movable views. A trigger
; with `visible: true` is still presented to clients via look's "Triggers:"
; section, which derives from the same children map. Exits are not children at
; all: they are top-level nodes referenced from the `exits` map.
(define (room-admissible-kind? kind)
  (or (movable-kind? kind)
      (equal? kind "trigger")))

(define (movable-children)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (reverse acc))
          ((and (room-child-visible? (car ctxs))
                (movable-kind? (ctx-text (car ctxs) "kind")))
           (loop (cdr ctxs) (cons (car ctxs) acc)))
          (else (loop (cdr ctxs) acc)))))

(define (actor-token-valid? token)
  (non-empty-string? token))

; Things, containers, agents, and h00mans share one room-visible object
; namespace: a h00man is a movable thing like any other, and so is a duckie.
; The presentation views (things?, look's "Things:") still narrow by kind;
; lookup does not.
(define (thing-ref token)
  (if (is-did-url? token)
      (let ((ctx (child-ctx token)))
        (if (and (map? ctx) (movable-kind? (ctx-text ctx "kind")))
            (canonical-actor token)
            #f))
      (let loop ((ctxs (movable-children)))
        (cond ((null? ctxs) #f)
              ((child-token-matches? token (car ctxs))
               (canonical-actor (ctx-text (car ctxs) "actor")))
              (else (loop (cdr ctxs)))))))

(define (movable-ref token)
  (thing-ref token))

; Removal updates children first; token aliases are now derived from that ctx.
(define (set-thing! token did)
  (let ((existing (child-ctx did)))
    (remember-child!
      (if (map? existing)
          (map-set existing "nick" token)
          (presentation-entry "thing" "/ma/thing/0.0.1" did token token token)))))

(define (remove-thing! token)
  (let ((actor (thing-ref token)))
    (if actor (forget-child! actor) #f)))

(define (things-list)
  (thing-entry-list (thing-token-names)))

; DID listing is owner-only because it exposes every visible actor address.
(define (dids-text)
  (let ((lines (list-append (list-append (occupant-did-lines) (thing-did-lines)) (exit-did-lines))))
    (if (null? lines)
        "DIDs: none."
        (string-append "DIDs:\n" (actor-entry-lines lines)))))

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
              (reply-command-ok msg mediated (actor-entry-lines lines))))))
       (else
        (let* ((token (join-words did-args))
            (matches (did-matches token))
            (lines (did-match-lines token)))
         (cond ((null? matches)
              (reply-command-error msg mediated (string-append "No visible occupant, thing, or exit: " token)))
             ((null? (cdr matches))
              (reply-command-ok msg mediated (string-append token " = " (canonical-actor (car matches)))))
             (else
              (reply-command-ok msg mediated (string-append "Ambiguous name: " token "\n" (actor-entry-lines lines))))))))))

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
                 (reply-command-ok msg mediated (string-append "Ambiguous name: " token "\n" (actor-entry-lines lines)))))))))

; ── Exits ─────────────────────────────────────────────────────────
; Exits are top-level nodes (parented to root), NOT room children. The room
; keeps a plain `exits` map of local name -> exit DID-URL; the local name is
; purely cosmetic. One exit per passage means one lock, never per-player.

(define (exits-map)
  (let ((value (get-prop "exits")))
    (if (map? value) value (make-map))))

(define (set-exits! m)
  (begin (set-prop! "exits" m) (ma-save-state!)))

(define (exit-target name)
  (let ((did (map-ref (exits-map) name #f)))
    (if (non-empty-string? did) did #f)))

(define (exit-directions)
  (map-keys (exits-map)))

; A passage carries at most one local name in a room. Registering it again
; under a different name is how an exit is renamed: the previous local name is
; dropped first, so `exits` never lists the same passage twice.
(define (register-exit! name exit-did)
  (let ((did (canonical-actor exit-did)))
    (set-exits! (map-set (exits-without did) name did))
    (broadcast-room-ctx!)))

; The `exits` map without any local name pointing at `exit-did`.
(define (exits-without exit-did)
  (let loop ((entries (map->alist (exits-map))) (out (make-map)))
    (if (null? entries)
        out
        (let ((entry (car entries)))
          (if (same-actor? (cdr entry) exit-did)
              (loop (cdr entries) out)
              (loop (cdr entries) (map-set out (car entry) (cdr entry))))))))

(define (unregister-exit! name)
  (begin
    (set-exits! (map-delete (exits-map) name))
    (broadcast-room-ctx!)))

; Drop every local name pointing at this exit (used when the exit ends or is
; detached from one side).
(define (drop-exit-ref! exit-did)
  (let loop ((m (exits-map)) (changed #f))
    (if (null? (map->alist m))
        (if changed (broadcast-room-ctx!) #f)
        (let* ((entry (car (map->alist m)))
               (name (car entry))
               (did (cdr entry)))
          (if (same-actor? did exit-did)
              (begin (set-exits! (map-delete m name)) (loop (map-delete m name) #t))
              (loop m changed))))))

; True when this actor is one of the exits this room references. Used to gate
; relayed departures: only an exit we actually link to may remove a traveller.
(define (registered-exit? actor)
  (let loop ((entries (map->alist (exits-map))))
    (cond ((null? entries) #f)
          ((same-actor? (cdr (car entries)) actor) #t)
          (else (loop (cdr entries))))))

(define (random-exit-direction)
  (let ((directions (exit-directions)))
    (if (null? directions)
        #f
        (list-ref-at directions (random (list-length directions))))))

; A move ctx arriving here is either a walk request from a present traveller
; (no live traversal yet) or the passage relaying the destination's answer
; (a live traversal, and this room is its `entrance`).
(define (handle-walk-request! ctx msg)
  (let* ((did (msg-from msg))
         (tid (ctx-text ctx "traverseId"))
         (key (ctx-text ctx "key"))
         (named (ctx-text ctx "direction"))
         (direction (if (non-empty-string? named) named (random-exit-direction)))
         (passage (if (non-empty-string? direction) (exit-target direction) #f)))
    (cond ((not (and (non-empty-string? tid) passage))
           (reply-error msg "move requires a direction, a traverseId and a known passage"))
          ((not (member-entry? did (occupants)))
           (reply-error msg "only a present traveller may walk a passage"))
          (else
           (set! move-owners (map-set move-owners tid did))
           (ma-send! (canonical-actor passage)
                     (list :move (make-map
                                   "did" did
                                   "traverseId" tid
                                   "key" key
                                   "name" (speaker-name did)
                                   "nick" (speaker-name did)
                                   "entrance" (canonical-actor (self)))))
           (reply-ok msg)))))

(define (handle-move-result! ctx tid did msg)
  (let* ((child (claim-ctx did))
         (deny (ctx-text ctx "deny"))
         (dest (ctx-text ctx "exit"))
         (key (ctx-text ctx "key"))
         (nogo (ctx-text ctx "nogo")))
    (set! move-owners (map-delete move-owners tid))
    (if (non-empty-string? deny)
        (begin
          (if (map? child)
              (broadcast-term (list :nogo (map-set child "narrate"
                                                   (if (non-empty-string? nogo) nogo deny))))
              #f)
          (ma-send! (canonical-actor did)
                    (list :move (make-map "traverseId" tid "deny" deny)))
          (reply-ok msg))
        (begin
          (if (map? child)
              (begin
                (forget-child! did)
                (ma-save-state!)
                (if (room-child-visible? child)
                    (broadcast-term (list :leave child))
                    #f)
                (broadcast-room-ctx!))
              #f)
          (ma-send! (canonical-actor did)
                    (list :move (make-map "traverseId" tid "exit" dest "key" key)))
          (reply-ok msg)))))

(define (exits-text)
  (let ((directions (exit-directions)))
    (if (null? directions)
        "Exits: none."
        (string-append "Exits: " (names-of directions)))))

; Exit and destination fragments are deterministic so repeated dig is idempotent.
(define (room-fragment direction)
  (string-append "room-" (blake3 (string-append "lambda-ma exit v2\n" (canonical-actor (self)) "\n" direction "\ndst") 8)))

(define (passage-fragment direction)
  (string-append "passage-" (blake3 (string-append "lambda-ma exit v2\n" (canonical-actor (self)) "\n" direction) 8)))

; Room-facing text surfaces.
(define (thing-token-names)
  (let loop ((ctxs (list-append (child-ctxs-by-kind "thing")
                                (child-ctxs-by-kind "container")))
             (acc '()))
    (if (null? ctxs)
        (reverse acc)
        (if (room-child-visible? (car ctxs))
            (loop (cdr ctxs) (cons (child-label (car ctxs)) acc))
            (loop (cdr ctxs) acc)))))

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
    "  who?              list the people here (h00mans)\n"
    "  occupants?        list occupants here\n"
    "  things?           list known things\n"
    "  did? [kind] <name> show the DID for a visible occupant, thing, or exit\n"
    "  owner? <name>     show who owns a visible occupant, thing, or exit\n"
    "  take <thing>      move something here into your inventory\n"
    "  drop [<thing>]    drop what you hold, or a named inventory item, here\n"
    "  put <item...> in <container...> place something inside a container\n"
    "  forge <kind> named <name...> create something held by you\n"
    "  tell <target...> to <verb> [args...] ask something to perform a verb\n"
    "  recycle <thing>  remove an owned agent, thing, or container\n"
    "  where? <thing>    ask where an occupant says it is\n"
    "  say <text>        speak here\n"
    "  emote <text>      act here\n"
    "  narrate <text>    the room describes the scene (owner/scheduler/trigger)\n"
    "  :narrate-to <did> <text>  narrate to one child (owner/scheduler/trigger)\n"
    "  look <exit>       inspect an exit\n"
    "  :leave           stop being shown here until you return\n"
    "  :remove <who>     owner removes an occupant by unique nick or DID\n"
    "  move              move through one available exit\n"
    "  claim             claim this room if it is unowned\n"
    "  owner [did]       show or transfer ownership\n"
    "  dig <dir> [to name] [back dir] [with code] create a two-ended exit\n"
    "  dig <passage-did> <name> rename a local exit\n"
    "  fill <dir>        remove an exit\n"
    "  :exit <name> <did> register an exit dug from the far side\n"
    "  :dids?            owner lists occupants, things, and exits with DIDs\n"
    "  :thing <name> [did] set/list local occupant alias\n"
    "  :behaviour /ipfs/<cid> add or replace this actor's own code\n"
    "  :prop <key> [value] set or reset room text (owner)\n"
    "Agents and things enter with :enter ctx; their own parent state is the authority.\n"
    "Commands with and without : target this place directly."))

; Ownership and room text mutation.
(define (owner) (get-prop "owner"))
(define (owned?) (if (owner) #t #f))
(define (owner? did)
  (equal? did (owner)))

(define (valid-owner? value)
  (is-did-url? value))

(define (owner-message? msg)
  (msg-from-owner? (owner) msg))

; Room props are owner-only, and an unowned room has no editable text at all:
; it must be claimed first. This is deliberately stricter than the shared
; node.ma gate, which leaves an unowned node's props open to any caller.
(define (prop-caller-authorised? msg)
  (and (owned?) (owner-message? msg)))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (claim-owner-did args msg)
  (msg-from msg))

; Delegated replies are printed to the controlling DID as well as acked.
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
  (msg-from msg))

(define (recycle-command-args args msg) args)

(define (go-delegated-call? args msg)
  (delegated-call? args msg))

(define (go-caller-did args msg)
  (msg-from msg))

(define (go-command-args args msg)
  (if (go-delegated-call? args msg) (cdr args) args))

; Owner guard helpers keep user-facing failure text local to room policy.
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

; The room's voice gate: the room owner, the runtime scheduler (for
; time-based narration), or the designated trigger actor (the `trigger` prop),
; provided it is still present as a child. Designation-based, never name-based.
(define (narrate-authorised? msg)
  (let ((trigger (get-prop "trigger")))
    (or (owner-message? msg)
        (same-actor? (msg-from msg) (entity-url "scheduler"))
        (and (non-empty-string? trigger)
             (same-actor? (msg-from msg) trigger)
             (map? (child-ctx trigger))))))

; Presence and room event handlers.
(define (on-event event args msg)
  (cond ((equal? event :leave-occupant)
         (let ((actor (msg-from msg)))
           (let ((ctx (claim-ctx actor)))
           (forget-child! actor)
           (ma-save-state!)
           (if (and (map? ctx) (room-child-visible? ctx))
               (broadcast-term (list :leave ctx))
               #f))))
        (else #f)))

(define (leave-candidate args msg)
  (if (null? args) (canonical-actor (msg-from msg)) #f))

  ; Leave is idempotent: absent occupants still receive an ok.
(define (handle-leave! msg args)
  (cond
    ((and (not (null? args)) (map? (car args))
          (non-empty-string? (ctx-text (car args) "did")))
     (handle-relayed-leave! msg (car args)))
    (else
     (let ((actor (leave-candidate args msg)))
       (if actor
           (let ((ctx (claim-ctx actor)))
             (forget-child! actor)
             (ma-save-state!)
             (if (and (map? ctx) (room-child-visible? ctx))
                 (broadcast-term (list :leave ctx))
                 #f)
             (reply-ok msg))
           (reply-error msg "usage: :leave"))))))

; A local exit relays a departure it authorised: it removes the traveller and
; narrates the exit's leave messages. Only a child exit we own may do this.
(define (handle-relayed-leave! msg lctx)
  (let* ((did (ctx-text lctx "did"))
         (relay (msg-from msg))
         (ctx (claim-ctx did)))
    (if (not (registered-exit? relay))
        (reply-error msg "only a local exit may relay a departure")
        (let ((nick (let ((n (if (map? ctx) (ctx-text ctx "nick") #f)))
                      (if (non-empty-string? n) n did))))
          (forget-child! did)
          (ma-save-state!)
          (if (and (map? ctx) (room-child-visible? ctx))
              (begin
                (broadcast-term-except did
                  (list :leave (map-set ctx "narrate"
                    (let ((t (ctx-text lctx "oleave_msg")))
                      (if (non-empty-string? t) t (string-append nick " leaves."))))))
                (ma-send! (canonical-actor did)
                  (list :event :narrate
                    (let ((t (ctx-text lctx "leave_msg")))
                      (if (non-empty-string? t) t "You leave.")))))
              #f)
          (broadcast-room-ctx!)
          (reply-ok msg)))))

  ; Owner removal accepts exact actor refs or a unique visible child label.
(define (remove-candidate token)
  (cond ((not (string? token)) #f)
    ((and (is-did-url? token)
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
                   (if (and (map? ctx) (room-child-visible? ctx))
                       (broadcast-term (list :leave ctx))
                       #f)
                   (ma-send! actor (list :report-parent (canonical-actor (self)))))
                   (reply-ok-with msg (string-append "Removed " name " from this room; asked it to reannounce."))))
                   (else
                    (reply-error msg (string-append "No such child: " target)))))))))

; Room events are broadcast to every actor present in the room — every child,
; regardless of kind. Each receiving actor decides what to do: a human client
; renders it, an agent may remember or react, and any actor may ignore it.
;
; Every world event travels in a single :event envelope (:event <name> <args…>)
; so a room broadcast never collides with a direct term addressed to the same
; actor (:say <text> is direct speech; :event :say <ctx> <text> is a broadcast).
(define (broadcast-term term)
  (let loop ((ctxs (child-ctxs)))
    (cond ((null? ctxs) #f)
          (else
           (begin
             (ma-send! (canonical-actor (ctx-text (car ctxs) "actor"))
                       (cons :event term))
             (loop (cdr ctxs)))))))

(define (broadcast-term-except excluded term)
  (let loop ((ctxs (child-ctxs)))
    (cond ((null? ctxs) #f)
          ((same-actor? (ctx-text (car ctxs) "actor") excluded)
           (loop (cdr ctxs)))
          (else
           (begin
             (ma-send! (canonical-actor (ctx-text (car ctxs) "actor"))
                       (cons :event term))
             (loop (cdr ctxs)))))))

(define (event-subject-ctx actor)
  (claim-ctx actor))

; Exit build state. New-room digs wait for a child-alive callback before
; installing the exit; existing-room links install the exit directly (the
; destination's :enter is the admission authority, not a link handshake).
(define (pending-new-room-key direction) (string-append "pending-new-room:" direction))
(define (pending-new-room-did-key direction) (string-append "pending-new-room-did:" direction))
(define (pending-new-room-target-name-key direction) (string-append "pending-new-room-target-name:" direction))
(define (pending-new-room-nonce-key direction) (string-append "pending-new-room-nonce:" direction))
(define (pending-new-room-lock-key-key direction) (string-append "pending-new-room-lock-key:" direction))
(define (pending-new-room-back-key direction) (string-append "pending-new-room-back:" direction))

(define (clear-pending-new-room! direction)
  (begin
    (del-prop! (pending-new-room-key direction))
    (del-prop! (pending-new-room-did-key direction))
    (del-prop! (pending-new-room-target-name-key direction))
    (del-prop! (pending-new-room-nonce-key direction))
    (del-prop! (pending-new-room-lock-key-key direction))
    (del-prop! (pending-new-room-back-key direction))))

; The exit this room remembers for a direction. Dig fragments are deterministic,
; so a direction always names the same passage and destination room.
(define (remembered-new-room-target direction)
  (exit-target direction))

(define (create-exit! direction target-room lock-key)
  (let ((exit (ma-create-actor PASSAGE_KIND #f (exit-init direction (canonical-actor (self)) target-room lock-key) (passage-fragment direction))))
    (ma-send! exit (list :report-parent (root)))
    exit))

(define (room-init name owner-did custom-init ready-init)
  (string-append
    "(set-init-prop! \"root\" \"" (root) "\")\n"
    (if name (string-append "(set-init-prop! \"name\" \"" name "\")\n") "")
    "(set-init-prop! \"owner\" \"" owner-did "\")\n"
    ; A newly dug room starts under construction: only a key-holder (a passage
    ; ticket or the break-glass global key) may enter until its builder makes
    ; it public. `custom-init` runs after this and may override the prop.
    "(set-init-prop! \"public\" \"false\")\n"
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

(define (exit-init direction src dst lock-key)
  (string-append
    (if (owner) (string-append "(set-init-prop! \"owner\" \"" (owner) "\")\n") "")
    "(set-init-prop! \"entrance\" \"" (canonical-actor src) "\")\n"
    "(set-init-prop! \"exit\" \"" (canonical-actor dst) "\")\n"
    (if (non-empty-string? lock-key)
        (string-append "(set-init-prop! \"lock-secret\" \"" lock-key "\")\n")
        "")
    "(set-init-prop! \"parent\" \"" (root) "\")\n"
    "(ma-save-state!)\n"))

(define (dig-target-args args)
  (if (null? args)
      '()
      (let ((rest (cdr args)))
        (if (and (not (null? rest)) (equal? (car rest) "to"))
            (cdr rest)
            rest))))

; `dig` grammar: `dig <direction> [to <name>] [back <direction>] [with <code>]`.
; `to`, `back`, and `with` are hard separators, so a multi-word room name
; cannot swallow the return direction. The far room names the same passage
; `back` when given and the source direction otherwise, so a plain dig stays
; symmetric; `dig <passage-did-url> <name>` renames a local exit later.
(define (dig-payload-args args)
  (take-before "with" (dig-target-args args)))

(define (dig-target-text args)
  (let ((target-args (take-before "back" (dig-payload-args args))))
    (if (null? target-args) #f (join-words target-args))))

(define (dig-back-direction args)
  (let ((back-args (drop-through "back" (dig-payload-args args))))
    (if (null? back-args) #f (car back-args))))

(define (dig-back-name direction back)
  (if (non-empty-string? back) back direction))

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
  (if (and target (is-did-url? target)) target #f))

(define (pending-new-room-nonce direction requester did target-name)
  (blake3 (string-append "lambda-ma pending room v1\n"
                         (canonical-actor (self)) "\n"
                         direction "\n"
                         (canonical-actor requester) "\n"
                         did "\n"
                         (if target-name target-name "")) 16))

(define (remember-pending-new-room! direction target-room did target-name back nonce lock-key)
  (begin
    (set-prop! (pending-new-room-key direction) (canonical-actor target-room))
    (set-prop! (pending-new-room-did-key direction) did)
    (if target-name
        (set-prop! (pending-new-room-target-name-key direction) target-name)
        (del-prop! (pending-new-room-target-name-key direction)))
    (set-prop! (pending-new-room-nonce-key direction) nonce)
    (set-prop! (pending-new-room-lock-key-key direction) lock-key)
    (set-prop! (pending-new-room-back-key direction) back)
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
            (let* ((did (get-prop (pending-new-room-did-key direction)))
                   (lock-key (get-prop (pending-new-room-lock-key-key direction)))
                   (pending-back (get-prop (pending-new-room-back-key direction)))
                   (back (if (non-empty-string? pending-back) pending-back direction)))
              (begin
                (let ((exit (create-exit! direction target-room lock-key)))
                  (register-exit! direction exit)
                  (ma-send! (canonical-actor target-room) (list :exit back exit)))
                (clear-pending-new-room! direction)
                (ma-save-state!)
                (let ((ctx (claim-ctx did)))
                  (if (and (map? ctx) (room-child-visible? ctx))
                      (broadcast-term (list :dig ctx direction))
                      #f))))
            #f))))

; Start a new-room dig and persist pending state until the child-alive callback
; arrives from the freshly created room.
(define (request-new-room! msg did direction back target custom-init custom-behaviour)
  (let* ((target-fragment (room-fragment direction))
         (requester (canonical-actor (msg-from msg)))
         (nonce (pending-new-room-nonce direction requester did target))
         ; The new passage's lock key is minted now and reported to the digger;
         ; it becomes the exit's `lock-secret` when the exit is installed.
         (lock-key (generate-lock-key))
         (target-room (ma-create-actor ROOM_KIND
                                       custom-behaviour
                                       (room-init target did custom-init (child-alive-init nonce direction))
                                       target-fragment)))
    (remember-pending-new-room! direction target-room did target back nonce lock-key)
    (reply-to-sender msg (string-append "Dug " direction " to " target-fragment " with key " lock-key))
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

; Standard read-only node getters required by the /ma/node/0.0.1 base
; protocol (ma-node-v1). Rooms override the generic node defaults so an
; unnamed room reports "Construct" rather than its own address.
(set-rpc-method! :name? (lambda (args msg) (reply-ok-with msg (room-name))))
(set-rpc-method! :nick? (lambda (args msg) (reply-ok-with msg (room-nick))))
(set-rpc-method! :description? (lambda (args msg) (reply-ok-with msg (room-description))))

(set-cmd-method! :exits?
  (lambda (args msg)
    (reply-ok-with msg (exit-entry-list (exit-directions)))))

(define (handle-who-query! args msg)
  (reply-ok-with msg (agent-entry-list (who))))

(define (handle-occupants-query! args msg)
  (reply-ok-with msg (agent-entry-list (occupants))))

(set-cmd-method! :who? handle-who-query!)
(set-cmd-method! :occupants? handle-occupants-query!)

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
           (reply-ok-with msg (things-list)))
            ((null? (cdr thing-args))
             (let ((token (car thing-args))
                   (did (thing-ref (car thing-args))))
               (if did
               (reply-ok-with msg did)
                   (reply-error msg (string-append "Unknown thing alias: " token)))))
            ((not (owner-message? msg))
             (reply-error msg "Only this room's owner can change thing aliases."))
            ((is-did-url? (car thing-args))
             (reply-error msg "thing alias must not be a DID or DID-URL"))
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
           (actor (if token (if (is-did-url? token) token (movable-ref token)) #f)))
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

; Pre-check: caller must be present and room must have capacity before avatar sends :set-parent.
(set-rpc-method! :drop
  (lambda (args msg)
    (cond ((null? args)
           (reply-error msg "usage: :drop <item-did-url>"))
          ((not (child-ctx (canonical-actor (msg-from msg))))
           (reply-error msg "not in room"))
          ((>= (node-children-count) (node-max-children))
           (reply-error msg "room is full"))
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

; Register an exit dug from the far side. An exit is a top-level node with hardcoded
; src/dst; each room keeps only its own `exits` map of local name -> exit DID-URL,
; and that local name is purely cosmetic. `dig` mirrors a new passage into the
; destination room with this verb, so a non-owner may link a public room to a new
; passage without owning it.
(set-rpc-method! :exit
  (lambda (args msg)
    (let ((name (if (null? args) #f (car args)))
          (exit-did (if (or (null? args) (null? (cdr args))) #f (cadr args))))
      (cond ((not (and (non-empty-string? name) (is-did-url? exit-did)))
             (reply-error msg "usage: :exit <name> <exit-did-url>"))
            (else
             (register-exit! name exit-did)
             (reply-ok msg))))))

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
            (if (room-child-visible? ctx)
                (broadcast-term (list :say ctx text))
                #f)
            (reply-ok msg))
          (reply-error msg "speaker has no event ctx")))))

(set-cmd-method! :emote
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args))
          (ctx (event-subject-ctx (msg-from msg))))
      (if (map? ctx)
          (begin
            (if (room-child-visible? ctx)
                (broadcast-term (list :emote ctx text))
                #f)
            (reply-ok msg))
          (reply-error msg "speaker has no event ctx")))))

; Ambient room narration — the room's own voice, subject-less. The owner,
; scheduler, or designated trigger sets the scene ("a cat walks past the
; doorway"); occupants still use :say and :emote for themselves.
(set-cmd-method! :narrate
  (lambda (args msg)
    (let ((text (join-words args)))
      (cond ((not (narrate-authorised? msg))
             (reply-error msg "not authorised to narrate here"))
            ((non-empty-string? text)
             (begin
               (broadcast-term (list :narrate text))
               (reply-ok msg)))
            (else
             (reply-error msg "usage: :narrate <text>"))))))

; Targeted narration: the same subject-less event, delivered to one child only.
(set-cmd-method! :narrate-to
  (lambda (args msg)
    (let ((target (if (pair? args) (canonical-actor (car args)) #f))
          (text (join-words (if (pair? args) (cdr args) '()))))
      (cond ((not (narrate-authorised? msg))
             (reply-error msg "not authorised to narrate here"))
            ((not (non-empty-string? target))
             (reply-error msg "usage: :narrate-to <did> <words...>"))
            ((not (map? (child-ctx target)))
             (reply-error msg "no such child in this room"))
            ((non-empty-string? text)
             (begin
               (ma-send! target (list :event :narrate text))
               (reply-ok msg)))
            (else
             (reply-error msg "usage: :narrate-to <did> <words...>"))))))

; The designated trigger actor, for avatar-side trigger management.
(set-rpc-method! :trigger?
  (lambda (args msg)
    (let ((trigger (get-prop "trigger")))
      (reply-ok-with msg (if (non-empty-string? trigger) trigger "none")))))

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

(set-internal-rpc-method! :child-alive
  (lambda (args msg)
    (handle-child-alive! msg args)))

; ── Building, movement, and direct entry methods ──────────────────────────

; `dig <passage-did-url> <name>` re-binds a passage in this room's `exits`
; map under a new local name, dropping the old name for that same passage. It is
; a local rename only: the passage and the far room are untouched, and each
; room names the same passage independently.
(define (handle-dig-rebind! args msg)
  (let ((exit-did (car args))
        (name (if (null? (cdr args)) #f (car (cdr args)))))
    (cond ((not (non-empty-string? name))
           (reply-error msg "usage: dig <passage-did-url> <name>"))
          ((not (null? (cdr (cdr args))))
           (reply-error msg "usage: dig <passage-did-url> <name>"))
          ((is-did-url? name)
           (reply-error msg "the local exit name must not be a DID-URL"))
          (else
           (begin
             (register-exit! name exit-did)
             (reply-ok-with msg (string-append "Exit " name " -> " (canonical-actor exit-did))))))))

(define (handle-dig-build! did args msg)
  (let* ((dig-args args)
         (direction (if (null? dig-args) "out" (car dig-args)))
         (target (dig-target-text dig-args))
         (back (dig-back-name direction (dig-back-direction dig-args)))
         (custom-init (dig-custom-init-text dig-args))
         (custom-behaviour (dig-custom-behaviour-ref dig-args))
         (existing-room (existing-room-target target)))
    (cond ((remembered-new-room-target direction)
           (let ((existing (remembered-new-room-target direction)))
             (reply-error msg (string-append "An exit " direction " already exists here (" existing
                                            "). Rename it with: dig <passage-did-url> <new-name>."))))
          ((and existing-room (or custom-init custom-behaviour))
           (reply-error msg "Custom room code only applies when digging a new room."))
          (existing-room
           (let* ((lock-key (generate-lock-key))
                  (exit (create-exit! direction existing-room lock-key)))
             (register-exit! direction exit)
             (ma-send! (canonical-actor existing-room) (list :exit back exit))
             (let ((ctx (claim-ctx did)))
               (if (map? ctx) (broadcast-term (list :dig ctx direction)) #f))
             (reply-to-sender msg (string-append "Dug " direction " to " existing-room " with key " lock-key))
             (reply-ok msg)))
          (else
           (request-new-room! msg did direction back target custom-init custom-behaviour)))))

(set-cmd-method! :dig
  (lambda (args msg)
    (let ((did (owner)))
      (require-valid-owner did msg
        (lambda ()
          (require-owner msg
            (lambda ()
              (if (and (pair? args) (is-did-url? (car args)))
                  (handle-dig-rebind! args msg)
                  (handle-dig-build! did args msg)))))))))

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
                          (unregister-exit! direction)
                          (let ((ctx (claim-ctx did)))
                            (if (map? ctx) (broadcast-term (list :fill ctx direction)) #f))
                          (reply-ok msg))
                        (reply-error msg (string-append "No exit " direction "."))))))))))))

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

(define (handle-room-orphan! args msg)
  (let ((result (orphan-node-child! args msg)))
    (cond ((equal? result :orphaned)
           (broadcast-room-ctx!)
           (reply-ok msg))
          ((equal? result :missing)
           (reply-ok msg))
          (else
           (reply-error msg result)))))

(set-cmd-method! :orphan handle-room-orphan!)

(set-cmd-method! :enter
  (lambda (args msg)
    (let ((ctx (if (null? args) #f (car args))))
      (cond ((and (map? ctx) (non-empty-string? (ctx-text ctx "traverseId"))
                  (not (self-entry? ctx msg)))
             (handle-delegated-enter! msg ctx))
            ((and (map? ctx) (non-empty-string? (ctx-text ctx "traverseId")))
             (handle-move-enter! msg ctx))
            ((enter-ctx-args? args)
             (if (self-entry? (car args) msg)
                 (handle-enter-ctx! msg (car args))
                 (reply-error msg "agent, h00man and thing entries must be self-sent")))
            ((present-entry? (msg-from msg))
             (commit-did-entry! msg args))
            ((public?)
             (commit-did-entry! msg args))
            (else
             (reply-error msg "this room is not public; a key is required"))))))

(set-cmd-method! :move
  (lambda (args msg)
    (let* ((ctx (if (null? args) #f (car args)))
           (tid (if (map? ctx) (ctx-text ctx "traverseId") #f))
           (owner (if (non-empty-string? tid) (map-ref move-owners tid #f) #f)))
      (cond ((not (map? ctx))
             (reply-error msg "move requires a move ctx"))
            ((and (same-actor? (ctx-text ctx "exit") (self))
                  (not (same-actor? (msg-from msg) (self))))
             (handle-delegated-enter! msg ctx))
            ((and owner (same-actor? (ctx-text ctx "entrance") (self)))
             (handle-move-result! ctx tid owner msg))
            (else
             (handle-walk-request! ctx msg))))))

; Lifecycle signals from the runtime.
(define (on-signal term)
  (cond ((or (equal? (verb-of term) :init)
             (equal? (verb-of term) :start))
         (begin
           (ma-save-state!)
           (propose-node-parent! (room-ctx-parent))))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
