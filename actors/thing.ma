; Free movable thing actor.
; Authority lives in thing state: owner + parent.

(define THING_PROTOCOL "/ma/thing/0.0.1")

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

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (recovery-secret) (get-prop "recovery-secret"))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (set-parent! did)
  (set-prop! "parent" (canonical-actor did))
  (ma-save-state!))

(define (clear-parent!)
  (del-prop! "parent")
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

(define (owner-did? did)
  (let ((o (owner)))
    (and o (equal? did o))))

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

; Transfer ctx is optional, but when present it must be a full room-local ctx
; payload so future parent displays can use stable name/nick/description data.
(define (valid-transfer-ctx? ctx)
  (and (actor-ctx-shape? ctx)
       (valid-transfer-kind? (ctx-text ctx "kind"))))

(define (thing-ctx-for-parent target-parent)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (canonical-actor (self)))
              "kind" "thing")
            "protocol" THING_PROTOCOL)
          "parent" (canonical-actor target-parent))
        "name" (name))
      "nick" (nick))
    "description" (description)))

(define (thing-ctx)
  (thing-ctx-for-parent (parent)))

(define (announce-parent!)
  (let ((p (parent)))
    (if (equal? p "")
        #f
        (ma-send! (canonical-actor p) (list :parent (thing-ctx))))))

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
    (announce-parent!)
    (if (and (non-empty-string? old-parent)
             (not (same-actor? old-parent target-parent)))
      (ma-send! (canonical-actor old-parent) (list :parent (thing-ctx)))
        #f))))

(define (propose-parent-change! target-parent)
  (ma-send! (canonical-actor target-parent) (list :parent (thing-ctx-for-parent target-parent))))

(define (owner-or-unowned? did)
  (let ((o (owner)))
    (or (not o) (equal? o did))))

(define (caller-is-parent? msg)
  (let ((p (parent)))
    (and (not (equal? p "")) (same-actor? (msg-from msg) p))))

(define (orphan-owner-recovery? did)
  (and (equal? (parent) "") (owner-did? did)))

(define (owner-avatar-delegation? did msg)
  (and (owner-did? did) (msg-from-owner? did msg)))

(define (transfer-caller-authorised? did msg)
  (or (caller-is-parent? msg)
      (orphan-owner-recovery? did)
      (owner-avatar-delegation? did msg)))

(define (recycle-caller-authorised? did msg)
  (and (caller-is-parent? msg) (owner-did? did)))

(define (recycle! msg)
  (let ((old-parent (parent)))
    (begin
      (clear-parent!)
      (if (non-empty-string? old-parent)
          (ma-send! (canonical-actor old-parent) (list :parent (thing-ctx-for-parent "")))
          #f)
      (reply-ok-with msg "recycled")
      (ma-end))))

(define (editable-prop? key)
  (or (equal? key "name")
      (equal? key "nick")
      (equal? key "description")))

(define (set-thing-prop! key value)
  (if (equal? value "")
      (del-prop! key)
      (set-prop! key value))
  (ma-save-state!)
  (announce-parent!))

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
        "parent: " (if (equal? (parent) "") "(none)" (parent))))))

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
    (reply-ok-with msg (if (equal? (parent) "") "(none)" (parent)))))

(set-meta-method! :parent
  (lambda (args msg)
    (cond ((null? args)
           (if (owner-caller? msg)
               (reply-ok-with msg (if (equal? (parent) "") "(none)" (parent)))
               (reply-error msg "only owner may inspect parent")))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :parent [ctx]"))
          (else
           (reply-error msg "thing is not a parent for ctx updates")))))

(set-meta-method! :child
  (lambda (args msg)
    (cond ((or (null? args) (not (null? (cdr args))))
           (reply-error msg "usage: :child <ctx>"))
          ((not (valid-parent-ctx? (car args)))
           (reply-error msg "child ctx must include target parent"))
          ((not (same-actor? (msg-from msg) (parent-target-from-ctx (car args))))
           (reply-error msg "child ctx must come from target parent"))
          (else
           (begin
             (apply-parent-ctx! (car args))
             (reply-ok msg))))))

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
    (if (null? args)
        (reply-error msg "usage: :claim <secret>")
        (let ((secret (car args))
              (stored (recovery-secret))
              (did (msg-from msg)))
          (if (and stored (equal? secret stored))
              (begin
                (set-owner! did)
                (set-recovery-secret! "")
                (reply-ok-with msg "claimed"))
              (reply-error msg "claim failed"))))))

; Parent-mediated transfer. The parent room asks the thing to bind to a did
; or move to another parent; direct did calls are deliberately rejected.
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
             (reply-error msg "only owner may take this thing"))
            ((null? rest)
             (reply-error msg "usage: :take <did> <carrier-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "take requires carrier parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "take accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (begin
               (if (not (owner)) (set-owner! did) #f)
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
               (propose-parent-change! (car rest))
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
             (reply-error msg "only owner may drop this thing"))
            ((null? rest)
             (reply-error msg "usage: :drop <target-parent-did-url> [ctx-map]"))
            ((not (valid-parent-ref? (car rest)))
             (reply-error msg "drop requires target parent as DID-URL"))
            ((and (not (null? (cdr rest))) (not (null? (cdr (cdr rest)))))
             (reply-error msg "drop accepts at most one optional ctx-map"))
            ((and (not (null? (cdr rest))) (not (valid-transfer-ctx? (car (cdr rest)))))
             (reply-error msg "ctx-map must include non-empty parent, kind, protocol, name, nick, description"))
            (else
             (begin
               (if (not (owner)) (set-owner! did) #f)
               (if (and (not (null? (cdr rest))) (valid-transfer-ctx? (car (cdr rest))))
                   (set-claim! did (car (cdr rest)))
                   #f)
               (propose-parent-change! (car rest))
                 (reply-ok-with msg "drop requested")))))))

(set-cmd-method! :recycle
  (lambda (args msg)
    (let ((did (effective-did args msg))
          (rest (effective-args args msg)))
      (cond ((not (null? rest))
             (reply-error msg "usage: :recycle <did>"))
            ((not (valid-did? did))
             (reply-error msg "recycle requires DID with did:ma: prefix"))
            ((not (recycle-caller-authorised? did msg))
             (reply-error msg "only owner via current parent may recycle this thing"))
            (else
             (recycle! msg))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (announce-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
