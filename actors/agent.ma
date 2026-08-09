; Generic free movable Scheme agent.
; Concrete agents extend this behaviour with movement policy.


(define AGENT_PROTOCOL "/ma/scheme/agent/0.0.1")

; Persistent state accessors.
(define (owner) (get-prop "owner"))

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
    (ma-send! (canonical-actor room) (list :enter (node-ctx-for-parent room)))))

(define (leave-current-parent!)
  (let ((p (node-parent)))
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
      (node-valid-parent-ref? (ctx-text ctx "room"))))

      ; Transfer validation keeps take/drop strict at the room boundary.
(define (valid-room-ctx? ctx)
  (and (pair? ctx)
       (equal? (ctx-alist-ref ctx :kind) "agent")
       (non-empty-string? (ctx-alist-ref ctx :room))))

(define (authorised-room-ctx? room)
  (or (same-actor? room (pending-room))
      (same-actor? room (node-parent))))

(define (recycle! msg)
  (let ((old-parent (node-parent)))
    (begin
      (set-node-parent! "")
      (del-prop! "pending-room")
      (if (non-empty-string? old-parent)
          (ma-send! (canonical-actor old-parent) (list :parent (node-ctx-for-parent "")))
          #f)
      (ma-end))))

; Room context and movement helpers.
(define (node-protocol) AGENT_PROTOCOL)
(define (node-kind) "agent")
(define (node-name) (name))
(define (node-nick) (nick))
(define (node-description) (description))

(define (node-parent-committed!)
  (begin
    (del-prop! "pending-room")
    (ma-save-state!)))

(define (send-parent-room! msg term)
  (let ((p (node-parent)))
    (if (equal? p "")
        (reply-error msg (string-append (nick) " is nowhere"))
        (begin
          (ma-send! (canonical-actor p) term)
          (reply-ok msg)))))

(define (move-to-room! target-room source-room)
  (if (and (not (movement-pending?))
           (same-actor? source-room (node-parent)))
      (begin
        (enter (canonical-actor target-room)))
      #f))

; Public methods.
(set-rpc-method! :about
  (lambda (args msg)
    (reply-ok-with msg
      (string-append
        (name) "\n"
        (description) "\n"
        "owner: " (if (owner) (owner) "(none)") "\n"
        "parent: " (if (equal? (node-parent) "") "(none)" (node-parent))))))

(set-rpc-method! :look
  (lambda (args msg)
    (let ((text (string-append (name) "\n" (description))))
      (if (and (not (null? args))
               (non-empty-string? (car args))
               (presentation-did-authorised? (car args) msg))
          (begin
            (ma-send! (car args) (list :print text))
            (reply-ok msg))
          (reply-ok-with msg text)))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (equal? (node-parent) "") "(none)" (node-parent)))))

(set-internal-rpc-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args))))))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (canonical-actor (self)) (node-parent) tick nonce)))))

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
        (if (or (owner-caller? msg) (node-caller-is-parent? msg))
          (ma-reply! msg (list :ok (node-ctx)))
            #f)
           (let ((ctx (car args)))
          (cond
            ((and (valid-room-ctx? ctx)
               (same-actor? (msg-from msg) (ctx-alist-ref ctx :room))
               (authorised-room-ctx? (ctx-alist-ref ctx :room)))
             (let ((old-parent (node-parent))
                   (target-parent (ctx-alist-ref ctx :room)))
               (begin
               (set-node-parent! target-parent)
               (node-parent-committed!)
                 (ma-save-state!)
                 (if (and (non-empty-string? old-parent)
                          (not (same-actor? old-parent target-parent)))
                   (ma-send! (canonical-actor old-parent) (list :parent (node-ctx)))
                             #f)
                 (if (not (same-actor? old-parent target-parent))
                   (announce-node-parent!)
                   #f))))
            ((and (valid-move-ctx? ctx)
                  (or (node-caller-is-parent? msg) (owner-caller? msg)))
             (begin
               (if (non-empty-string? (ctx-text ctx "text"))
                 (set-last-message! (ctx-text ctx "text"))
                 #f)
               (if (same-actor? (ctx-text ctx "room") (node-parent))
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
          (did (claim-caller-did args msg))
          (claim-args (claim-command-args args msg)))
      (cond ((and (not (owner)) (not stored) (null? claim-args))
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
         (reply-user-error msg did "claim failed"))))))

; Parent-mediated transfer. The parent room asks the agent to bind to a did
; or re-enter another parent; direct did calls are deliberately rejected.
(set-cmd-method! :take
  (lambda (args msg)
        (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
  (cond ((handle-parent-take did rest msg) #t)
        ((not (node-transfer-caller-authorised? did msg))
             (reply-error msg "take must be requested by current parent"))
            ((not (valid-did? did))
             (reply-error msg "take requires DID with did:ma: prefix"))
            ((not (node-owner-or-unowned? did))
             (reply-error msg "only owner may take this agent"))
            ((null? rest)
             (reply-error msg "usage: :take <did> <carrier-parent-did-url> [ctx-map]"))
            ((not (node-valid-parent-ref? (car rest)))
             (reply-error msg "take requires carrier parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "take accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (node-valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (let ((target-parent (car rest)))
               (if (not (owner)) (set-owner! did) #f)
               (if (and (not (null? (cdr rest))) (node-valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
               (propose-node-parent! target-parent)
                 (reply-ok-with msg "take requested")))))))

(set-cmd-method! :drop
  (lambda (args msg)
    (let ((did (node-effective-did args msg))
        (rest (node-effective-args args msg)))
      (cond ((handle-stale-parent-drop did rest msg) #t)
            ((not (node-transfer-caller-authorised? did msg))
             (reply-error msg "drop must be requested by current parent"))
            ((not (valid-did? did))
             (reply-error msg "drop requires DID with did:ma: prefix"))
            ((not (node-owner-or-unowned? did))
             (reply-error msg "only owner may drop this agent"))
            ((null? rest)
             (reply-error msg "usage: :drop <target-parent-did-url> [ctx-map]"))
            ((not (node-valid-parent-ref? (car rest)))
             (reply-error msg "drop requires target parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "drop accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (node-valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (let ((target-parent (car rest)))
               (if (not (owner)) (set-owner! did) #f)
               (if (and (not (null? (cdr rest))) (node-valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
               (enter target-parent)
                 (reply-ok-with msg "drop requested")))))))

(set-cmd-method! :recycle
  (lambda (args msg)
        (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((not (null? rest))
             (reply-error msg "usage: :recycle <did>"))
            ((not (valid-did? did))
             (reply-error msg "recycle requires DID with did:ma: prefix"))
            ((not (node-recycle-caller-authorised? did msg))
             (reply-error msg "only owner may recycle this agent"))
            (else
             (recycle! msg))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (announce-node-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
