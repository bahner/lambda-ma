; Generic free movable Scheme agent.
; Concrete agents extend this behaviour with movement policy.


(define AGENT_PROTOCOL "/ma/scheme/agent/0.0.1")

; Persistent state accessors. owner/set-owner!/recovery-secret/set-recovery-secret!/
; recovery-secret/set-recovery-secret!/owner-caller? live in node.ma.
(define (name)
  (let ((n (get-prop "name")))
    (if n n "agent")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable agent.")))

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (pending-room)
  (let ((p (get-prop "pending-room")))
    (if p p "")))

(define (movement-pending?)
  (not (equal? (pending-room) "")))

(define (set-pending-room! room)
  (set-prop! "pending-room" room)
  (ma-save-state!))

(define (set-last-message! text)
  (set-prop! "last-message" text)
  (ma-save-state!))

(define (enter room)
  (begin
    (set-pending-room! (canonical-actor room))
    (ma-send! (canonical-actor room) (list :enter (node-ctx-for-parent room)))))

(define (leave-current-parent!)
  (let ((p (node-parent)))
    (if (equal? p "")
        #f
        (ma-send! (canonical-actor p) (list :leave-occupant)))))

(define (movement-caller? msg)
  (or
  (not (owner))
  (owner-caller? msg)))

(define (ctx-alist-ref ctx key)
  (cond ((null? ctx) #f)
        ((and (pair? (car ctx))
              (equal? (car (car ctx)) key)
              (pair? (cdr (car ctx))))
         (car (cdr (car ctx))))
        (else (ctx-alist-ref (cdr ctx) key))))

(define (recycle! msg)
  (let ((old-parent (node-parent)))
    (begin
      (set-node-parent! "")
      (del-prop! "pending-room")
      (if (non-empty-string? old-parent)
          (ma-send! (canonical-actor old-parent) (list :parent (node-ctx-for-parent "")))
          #f)
      (ma-end))))

; Room context and movement helpers.
(define (node-protocol) AGENT_PROTOCOL)
(define (node-kind) "agent")
(define (node-name) (name))
(define (node-nick) (nick))
(define (node-description) (description))

(define (node-parent-committed!)
  (begin
    (del-prop! "pending-room")
    (ma-save-state!)))

(define (send-parent-room! msg term)
  (let ((p (node-parent)))
    (if (equal? p "")
        (reply-error msg (string-append (nick) " is nowhere"))
        (begin
          (ma-send! (canonical-actor p) term)
          (reply-ok msg)))))

; Public methods.
(set-rpc-method! :about
  (lambda (args msg)
    (reply-ok-with msg
      (string-append
        (name) "\n"
        (description) "\n"
        "owner: " (if (owner) (owner) "(none)") "\n"
        "parent: " (if (equal? (node-parent) "") "(none)" (node-parent))))))

(set-rpc-method! :look
  (lambda (args msg)
    (reply-ok-with msg (string-append (name) "\n" (description)))))

(set-rpc-method! :where?
  (lambda (args msg)
    (reply-ok-with msg (if (equal? (node-parent) "") "(none)" (node-parent)))))

(set-internal-rpc-method! :report-parent handle-node-report-parent!)

(set-rpc-method! :owner handle-node-owner)
(set-rpc-method! :owner? handle-node-owner?)

(set-internal-rpc-method! :print
  (lambda (args msg)
    (set-last-message! (join-words args))))

(set-rpc-method! :exits?
  (lambda (args msg)
    (send-parent-room! msg (list :exits?))))

(set-cmd-method! :move
  (lambda (args msg)
    (cond ((movement-pending?)
           (reply-error msg "movement already pending"))
          ((movement-caller? msg)
           (send-parent-room! msg (list :move)))
          (else
           (reply-error msg "only a free agent or owner may move this agent")))))

(set-rpc-method! :set-recovery-secret handle-node-set-recovery-secret!)

(set-cmd-method! :claim handle-node-claim!)

; Avatar-initiated item moves (:put) and root-relayed item takes (:take) share
; one node.ma body each — identical for thing/agent/trigger.
(set-rpc-method! :put handle-node-put!)
(set-cmd-method! :take handle-node-take-relay!)

; Parent-mediated transfer: the agent proposes itself to a new (non-room)
; parent, e.g. being picked up as an object. Shared body lives in node.ma as
; handle-node-set-parent!. Room entry is a distinct protocol (:ctx/:enter,
; below) with its own occupancy/exit checks and is NOT folded in here.
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
            ((not (node-recycle-caller-authorised? did msg))
             (reply-error msg "only owner may recycle this agent"))
            (else
             (recycle! msg))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
         (announce-node-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
