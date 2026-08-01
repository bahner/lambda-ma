; Generic free movable Scheme agent.
; Concrete agents extend this behaviour and keep their own parent state.

(define AGENT_PROTOCOL "/ma/scheme/agent/0.0.1")

; Persistent state accessors.
(define (owner) (get-prop "owner"))
(define (parent)
  (let ((p (get-prop "parent")))
    (if p p "")))

(define (name)
  (let ((n (get-prop "name")))
    (if n n "agent")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable agent.")))

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (recovery-secret) (get-prop "recovery-secret"))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (set-parent! did)
  (set-prop! "parent" (canonical-actor did))
  (del-prop! "pending-room")
  (ma-save-state!))

(define (pending-room)
  (let ((p (get-prop "pending-room")))
    (if p p "")))

(define (movement-pending?)
  (not (equal? (pending-room) "")))

(define (set-pending-room! room)
  (set-prop! "pending-room" room)
  (ma-save-state!))

(define (set-last-message! text)
  (set-prop! "last-message" text)
  (ma-save-state!))

(define (announce-parent!)
  (let ((p (parent)))
    (if (equal? p "")
        #f
        (ma-send! (canonical-actor p) (list :child (agent-ctx))))))

(define (enter room)
  (begin
    (set-pending-room! (canonical-actor room))
    (ma-send! (canonical-actor room) (list :enter (agent-ctx-for-parent room)))))

(define (leave-current-parent!)
  (let ((p (parent)))
    (if (equal? p "")
        #f
        (ma-send! (canonical-actor p) (list :leave-occupant)))))

(define (claim-key actor)
  (string-append "claim:" (canonical-actor actor)))

(define (set-claim! actor ctx)
  (if (map? ctx)
      (begin
        (set-prop! (claim-key actor) ctx)
        (ma-save-state!))
      #f))

(define (set-recovery-secret! secret)
  (if (or (not secret) (equal? secret ""))
      (del-prop! "recovery-secret")
      (set-prop! "recovery-secret" secret))
  (ma-save-state!))

; Caller and reply helpers.
(define (owner-caller? msg)
  (let ((o (owner)))
    (and o (equal? (msg-from msg) o))))

(define (owner-did? did)
  (let ((o (owner)))
    (and o (equal? did o))))

(define (movement-caller? msg)
  (or
  (not (owner))
  (owner-caller? msg)))

(define (delegated-did-arg? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (local-actor-caller? msg)
  (local-actor-ref? (msg-from msg)))

(define (effective-did args msg)
  (if (and (delegated-did-arg? args) (local-actor-caller? msg))
      (car args)
      (msg-from msg)))

(define (effective-args args msg)
  (if (and (delegated-did-arg? args) (local-actor-caller? msg))
      (cdr args)
      args))

(define (owner-or-unowned? did)
  (let ((o (owner)))
    (or (not o) (equal? o did))))

(define (ctx-alist-ref ctx key)
  (cond ((null? ctx) #f)
        ((and (pair? (car ctx))
              (equal? (car (car ctx)) key)
              (pair? (cdr (car ctx))))
         (car (cdr (car ctx))))
        (else (ctx-alist-ref (cdr ctx) key))))

(define (valid-move-ctx? ctx)
  (and (map? ctx)
       (same-actor? (ctx-text ctx "actor") (self))
       (equal? (ctx-text ctx "kind") "agent")
       (valid-parent-ref? (ctx-text ctx "room"))))

      ; Transfer validation keeps take/drop strict at the room boundary.
(define (valid-room-ctx? ctx)
  (and (pair? ctx)
       (equal? (ctx-alist-ref ctx :protocol) LAMBDA_CTX_PROTOCOL)
       (equal? (ctx-alist-ref ctx :kind) "agent")
       (non-empty-string? (ctx-alist-ref ctx :room))))

(define (authorised-room-ctx? room)
  (or (same-actor? room (pending-room))
      (same-actor? room (parent))))

(define (valid-parent-ref? ref)
  (and (non-empty-string? ref)
       (or (valid-did-url? ref)
           (local-actor-ref? ref))))

(define (valid-transfer-kind? kind)
  (or (equal? kind "avatar")
      (equal? kind "thing")
  (equal? kind "agent")
  (equal? kind "container")
  (equal? kind "actor")))

(define (valid-transfer-ctx? ctx)
  (and (actor-ctx? ctx)
       (valid-transfer-kind? (ctx-text ctx "kind"))))

(define (caller-is-parent? msg)
  (let ((p (parent)))
    (and (not (equal? p "")) (same-actor? (msg-from msg) p))))

(define (orphan-owner-recovery? did)
  (and (equal? (parent) "") (owner-did? did)))

(define (transfer-caller-authorised? did msg)
  (or (caller-is-parent? msg) (orphan-owner-recovery? did)))

; Room context and movement helpers.
(define (agent-ctx-for-parent target-parent)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (canonical-actor (self)))
              "kind" "agent")
            "protocol" AGENT_PROTOCOL)
          "parent" (canonical-actor target-parent))
        "name" (name))
      "nick" (nick))
    "description" (description)))

(define (agent-ctx)
  (agent-ctx-for-parent (parent)))

(define (parent-target-from-ctx ctx)
  (let ((target (ctx-text ctx "parent")))
    (if (non-empty-string? target)
        target
        (ctx-text ctx "room"))))

(define (valid-parent-ctx? ctx)
  (let ((target-parent (parent-target-from-ctx ctx)))
    (and (map? ctx)
         (valid-parent-ref? target-parent))))

(define (set-prop-from-ctx! ctx key)
  (let ((value (ctx-text ctx key)))
    (if (non-empty-string? value)
        (set-prop! key value)
        #f)))

(define (apply-parent-ctx! ctx)
  (let ((old-parent (parent))
        (target-parent (parent-target-from-ctx ctx)))
  (begin
    (set-parent! target-parent)
    (set-prop-from-ctx! ctx "name")
    (set-prop-from-ctx! ctx "nick")
    (set-prop-from-ctx! ctx "description")
    (ma-save-state!)
    (if (and (non-empty-string? old-parent)
             (not (same-actor? old-parent target-parent)))
        (ma-send! (canonical-actor old-parent) (list :child (agent-ctx)))
        #f))))

(define (propose-parent-change! target-parent)
  (ma-send! (canonical-actor target-parent) (list :child (agent-ctx-for-parent target-parent))))

(define (send-parent-room! msg term)
  (let ((p (parent)))
    (if (equal? p "")
        (reply-error msg (string-append (nick) " is nowhere"))
        (begin
          (ma-send! (canonical-actor p) term)
          (reply-ok msg)))))

(define (move-to-room! target-room source-room)
  (if (and (not (movement-pending?))
           (same-actor? source-room (parent)))
      (begin
        (enter (canonical-actor target-room)))
      #f))

(define (agent-go! args msg)
  (cond ((movement-pending?)
         (reply-error msg "movement already pending"))
        ((movement-caller? msg)
         (send-parent-room! msg (cons :go args)))
        (else
         (reply-error msg "only a free agent or owner may move this agent"))))

; Public methods.
(set-rpc-method! :about
  (lambda (args msg)
    (reply-ok-with msg
      (string-append
        (name) "\n"
        (description) "\n"
        "owner: " (if (owner) (owner) "(none)") "\n"
        "parent: " (if (equal? (parent) "") "(none)" (parent))))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (equal? (parent) "") "(none)" (parent)))))

(set-internal-rpc-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args))))))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (canonical-actor (self)) (parent) tick nonce)))))

(set-rpc-method! :owner
  (lambda (args msg)
    (reply-ok-with msg (if (owner) (owner) "(none)"))))

(set-rpc-method! :owner?
  (lambda (args msg)
    (if (null? args)
        (reply-ok-with msg (string-append "Owner: " (if (owner) (owner) "(none)")))
        (begin
          (ma-send! (canonical-actor (car args)) (list :print (string-append "Owner: " (if (owner) (owner) "(none)"))))
          (reply-ok msg)))))

(set-internal-rpc-method! :print
  (lambda (args msg)
    (set-last-message! (join-words args))))

(set-rpc-method! :exits?
  (lambda (args msg)
    (send-parent-room! msg (list :exits?))))

(set-cmd-method! :go
  (lambda (args msg)
    (agent-go! args msg)))

(set-cmd-method! :move
  (lambda (args msg)
    (cond ((movement-pending?)
           (reply-error msg "movement already pending"))
          ((movement-caller? msg)
           (send-parent-room! msg (list :move)))
          (else
           (reply-error msg "only a free agent or owner may move this agent")))))

(set-internal-rpc-method! :enter-room
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)))
        #f
        (move-to-room! (car args) (car (cdr args))))))

(set-internal-rpc-method! :ctx
  (lambda (args msg)
    (if (null? args)
        (if (or (owner-caller? msg) (caller-is-parent? msg))
            (ma-reply! msg (list :ok (agent-ctx)))
            #f)
           (let ((ctx (car args)))
          (cond
            ((and (valid-room-ctx? ctx)
               (same-actor? (msg-from msg) (ctx-alist-ref ctx :room))
               (authorised-room-ctx? (ctx-alist-ref ctx :room)))
             (let ((old-parent (parent))
                   (target-parent (ctx-alist-ref ctx :room)))
               (begin
                 (set-parent! target-parent)
                 (if (and (non-empty-string? old-parent)
                          (not (same-actor? old-parent target-parent)))
                     (ma-send! (canonical-actor old-parent) (list :child (agent-ctx)))
                     #f))))
            ((and (valid-move-ctx? ctx)
                  (or (caller-is-parent? msg) (owner-caller? msg)))
             (begin
               (if (non-empty-string? (ctx-text ctx "text"))
                 (set-last-message! (ctx-text ctx "text"))
                 #f)
               (if (same-actor? (ctx-text ctx "room") (parent))
                 #f
                 (move-to-room! (ctx-text ctx "room") (msg-from msg)))))
            (else #f))))))

(set-rpc-method! :set-recovery-secret
  (lambda (args msg)
    (if (owner-caller? msg)
        (begin
          (set-recovery-secret! (if (null? args) "" (join-words args)))
          (reply-ok-with msg "recovery secret updated"))
        (reply-error msg "only owner may set recovery secret"))))

(set-cmd-method! :claim
  (lambda (args msg)
    (let ((stored (recovery-secret))
          (did (msg-from msg)))
      (cond ((and (not (owner)) (not stored) (null? args))
             (begin
               (set-owner! did)
               (reply-ok-with msg "claimed")))
            ((null? args)
             (reply-error msg "usage: :claim <secret>"))
            ((and stored (equal? (car args) stored))
             (begin
               (set-owner! did)
               (set-recovery-secret! "")
               (reply-ok-with msg "claimed")))
            (else
             (reply-error msg "claim failed"))))))

; Parent-mediated transfer. The parent room asks the agent to bind to a did
; or re-enter another parent; direct did calls are deliberately rejected.
(set-cmd-method! :take
  (lambda (args msg)
    (let ((did (effective-did args msg))
          (rest (effective-args args msg)))
  (cond ((handle-parent-take did rest msg) #t)
    ((not (transfer-caller-authorised? did msg))
             (reply-error msg "take must be requested by current parent"))
            ((not (valid-did? did))
             (reply-error msg "take requires DID with did:ma: prefix"))
            ((not (owner-or-unowned? did))
             (reply-error msg "only owner may take this agent"))
            ((null? rest)
             (reply-error msg "usage: :take <did> <carrier-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "take requires carrier parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "take accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (let ((target-parent (car rest)))
               (if (not (owner)) (set-owner! did) #f)
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
               (propose-parent-change! target-parent)
                 (reply-ok-with msg "take requested")))))))

(set-cmd-method! :drop
  (lambda (args msg)
    (let ((did (effective-did args msg))
          (rest (effective-args args msg)))
      (cond ((not (transfer-caller-authorised? did msg))
             (reply-error msg "drop must be requested by current parent"))
            ((not (valid-did? did))
             (reply-error msg "drop requires DID with did:ma: prefix"))
            ((not (owner-or-unowned? did))
             (reply-error msg "only owner may drop this agent"))
            ((null? rest)
             (reply-error msg "usage: :drop <target-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "drop requires target parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "drop accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (let ((target-parent (car rest)))
               (if (not (owner)) (set-owner! did) #f)
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
               (enter target-parent)
                 (reply-ok-with msg "drop requested")))))))

(set-meta-method! :parent
  (lambda (args msg)
    (cond ((null? args)
           (if (owner-caller? msg)
               (reply-ok-with msg (if (equal? (parent) "") "(none)" (parent)))
               (reply-error msg "only owner may inspect parent")))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :parent [ctx]"))
          ((not (valid-parent-ctx? (car args)))
           (reply-error msg "parent ctx must include target parent"))
          ((not (same-actor? (msg-from msg) (parent-target-from-ctx (car args))))
           (reply-error msg "parent ctx must come from target parent"))
          (else
           (begin
             (apply-parent-ctx! (car args))
             (reply-ok msg))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (announce-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
