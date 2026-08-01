; Movable container actor.
; A container behaves like a thing for parent transfer, and keeps its contents as
; child ctx entries while unlocked.

(define CONTAINER_PROTOCOL "/ma/container/0.0.1")

; Persistent state accessors.
(define (owner) (get-prop "owner"))
(define (parent)
  (let ((p (get-prop "parent")))
    (if p p "")))

(define (name)
  (let ((n (get-prop "name")))
    (if n n "container")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable container.")))

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (recovery-secret) (get-prop "recovery-secret"))

(define (locked?)
  (equal? (get-prop "locked") "true"))

(define (locked-message)
  (let ((text (get-prop "locked-message")))
    (if (non-empty-string? text) text "The container is locked.")))

(define (set-owner! did)
  (set-prop! "owner" did)
  (ma-save-state!))

(define (set-parent! did)
  (set-prop! "parent" (canonical-actor did))
  (ma-save-state!))

(define (set-locked! value)
  (set-prop! "locked" (if value "true" "false"))
  (ma-save-state!))

(define (set-locked-message! text)
  (if (non-empty-string? text)
      (set-prop! "locked-message" text)
      #f)
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

(define (contents-map)
  (children-map))

(define (set-contents-map! contents)
  (set-children-map! contents))

(define (content-ctx actor)
  (child-ctx actor))

(define (remember-content! ctx)
  (remember-child! ctx))

(define (forget-content! actor)
  (forget-child! actor))

(define (content-lines entries)
  (cond ((null? entries) '())
        (else (cons (child-line (car entries)) (content-lines (cdr entries))))))

(define (contents-text)
  (let ((lines (content-lines (map->alist (contents-map)))))
    (if (null? lines)
        "Contents: none."
        (string-append "Contents:\n" (actor-entry-lines lines)))))

(define (content-ref token)
  (child-ref token))

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
  (and (actor-ctx? ctx)
       (valid-transfer-kind? (ctx-text ctx "kind"))))

(define (container-ctx-for-parent target-parent)
  (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (canonical-actor (self)))
              "kind" "container")
            "protocol" CONTAINER_PROTOCOL)
          "parent" (canonical-actor target-parent))
        "name" (name))
      "nick" (nick))
    "description" (description)))

(define (container-ctx)
  (container-ctx-for-parent (parent)))

(define (announce-parent!)
  (let ((p (parent)))
    (if (equal? p "")
        #f
        (ma-send! (canonical-actor p) (list :child (container-ctx))))))

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
        (ma-send! (canonical-actor old-parent) (list :child (container-ctx)))
        #f))))

(define (propose-parent-change! target-parent)
  (ma-send! (canonical-actor target-parent) (list :child (container-ctx-for-parent target-parent))))

(define (owner-or-unowned? did)
  (let ((o (owner)))
    (or (not o) (equal? o did))))

(define (caller-is-parent? msg)
  (let ((p (parent)))
    (and (not (equal? p "")) (same-actor? (msg-from msg) p))))

(define (orphan-owner-recovery? did)
  (and (equal? (parent) "") (owner-did? did)))

(define (transfer-caller-authorised? did msg)
  (or (caller-is-parent? msg) (orphan-owner-recovery? did)))

(define (editable-prop? key)
  (or (equal? key "name")
      (equal? key "nick")
      (equal? key "description")
      (equal? key "locked-message")))

(define (set-container-prop! key value)
  (if (equal? value "")
      (del-prop! key)
      (set-prop! key value))
  (ma-save-state!))

(define (handle-container-prop! msg args)
  (cond ((not (owner-caller? msg))
         (reply-error msg "only owner may edit container props"))
        ((null? args)
         (reply-error msg "usage: :prop <name|nick|description|locked-message> [value]"))
        ((not (editable-prop? (car args)))
         (reply-error msg "editable container props: name, nick, description, locked-message"))
        (else
         (begin
           (set-container-prop! (car args) (join-words (cdr args)))
           (reply-ok-with msg "prop updated")))))

(define (ensure-owner! did)
  (if (not (owner)) (set-owner! did) #f))

(define (reply-locked msg)
  (reply-error msg (locked-message)))

; Public methods.
(set-rpc-method! :about
  (lambda (args msg)
    (reply-ok-with msg
      (string-append
        (name) "\n"
        (description) "\n"
        "owner: " (if (owner) (owner) "(none)") "\n"
        "parent: " (if (equal? (parent) "") "(none)" (parent)) "\n"
        "locked: " (if (locked?) "true" "false")))))

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
          ((not (valid-parent-ctx? (car args)))
           (reply-error msg "parent ctx must include target parent"))
          ((not (same-actor? (msg-from msg) (parent-target-from-ctx (car args))))
           (reply-error msg "parent ctx must come from target parent"))
          (else
           (begin
             (apply-parent-ctx! (car args))
             (reply-ok msg))))))

(set-meta-method! :child
  (lambda (args msg)
    (cond ((null? args)
           (if (locked?)
               (reply-locked msg)
               (reply-ok-with msg (contents-text))))
          ((not (null? (cdr args)))
           (reply-error msg "usage: :child [ctx]"))
          ((locked?)
           (reply-locked msg))
          ((not (child-ctx-valid? (car args)))
           (reply-error msg "child ctx must include actor, parent, kind, protocol, name, nick, description"))
          ((not (same-actor? (msg-from msg) (ctx-text (car args) "actor")))
           (reply-error msg "child ctx actor must match sender"))
          ((not (same-actor? (ctx-text (car args) "parent") (self)))
           (begin
             (forget-content! (ctx-text (car args) "actor"))
             (reply-ok msg)))
          (else
           (begin
             (remember-content! (car args))
             (ma-send! (canonical-actor (ctx-text (car args) "actor")) (list :parent (car args)))
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
    (handle-container-prop! msg args)))

(set-rpc-method! :locked?
  (lambda (args msg)
    (reply-ok-with msg (if (locked?) "true" "false"))))

(set-rpc-method! :contents?
  (lambda (args msg)
    (if (locked?)
        (reply-locked msg)
        (reply-ok-with msg (contents-text)))))

(set-rpc-method! :lock
  (lambda (args msg)
    (let ((did (effective-did args msg))
          (rest (effective-args args msg)))
      (cond ((not (valid-did? did))
             (reply-error msg "lock requires DID with did:ma: prefix"))
            ((not (owner-or-unowned? did))
             (reply-error msg "only owner may lock this container"))
            (else
             (begin
               (ensure-owner! did)
               (if (null? rest) #f (set-locked-message! (join-words rest)))
               (set-locked! #t)
               (reply-ok-with msg "locked")))))))

(set-rpc-method! :unlock
  (lambda (args msg)
    (let ((did (effective-did args msg)))
      (cond ((not (valid-did? did))
             (reply-error msg "unlock requires DID with did:ma: prefix"))
            ((not (owner-or-unowned? did))
             (reply-error msg "only owner may unlock this container"))
            (else
             (begin
               (ensure-owner! did)
               (set-locked! #f)
               (reply-ok-with msg "unlocked")))))))

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

(set-cmd-method! :put-in
  (lambda (args msg)
    (cond ((locked?)
           (reply-locked msg))
          ((or (null? args) (not (null? (cdr args))))
           (reply-error msg "usage: :put-in <ctx-map>"))
          ((not (child-ctx-valid? (car args)))
           (reply-error msg "put-in requires ctx-map with actor, parent, kind, protocol, name, nick, description"))
          ((not (same-actor? (msg-from msg) (ctx-text (car args) "actor")))
           (reply-error msg "put-in ctx actor must match sender"))
          ((not (same-actor? (ctx-text (car args) "parent") (self)))
           (reply-error msg "put-in ctx parent must be this container"))
          (else
           (begin
             (remember-content! (car args))
             (ma-send! (canonical-actor (ctx-text (car args) "actor")) (list :parent (car args)))
             (reply-ok-with msg "put in"))))))

; Parent-mediated transfer. The parent room asks the container to bind to a did
; or move to another parent; direct did calls are deliberately rejected.
(set-cmd-method! :take
  (lambda (args msg)
    (let ((did (effective-did args msg))
          (rest (effective-args args msg)))
      (cond ((and (not (null? rest)) (child-ref (car rest)) (locked?))
             (reply-locked msg))
            ((handle-parent-take did rest msg) #t)
            ((not (transfer-caller-authorised? did msg))
             (reply-error msg "take must be requested by current parent"))
            ((not (valid-did? did))
             (reply-error msg "take requires DID with did:ma: prefix"))
            ((not (owner-or-unowned? did))
             (reply-error msg "only owner may take this container"))
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
             (reply-error msg "only owner may drop this container"))
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

(set-cmd-method! :take-from
  (lambda (args msg)
    (cond ((locked?)
           (reply-locked msg))
          ((or (null? args) (not (null? (cdr args))))
           (reply-error msg "usage: :take-from <child>"))
          ((not (content-ref (car args)))
           (reply-error msg "unknown container content"))
          (else
           (let* ((actor (content-ref (car args)))
                  (ctx (content-ctx actor)))
             (begin
               (forget-content! actor)
               (reply-ok-with msg ctx)))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (announce-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))