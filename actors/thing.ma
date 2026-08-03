; Free movable thing actor.
; Authority lives in thing state: owner + parent.

(define THING_PROTOCOL "/ma/thing/0.0.1")

; Persistent state accessors.
(define (owner) (get-prop "owner"))

(define (name)
  (let ((n (get-prop "name")))
    (if n n "thing")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable thing.")))

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (recovery-secret) (get-prop "recovery-secret"))

(define (set-owner! did)
  (set-prop! "owner" did)
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

(define (node-protocol) THING_PROTOCOL)
(define (node-kind) "thing")
(define (node-name) (name))
(define (node-nick) (nick))
(define (node-description) (description))

(define (clean-stale-parent-claim! ctx target-parent)
  (let ((claimed-parent (ctx-text ctx "parent")))
    (if (and (map? ctx)
             (same-actor? (ctx-text ctx "actor") (self))
             (valid-did-url? claimed-parent)
             (not (same-actor? claimed-parent (node-parent)))
             (not (same-actor? claimed-parent target-parent)))
        (ma-send! (canonical-actor claimed-parent) (list :parent (node-ctx)))
        #f)))

(define (recycle! msg)
  (let ((old-parent (node-parent)))
    (begin
      (set-node-parent! "")
      (if (non-empty-string? old-parent)
          (ma-send! (canonical-actor old-parent) (list :parent (node-ctx-for-parent "")))
          #f)
      (ma-end))))

(define (editable-prop? key)
  (or (equal? key "name")
      (equal? key "nick")
      (equal? key "description")))

(define (set-thing-prop! key value)
  (set-node-prop! key value))

(define (handle-thing-prop! msg args)
  (cond ((not (owner-caller? msg))
         (reply-error msg "only owner may edit thing props"))
        ((null? args)
         (reply-error msg "usage: :prop <name|nick|description> [value]"))
        ((not (editable-prop? (car args)))
         (reply-error msg "editable thing props: name, nick, description"))
        (else
         (begin
           (set-thing-prop! (car args) (join-words (cdr args)))
           (reply-ok-with msg "prop updated")))))

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
               (local-actor-ref? (msg-from msg)))
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

(set-rpc-method! :prop
  (lambda (args msg)
    (handle-thing-prop! msg args)))

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

; Parent-mediated transfer. The parent room asks the thing to bind to a did
; or move to another parent; direct did calls are deliberately rejected.
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
             (reply-error msg "only owner may take this thing"))
            ((null? rest)
             (reply-error msg "usage: :take <did> <carrier-parent-did-url> [ctx-map]"))
            ((not (node-valid-parent-ref? (car rest)))
             (reply-error msg "take requires carrier parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "take accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (node-valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (begin
               (if (not (owner)) (set-owner! did) #f)
                 (if (and (not (null? (cdr rest))) (node-valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
                 (propose-node-parent! (car rest))
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
             (reply-error msg "only owner may drop this thing"))
            ((null? rest)
             (reply-error msg "usage: :drop <target-parent-did-url> [ctx-map]"))
            ((not (node-valid-parent-ref? (car rest)))
             (reply-error msg "drop requires target parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "drop accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (node-valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (begin
               (if (not (owner)) (set-owner! did) #f)
                 (if (and (not (null? (cdr rest))) (node-valid-transfer-ctx? (car (cdr rest))))
                   (clean-stale-parent-claim! (car (cdr rest)) (car rest))
                   #f)
                 (propose-node-parent! (car rest))
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
             (reply-error msg "only owner may recycle this thing"))
            (else
             (recycle! msg))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
      (announce-node-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
