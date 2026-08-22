; Free movable thing actor.
; Authority lives in thing state: owner + parent.

(define THING_PROTOCOL "/ma/thing/0.0.1")

; Persistent state accessors. owner/set-owner!/claim-key/set-claim!/
; recovery-secret/set-recovery-secret!/owner-caller? live in node.ma.
(define (name)
  (let ((n (get-prop "name")))
    (if n n "thing")))

(define (description)
  (let ((d (get-prop "description")))
    (if d d "A small movable thing.")))

(define (nick)
  (let ((n (get-prop "nick")))
    (if n n (name))))

(define (node-protocol) THING_PROTOCOL)
(define (node-kind) "thing")
(define (node-name) (name))
(define (node-nick) (nick))
(define (node-description) (description))

(define (recycle! msg)
  (let ((old-parent (node-parent)))
    (begin
      (set-node-parent! "")
      (if (non-empty-string? old-parent)
          (ma-send! (canonical-actor old-parent) (list :parent (node-ctx-for-parent "")))
          #f)
      (ma-end))))

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
    (handle-node-text-prop! "thing" msg args)))

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

; Parent-mediated transfer: the object proposes itself to a new parent
; (ma-spec sec 6); direct did calls are deliberately rejected. Shared body
; lives in node.ma as handle-node-set-parent!.
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
             (reply-error msg "only owner may recycle this thing"))
            (else
             (recycle! msg))))))

(define (on-signal term)
  (cond ((equal? (verb-of term) :start)
      (announce-node-parent!))
        ((equal? (verb-of term) :shutdown)
         (ma-save-state!))
        (else #f)))
