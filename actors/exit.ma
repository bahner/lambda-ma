; Locked exit actor.
; An exit is a top-level node with two hardcoded endpoints, `src` and `dst`.
; It is NOT a child of either room: each room references it by DID-URL in its
; own `exits` state (local name -> exit DID-URL), and that local name is purely
; cosmetic. One exit per passage means one lock, never one per player.

(define (src) (get-prop "src"))
(define (dst) (get-prop "dst"))
(define (owner) (get-prop "owner"))

(register-ctx-props! (list "src" "dst" "owner" "name" "nick" "description"))

(define (name)
  (let ((n (get-prop "name")))
    (if n n "A passage")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A passage between two rooms.")))

(define (locked?)
  (equal? (get-prop "locked") "true"))

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

; The exit's owner is the DID that dug it; only the owner manages it.
(define (exit-owner-caller? msg)
  (let ((o (owner)))
    (and o (equal? o (msg-from msg)))))

; The exit's narration props (LambdaCore names) may be set by its owner or by
; the source room, which has final cut over how the passage is told.
(define (prop-caller-authorised? msg)
  (or (exit-owner-caller? msg)
      (same-actor? (msg-from msg) (src))))

; Owner-or-source-room gate shared by :lock and :unlock. A lock is global, so
; the exit that guards the passage decides who may open and close it.
(define (lock-authorised? msg)
  (or (exit-owner-caller? msg)
      (same-actor? (msg-from msg) (src))))

; A per-side message: `key_src` / `key_dst` gives one message per endpoint,
; falling back to the generic `key`, then `default`. A side that is the source
; room is "_src"; the other is "_dst".
(define (side-msg key side-is-src default)
  (let ((specific (get-prop (if side-is-src (string-append key "_src") (string-append key "_dst")))))
    (cond ((non-empty-string? specific) specific)
          ((non-empty-string? (get-prop key)) (get-prop key))
          (else default))))

(define (nogo-text)
  (let ((t (get-prop "nogo_msg")))
    (if (non-empty-string? t) t "This is not the way.")))

; The other endpoint, given the room the caller is standing in.
(define (other-end from-room)
  (cond ((same-actor? from-room (src)) (dst))
        ((same-actor? from-room (dst)) (src))
        (else #f)))

; Memory-only: player DID -> { msg, from-room } awaiting the far end's decision.
; Never CBOR-serialised (a msg record is safe here); lost on reload/reboot,
; which is fine for an in-flight request.
(define pending-uses (make-map))

(define (bless-ctx did from-room to-room)
  (make-map "did" did
            "src" (canonical-actor from-room)
            "arrive_msg"  (side-msg "arrive_msg"  (same-actor? to-room (src)) "You arrive.")
            "oarrive_msg" (side-msg "oarrive_msg" (same-actor? to-room (src)) "arrives.")))

; Narrate a refusal to the traveller as an in-world :say event (the
; operator/visualisation stream) — never an inbox message.
(define (send-refusal-say! did text)
  (if (non-empty-string? did)
      (ma-send! (canonical-actor did) (list :event :say (node-ctx) text))
      #f))

; :use — authorise traversal. A locked exit refuses at once (nogo_msg); an open
; one blesses the far endpoint and waits for its `:reply`.
(set-rpc-method! :use
  (lambda (args msg)
    (let* ((ctx (if (null? args) #f (car args)))
           (did (if (map? ctx) (ctx-text ctx "did") #f))
           (from-room (if (map? ctx) (ctx-text ctx "parent") #f))
           (other (if (non-empty-string? from-room) (other-end from-room) #f)))
      (cond ((not (and (map? ctx) (non-empty-string? did) other))
             (reply-error msg "use requires a did and a current endpoint room"))
            ((not (or (equal? (msg-from msg) did)
                      (same-actor? (msg-from msg) from-room)))
             (reply-error msg "only the traveller or the source room may use this exit"))
            ((locked?)
             (send-refusal-say! did (nogo-text))
             (reply-ok-with msg (make-map "text" (nogo-text))))
            (else
             (begin
               (set! pending-uses
                 (map-set pending-uses did
                          (make-map "msg" msg "from-room" (canonical-actor from-room))))
               (ma-send! (canonical-actor other) (list :bless (bless-ctx did from-room other)))
               #f))))))

; The far endpoint's answer, keyed by the traveller's DID.
(set-rpc-method! :reply
  (lambda (args msg)
    (let* ((did (if (null? args) #f (car args)))
           (r   (if (or (null? args) (null? (cdr args))) #f (cadr args)))
           (entry (if (non-empty-string? did) (map-ref pending-uses did #f) #f))
           (from-room (if (map? entry) (map-ref entry "from-room" #f) #f))
           (to-room (if (non-empty-string? from-room) (other-end from-room) #f)))
      (if (and (map? entry) (non-empty-string? from-room)
               (same-actor? (msg-from msg) to-room))
          (let ((use-msg (map-ref entry "msg")))
            (set! pending-uses (map-delete pending-uses did))
            (if (string? r)
                ; Accept: r is the room's one-time entry secret.
                (begin
                  (reply-ok-with use-msg
                    (make-map "dst" (canonical-actor to-room) "secret" r))
                  ; Departure: the exit drives the source room's narration.
                  (ma-send! (canonical-actor from-room)
                            (list :leave (make-map
                                           "did" did
                                           "leave_msg"  (side-msg "leave_msg"  (same-actor? from-room (src)) "You leave.")
                                           "oleave_msg" (side-msg "oleave_msg" (same-actor? from-room (src)) "leaves.")))))
                ; Deny: the room may override our nogo message.
                (let ((t (if (map? r) (ctx-text r "nogo_msg") #f)))
                  (reply-ok-with use-msg
                    (make-map "text" (if (non-empty-string? t) t (nogo-text)))))))
          (reply-error msg "no pending use for that did")))))

; Read-only introspection. `:to? <from-room>` gives the far endpoint.
(set-rpc-method! :to?
  (lambda (args msg)
    (let ((from-room (if (null? args) #f (car args))))
      (reply-ok-with msg
        (let ((other (if (non-empty-string? from-room) (other-end from-room) #f)))
          (cond (other (canonical-actor other))
                ((non-empty-string? (dst)) (canonical-actor (dst)))
                (else "(none)")))))))

(define (about-text)
  (string-append
    (name) "\n"
    (description) "\n"
    "src: " (if (non-empty-string? (src)) (src) "(none)") "\n"
    "dst: " (if (non-empty-string? (dst)) (dst) "(none)") "\n"
    "owner: " (if (owner) (owner) "(none)") "\n"
    "locked: " (if (locked?) "true" "false")))

(set-rpc-method! :about
  (lambda (args msg) (reply-ok-with msg (about-text))))

(set-rpc-method! :look
  (lambda (args msg) (reply-ok-with msg (about-text))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (non-empty-string? (src)) (src) "(none)"))))

(set-rpc-method! :owner
  (lambda (args msg)
    (reply-ok-with msg (if (owner) (owner) "(none)"))))

(set-rpc-method! :owner?
  (lambda (args msg)
    (if (null? args)
        (reply-ok-with msg (string-append "Owner: " (if (owner) (owner) "(none)")))
        (begin
          (ma-send! (canonical-actor (car args))
                    (list :print (string-append "Owner: " (if (owner) (owner) "(none)"))))
          (reply-ok msg)))))

(set-rpc-method! :locked?
  (lambda (args msg)
    (reply-ok-with msg (if (locked?) "true" "false"))))

; The narration vocabulary in one place: `:message <slot> <text>` writes the
; LambdaCore-named prop for that slot, so authors set the words by a short,
; validated name (`leave`, `oleave`, `arrive`, `oarrive`, `nogo`) rather than
; remembering the `_msg` spelling, and a typo is refused rather than silently
; creating a stray prop.
(define message-slots
  (make-map "leave"   "leave_msg"
            "oleave"  "oleave_msg"
            "arrive"  "arrive_msg"
            "oarrive" "oarrive_msg"
            "nogo"    "nogo_msg"))

(set-rpc-method! :message
  (lambda (args msg)
    (let ((slot (if (null? args) #f (car args)))
          (text (join-words (if (or (null? args) (null? (cdr args))) '() (cdr args)))))
      (cond ((not (prop-caller-authorised? msg))
             (reply-error msg "only this exit's owner or source room may set its messages"))
            ((not (and (non-empty-string? slot) (map-has-key? message-slots slot)))
             (reply-error msg "unknown message slot; use leave, oleave, arrive, oarrive, or nogo"))
            (else
             (set-prop! (map-ref message-slots slot #f) text)
             (ma-save-state!)
             (reply-ok-with msg slot))))))

; A lock is global, never per-player: `:lock <secret>` closes it for everyone;
; `:unlock [<secret>]` opens it for everyone until the owner locks it again.
(set-rpc-method! :lock
  (lambda (args msg)
    (if (not (lock-authorised? msg))
        (reply-error msg "only this exit's owner may lock it")
        (begin
          (if (null? args) #f (set-lock-secret! (join-words args)))
          (set-locked! #t)
          (reply-ok-with msg "locked")))))

(set-rpc-method! :unlock
  (lambda (args msg)
    (cond ((lock-authorised? msg)
           (set-locked! #f)
           (reply-ok-with msg "unlocked"))
          ((and (not (null? args))
                (equal? (join-words args) (lock-secret)))
           (set-locked! #f)
           (reply-ok-with msg "unlocked"))
          (else
           (reply-error msg "only this exit's owner or lock secret may unlock it")))))

; Only the source room may end this passage: an exit is shared by two rooms,
; and the room that dug it owns its lifetime from the near side.
(set-internal-rpc-method! :fill
  (lambda (args msg)
    (if (same-actor? (msg-from msg) (src))
        (ma-end)
        #f)))

(define (node-protocol) "/ma/exit/0.0.1")
(define (node-kind) "exit")
(define (node-name) (name))
(define (node-nick) (name))
(define (node-description) (description))

(define (extend-node-ctx ctx)
  (map-set
    (map-set ctx "src" (src))
    "dst" (dst)))
