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

(define (editable-prop? key)
  (or (equal? key "name")
      (equal? key "nick")
      (equal? key "description")))

(define (set-thing-prop! key value)
  (set-node-prop! key value))

(define (handle-thing-prop! msg args)
  (cond ((not (owner-caller? msg))
         (reply-error msg "only owner may edit thing props"))
        ((null? args)
         (reply-error msg "usage: :prop <name|nick|description> [value]"))
        ((not (editable-prop? (car args)))
         (reply-error msg "editable thing props: name, nick, description"))
        (else
         (begin
           (set-thing-prop! (car args) (join-words (cdr args)))
           (reply-ok-with msg "prop updated")))))

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

(set-internal-rpc-method! :report-parent
  (lambda (args msg)
    (let ((tick (if (or (null? args) (null? (cdr args))) "" (car (cdr args))))
          (nonce (if (or (null? args) (null? (cdr args)) (null? (cdr (cdr args)))) "" (car (cdr (cdr args))))))
      (ma-send! (canonical-actor (msg-from msg))
                (list :parent-report (canonical-actor (self)) (node-parent) tick nonce)))))

(set-rpc-method! :owner handle-node-owner)
(set-rpc-method! :owner? handle-node-owner?)

(set-rpc-method! :prop
  (lambda (args msg)
    (handle-thing-prop! msg args)))

(set-rpc-method! :set-recovery-secret handle-node-set-recovery-secret!)

(set-cmd-method! :claim handle-node-claim!)

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
