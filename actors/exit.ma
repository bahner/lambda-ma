; Locked exit actor.
; Exits are traversal entities owned by rooms or by root for world entry.

; Persistent endpoint state.
(define (target-room) (get-prop "target-room"))
(define (direction) (get-prop "direction"))
(define (owner) (get-prop "owner"))

(register-ctx-props! (list "direction" "target-room" "target-name"))

(define (name)
  (let ((n (get-prop "name")))
    (if n n (string-append "exit " (direction)))))

(define (description)
  (let ((d (get-prop "description")))
    (if d d (string-append "An exit leading " (direction) "."))))

(define (locked?)
  (equal? (get-prop "locked") "true"))

(define (traveller-message)
  (let ((text (get-prop "traveller-message")))
    (if text text (string-append "You go " (direction) "."))))

(define (blocked-message)
  (let ((text (get-prop "blocked-message")))
    (if text text "The way is locked.")))

(define (source-room-caller? msg)
  (same-actor? (msg-from msg) (node-parent)))

(define (traveller-caller? ctx msg)
  (and (valid-did? (ctx-text ctx "did"))
       (equal? (ctx-text ctx "did") (msg-from msg))))

(define (set-locked! value)
  (set-prop! "locked" (if value "true" "false"))
  (ma-save-state!))

(define (set-message! slot text)
  (begin
    (set-prop! (string-append slot "-message") text)
    (ma-save-state!)))

(define (valid-did-traversal-ctx? ctx)
  (and (map? ctx)
  (valid-did? (ctx-text ctx "did"))
  (same-actor? (ctx-text ctx "parent") (node-parent))))

(define (traversal-reply ctx room text)
  (make-map "did" (ctx-text ctx "did")
            "parent" (canonical-actor room)
            "text" text
            "exit" (canonical-actor (self))
            "direction" (direction)))

(define (blocked-ctx ctx)
  (traversal-reply ctx (node-parent) (blocked-message)))

(define (target-ctx ctx)
  (traversal-reply ctx (target-room) (traveller-message)))

(define (about-text)
  (string-append
    (name) "\n"
    (description) "\n"
    "owner: " (if (owner) (owner) "(none)") "\n"
    "source: " (if (non-empty-string? (node-parent)) (node-parent) "(none)") "\n"
    "target: " (if (target-room) (target-room) "(none)") "\n"
    "direction: " (if (direction) (direction) "(none)") "\n"
    "locked: " (if (locked?) "true" "false")))

(define (about-recipient ctx)
  (ctx-text ctx "did"))

(define (send-about! ctx)
  (let ((recipient (about-recipient ctx)))
    (if recipient
        (ma-send! (canonical-actor recipient) (list :print (about-text)))
        #f)))

; Public inspection methods.
(set-rpc-method! :about
  (lambda (args msg)
    (if (null? args)
        (reply-ok-with msg (about-text))
        (begin
          (send-about! (car args))
          (reply-ok msg)))))

(set-rpc-method! :look
  (lambda (args msg)
    (reply-ok-with msg (about-text))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (non-empty-string? (node-parent)) (node-parent) "(none)"))))

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

(set-internal-rpc-method! :report-parent handle-node-report-parent!)

(set-rpc-method! :locked?
  (lambda (args msg)
    (reply-ok-with msg (if (locked?) "true" "false"))))

(set-rpc-method! :lock
  (lambda (args msg)
    (if (source-room-caller? msg)
        (begin
          (set-locked! #t)
          (reply-ok-with msg "locked"))
        (reply-error msg "only source room may lock this exit"))))

(set-rpc-method! :unlock
  (lambda (args msg)
    (if (source-room-caller? msg)
        (begin
          (set-locked! #f)
          (reply-ok-with msg "unlocked"))
        (reply-error msg "only source room may unlock this exit"))))

(set-rpc-method! :message
  (lambda (args msg)
    (cond ((not (source-room-caller? msg))
           (reply-error msg "only source room may update exit messages"))
          ((or (null? args) (null? (cdr args)))
           (reply-error msg "usage: :message <traveller|source|target|blocked> <text>"))
          ((or (equal? (car args) "traveller")
               (equal? (car args) "source")
               (equal? (car args) "target")
               (equal? (car args) "blocked"))
           (begin
             (set-message! (car args) (join-words (cdr args)))
             (reply-ok-with msg "message updated")))
          (else
           (reply-error msg "unknown exit message slot")))))

; Only the source room that created the exit may ask it to end.
(set-internal-rpc-method! :fill
  (lambda (args msg)
    (if (source-room-caller? msg)
        (ma-end)
        #f)))

; Exit traversal returns the selected room to the direct DID. The DID then
; asks that room to enter, preserving target-room admission authority.
(set-rpc-method! :traverse
  (lambda (args msg)
    (let ((ctx (if (null? args) #f (car args))))
        (if (not (valid-did-traversal-ctx? ctx))
          (reply-error msg "traverse requires DID ctx with did and current parent")
          (if (not (or (source-room-caller? msg) (traveller-caller? ctx msg)))
              (reply-error msg "only source room or traveller may use this exit")
              (if (locked?)
                  (reply-ok-with msg (blocked-ctx ctx))
                  (if (target-room)
                      (reply-ok-with msg (target-ctx ctx))
                      (reply-ok-with msg
                        (traversal-reply ctx (node-parent) "This exit leads nowhere.")))))))))

(define (node-protocol) "/ma/exit/0.0.1")
(define (node-kind) "exit")
(define (node-name) (name))
(define (node-nick) (name))
(define (node-description) (description))

(define (extend-node-ctx ctx)
  (map-set
    (map-set
      (map-set ctx "direction" (direction))
      "target-room" (target-room))
    "target-name" (let ((target-name (get-prop "target-name")))
                     (if target-name target-name ""))))
