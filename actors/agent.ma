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

(define (generate-transfer-id)
  (string-append (number->string (ma-random 9223372036854775807))
                 "-"
                 (number->string (ma-random 9223372036854775807))))

; Avatar initiates (owner/parent-gated); root executes (drives propose-node-parent!).
(set-rpc-method! :put
  (lambda (args msg)
    (if (node-caller-is-local-root? msg)
        (if (and (not (null? args)) (map? (car args)))
            (begin (propose-node-parent! (map-ref (car args) "container" ""))
                   (reply-ok msg))
            (reply-error msg "usage: :put <put-ctx>"))
        (let ((caller (node-effective-did args msg))
              (rest (node-effective-args args msg)))
          (cond ((not (node-transfer-caller-authorised? caller msg))
                 (reply-error msg "not authorised to put this item"))
                ((null? rest)
                 (reply-error msg "usage: :put <container-did-url>"))
                ((not (is-did-url? (car rest)))
                 (reply-error msg ":put requires a container DID-URL"))
                (else
                 (let* ((container (canonical-actor (car rest)))
                        (id (generate-transfer-id))
                        (put-ctx (make-map "action" "put"
                                           "id" id
                                           "requestor" caller
                                           "item" (canonical-actor (self))
                                           "container" container)))
                   (ma-send! (root) (list :put-request put-ctx))
                   (reply-ok-with msg id))))))))

; Root sends :take after spatial validation; item validates requestor (override to restrict).
(set-cmd-method! :take
  (lambda (args msg)
    (when (and (not (null? args))
               (map? (car args))
               (take-relay-authorised? (car args) msg))
      ; Requestor is a bare avatar DID, so use hold-admissibility (not DID-URL).
      (propose-node-hold! (map-ref (car args) "requestor" "")))))

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
