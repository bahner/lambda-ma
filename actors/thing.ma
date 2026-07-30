; Free movable thing actor.
; Authority lives in thing state: owner + parent.

; Persistent state accessors.
(define (owner) (get-prop "owner"))
(define (parent)
  (let ((p (get-prop "parent")))
    (if p p "")))

(define (name)
  (let ((n (get-prop "name")))
    (if n n "thing")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable thing.")))

(define (recovery-secret) (get-prop "recovery-secret"))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (set-parent! did)
  (set-prop! "parent" (canonical-actor did))
  (ma-save-state!))

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

(define (valid-parent-ref? ref)
  (and (non-empty-string? ref)
       (or (did-url? ref)
           (local-actor-ref? ref))))

(define (valid-transfer-kind? kind)
  (or (equal? kind "avatar")
      (equal? kind "thing")
      (equal? kind "agent")))

; Transfer ctx is optional, but when present it must be a full room-local ctx
; payload so future parent displays can use stable name/nick/description data.
(define (valid-transfer-ctx? ctx)
  (and (map? ctx)
       (non-empty-string? (ctx-text ctx "kind"))
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))
       (valid-transfer-kind? (ctx-text ctx "kind"))))

(define (owner-or-unowned? user)
  (let ((o (owner)))
    (or (not o) (equal? o user))))

(define (caller-is-parent? msg)
  (let ((p (parent)))
    (and (not (equal? p "")) (same-actor? (msg-from msg) p))))

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

; Parent-mediated transfer. The parent room asks the thing to bind to a user
; or move to another parent; direct user calls are deliberately rejected.
(set-method! :take
  (lambda (args msg)
    (let ((user (effective-user args msg))
          (rest (effective-args args msg)))
      (cond ((not (caller-is-parent? msg))
             (reply-error msg "take must be requested by current parent"))
            ((not (valid-user-did? user))
             (reply-error msg "take requires user DID with did:ma: prefix"))
            ((not (owner-or-unowned? user))
             (reply-error msg "only owner may take this thing"))
            ((null? rest)
             (reply-error msg "usage: :take <user-did> <carrier-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "take requires carrier parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "take accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty kind, name, nick, description"))
            (else
             (begin
               (if (not (owner)) (set-owner! user) #f)
               (set-parent! (car rest))
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
             (reply-error msg "only owner may drop this thing"))
            ((null? rest)
             (reply-error msg "usage: :drop <target-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "drop requires target parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "drop accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty kind, name, nick, description"))
            (else
             (begin
               (if (not (owner)) (set-owner! user) #f)
               (set-parent! (car rest))
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! user (car (cdr rest)))
                   #f)
                 (reply-ok msg "dropped")))))))
