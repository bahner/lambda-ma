; Generic free movable Scheme agent.
; Concrete agents extend this behaviour and keep their own parent state.

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

(define (enter room)
  (begin
    (set-pending-room! (canonical-actor room))
    (ma-send! (canonical-actor room) (list :enter (agent-ctx)))))

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

(define (movement-caller? msg)
  (or
  (not (owner))
  (owner-caller? msg)))

(define (delegated-user-arg? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (local-actor-caller? msg)
  (local-actor-ref? (msg-from msg)))

(define (effective-user args msg)
  (if (and (delegated-user-arg? args) (local-actor-caller? msg))
      (car args)
      (msg-from msg)))

(define (effective-args args msg)
  (if (and (delegated-user-arg? args) (local-actor-caller? msg))
      (cdr args)
      args))

(define (owner-or-unowned? user)
  (let ((o (owner)))
    (or (not o) (equal? o user))))

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
       (or (did-url? ref)
           (local-actor-ref? ref))))

(define (valid-transfer-kind? kind)
  (or (equal? kind "avatar")
      (equal? kind "thing")
      (equal? kind "agent")))

(define (valid-transfer-ctx? ctx)
  (and (actor-ctx? ctx)
       (valid-transfer-kind? (ctx-text ctx "kind"))))

(define (caller-is-parent? msg)
  (let ((p (parent)))
    (and (not (equal? p "")) (same-actor? (msg-from msg) p))))

; Room context and movement helpers.
(define (agent-ctx)
  (map-set
    (map-set
      (map-set
        (map-set (make-map) "kind" "agent")
        "name" (name))
      "nick" (nick))
    "description" (description)))

(define (send-parent-room! msg term)
  (let ((p (parent)))
    (if (equal? p "")
        (reply-error msg (string-append (nick) " is nowhere"))
        (begin
          (ma-send! (canonical-actor p) term)
          (reply-ok msg "")))))

(define (move-to-room! target-room source-room)
  (if (and (not (movement-pending?))
           (same-actor? source-room (parent)))
      (begin
        (leave-current-parent!)
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
(set-method! :about
  (lambda (args msg)
    (reply-ok msg
      (string-append
        (name) "\n"
        (description) "\n"
        "owner: " (if (owner) (owner) "(none)") "\n"
        "parent: " (if (equal? (parent) "") "(none)" (parent))))))

(set-method! :where
  (lambda (args msg)
    (reply-ok msg (if (equal? (parent) "") "(none)" (parent)))))

(set-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args))))))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (canonical-actor (self)) (parent) tick nonce)))))

(set-method! :owner
  (lambda (args msg)
    (reply-ok msg (if (owner) (owner) "(none)"))))

(set-method! :print
  (lambda (args msg)
    (set-last-message! (join-words args))))

(set-method! :exits?
  (lambda (args msg)
    (send-parent-room! msg (list :exits?))))

(set-method! :go
  (lambda (args msg)
    (agent-go! args msg)))

(set-method! :move
  (lambda (args msg)
    (cond ((movement-pending?)
           (reply-error msg "movement already pending"))
          ((movement-caller? msg)
           (send-parent-room! msg (list :move)))
          (else
           (reply-error msg "only a free agent or owner may move this agent")))))

(set-method! :enter-room
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)))
        #f
        (move-to-room! (car args) (car (cdr args))))))

(set-method! :ctx
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
             (set-parent! (ctx-alist-ref ctx :room)))
            ((and (valid-move-ctx? ctx)
                  (or (caller-is-parent? msg) (owner-caller? msg)))
             (if (same-actor? (ctx-text ctx "room") (parent))
                 (set-last-message! (ctx-text ctx "text"))
                 (move-to-room! (ctx-text ctx "room") (msg-from msg))))
            (else #f))))))

(set-method! :set-recovery-secret
  (lambda (args msg)
    (if (owner-caller? msg)
        (begin
          (set-recovery-secret! (if (null? args) "" (join-words args)))
          (reply-ok msg "recovery secret updated"))
        (reply-error msg "only owner may set recovery secret"))))

(set-method! :claim
  (lambda (args msg)
    (let ((stored (recovery-secret))
          (user (msg-from msg)))
      (cond ((and (not (owner)) (not stored) (null? args))
             (begin
               (set-owner! user)
               (reply-ok msg "claimed")))
            ((null? args)
             (reply-error msg "usage: :claim <secret>"))
            ((and stored (equal? (car args) stored))
             (begin
               (set-owner! user)
               (set-recovery-secret! "")
               (reply-ok msg "claimed")))
            (else
             (reply-error msg "claim failed"))))))

; Parent-mediated transfer. The parent room asks the agent to bind to a user
; or re-enter another parent; direct user calls are deliberately rejected.
(set-method! :take
  (lambda (args msg)
    (let ((user (effective-user args msg))
          (rest (effective-args args msg)))
      (cond ((not (caller-is-parent? msg))
             (reply-error msg "take must be requested by current parent"))
            ((not (valid-user-did? user))
             (reply-error msg "take requires user DID with did:ma: prefix"))
            ((not (owner-or-unowned? user))
             (reply-error msg "only owner may take this agent"))
            ((null? rest)
             (reply-error msg "usage: :take <user-did> <carrier-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "take requires carrier parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "take accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty kind, name, nick, description"))
            (else
             (let ((target-parent (car rest)))
               (if (not (owner)) (set-owner! user) #f)
               (leave-current-parent!)
               (set-parent! target-parent)
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! user (car (cdr rest)))
                   #f)
               (reply-ok msg "taken")))))))

(set-method! :drop
  (lambda (args msg)
    (let ((user (effective-user args msg))
          (rest (effective-args args msg)))
      (cond ((not (caller-is-parent? msg))
             (reply-error msg "drop must be requested by current parent"))
            ((not (valid-user-did? user))
             (reply-error msg "drop requires user DID with did:ma: prefix"))
            ((not (owner-or-unowned? user))
             (reply-error msg "only owner may drop this agent"))
            ((null? rest)
             (reply-error msg "usage: :drop <target-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "drop requires target parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "drop accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty kind, name, nick, description"))
            (else
             (let ((target-parent (car rest)))
               (if (not (owner)) (set-owner! user) #f)
               (enter target-parent)
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! user (car (cdr rest)))
                   #f)
               (reply-ok msg "dropped")))))))
