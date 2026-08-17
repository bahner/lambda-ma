; ma node library.
;
; Composed by /ma/node/0.0.1 above the state library. A node has one
; authoritative parent and stores authoritative ctx records for its children.

(define (node-parent)
  (let ((value (get-prop "parent")))
    (if (non-empty-string? value) (canonical-actor value) "")))

(define (set-node-parent! actor)
  (if (non-empty-string? actor)
  (set-init-prop! "parent" (canonical-actor actor))
  (del-init-prop! "parent"))
  (ma-save-state!))

(register-ctx-props! (list "parent" "name" "nick" "description"))

(define (node-protocol)
  (let ((value (ma-get-config-key "kind")))
    (if (non-empty-string? value) value "/ma/node/0.0.1")))

(define (node-kind) "actor")

(define (node-name)
  (let ((value (get-prop "name")))
    (if (non-empty-string? value) value (self))))

(define (node-nick)
  (let ((value (get-prop "nick")))
    (if (non-empty-string? value) value (node-name))))

(define (node-description)
  (let ((value (get-prop "description")))
    (if (non-empty-string? value) value "A node.")))

  (define (extend-node-ctx ctx) ctx)

(define (node-ctx-for-parent target-parent)
  (extend-node-ctx
    (make-map
      "actor" (canonical-actor (self))
      "kind" (node-kind)
      "protocol" (node-protocol)
      "parent" (canonical-actor target-parent)
      "name" (node-name)
      "nick" (node-nick)
      "description" (node-description))))

(define (node-ctx)
  (node-ctx-for-parent (node-parent)))

; A :child ack's own actor/parent fields must always name the child and us
; (ma-lambda-ma-v1.md sec 6), so our own richer self-description can only
; travel as a nested extra field, never by replacing those top-level ones.
(define (child-ack-ctx child-side-ctx)
  (map-set child-side-ctx "parent-ctx" (node-ctx)))

(define (send-fresh-child-ctx! actor)
  (let ((stored (child-ctx actor)))
    (if (map? stored)
        (ma-send! (canonical-actor actor) (list :child (child-ack-ctx stored)))
        #f)))

; Any parent whose own presentation changes pushes a fresh :child to every
; current child - e.g. a held lamp learns its holder's new nick, not just a
; room's occupants. Callable by any kind, not only room.ma.
(define (broadcast-ctx-to-children!)
  (let loop ((ctxs (child-ctxs)))
    (if (null? ctxs)
        #f
        (begin
          (send-fresh-child-ctx! (ctx-text (car ctxs) "actor"))
          (loop (cdr ctxs))))))

(define (ctx-props-changed! keys)
  (begin
    (announce-node-parent!)
    (broadcast-ctx-to-children!)))

(define (set-node-prop! key value)
  (if (equal? value "")
      (del-prop! key)
      (set-prop! key value))
  (ma-save-state!))

(define (set-node-prop-from-ctx! ctx key)
  (let ((value (ctx-text ctx key)))
    (if (non-empty-string? value)
        (set-init-prop! key value)
        #f)))

; Cached beside the derived name/nick/description fields so a composed actor
; can introspect its parent's own kind/occupants/etc. later (e.g. a room's
; full ctx), not just the trimmed fields node.ma mirrors. Extracted from the
; nested "parent-ctx" field child-ack-ctx embeds in every :child message;
; cleared (not left stale) when a new parent's ack carries none, e.g. a bare
; avatar holder.
(define (parent-ctx) (get-prop "parent-ctx"))

(define (set-parent-ctx! ctx)
  (let ((nested (map-ref ctx "parent-ctx" #f)))
    (if (map? nested)
        (set-prop! "parent-ctx" nested)
        (del-prop! "parent-ctx"))
    (ma-save-state!)))

(define (parent-kind)
  (if (map? (parent-ctx)) (ctx-text (parent-ctx) "kind") ""))

(define (parent-who-map)
  (let ((value (if (map? (parent-ctx)) (map-ref (parent-ctx) "who" #f) #f)))
    (if (map? value) value (make-map))))

; :hold's sole gate: presence in the room, per the cached occupant list -
; not cryptographically authoritative (parent-ctx is unauthenticated data
; the parent chose to hand over), but it is the whole requirement, since
; :hold is deliberately ownership-blind (see handle-node-hold! below). #t
; whenever the parent isn't (cached as) a room, so it never blocks non-room
; holders.
(define (node-same-room-as-parent? did)
  (or (not (equal? (parent-kind) "room"))
      (map-ref (parent-who-map) did #f)))

(define (apply-node-parent-ctx! ctx)
  (begin
    (set-parent-ctx! ctx)
    (set-node-prop-from-ctx! ctx "name")
    (set-node-prop-from-ctx! ctx "nick")
    (set-node-prop-from-ctx! ctx "description")))

(define (node-delegated-did-arg? args msg)
  (and (not (null? args))
       (valid-did? (car args))
       (or (local-actor-ref? (msg-from msg))
           (node-caller-is-parent? msg))))

(define (node-effective-did args msg)
  (if (node-delegated-did-arg? args msg)
      (car args)
      (msg-from msg)))

(define (node-effective-args args msg)
  (if (node-delegated-did-arg? args msg)
      (cdr args)
      args))

(define (node-valid-parent-ref? ref)
  (and (non-empty-string? ref) (valid-did-url? ref)))

(define (node-valid-transfer-ctx? ctx)
  (and (actor-ctx-shape? ctx)
       (not (node-room-ctx? ctx))))

(define (node-owner)
  (get-prop "owner"))

(define (node-owner-did? did)
  (let ((value (node-owner)))
    (and value (equal? value did))))

(define (node-owner-or-unowned? did)
  (let ((value (node-owner)))
    (or (not value) (equal? value did))))

(define (node-caller-is-parent? msg)
  (let ((current (node-parent)))
    (and (non-empty-string? current)
         (same-actor? (msg-from msg) current))))

(define (node-orphan-owner-recovery? did)
  (and (equal? (node-parent) "") (node-owner-did? did)))

(define (node-owner-delegation? did msg)
  (and (node-owner-did? did) (msg-from-owner? did msg)))

(define (node-owner-ref-authorised? actor)
  (and (node-owner)
  (equal? (node-owner) actor)))

; Shared owner/claim/recovery-secret state, identical across thing/container/
; agent — kept here once instead of duplicated per kind file.
(define (owner) (node-owner))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (recovery-secret) (get-prop "recovery-secret"))

(define (set-recovery-secret! secret)
  (if (or (not secret) (equal? secret ""))
      (del-prop! "recovery-secret")
      (set-prop! "recovery-secret" secret))
  (ma-save-state!))

(define (claim-key actor)
  (string-append "claim:" (canonical-actor actor)))

(define (set-claim! actor ctx)
  (if (map? ctx)
      (begin
        (set-prop! (claim-key actor) ctx)
        (ma-save-state!))
      #f))

(define (owner-caller? msg)
  (let ((o (owner)))
    (and o (msg-from-owner? o msg))))

; Shared editable ctx text for ordinary movable nodes.
(define (node-editable-text-prop? key)
  (or (equal? key "name")
      (equal? key "nick")
      (equal? key "description")))

(define (handle-node-text-prop! kind msg args)
  (cond ((not (owner-caller? msg))
         (reply-error msg (string-append "only owner may edit " kind " props")))
        ((null? args)
         (reply-error msg "usage: :prop <name|nick|description> [value]"))
        ((not (node-editable-text-prop? (car args)))
         (reply-error msg (string-append "editable " kind " props: name, nick, description")))
        (else
         (begin
           (set-node-prop! (car args) (join-words (cdr args)))
           (reply-ok-with msg "prop updated")))))

; Shared :owner/:owner?/:set-recovery-secret/:claim bodies — identical across
; thing/container/agent, kept here once instead of duplicated per kind file.
(define (handle-node-owner args msg)
  (reply-ok-with msg (if (owner) (owner) "(none)")))

(define (handle-node-owner? args msg)
  (if (null? args)
      (reply-ok-with msg (string-append "Owner: " (if (owner) (owner) "(none)")))
      (begin
        (ma-send! (canonical-actor (car args)) (list :print (string-append "Owner: " (if (owner) (owner) "(none)"))))
        (reply-ok msg))))

(define (handle-node-set-recovery-secret! args msg)
  (if (owner-caller? msg)
      (begin
        (set-recovery-secret! (if (null? args) "" (join-words args)))
        (reply-ok-with msg "recovery secret updated"))
      (reply-error msg "only owner may set recovery secret")))

(define (handle-node-claim! args msg)
  (let ((stored (recovery-secret))
        (did (claim-caller-did args msg))
        (claim-args (claim-command-args args msg)))
    (cond ((not (valid-did? did))
           (reply-user-error msg did "claim requires a bare DID sender"))
          ((not (or (null? claim-args) (null? (cdr claim-args))))
           (reply-user-error msg did "usage: :claim [secret]"))
          ((and (not (owner)) (not stored) (null? claim-args))
           (begin
             (set-owner! did)
             (reply-user-ok msg did "claimed")))
          ((and (owner) (null? claim-args))
           (reply-user-error msg did "already claimed. Reclaim with :claim <secret>"))
          ((null? claim-args)
           (reply-user-error msg did "usage: :claim <secret>"))
          ((and stored (equal? (car claim-args) stored))
           (begin
             (set-owner! did)
             (set-recovery-secret! "")
             (reply-user-ok msg did "claimed")))
          (else
           (reply-user-error msg did "claim failed")))))

; An unowned node has no owner to defend it, so anyone may propose a
; transfer for it directly — this is what lets a bare `hold` pick up a
; lying-around thing without first requiring :claim or a parent proxy.
; Claiming ownership on a real "take" is avatar.zscheme's job (it calls
; :claim before proposing itself as parent), not a side effect of this gate.
(define (node-transfer-caller-authorised? did msg)
  (or (node-caller-is-parent? msg)
      (node-orphan-owner-recovery? did)
      (node-owner-delegation? did msg)
      (not (node-owner))))

(define (node-recycle-caller-authorised? did msg)
  (and (node-owner-did? did)
       (or (node-caller-is-parent? msg)
           (node-owner-delegation? did msg))))

(define (node-parent-admissible? target-parent)
  (and (valid-did-url? target-parent)
       (not (same-actor? target-parent (self)))
       (not (node-root?))
       (or (not (equal? (node-protocol) "/ma/room/0.0.1"))
           (same-actor? target-parent (root)))))

(define (propose-node-parent! target-parent)
  (let ((target (canonical-actor target-parent)))
    (if (node-parent-admissible? target)
        (begin
          (ma-send! target (list :parent (node-ctx-for-parent target)))
          #t)
        #f)))

; :hold's target parent is always a bare avatar DID, never a DID-URL - a
; deliberately separate admissibility gate from :set-parent's, which keeps
; requiring a DID-URL (ma-lambda-ma-v1.md sec 6).
(define (node-hold-admissible? target)
  (and (valid-did? target)
       (not (same-actor? target (self)))))

(define (propose-node-hold! did)
  (if (node-hold-admissible? did)
      (begin
        (ma-send! did (list :parent (node-ctx-for-parent did)))
        #t)
      #f))

(define (announce-node-parent!)
  (let ((current (node-parent)))
    (if (equal? current "")
        #f
        (ma-send! current (list :parent (node-ctx))))))

; A room repair asks a child with the short existing :report-parent form to
; reannounce through the normal parent handshake. The scheduler's older
; three-argument probe form keeps its :parent-report reply unchanged.
(define (handle-node-report-parent! args msg)
  (cond ((and (not (null? args))
              (null? (cdr args))
              (same-actor? (car args) (node-parent)))
         (announce-node-parent!))
        ((and (not (null? args))
              (not (null? (cdr args)))
              (not (null? (cdr (cdr args))))
              (null? (cdr (cdr (cdr args)))))
         (ma-send! (canonical-actor (msg-from msg))
                   (list :parent-report
                         (canonical-actor (self))
                         (node-parent)
                         (car (cdr args))
                         (car (cdr (cdr args))))))
        (else #f)))

(define (children-map)
  (prop-map "children"))

(define (set-children-map! children)
  (set-prop-map! "children" children))

(define (child-ctx-valid? ctx)
  (and (actor-ctx-shape? ctx)
       (or (valid-did? (ctx-text ctx "actor"))
           (valid-did-url? (ctx-text ctx "actor")))
       (valid-did-url? (ctx-text ctx "parent"))))

(define (child-ctx-self-authentic? ctx msg)
  (and (child-ctx-valid? ctx)
       (ctx-sender-valid? ctx msg)))

; A terminating child (e.g. recycle) has no real new parent to report, so its
; announcement carries an explicitly empty "parent" rather than a DID-URL.
; This is distinct from child-ctx-valid?, which requires a real target parent
; and is used for admission/reparenting.
(define (child-departure-ctx? ctx)
  (and (map? ctx)
       (valid-did-url? (ctx-text ctx "actor"))
       (equal? (ctx-text ctx "parent") "")
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))))

(define (child-ctx-self-authentic-departure? ctx msg)
  (and (child-departure-ctx? ctx)
       (ctx-sender-valid? ctx msg)))

(define (child-ctx-parent-is-self? ctx)
  (same-actor? (ctx-text ctx "parent") (self)))

(define (node-room-ctx? ctx)
  (or (equal? (ctx-text ctx "kind") "room")
      (equal? (ctx-text ctx "protocol") "/ma/room/0.0.1")))

(define (node-root?)
  (same-actor? (self) (root)))

(define (node-child-admissible? ctx)
  (and (not (same-actor? (ctx-text ctx "actor") (self)))
       (or (not (node-room-ctx? ctx)) (node-root?))))

(define (node-orphanable-child-ctx? ctx)
  (or (equal? (ctx-text ctx "kind") "thing")
      (equal? (ctx-text ctx "kind") "agent")
      (equal? (ctx-text ctx "kind") "container")))

(define (node-root-orphan-ctx? ctx msg)
  (let ((actor (ctx-text ctx "actor"))
        (orphan-root (ctx-text ctx "parent")))
    (and (map? ctx)
         (valid-did-url? actor)
         (valid-did-url? orphan-root)
         (equal? (ctx-text ctx "kind") "orphan")
         (equal? (ctx-text ctx "protocol") "/ma/orphan/0.0.1")
         (non-empty-string? (ctx-text ctx "name"))
         (non-empty-string? (ctx-text ctx "nick"))
         (non-empty-string? (ctx-text ctx "description"))
         (same-actor? orphan-root (string-append (actor-runtime actor) "#root"))
         (same-actor? (msg-from msg) orphan-root))))

    (define (node-child-admission-error ctx msg) #f)
    (define (node-children-changed!) #f)
    (define (node-child-departed! ctx) #f)
    (define (node-child-orphaned! ctx) #f)
    (define (node-parent-committed!) #f)
    (define (node-confirmation-stale? ctx) #f)
    (define (node-children-query-error msg) #f)
    (define (node-children-query-authorised? msg) (owner-authorised? msg))
    (define (node-children-query-text) (children-text))

; "max-children" is a plain prop, defaulting to 20 when unset or unparseable.
(define (node-max-children)
  (let* ((value (get-prop "max-children"))
         (parsed (if (non-empty-string? value) (string->number value) #f)))
    (if (number? parsed) parsed 20)))

(define (node-children-count)
  (list-length (child-ctxs)))

; Overridable per kind (ma-lambda-ma-v1.md §7's "local policy reason", e.g. a
; room enforcing an occupancy limit); default is the generic max-children cap.
(define (node-forge-admission-error ctx msg)
  (if (>= (node-children-count) (node-max-children))
      "forge refused: max-children limit reached"
      #f))

(define (remember-child! ctx)
  (set-children-map!
    (map-set (children-map)
             (canonical-actor (ctx-text ctx "actor"))
             ctx)))

(define (forget-child! actor)
  (set-children-map!
    (map-delete (children-map) (canonical-actor actor))))

(define (child-ctx actor)
  (map-ref (children-map) (canonical-actor actor) #f))

(define (child-ctxs)
  (map-values (children-map)))

(define (prune-dead-local-children!)
  (let loop ((ctxs (child-ctxs)) (changed #f))
    (if (null? ctxs)
        (begin
          (if changed
              (begin
                (ma-save-state!)
                (node-children-changed!))
              #f)
          (child-ctxs))
        (let ((actor (ctx-text (car ctxs) "actor")))
          (if (dead-local-actor? actor)
              (begin
                (forget-child! actor)
                (loop (cdr ctxs) #t))
              (loop (cdr ctxs) changed))))))

(define (child-ctxs-by-kind kind)
  (let loop ((ctxs (child-ctxs)) (acc '()))
    (cond ((null? ctxs) (reverse acc))
          ((equal? (ctx-text (car ctxs) "kind") kind)
           (loop (cdr ctxs) (cons (car ctxs) acc)))
          (else
           (loop (cdr ctxs) acc)))))

(define (child-actors-by-kind kind)
  (let loop ((ctxs (child-ctxs-by-kind kind)) (acc '()))
    (if (null? ctxs)
        (reverse acc)
        (loop (cdr ctxs)
              (cons (canonical-actor (ctx-text (car ctxs) "actor")) acc)))))

(define (child-label ctx)
  (let ((nick (ctx-text ctx "nick"))
        (name (ctx-text ctx "name"))
        (actor (ctx-text ctx "actor")))
    (cond ((non-empty-string? nick) nick)
          ((non-empty-string? name) name)
          (else actor))))

(define (child-line entry)
  (string-append (child-label (cdr entry))
                 " = "
                 (canonical-actor (car entry))))

(define (actor-entry-lines xs)
  (cond ((null? xs) "")
        ((null? (cdr xs)) (car xs))
        (else (string-append (car xs) "\n" (actor-entry-lines (cdr xs))))))

(define (child-lines entries)
  (if (null? entries)
      '()
      (cons (child-line (car entries)) (child-lines (cdr entries)))))

(define (children-text)
  (let ((lines (child-lines (map->alist (children-map)))))
    (if (null? lines)
        "Children: none."
        (string-append "Children:\n" (actor-entry-lines lines)))))

(define (child-token-text-matches? token text)
  (and (non-empty-string? token)
       (non-empty-string? text)
       (equal? (string-downcase token) (string-downcase text))))

(define (child-token-matches? token ctx)
  (and (non-empty-string? token)
       (or (same-actor? token (ctx-text ctx "actor"))
           (child-token-text-matches? token (child-label ctx))
           (child-token-text-matches? token (ctx-text ctx "name"))
           (child-token-text-matches? token (ctx-text ctx "nick")))))

(define (child-ref token)
  (if (valid-did-url? token)
      (if (child-ctx token) (canonical-actor token) #f)
      (let loop ((entries (map->alist (children-map))))
        (cond ((null? entries) #f)
              ((child-token-matches? token (cdr (car entries)))
               (canonical-actor (car (car entries))))
              (else (loop (cdr entries)))))))

(define (handle-stale-parent-drop did rest msg)
  (if (or (null? rest)
          (null? (cdr rest))
          (not (null? (cdr (cdr rest)))))
      #f
      (let ((target-parent (car rest))
            (ctx (car (cdr rest)))
            (caller (msg-from msg)))
        (if (and (node-owner-did? did)
                 (node-valid-parent-ref? target-parent)
                 (node-valid-transfer-ctx? ctx)
                 (same-actor? (ctx-text ctx "actor") (self))
                 (same-actor? (ctx-text ctx "parent") caller)
                 (same-actor? target-parent (node-parent))
                 (not (same-actor? caller (node-parent))))
            (begin
              (ma-send! (canonical-actor caller) (list :parent (node-ctx)))
              (propose-node-parent! target-parent)
              (reply-ok-with msg "drop requested")
              #t)
            #f))))

; Shared body for the single :set-parent trigger (thing/container/agent):
; propose self to a new parent. Sent directly to the actor being relocated,
; never relayed by a third party — the caller must already be authorised.
; Parenting is not ownership: node-transfer-caller-authorised? is the only
; authority gate here (current parent, orphan-owner recovery, owner
; delegation, or unowned). Whoever currently holds/carries a thing may
; relocate it regardless of who node-owner says owns it — you can carry (and
; drop) another's property; ownership only matters for :claim/:lock/:owner.
(define (handle-node-set-parent! args msg)
  (let ((did (node-effective-did args msg))
        (rest (node-effective-args args msg)))
    (cond ((handle-stale-parent-drop did rest msg) #t)
          ((not (node-transfer-caller-authorised? did msg))
           (reply-error msg "set-parent must be requested by current parent"))
          ((not (valid-did? did))
           (reply-error msg "set-parent requires DID with did:ma: prefix"))
          ((null? rest)
           (reply-error msg "usage: :set-parent <target-parent-did-url> [ctx-map]"))
          ((not (node-valid-parent-ref? (car rest)))
           (reply-error msg "set-parent requires target parent as DID-URL"))
          ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
           (reply-error msg "set-parent accepts at most one optional ctx-map"))
          ((and (not (null? (cdr rest))) (not (node-valid-transfer-ctx? (car (cdr rest)))))
           (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
          (else
           (begin
             (if (and (not (null? (cdr rest))) (node-valid-transfer-ctx? (car (cdr rest))))
                 (set-claim! did (car (cdr rest)))
                 #f)
             (propose-node-parent! (car rest))
             (reply-ok-with msg "set-parent requested"))))))

; :hold implicitly targets the caller (a bare avatar DID, msg-from) as new
; parent - unlike :set-parent, no argument is accepted at all. This is the
; wire verb a client's `hold`/`take`/`take-from` send (ma-zion's
; inbox_poll.rs replies :child to confirm the resulting unsolicited
; :parent proposal). Shared body lives in node.ma; registered by thing.ma/
; container.ma/agent.ma only — rooms are not holdable.
;
; Ownership plays no part here at all: anyone (or anything) present in the
; same room may hold any item regardless of who owns it, and holding never
; assigns or changes ownership as a side effect. Ownership is set only by
; the explicit :claim verb (handle-node-claim!).
(define (handle-node-hold! args msg)
  (let ((did (msg-from msg)))
    (cond ((not (null? args))
           (reply-error msg "usage: :hold"))
          ((not (valid-did? did))
           (reply-error msg "hold requires caller with did:ma: prefix, not a DID-URL"))
          ((not (node-same-room-as-parent? did))
           (begin
             (announce-node-parent!)
             (reply-error msg "hold refused: not in the same room, try again")))
          (else
           (begin
             (propose-node-hold! did)
             (reply-ok-with msg "hold requested"))))))

(define (handle-node-parent args msg)
  (cond ((null? args)
         (reply-ok-with msg (if (equal? (node-parent) "") "(none)" (node-parent))))
        ((not (null? (cdr args)))
         (reply-error msg "usage: :parent [ctx]"))
        ((child-departure-ctx? (car args))
         (if (not (child-ctx-self-authentic-departure? (car args) msg))
             (reply-error msg "parent ctx actor must match sender")
             (begin
               (if (child-ctx (ctx-text (car args) "actor"))
                   (node-child-departed! (car args))
                   #f)
               (forget-child! (ctx-text (car args) "actor"))
               (node-children-changed!)
               (reply-ok msg))))
        ((node-root-orphan-ctx? (car args) msg)
         (let ((existing (child-ctx (ctx-text (car args) "actor"))))
           (cond ((not existing)
                  (reply-ok msg))
                 ((not (node-orphanable-child-ctx? existing))
                  (reply-error msg "root may orphan only thing, agent, or container children"))
                 (else
                  (begin
                    (forget-child! (ctx-text (car args) "actor"))
                      (node-child-orphaned! existing)
                    (node-children-changed!)
                    (reply-ok msg))))))
                  ((not (child-ctx-valid? (car args)))
                   (reply-error msg "parent ctx must include actor, parent, kind, protocol, name, nick, description"))
        ((not (child-ctx-self-authentic? (car args) msg))
         (reply-error msg "parent ctx actor must match sender"))
        ((node-child-admission-error (car args) msg)
         (reply-error msg (node-child-admission-error (car args) msg)))
        ((not (node-child-admissible? (car args)))
         (reply-error msg "parent ctx is not admissible"))
        ((not (child-ctx-parent-is-self? (car args)))
         (begin
           (if (child-ctx (ctx-text (car args) "actor"))
               (node-child-departed! (car args))
               #f)
           (forget-child! (ctx-text (car args) "actor"))
           (node-children-changed!)
           (reply-ok msg)))
        (else
         (begin
           (remember-child! (car args))
           (ma-send! (canonical-actor (ctx-text (car args) "actor"))
                     (list :child (child-ack-ctx (car args))))
           (node-children-changed!)
           (reply-ok msg)))))

(define (handle-node-parent? args msg)
  (if (null? args)
      (reply-ok-with msg (if (equal? (node-parent) "") "(none)" (node-parent)))
      (reply-error msg "usage: :parent?")))

(define (handle-node-children? args msg)
  (cond ((not (null? args))
         (reply-error msg "usage: :children?"))
        ((not (owner-authorised? msg))
         (reply-error msg "only owner may inspect children"))
        (else
         (reply-ok-with msg (children-map)))))

(define (handle-node-orphan! args msg)
  (cond ((or (null? args) (not (null? (cdr args))))
         (reply-error msg "usage: :orphan <actor>"))
        ((not (valid-did-url? (car args)))
         (reply-error msg "orphan actor must be a DID-URL"))
        ((not (owner-authorised? msg))
         (reply-error msg "only owner may orphan a child"))
        (else
         (let ((existing (child-ctx (car args))))
           (cond ((not existing)
                  (reply-ok msg))
                 ((not (node-orphanable-child-ctx? existing))
                  (reply-error msg "only thing, agent, or container children may be orphaned"))
                 (else
                  (begin
                    (forget-child! (car args))
                      (node-child-orphaned! existing)
                    (node-children-changed!)
                    (reply-ok msg))))))))

(define (handle-node-root-orphan! args msg)
  (cond ((or (null? args)
             (null? (cdr args))
             (not (null? (cdr (cdr args)))))
         (reply-error msg "usage: :orphan-root <owner> <old-parent>"))
        ((not (same-actor? (msg-from msg) (root)))
         (reply-error msg "only root may request orphan adoption"))
        ((not (node-orphanable-child-ctx? (node-ctx)))
         (reply-error msg "only thing, agent, or container may be orphaned"))
        ((not (node-owner-ref-authorised? (car args)))
         (reply-error msg "only owner may orphan this actor"))
        ((not (valid-did-url? (car (cdr args))))
         (reply-error msg "orphan parent must be a DID-URL"))
        ((not (same-actor? (node-parent) (car (cdr args))))
         (reply-error msg "orphan parent does not match actor parent"))
        (else
         (begin
           (propose-node-parent! (root))
           (reply-ok msg)))))

(define (node-confirmation-valid? ctx msg)
  (let ((target-parent (ctx-text ctx "parent")))
    (and (actor-ctx-shape? ctx)
         (same-actor? (ctx-text ctx "actor") (self))
         (same-actor? (msg-from msg) target-parent)
         (or (node-parent-admissible? target-parent)
             (node-hold-admissible? target-parent)))))

(define (commit-node-parent! ctx)
  (let ((old-parent (node-parent))
        (new-parent (canonical-actor (ctx-text ctx "parent"))))
    (begin
      (apply-node-parent-ctx! ctx)
      (set-node-parent! new-parent)
      (node-parent-committed!)
      (announce-node-parent!)
      (if (and (non-empty-string? old-parent)
               (not (same-actor? old-parent new-parent)))
          (ma-send! old-parent (list :parent (node-ctx)))
          #f))))

(define (handle-node-child args msg)
  (cond ((null? args)
    (cond ((node-children-query-error msg)
      (reply-error msg (node-children-query-error msg)))
     ((node-children-query-authorised? msg)
      (reply-ok-with msg (node-children-query-text)))
     (else
      (reply-error msg "only owner may list children"))))
        ((not (null? (cdr args)))
         (reply-error msg "usage: :child [ctx]"))
        ((not (node-confirmation-valid? (car args) msg))
         (reply-error msg "child ctx must name self and come from target parent"))
          ((node-confirmation-stale? (car args))
           (reply-ok msg))
        ((and (same-actor? (node-parent) (ctx-text (car args) "parent"))
              (equal? (node-ctx) (map-delete (car args) "parent-ctx")))
         (begin (set-parent-ctx! (car args)) (reply-ok msg)))
        (else
         (begin
           (commit-node-parent! (car args))
           (reply-ok msg)))))

(set-meta-method! :parent handle-node-parent)
(set-rpc-method! :parent? handle-node-parent?)
(set-rpc-method! :children? handle-node-children?)
(set-meta-method! :child handle-node-child)
(set-internal-rpc-method! :orphan-root handle-node-root-orphan!)

; ma-lambda-ma-v1.md §7: forge ctx MUST have kind/name, MUST NOT carry an
; owner or caller-supplied init — owner is always msg-from, parent is self.
(define (forge-ctx-valid? ctx)
  (and (map? ctx)
       (non-empty-string? (ctx-text ctx "kind"))
       (non-empty-string? (ctx-text ctx "name"))))

(define (forge-init name owner-did)
  (string-append
    "(set-init-prop! \"name\" \"" name "\")\n"
    "(set-init-prop! \"owner\" \"" owner-did "\")\n"
    "(set-init-prop! \"parent\" \"" (canonical-actor (self)) "\")\n"
    "(ma-save-state!)\n"))

(define (handle-node-forge args msg)
  (let ((ctx (if (pair? args) (car args) #f)))
    (cond ((not (forge-ctx-valid? ctx))
           (reply-error msg "usage: :forge <ctx with kind, name>"))
          ((node-forge-admission-error ctx msg)
           (reply-error msg (node-forge-admission-error ctx msg)))
          (else
           (let* ((owner (canonical-actor (msg-from msg)))
                  (behaviour (ctx-text ctx "behaviour"))
                (actor (ma-create-actor (ctx-text ctx "kind")
                                (if (non-empty-string? behaviour) behaviour #f)
                                (forge-init (ctx-text ctx "name") owner)
                                #f)))
             (reply-ok-with msg actor))))))

(set-rpc-method! :forge handle-node-forge)