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
    (map-set
    (map-set
      (map-set
        (map-set
          (map-set
            (map-set
              (map-set (make-map) "actor" (canonical-actor (self)))
              "kind" (node-kind))
            "protocol" (node-protocol))
          "parent" (canonical-actor target-parent))
        "name" (node-name))
      "nick" (node-nick))
      "description" (node-description))))

(define (node-ctx)
  (node-ctx-for-parent (node-parent)))

(define (ctx-props-changed! keys)
  (announce-node-parent!))

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

(define (apply-node-parent-ctx! ctx)
  (begin
    (set-node-prop-from-ctx! ctx "name")
    (set-node-prop-from-ctx! ctx "nick")
    (set-node-prop-from-ctx! ctx "description")))

(define (node-delegated-did-arg? args)
  (and (not (null? args)) (valid-did? (car args))))

(define (node-effective-did args msg)
  (if (and (node-delegated-did-arg? args)
           (local-actor-ref? (msg-from msg)))
      (car args)
      (msg-from msg)))

(define (node-effective-args args msg)
  (if (and (node-delegated-did-arg? args)
           (local-actor-ref? (msg-from msg)))
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

(define (node-owner-avatar-delegation? did msg)
  (and (node-owner-did? did) (msg-from-owner? did msg)))

(define (node-transfer-caller-authorised? did msg)
  (or (node-caller-is-parent? msg)
      (node-orphan-owner-recovery? did)
      (node-owner-avatar-delegation? did msg)))

(define (node-recycle-caller-authorised? did msg)
  (and (node-caller-is-parent? msg) (node-owner-did? did)))

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

(define (announce-node-parent!)
  (let ((current (node-parent)))
    (if (equal? current "")
        #f
        (ma-send! current (list :parent (node-ctx))))))

(define (children-map)
  (prop-map "children"))

(define (set-children-map! children)
  (set-prop-map! "children" children))

(define (child-ctx-valid? ctx)
  (and (actor-ctx-shape? ctx)
       (valid-did-url? (ctx-text ctx "actor"))
       (valid-did-url? (ctx-text ctx "parent"))))

(define (child-ctx-self-authentic? ctx msg)
  (and (child-ctx-valid? ctx)
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

    (define (node-child-admission-error ctx msg) #f)
    (define (node-children-changed!) #f)
    (define (node-parent-committed!) #f)
    (define (node-confirmation-stale? ctx) #f)
    (define (node-children-query-error msg) #f)
    (define (node-children-query-authorised? msg) (owner-authorised? msg))
    (define (node-children-query-text) (children-text))

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

(define (take-target-parent rest msg)
  (let ((candidate
          (if (or (null? rest) (null? (cdr rest))) #f (car (cdr rest)))))
    (if (and (non-empty-string? candidate)
             (valid-did-url? candidate))
        (canonical-actor candidate)
        (canonical-actor (msg-from msg)))))

(define (take-feedback-verb rest)
  (let ((hint
          (if (or (null? rest)
                  (null? (cdr rest))
                  (null? (cdr (cdr rest))))
              #f
              (car (cdr (cdr rest))))))
    (if (equal? hint :drop) "drop" "take")))

(define (take-transfer-verb rest)
  (if (equal? (take-feedback-verb rest) "drop") :drop :take))

(define (handle-parent-take did rest msg)
  (if (null? rest)
      #f
      (let* ((actor (child-ref (car rest)))
             (ctx (if actor (child-ctx actor) #f)))
        (if actor
            (let ((target-parent (take-target-parent rest msg))
                  (feedback-verb (take-feedback-verb rest))
                  (transfer-verb (take-transfer-verb rest)))
              (begin
                ; Keep the child until its committed ctx reports another parent.
                (ma-send! actor (list transfer-verb did target-parent ctx))
                (ma-send! (canonical-actor (msg-from msg))
                          (list :print
                                (string-append "You " feedback-verb " "
                                               (child-label ctx) ".")))
                (reply-ok msg)
                #t))
            #f))))

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

(define (handle-node-parent args msg)
  (cond ((null? args)
         (reply-ok-with msg (if (equal? (node-parent) "") "(none)" (node-parent))))
        ((not (null? (cdr args)))
         (reply-error msg "usage: :parent [ctx]"))
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
           (forget-child! (ctx-text (car args) "actor"))
           (node-children-changed!)
           (reply-ok msg)))
        (else
         (begin
           (remember-child! (car args))
           (ma-send! (canonical-actor (ctx-text (car args) "actor"))
                     (list :child (car args)))
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

(define (node-confirmation-valid? ctx msg)
  (let ((target-parent (ctx-text ctx "parent")))
    (and (actor-ctx-shape? ctx)
         (same-actor? (ctx-text ctx "actor") (self))
         (valid-did-url? target-parent)
         (same-actor? (msg-from msg) target-parent)
         (node-parent-admissible? target-parent))))

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
              (equal? (node-ctx) (car args)))
         (reply-ok msg))
        (else
         (begin
           (commit-node-parent! (car args))
           (reply-ok msg)))))

(set-meta-method! :parent handle-node-parent)
(set-rpc-method! :parent? handle-node-parent?)
(set-rpc-method! :children? handle-node-children?)
(set-meta-method! :child handle-node-child)