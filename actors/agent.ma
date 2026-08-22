; Generic free movable Scheme agent.
; Concrete agents extend this behaviour with movement policy.


(define AGENT_PROTOCOL "/ma/scheme/agent/0.0.1")

; Persistent state accessors. owner/set-owner!/claim-key/set-claim!/
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

(define (valid-move-ctx? ctx)
  (and (map? ctx)
       (same-actor? (ctx-text ctx "actor") (self))
       (equal? (ctx-text ctx "kind") "agent")
      (node-valid-parent-ref? (ctx-text ctx "room"))))

      ; Transfer validation keeps take/drop strict at the room boundary.
(define (valid-room-ctx? ctx)
  (and (pair? ctx)
       (equal? (ctx-alist-ref ctx :kind) "agent")
       (non-empty-string? (ctx-alist-ref ctx :room))))

(define (authorised-room-ctx? room)
  (or (same-actor? room (pending-room))
      (same-actor? room (node-parent))))

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

(define (move-to-room! target-room source-room)
  (if (and (not (movement-pending?))
           (same-actor? source-room (node-parent)))
      (begin
        (enter (canonical-actor target-room)))
      #f))

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

(set-rpc-method! :prop
  (lambda (args msg)
    (handle-node-text-prop! "agent" msg args)))

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

(set-internal-rpc-method! :enter-room
  (lambda (args msg)
    (if (or (null? args) (null? (cdr args)))
        #f
        (move-to-room! (car args) (car (cdr args))))))

(set-internal-rpc-method! :ctx
  (lambda (args msg)
    (if (null? args)
        (if (or (owner-caller? msg) (node-caller-is-parent? msg))
          (ma-reply! msg (list :ok (node-ctx)))
            #f)
           (let ((ctx (car args)))
          (cond
            ((and (valid-room-ctx? ctx)
               (same-actor? (msg-from msg) (ctx-alist-ref ctx :room))
               (authorised-room-ctx? (ctx-alist-ref ctx :room)))
             (let ((old-parent (node-parent))
                   (target-parent (ctx-alist-ref ctx :room)))
               (begin
               (set-node-parent! target-parent)
               (node-parent-committed!)
                 (ma-save-state!)
                 (if (and (non-empty-string? old-parent)
                          (not (same-actor? old-parent target-parent)))
                   (ma-send! (canonical-actor old-parent) (list :parent (node-ctx)))
                             #f)
                 (if (not (same-actor? old-parent target-parent))
                   (announce-node-parent!)
                   #f))))
            ((and (valid-move-ctx? ctx)
                  (or (node-caller-is-parent? msg) (owner-caller? msg)))
             (begin
               (if (non-empty-string? (ctx-text ctx "text"))
                 (set-last-message! (ctx-text ctx "text"))
                 #f)
               (if (same-actor? (ctx-text ctx "room") (node-parent))
                 #f
                 (move-to-room! (ctx-text ctx "room") (msg-from msg)))))
            (else #f))))))

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
                ((not (valid-did-url? (car rest)))
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
    (when (and (node-caller-is-local-root? msg)
               (not (null? args))
               (map? (car args)))
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
            ((not (valid-did? did))
             (reply-error msg "recycle requires DID with did:ma: prefix"))
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
