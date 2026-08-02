; Locked exit actor.
; Exits are traversal entities owned by rooms or by root for world entry.

; Persistent endpoint state.
(define (target-room) (get-prop "target-room"))
(define (direction) (get-prop "direction"))
(define (owner) (get-prop "owner"))

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

(define (ctx-recipient ctx)
  (let ((avatar (ctx-text ctx "avatar"))
        (actor (ctx-text ctx "actor")))
    (if (non-empty-string? avatar) avatar actor)))

(define (traveller-caller? ctx msg)
  (same-actor? (msg-from msg) (ctx-recipient ctx)))

(define (set-locked! value)
  (set-prop! "locked" (if value "true" "false"))
  (ma-save-state!))

(define (set-message! slot text)
  (begin
    (set-prop! (string-append slot "-message") text)
    (ma-save-state!)))

(define (valid-avatar-ctx? ctx)
  (and (map? ctx)
  (equal? (ctx-text ctx "kind") "avatar")
  (non-empty-string? (ctx-text ctx "did"))
  (non-empty-string? (ctx-text ctx "avatar"))
      (same-actor? (ctx-text ctx "room") (node-parent))))

(define (annotate-avatar-ctx ctx room text)
  (map-set
    (map-set
      (map-set
        (map-set ctx "room" (canonical-actor room))
        "text" text)
      "exit" (canonical-actor (self)))
    "direction" (direction)))

(define (blocked-ctx ctx)
  (annotate-avatar-ctx ctx (node-parent) (blocked-message)))

(define (target-ctx ctx)
  (annotate-avatar-ctx ctx (target-room) (traveller-message)))

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
  (let ((avatar (ctx-text ctx "avatar"))
        (did (ctx-text ctx "did"))
        (actor (ctx-text ctx "actor")))
    (cond ((non-empty-string? avatar) avatar)
          ((non-empty-string? did) did)
          ((non-empty-string? actor) actor)
          (else #f))))

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
    ((find-method :about) args msg)))

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

(set-internal-rpc-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args))))))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (canonical-actor (self)) (node-parent) tick nonce)))))

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

; Exit policy transforms avatar ctx state and returns it to the moving avatar.
; The avatar asks the target room to :enter, so target admission remains target-room authority.
(set-internal-rpc-method! :ctx
  (lambda (args msg)
    (let ((ctx (if (null? args) #f (car args))))
      (cond ((not (valid-avatar-ctx? ctx))
             (reply-error msg "exit ctx requires avatar ctx with did, avatar, kind, protocol, and current room"))
            ((not (or (source-room-caller? msg) (traveller-caller? ctx msg)))
             (reply-error msg "only source room or traveller may use this exit"))
            ((locked?)
             (begin
               (ma-send! (canonical-actor (ctx-recipient ctx)) (list :ctx (blocked-ctx ctx)))
               (reply-ok msg)))
            ((target-room)
             (begin
               (ma-send! (canonical-actor (ctx-recipient ctx)) (list :ctx (target-ctx ctx)))
               (reply-ok msg)))
            (else
             (begin
               (ma-send! (canonical-actor (ctx-recipient ctx))
                         (list :ctx (annotate-avatar-ctx ctx (node-parent) "This exit leads nowhere.")))
               (reply-ok msg)))))))

(define (node-protocol) "/ma/exit/0.0.1")
(define (node-kind) "actor")
(define (node-name) (name))
(define (node-nick) (name))
(define (node-description) (description))
