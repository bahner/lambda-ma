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

; The refusal message a traveller hears when a traverse is refused. Generic
; default; settable via `:prop locked-msg-text <text>` (or the `:message
; blocked <text>` slot).
(define (locked-msg-text)
  (let ((text (get-prop "locked-msg-text")))
    (if (non-empty-string? text) text "This is not the way.")))

(define (source-room-caller? msg)
  (same-actor? (msg-from msg) (node-parent)))

(define (traveller-caller? ctx msg)
  (equal? (ctx-text ctx "did") (msg-from msg)))

(define (set-locked! value)
  (set-prop! "locked" (if value "true" "false"))
  (ma-save-state!))

(define (lock-secret)
  (get-prop "lock-secret"))

(define (set-lock-secret! secret)
  (if (non-empty-string? secret)
      (set-prop! "lock-secret" secret)
      #f)
  (ma-save-state!))

(define (set-message! slot text)
  (begin
    (set-prop! (if (equal? slot "blocked") "locked-msg-text" (string-append slot "-message")) text)
    (ma-save-state!)))

(define (valid-did-traversal-ctx? ctx)
  (and (map? ctx)
       (non-empty-string? (ctx-text ctx "did"))
       (same-actor? (ctx-text ctx "parent") (node-parent))))

(define (traversal-reply ctx room text)
  (make-map "did" (ctx-text ctx "did")
            "parent" (canonical-actor room)
            "text" text
            "exit" (canonical-actor (self))
            "direction" (direction)))

(define (blocked-ctx ctx)
  (traversal-reply ctx (node-parent) (locked-msg-text)))

; Narrate a refused traverse to the traveller as an in-world :say event (the
; zion/visualisation stream) — never an inbox message. The blocked-ctx reply
; stays the structured data channel.
(define (send-refusal-say! ctx text)
  (let ((recipient (ctx-text ctx "did")))
    (if (non-empty-string? recipient)
        (ma-send! (canonical-actor recipient) (list :say (node-ctx) text))
        #f)))

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

; Only the source room may lock an exit. `:lock <secret>` also stores the
; secret for a traveller to use with `:unlock <secret>`; a bare `:lock`
; preserves an existing secret, matching container lock semantics.
(set-rpc-method! :lock
  (lambda (args msg)
    (cond ((not (source-room-caller? msg))
           (reply-error msg "only source room may lock this exit"))
          (else
           (if (null? args) #f (set-lock-secret! (join-words args)))
           (set-locked! #t)
           (reply-ok-with msg "locked")))))

; The source room may always unlock. A traveller holding the stored lock
; secret may also unlock; anyone else is refused.
(set-rpc-method! :unlock
  (lambda (args msg)
    (cond ((source-room-caller? msg)
           (set-locked! #f)
           (reply-ok-with msg "unlocked"))
          ((and (not (null? args))
                (equal? (join-words args) (lock-secret)))
           (set-locked! #f)
           (reply-ok-with msg "unlocked"))
          (else
           (reply-error msg "only source room or lock secret may unlock this exit")))))

; Settable exit props, mirroring container.ma's :prop shape. Only the
; source room may edit them, like :message/:lock/:unlock.
(define (editable-prop? key)
  (or (equal? key "name")
      (equal? key "nick")
      (equal? key "description")
      (equal? key "locked-msg-text")))

(define (handle-exit-prop! msg args)
  (cond ((not (source-room-caller? msg))
         (reply-error msg "only source room may edit exit props"))
        ((null? args)
         (reply-error msg "usage: :prop <name|nick|description|locked-msg-text> [value]"))
        ((not (editable-prop? (car args)))
         (reply-error msg "editable exit props: name, nick, description, locked-msg-text"))
        (else
         (begin
           (set-node-prop! (car args) (join-words (cdr args)))
           (reply-ok-with msg "prop updated")))))

(set-rpc-method! :prop
  (lambda (args msg)
    (handle-exit-prop! msg args)))

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
; A locked exit refuses unless the caller supplies the stored lock secret:
; `:traverse <ctx> <secret>`. Every refusal is also narrated to the traveller
; as a :say event so the refusal is visible without decoding the reply.
(set-rpc-method! :traverse
  (lambda (args msg)
    (let ((ctx (if (null? args) #f (car args)))
          (secret (if (or (null? args) (null? (cdr args))) #f (join-words (cdr args)))))
        (if (not (valid-did-traversal-ctx? ctx))
          (reply-error msg "traverse requires DID ctx with did and current parent")
          (if (not (or (source-room-caller? msg) (traveller-caller? ctx msg)))
              (reply-error msg "only source room or traveller may use this exit")
              (if (and (locked?) (not (and secret (equal? secret (lock-secret)))))
                  (begin
                    (send-refusal-say! ctx (locked-msg-text))
                    (reply-ok-with msg (blocked-ctx ctx)))
                  (if (target-room)
                      (reply-ok-with msg (target-ctx ctx))
                      (begin
                        (send-refusal-say! ctx "This exit leads nowhere.")
                        (reply-ok-with msg
                          (traversal-reply ctx (node-parent) "This exit leads nowhere."))))))))))

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
