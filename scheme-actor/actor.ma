; ma-scheme actor library.
;
; Composed by /ma/scheme/actor/0.0.1 above the generic stdlib. This layer may
; use actor host functions and runtime configuration, but not persisted state.

; Signal/verb term helpers (ma-scheme-v1.md §6).
(define (verb-of term) (if (pair? term) (car term) term))
(define (args-of term) (if (pair? term) (cdr term) '()))

; Lambda-ma identity helpers. Room ownership is DID based; an avatar is
; recognised by deterministic fragment derivation, not by a forwarded argument.
(define AVATAR_KIND "/ma/avatar/0.0.1")
(define LAMBDA_CTX_PROTOCOL "/ma/lambda/ctx/0.0.1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (root)
  (let ((configured (ma-get-config-key "root")))
    (if configured configured (entity-url "root"))))

(define (entity-url fragment)
  (string-append (runtime) "#" fragment))

(define (valid-did? value)
  (and (string? value)
       (string-prefix? "did:ma:" value)
       (not (string-contains? "#" value))))

(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor))
      (string-append (runtime) actor)
      actor))

(define (config-actor actor)
  (cond ((not actor) #f)
        ((string-prefix? "did:ma:" actor) actor)
        ((string-prefix? "#" actor) (string-append (runtime) actor))
        (else (string-append (runtime) "#" actor))))

(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))

(define (local-actor-ref? actor)
  (and (string? actor)
  (string-prefix? (string-append (runtime) "#") actor)))

(define (valid-did-url? actor)
  (and (string? actor)
       (string-prefix? "did:ma:" actor)
       (string-contains? "#" actor)))

(define (entity-live? actor)
  (and actor (ma-entity-exists? (canonical-actor actor))))

(define (dead-local-actor? actor)
  (and (local-actor-ref? actor) (not (entity-live? actor))))

(define (avatar-fragment did)
  (blake3 (string-append "lambda-ma avatar v1\n" (runtime) "\n" did) 8))

(define (avatar-for-did did)
  (string-append (runtime) "#" (avatar-fragment did)))

(define (did-avatar? did actor)
  (and (valid-did? did)
       (string? actor)
       (equal? (canonical-actor actor) (avatar-for-did did))))

(define (msg-from-owner? owner msg)
  (let ((from (msg-from msg)))
    (or (equal? from owner)
        (did-avatar? owner from))))

(define (canonical-entry entry)
  (canonical-actor entry))

(define (same-entry? a b)
  (equal? (canonical-entry a) (canonical-entry b)))

(define (member-entry? entry xs)
  (cond ((null? xs) #f)
        ((same-entry? entry (car xs)) #t)
        (else (member-entry? entry (cdr xs)))))

(define (unique-entries xs)
  (let loop ((rest xs) (acc '()))
    (cond ((null? rest) acc)
          ((member-entry? (car rest) acc) (loop (cdr rest) acc))
          (else (loop (cdr rest) (cons (canonical-entry (car rest)) acc))))))

(define (unique-actor-entries xs)
  (unique-entries (unique-string-entries xs)))

(define (without-entries xs drop)
  (cond ((null? xs) '())
        ((member-entry? (car xs) drop)
         (without-entries (cdr xs) drop))
        (else
         (cons (car xs) (without-entries (cdr xs) drop)))))

(define (reply-ok msg)
  (ma-reply! msg :ok))

(define (reply-ok-with msg payload)
  (ma-reply! msg (list :ok payload)))

(define (reply-error msg text)
  (ma-reply! msg (list :error text)))

(define (actor-prop-or-none key)
  (let ((value (get-prop key)))
    (if (and value (not (equal? value ""))) value "(none)")))

(define (actor-config-or-none key)
  (let ((value (ma-get-config-key key)))
    (if (and value (not (equal? value ""))) value "(none)")))

(define (actor-name) (actor-prop-or-none "name"))
(define (actor-description) (actor-prop-or-none "description"))
(define (actor-kind) (actor-config-or-none "kind"))

(define (actor-owner) (actor-prop-or-none "owner"))
(define (actor-parent) (actor-prop-or-none "parent"))

(define (children-map)
  (prop-map "children"))

(define (set-children-map! children)
  (set-prop-map! "children" children)
  (ma-save-state!))

(define (child-ctx-valid? ctx)
  (and (actor-ctx? ctx)
       (valid-did-url? (ctx-text ctx "actor"))))

(define (remember-child! ctx)
  (set-children-map! (map-set (children-map) (canonical-actor (ctx-text ctx "actor")) ctx)))

(define (child-label ctx)
  (let ((nick (ctx-text ctx "nick"))
        (name (ctx-text ctx "name"))
        (actor (ctx-text ctx "actor")))
    (cond ((non-empty-string? nick) nick)
          ((non-empty-string? name) name)
          (else actor))))

(define (child-line entry)
  (let ((actor (car entry))
        (ctx (cdr entry)))
    (string-append (child-label ctx) " = " (canonical-actor actor))))

(define (actor-entry-lines xs)
  (cond ((null? xs) "")
        ((null? (cdr xs)) (car xs))
        (else (string-append (car xs) "\n" (actor-entry-lines (cdr xs))))))

(define (child-lines entries)
  (cond ((null? entries) '())
        (else (cons (child-line (car entries)) (child-lines (cdr entries))))))

(define (children-text)
  (let ((lines (child-lines (map->alist (children-map)))))
    (if (null? lines)
        "Children: none."
        (string-append "Children:\n" (actor-entry-lines lines)))))

(define (owner-authority)
  (let ((owner (get-prop "owner")))
    (if owner owner (get-prop "did"))))

(define (owner-authorised? msg)
  (let ((owner (owner-authority)))
    (and owner (msg-from-owner? owner msg))))

(define (handle-actor-owner args msg)
  (reply-ok-with msg (actor-owner)))

(define (handle-actor-text-prop! key getter args msg)
  (if (null? args)
      (reply-ok-with msg (getter))
      (if (owner-authorised? msg)
          (begin
            (set-prop! key (join-words args))
            (ma-save-state!)
            (reply-ok-with msg (getter)))
          (reply-error msg (string-append "only owner may set " key)))))

(define (handle-actor-name args msg)
  (handle-actor-text-prop! "name" actor-name args msg))

(define (handle-actor-description args msg)
  (handle-actor-text-prop! "description" actor-description args msg))

(define (handle-actor-kind args msg)
  (if (null? args)
  (reply-ok-with msg (actor-kind))
      (reply-error msg "kind is read-only")))

(define (handle-actor-owner? args msg)
  (if (null? args)
      (reply-ok-with msg (string-append "Owner: " (actor-owner)))
      (reply-error msg "usage: :owner?")))

(define (handle-actor-parent args msg)
  (reply-ok-with msg (actor-parent)))

(define (handle-actor-parent? args msg)
  (if (null? args)
  (reply-ok-with msg (actor-parent))
      (reply-error msg "usage: :parent?")))

(define (handle-actor-children args msg)
  (cond ((null? args)
         (if (owner-authorised? msg)
             (reply-ok-with msg (children-text))
             (reply-error msg "only owner may list children")))
        ((not (null? (cdr args)))
         (reply-error msg "usage: :children [ctx]"))
        ((not (child-ctx-valid? (car args)))
         (reply-error msg "children ctx must include actor, kind, name, nick, description"))
        ((not (same-actor? (msg-from msg) (ctx-text (car args) "actor")))
         (reply-error msg "children ctx actor must match sender"))
        (else
         (begin
           (remember-child! (car args))
           (reply-ok msg)))))

(define (handle-actor-behaviour! args msg)
  (cond ((null? args)
         (let ((current (ma-get-config-key "behaviour")))
           (if current
               (reply-ok-with msg current)
               (reply-ok-with msg "No custom behaviour is set for this actor."))))
        ((null? (cdr args))
         (let ((actor-owner (get-prop "owner")))
           (cond ((not actor-owner)
                  (reply-error msg "This actor is unowned. Claim it before editing behaviour."))
                 ((not (msg-from-owner? actor-owner msg))
                  (reply-error msg "Only this actor's owner can edit behaviour."))
                 (else
                  (begin
                    (ma-set-behaviour! (car args))
                    (reply-ok-with msg "Behaviour update queued."))))))
        (else
         (reply-error msg "Usage: behaviour /ipfs/<cid>"))))

(define (on-signal term)
  #f)

(define *methods* '())
(define *default-method* #f)
(define *rpc-methods* '())
(define *cmd-methods* '())
(define *meta-methods* '())

(define (method-member? verb xs)
  (cond ((null? xs) #f)
        ((equal? verb (car xs)) #t)
        (else (method-member? verb (cdr xs)))))

(define (add-method-entry entries verb)
  (if (method-member? verb entries)
      entries
      (list-append entries (list verb))))

(define (remove-method-entry entries verb)
  (cond ((null? entries) '())
        ((equal? verb (car entries)) (remove-method-entry (cdr entries) verb))
        (else (cons (car entries) (remove-method-entry (cdr entries) verb)))))

(define (remove-method-pairs entries verb)
  (cond ((null? entries) '())
        ((equal? verb (car (car entries))) (remove-method-pairs (cdr entries) verb))
        (else (cons (car entries) (remove-method-pairs (cdr entries) verb)))))

(define (forget-method-category! verb)
  (set! *rpc-methods* (remove-method-entry *rpc-methods* verb))
  (set! *cmd-methods* (remove-method-entry *cmd-methods* verb))
  (set! *meta-methods* (remove-method-entry *meta-methods* verb)))

(define (unset-method! verb)
  (set! *methods* (remove-method-pairs *methods* verb))
  (forget-method-category! verb))

(define (set-categorised-method! verb fn kind)
  (set-method! verb fn)
  (forget-method-category! verb)
  (cond ((equal? kind :rpc)
         (set! *rpc-methods* (add-method-entry *rpc-methods* verb)))
        ((equal? kind :cmd)
         (set! *cmd-methods* (add-method-entry *cmd-methods* verb)))
        ((equal? kind :meta)
         (set! *meta-methods* (add-method-entry *meta-methods* verb)))
        (else #f)))

(define (set-method! verb fn)
  (set! *methods* (cons (cons verb fn) *methods*)))

(define (set-rpc-method! verb fn)
  (set-categorised-method! verb fn :rpc))

(define (set-cmd-method! verb fn)
  (set-categorised-method! verb fn :cmd))

(define (set-meta-method! verb fn)
  (set-categorised-method! verb fn :meta))

(define (set-internal-rpc-method! verb fn)
  (set-method! verb fn)
  (forget-method-category! verb))

(define (actor-rpcs) *rpc-methods*)
(define (actor-cmds) *cmd-methods*)
(define (actor-metas) *meta-methods*)

(define (actor-api)
  (list (list :rpcs? (actor-rpcs))
        (list :cmds? (actor-cmds))
        (list :metas? (actor-metas))))

(define (handle-actor-rpcs? args msg)
  (reply-ok-with msg (actor-rpcs)))

(define (handle-actor-cmds? args msg)
  (reply-ok-with msg (actor-cmds)))

(define (handle-actor-metas? args msg)
  (reply-ok-with msg (actor-metas)))

(define (handle-actor-api? args msg)
  (reply-ok-with msg (actor-api)))

(set-rpc-method! :name handle-actor-name)
(set-rpc-method! :description handle-actor-description)
(set-rpc-method! :kind? handle-actor-kind)
(set-rpc-method! :owner handle-actor-owner)
(set-rpc-method! :owner? handle-actor-owner?)
(set-meta-method! :parent handle-actor-parent)
(set-rpc-method! :parent? handle-actor-parent?)
(set-meta-method! :children handle-actor-children)
(set-rpc-method! :behaviour handle-actor-behaviour!)
(set-rpc-method! :rpcs? handle-actor-rpcs?)
(set-rpc-method! :cmds? handle-actor-cmds?)
(set-rpc-method! :metas? handle-actor-metas?)
(set-rpc-method! :api? handle-actor-api?)

(define (set-default-method! fn)
  (set! *default-method* fn))

(define (find-method verb)
  (let loop ((table *methods*))
    (cond ((null? table) #f)
          ((equal? (car (car table)) verb) (cdr (car table)))
          (else (loop (cdr table))))))

(define (on-message msg)
  (let* ((term (msg-content msg))
         (verb (verb-of term))
         (args (args-of term))
         (fn (find-method verb)))
    (if fn
        (begin
          (fn args msg)
          #f)
      (if *default-method*
        (begin
          (*default-method* verb args msg)
          #f)
        (ma-reply! msg (list :error "unknown verb"))))))
