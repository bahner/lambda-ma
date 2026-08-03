; Movable container actor.
; A container behaves like a thing for parent transfer, and keeps its contents as
; child ctx entries while unlocked.

(define CONTAINER_PROTOCOL "/ma/container/0.0.1")

; Persistent state accessors.
(define (owner) (get-prop "owner"))

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

(define (container-ctx-rev)
  (let ((value (get-prop "ctx:rev")))
    (if (number? value) value 0)))

(define (extend-node-ctx ctx)
  (map-set
    (map-set ctx "rev" (container-ctx-rev))
    "contents" (contents-map)))

  (register-ctx-props! (list "ctx:rev" "children"))

(define (node-protocol) CONTAINER_PROTOCOL)
(define (node-kind) "container")
(define (node-name) (name))
(define (node-nick) (nick))
(define (node-description) (description))

(define (container-ctx-for-parent target-parent)
  (node-ctx-for-parent target-parent))

(define (container-ctx)
  (node-ctx))

(define (send-container-ctx!)
  (let ((p (node-parent)))
    (if (equal? p "")
        #f
        (begin
          (set-prop! "ctx:rev" (+ (container-ctx-rev) 1))
          (ma-save-state!)
          (announce-node-parent!)))))

  (define (ctx-props-changed! keys)
    (if (and (member-string? "ctx:rev" keys)
             (null? (cdr keys)))
        (announce-node-parent!)
        (send-container-ctx!)))

  (define (node-child-admission-error ctx msg)
    (if (locked?) (locked-message) #f))

  (define (node-children-changed!) (send-container-ctx!))

  (define (node-parent-committed!)
    (begin
      (set-prop! "ctx:rev" (+ (container-ctx-rev) 1))
      (ma-save-state!)))

  (define (node-confirmation-stale? ctx)
    (let ((confirmed-rev (map-ref ctx "rev" #f)))
      (and (number? confirmed-rev)
           (< confirmed-rev (container-ctx-rev)))))

  (define (node-children-query-error msg)
    (if (locked?) (locked-message) #f))

  (define (node-children-query-authorised? msg) #t)
  (define (node-children-query-text) (contents-text))

; Caller and reply helpers.
(define (owner-caller? msg)
  (let ((o (owner)))
    (and o (msg-from-owner? o msg))))

(define (local-actor-caller? msg)
  (local-actor-ref? (msg-from msg)))

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
      (equal? key "description")
      (equal? key "locked-message")))

(define (set-container-prop! key value)
  (set-node-prop! key value))

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

(define (container-look-text)
  (string-append
    (name) "\n"
    (description) "\n"
    (if (locked?) (locked-message) (contents-text))))

(define (present-to-did! target text)
  (ma-send! target (list :print text)))

(define (presentation-target-arg? args)
  (and (not (null? args))
       (non-empty-string? (car args))))

; Public methods.
(set-rpc-method! :about
  (lambda (args msg)
    (reply-ok-with msg
      (string-append
        (name) "\n"
        (description) "\n"
        "owner: " (if (owner) (owner) "(none)") "\n"
        "parent: " (if (equal? (node-parent) "") "(none)" (node-parent)) "\n"
        "locked: " (if (locked?) "true" "false")))))

(set-rpc-method! :look
  (lambda (args msg)
    (if (and (presentation-target-arg? args) (local-actor-caller? msg))
        (begin
          (present-to-did! (car args) (container-look-text))
          (reply-ok msg))
        (reply-ok-with msg (container-look-text)))))

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
        (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((not (valid-did? did))
             (reply-error msg "lock requires DID with did:ma: prefix"))
            ((not (node-owner-or-unowned? did))
             (reply-error msg "only owner may lock this container"))
            (else
             (begin
               (ensure-owner! did)
               (if (null? rest) #f (set-locked-message! (join-words rest)))
               (set-locked! #t)
               (reply-ok-with msg "locked")))))))

(set-rpc-method! :unlock
  (lambda (args msg)
    (let ((did (node-effective-did args msg)))
      (cond ((not (valid-did? did))
             (reply-error msg "unlock requires DID with did:ma: prefix"))
            ((not (node-owner-or-unowned? did))
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

(set-cmd-method! :orphan handle-node-orphan!)

(set-cmd-method! :put-in
  (lambda (args msg)
    (cond ((locked?)
           (reply-locked msg))
          ((or (null? args) (not (null? (cdr args))))
           (reply-error msg "usage: :put-in <ctx-map>"))
          ((not (child-ctx-valid? (car args)))
           (reply-error msg "put-in requires ctx-map with actor, parent, kind, protocol, name, nick, description"))
          ((not (child-ctx-self-authentic? (car args) msg))
           (reply-error msg "put-in ctx actor must match sender"))
          ((not (same-actor? (ctx-text (car args) "parent") (self)))
           (reply-error msg "put-in ctx parent must be this container"))
          (else
           (begin
             (remember-content! (car args))
             (ma-send! (canonical-actor (ctx-text (car args) "actor")) (list :child (car args)))
             (send-container-ctx!)
             (reply-ok-with msg "put in"))))))

; Parent-mediated transfer. The parent room asks the container to bind to a did
; or move to another parent; direct did calls are deliberately rejected.
(set-cmd-method! :take
  (lambda (args msg)
        (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((and (not (null? rest)) (child-ref (car rest)) (locked?))
             (reply-locked msg))
            ((handle-parent-take did rest msg) #t)
            ((not (node-transfer-caller-authorised? did msg))
             (reply-error msg "take must be requested by current parent"))
            ((not (valid-did? did))
             (reply-error msg "take requires DID with did:ma: prefix"))
            ((not (node-owner-or-unowned? did))
             (reply-error msg "only owner may take this container"))
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
             (reply-error msg "only owner may drop this container"))
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
                   (set-claim! did (car (cdr rest)))
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
             (reply-error msg "only owner may recycle this container"))
            (else
             (recycle! msg))))))

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
               (send-container-ctx!)
               (reply-ok-with msg ctx)))))))

(set-cmd-method! :recycle-from
  (lambda (args msg)
    (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((locked?)
             (reply-locked msg))
            ((or (null? rest) (not (null? (cdr rest))))
             (reply-error msg "usage: :recycle-from <child>"))
            ((not (content-ref (car rest)))
             (reply-error msg "unknown container content"))
            (else
             (let ((actor (content-ref (car rest))))
               (begin
                 (ma-send! (canonical-actor actor) (list :recycle did))
                 (reply-ok-with msg "recycle requested"))))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (send-container-ctx!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
