; Movable container actor.
; A container behaves like a thing for parent transfer, and keeps its contents as
; child ctx entries while unlocked.

(define CONTAINER_PROTOCOL "/ma/container/0.0.1")

; Persistent state accessors. owner/set-owner!/claim-key/set-claim!/
; recovery-secret/set-recovery-secret!/owner-caller? live in node.ma.
(define (name)
  (let ((n (get-prop "name")))
    (if n n "container")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable container.")))

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (locked?)
  (equal? (get-prop "locked") "true"))

(define (lock-secret)
  (get-prop "lock-secret"))

(define (set-lock-secret! secret)
  (if (non-empty-string? secret)
      (set-prop! "lock-secret" secret)
      #f)
  (ma-save-state!))

(define (locked-message)
  (let ((text (get-prop "locked-message")))
    (if (non-empty-string? text) text "The container is locked.")))

(define (set-locked! value)
  (set-prop! "locked" (if value "true" "false"))
  (ma-save-state!))

(define (set-locked-message! text)
  (if (non-empty-string? text)
      (set-prop! "locked-message" text)
      #f)
  (ma-save-state!))

(define (closed?)
  (equal? (get-prop "closed") "true"))

(define (set-closed! value)
  (set-prop! "closed" (if value "true" "false"))
  (ma-save-state!))

(define (content-lines entries)
  (cond ((null? entries) '())
        (else (cons (child-line (car entries)) (content-lines (cdr entries))))))

(define (contents-text)
  (let ((lines (content-lines (map->alist (children-map)))))
    (if (null? lines)
        "Contents: none."
        (string-append "Contents:\n" (actor-entry-lines lines)))))

(define (container-ctx-rev)
  (let ((value (get-prop "ctx:rev")))
    (if (number? value) value 0)))

(define (prune-dead-local-children!)
  (let loop ((ctxs (child-ctxs)) (live '()))
    (cond ((null? ctxs) (reverse live))
          ((dead-local-actor? (ctx-text (car ctxs) "actor"))
           (begin
             (forget-child! (ctx-text (car ctxs) "actor"))
             (loop (cdr ctxs) live)))
          (else
           (loop (cdr ctxs) (cons (car ctxs) live))))))

(define (extend-node-ctx ctx)
  (map-set ctx "rev" (container-ctx-rev)))

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
        (begin
          (announce-ctx-to-root!)
          (send-container-ctx!))))

  (define (node-child-admission-error ctx msg)
    (cond ((or (locked?) (closed?)) (locked-message))
          ((>= (node-children-count) (node-max-children)) "container is full")
          (else #f)))

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
    (cond ((locked?) (locked-message))
          ((closed?) (locked-message))
          (else #f)))

  (define (node-children-query-authorised? msg) #t)
  (define (node-children-query-text) (contents-text))

; Caller and reply helpers.
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

(define (reply-locked msg)
  (reply-error msg (locked-message)))

(define (generate-action-id)
  (string-append
    (number->string (ma-random 9223372036854775807))
    "-"
    (number->string (ma-random 9223372036854775807))))

(define (container-look-text)
  (string-append
    (name) "\n"
    (description) "\n"
    (if (or (locked?) (closed?)) (locked-message) (contents-text))))

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
    (reply-ok-with msg (container-look-text))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (equal? (node-parent) "") "(none)" (node-parent)))))

(set-internal-rpc-method! :report-parent handle-node-report-parent!)

(set-rpc-method! :owner handle-node-owner)
(set-rpc-method! :owner? handle-node-owner?)

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
        (reply-ok-with msg (prune-dead-local-children!)))))

; Root sends :put after spatial validation; container enforces locked/full and notifies requestor.
(set-cmd-method! :put
  (lambda (args msg)
    (when (and (node-caller-is-local-root? msg)
               (not (null? args))
               (map? (car args)))
      (let ((put-ctx (car args)))
        (cond ((or (locked?) (closed?))
               (ma-send! (canonical-actor (map-ref put-ctx "requestor" ""))
                         (list :put-event (map-set (map-set put-ctx "status" "failed")
                                                    "reason" (locked-message)))))
              ((>= (node-children-count) (node-max-children))
               (ma-send! (canonical-actor (map-ref put-ctx "requestor" ""))
                         (list :put-event (map-set (map-set put-ctx "status" "failed")
                                                    "reason" "container is full"))))
              (else #f))))))

; Avatar initiates take; container validates lock and child presence, then forwards to root.
(set-rpc-method! :take
  (lambda (args msg)
    (cond ((or (locked?) (closed?))
           (reply-locked msg))
          ((null? args)
           (reply-error msg "usage: :take <item-did-url>"))
          ((not (valid-did-url? (car args)))
           (reply-error msg ":take requires a DID-URL"))
          ((not (child-ctx (canonical-actor (car args))))
           (reply-error msg "item not in this container"))
          (else
           (let* ((id (generate-action-id))
                  (take-ctx (make-map
                               "action" "take"
                               "id" id
                               "requestor" (msg-from msg)
                               "item" (canonical-actor (car args))
                               "container" (canonical-actor (self)))))
             (ma-send! (root) (list :take-request take-ctx))
             (reply-ok-with msg id))))))

(set-rpc-method! :lock
  (lambda (args msg)
        (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((not (valid-did? did))
             (reply-error msg "lock requires DID with did:ma: prefix"))
            ((not (node-owner-did? did))
             (reply-error msg "only owner may lock this container"))
            (else
             (begin
               (if (null? rest) #f (set-lock-secret! (join-words rest)))
               (set-locked! #t)
               (reply-ok-with msg "locked")))))))

(set-rpc-method! :unlock
  (lambda (args msg)
    (let ((did (node-effective-did args msg))
          (rest (node-effective-args args msg)))
      (cond ((not (valid-did? did))
             (reply-error msg "unlock requires DID with did:ma: prefix"))
            ((and (not (node-owner-did? did))
                  (or (null? rest)
                      (not (equal? (join-words rest) (lock-secret)))))
             (reply-error msg "only owner or lock secret may unlock this container"))
            (else
             (begin
               (set-locked! #f)
               (reply-ok-with msg "unlocked")))))))

(set-rpc-method! :open
  (lambda (args msg)
    (cond ((not (owner-caller? msg))
           (reply-error msg "only owner may open this container"))
          (else
           (begin
             (set-closed! #f)
             (reply-ok-with msg "open"))))))

(set-rpc-method! :close
  (lambda (args msg)
    (cond ((not (owner-caller? msg))
           (reply-error msg "only owner may close this container"))
          (else
           (begin
             (set-closed! #t)
             (reply-ok-with msg "closed"))))))

(set-rpc-method! :set-recovery-secret handle-node-set-recovery-secret!)

(set-cmd-method! :claim handle-node-claim!)

(set-cmd-method! :orphan handle-node-orphan!)

; Parent-mediated transfer: the container proposes itself to a new parent
; (ma-spec sec 6); direct did calls are deliberately rejected. Shared body
; lives in node.ma as handle-node-set-parent!. Putting things into or taking
; things out of this container is done by addressing the content actor
; itself with :set-parent, discovered beforehand via :contents?.
(set-cmd-method! :set-parent handle-node-set-parent!)

; :hold implicitly targets the caller (msg-from) as new parent - no argument.
; Shared body lives in node.ma as handle-node-hold!.
(set-cmd-method! :hold handle-node-hold!)

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

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (begin
           (announce-ctx-to-root!)
           (send-container-ctx!)))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
