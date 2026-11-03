; Locked exit actor.
; An exit is a top-level node with two hardcoded endpoints, `src` and `dst`.
; It is NOT a child of either room: each room references it by DID-URL in its
; own `exits` state (local name -> exit DID-URL), and that local name is purely
; cosmetic. One exit per passage means one lock, never one per player.

(define (entrance) (get-prop "entrance"))
(define (exit) (get-prop "exit"))
(define (owner) (get-prop "owner"))

(register-ctx-props! (list "entrance" "exit" "owner" "name" "nick" "description"))

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
      (same-actor? (msg-from msg) (entrance))))

; Owner-or-source-room gate for :lock. A lock is global, so the exit that
; guards the passage decides who may close and reconfigure it.
(define (lock-authorised? msg)
  (or (exit-owner-caller? msg)
      (same-actor? (msg-from msg) (entrance))))

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
  (cond ((same-actor? from-room (entrance)) (exit))
        ((same-actor? from-room (exit)) (entrance))
        (else #f)))

; Memory-only: traverseId -> { source, did, ctx, target, hops } for one in-flight
; traversal. Never CBOR-serialised; lost on reload/reboot, which is fine for a
; request that is still in flight.
(define pending-traversals (make-map))

(define (onogo-text)
  (let ((t (get-prop "onogo_msg")))
    (if (non-empty-string? t) t "tries to leave but cannot.")))

; The move ctx the destination receives: the caller's ctx plus the endpoint this
; exit chose. `entrance` names the room being asked, so an answer can always be
; correlated even when a redirect changes it.
(define (routed-ctx ctx target)
  (map-set ctx "entrance" (canonical-actor target)))

; Relay a terminal move ctx to the source room and forget the traversal.
; The source room identifies an answer by `entrance` naming itself, so the
; relay adds it explicitly rather than relying on the destination's answer to
; carry a field it never saw.
(define (finish-move! tid ctx)
  (let ((entry (map-ref pending-traversals tid #f)))
    (set! pending-traversals (map-delete pending-traversals tid))
    (if (map? entry)
        (let ((source (map-ref entry "source")))
          (ma-send! source
                    (list :move (map-set (map-set ctx "traverseId" tid)
                                         "entrance" source))))
        #f)))

(define (refuse-move! tid ctx reason)
  (finish-move! tid
    (map-set ctx "deny" (if (non-empty-string? reason) reason (nogo-text)))))

; :move — route an actor described by a move ctx. Two shapes arrive here: a
; request from an endpoint room (`entrance` names that room, no live traversal)
; and an answer from the destination currently being asked. The passage never
; moves anyone; the traveller does that with `:enter`.
(set-cmd-method! :move
  (lambda (args msg)
    (let* ((ctx (if (null? args) #f (car args)))
           (tid (if (map? ctx) (ctx-text ctx "traverseId") #f))
           (entry (if (non-empty-string? tid) (map-ref pending-traversals tid #f) #f))
           (target (if (map? entry) (map-ref entry "target") #f))
           (from-room (if (map? ctx) (ctx-text ctx "entrance") #f))
           (other (if (non-empty-string? from-room) (other-end from-room) #f)))
      (cond
        ((and (map? entry) (same-actor? (msg-from msg) target))
         (let ((answer-exit (ctx-text ctx "exit"))
               (deny (ctx-text ctx "deny"))
               (hops (map-ref entry "hops" 0)))
           (cond ((non-empty-string? deny)
                  (finish-move! tid ctx)
                  (reply-ok msg))
                 ((not (equal? (canonical-actor answer-exit) (canonical-actor target)))
                  (let ((next (if (is-did-url? answer-exit) (canonical-actor answer-exit) #f)))
                    (cond ((not next)
                           (refuse-move! tid ctx "invalid redirect")
                           (reply-ok msg))
                          ((>= hops 3)
                           (refuse-move! tid ctx "too many redirects")
                           (reply-ok msg))
                          ((not (equal? (actor-runtime next) (actor-runtime target)))
                           (refuse-move! tid ctx "redirect crosses a runtime boundary")
                           (reply-ok msg))
                          (else
                           (set! pending-traversals
                             (map-set pending-traversals tid
                                      (map-set (map-set entry "target" next) "hops" (+ hops 1))))
                           (ma-send! next (list :move (map-set ctx "exit" next)))
                           (reply-ok msg)))))
                 (else
                  (finish-move! tid ctx)
                  (reply-ok msg)))))
        ((not (and (map? ctx) (non-empty-string? tid) other))
         (reply-error msg "move requires did, traverseId and a current endpoint room"))
        ((not (same-actor? (msg-from msg) from-room))
         (reply-error msg "only the entrance room may move through this passage"))
        ((locked?)
         (begin
           (ma-send! (canonical-actor from-room)
                     (list :move (make-map "traverseId" tid "deny" (nogo-text))))
           (reply-ok msg)))
        (else
         (begin
           (set! pending-traversals
             (map-set pending-traversals tid
                      (make-map "source" (canonical-actor from-room)
                                "did" (ctx-text ctx "did")
                                "ctx" ctx
                                "target" (canonical-actor other)
                                "hops" 0)))
           (ma-send! (canonical-actor other)
                     (list :move (map-set ctx "exit" (canonical-actor other))))
           (reply-ok msg)))))))

; Read-only introspection. `:to? <from-room>` gives the far endpoint.
(set-rpc-method! :to?
  (lambda (args msg)
    (let ((from-room (if (null? args) #f (car args))))
      (reply-ok-with msg
        (let ((other (if (non-empty-string? from-room) (other-end from-room) #f)))
          (cond (other (canonical-actor other))
                ((non-empty-string? (exit)) (canonical-actor (exit)))
                (else "(none)")))))))

(define (about-text)
  (string-append
    (name) "\n"
    (description) "\n"
    "entrance: " (if (non-empty-string? (entrance)) (entrance) "(none)") "\n"
    "exit: " (if (non-empty-string? (exit)) (exit) "(none)") "\n"
    "owner: " (if (owner) (owner) "(none)") "\n"
    "locked: " (if (locked?) "true" "false")))

(set-rpc-method! :about
  (lambda (args msg) (reply-ok-with msg (about-text))))

(set-rpc-method! :look
  (lambda (args msg) (reply-ok-with msg (about-text))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (non-empty-string? (entrance)) (entrance) "(none)"))))

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
            "nogo"    "nogo_msg"
            "onogo"   "onogo_msg"))

(set-rpc-method! :message
  (lambda (args msg)
    (let ((slot (if (null? args) #f (car args)))
          (text (join-words (if (or (null? args) (null? (cdr args))) '() (cdr args)))))
      (cond ((not (prop-caller-authorised? msg))
             (reply-error msg "only this exit's owner or source room may set its messages"))
            ((not (and (non-empty-string? slot) (map-has-key? message-slots slot)))
             (reply-error msg "unknown message slot; use leave, oleave, arrive, oarrive, nogo, or onogo"))
            (else
             (set-prop! (map-ref message-slots slot #f) text)
             (ma-save-state!)
             (reply-ok-with msg slot))))))

; The runtime-wide break-glass key, shared with rooms. It unlocks any locked
; exit and admits the admin to any private room. Set `global_key` in the
; runtime's `ma.yaml`; it is never exposed or writable over CRUD. Absent or
; empty disables it.
(define (global-key)
  (let ((k (ma-get-config-key "global_key")))
    (if (non-empty-string? k) k #f)))

; A lock is global, never per-player: `:lock <secret>` closes it for everyone;
; `:unlock <secret>` opens it for everyone until the owner locks it again. There
; is no owner bypass: the owner presents the lock secret or the runtime global
; key like anyone else, so the gate is testable.
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
    (let ((secret (if (null? args) #f (join-words args))))
      (cond ((and secret (equal? secret (lock-secret)))
             (set-locked! #f)
             (reply-ok-with msg "unlocked"))
            ((and secret (global-key) (equal? secret (global-key)))
             (set-locked! #f)
             (reply-ok-with msg "unlocked"))
            (else
             (reply-error msg "present the lock secret or the runtime global key"))))))

; Only the source room may end this passage: an exit is shared by two rooms,
; and the room that dug it owns its lifetime from the near side.
(set-internal-rpc-method! :fill
  (lambda (args msg)
    (if (same-actor? (msg-from msg) (entrance))
        (ma-end)
        #f)))

(define (node-protocol) "/ma/passage/0.0.1")
(define (node-kind) "passage")
(define (node-name) (name))
(define (node-nick) (name))
(define (node-description) (description))

(define (extend-node-ctx ctx)
  (map-set
    (map-set ctx "entrance" (entrance))
    "exit" (exit)))
