; Generic free movable Scheme agent.
; Concrete agents extend this behaviour and keep their own parent state.

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define LAMBDA_CTX_PROTOCOL "/ma/lambda/ctx/0.0.1")
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor)) (string-append (runtime) actor) actor))

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

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
  (set-prop! "parent" did)
  (del-prop! "pending-room")
  (ma-save-state!))

(define (pending-room)
  (let ((p (get-prop "pending-room")))
    (if p p "")))

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
        (ma-send! p (list :leave-occupant)))))

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

(define (owner-caller? msg)
  (let ((o (owner)))
    (and o (equal? (msg-from msg) o))))

(define (movement-caller? msg)
  (or
  (not (owner))
  (owner-caller? msg)))

(define (reply-ok msg text)
  (ma-reply! msg (list :ok text)))

(define (reply-error msg text)
  (ma-reply! msg (list :error text)))

(define (delegated-user-arg? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (local-actor-caller? msg)
  (string-prefix? "#" (msg-from msg)))

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

(define (non-empty-string? v)
  (and (string? v) (not (equal? v ""))))

(define (ctx-text ctx key)
  (let ((v (map-ref ctx key #f)))
    (if (string? v) v #f)))

(define (ctx-alist-ref ctx key)
  (cond ((null? ctx) #f)
        ((and (pair? (car ctx))
              (equal? (car (car ctx)) key)
              (pair? (cdr (car ctx))))
         (car (cdr (car ctx))))
        (else (ctx-alist-ref (cdr ctx) key))))

(define (valid-room-ctx? ctx)
  (and (pair? ctx)
       (equal? (ctx-alist-ref ctx :protocol) LAMBDA_CTX_PROTOCOL)
       (equal? (ctx-alist-ref ctx :kind) "agent")
       (non-empty-string? (ctx-alist-ref ctx :room))))

(define (authorised-room-ctx? room)
  (or (same-actor? room (pending-room))
      (same-actor? room (parent))))

(define (valid-user-did? did)
  (and (string? did)
       (string-prefix? "did:ma:" did)))

(define (valid-parent-ref? ref)
  (and (non-empty-string? ref)
       (or (string-prefix? "did:ma:" ref)
           (string-prefix? "#" ref))))

(define (valid-transfer-kind? kind)
  (or (equal? kind "avatar")
      (equal? kind "thing")
      (equal? kind "agent")))

(define (valid-transfer-ctx? ctx)
  (and (map? ctx)
       (non-empty-string? (ctx-text ctx "kind"))
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))
       (valid-transfer-kind? (ctx-text ctx "kind"))))

(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))

(define (caller-is-parent? msg)
  (let ((p (parent)))
    (and (not (equal? p "")) (same-actor? (msg-from msg) p))))

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
          (ma-send! p term)
          (reply-ok msg "queued")))))

(define (move-to-room! target-room source-room)
  (if (same-actor? source-room (parent))
      (begin
        (leave-current-parent!)
        (enter (canonical-actor target-room)))
      #f))

(define (agent-go! args msg)
  (if (movement-caller? msg)
      (send-parent-room! msg (cons :go args))
      (reply-error msg "only a free agent or owner may move this agent")))

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
    (if (movement-caller? msg)
        (send-parent-room! msg (list :move))
        (reply-error msg "only a free agent or owner may move this agent"))))

(set-method! :enter-room
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)))
        #f
        (move-to-room! (car args) (car (cdr args))))))

(set-method! :ctx
  (lambda (args msg)
    (if (null? args)
        #f
        (let* ((ctx (car args))
               (room (ctx-alist-ref ctx :room)))
          (if (and (valid-room-ctx? ctx)
                   (same-actor? (msg-from msg) room)
                   (authorised-room-ctx? room))
              (set-parent! room)
              #f)))))

(set-method! :set-recovery-secret
  (lambda (args msg)
    (if (owner-caller? msg)
        (begin
          (set-recovery-secret! (if (null? args) "" (join-words args)))
          (reply-ok msg "recovery secret updated"))
        (reply-error msg "only owner may set recovery secret"))))

(set-method! :claim
  (lambda (args msg)
    (if (null? args)
        (reply-error msg "usage: :claim <secret>")
        (let ((secret (car args))
              (stored (recovery-secret))
              (user (msg-from msg)))
          (if (and stored (equal? secret stored))
              (begin
                (set-owner! user)
                (set-recovery-secret! "")
                (reply-ok msg "claimed"))
              (reply-error msg "claim failed"))))))

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
             (reply-error msg "take requires carrier parent as did:ma:... or #fragment"))
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
             (reply-error msg "drop requires target parent as did:ma:... or #fragment"))
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
