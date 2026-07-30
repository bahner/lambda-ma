; ma-scheme actor library.
;
; Composed by /ma/scheme/actor/0.0.1 above the generic stdlib. This layer may
; use actor host functions and runtime configuration, but not persisted state.

; Signal/verb term helpers (ma-scheme-v1.md §6).
(define (verb-of term) (if (pair? term) (car term) term))
(define (args-of term) (if (pair? term) (cdr term) '()))

; Lambda-ma identity helpers. Room ownership is user-DID based; an avatar is
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

(define (user-did? value)
  (and (string? value)
       (string-prefix? "did:ma:" value)
       (not (string-contains? value "#"))))

(define (valid-user-did? did)
  (and (string? did)
       (string-prefix? "did:ma:" did)))

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
       (or (string-prefix? "#" actor)
           (string-prefix? (string-append (runtime) "#") actor))))

(define (did-url? actor)
  (and (string? actor)
       (string-prefix? "did:ma:" actor)
       (string-contains? "#" actor)))

(define (entity-live? actor)
  (and actor (ma-entity-exists? (canonical-actor actor))))

(define (dead-local-actor? actor)
  (and (local-actor-ref? actor) (not (entity-live? actor))))

(define (avatar-fragment did)
  (blake3 (string-append "lambda-ma avatar v1\n" (runtime) "\n" did) 8))

(define (avatar-for-user did)
  (string-append (runtime) "#" (avatar-fragment did)))

(define (did-avatar? did actor)
  (and (user-did? did)
       (string? actor)
       (equal? (canonical-actor actor) (avatar-for-user did))))

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

(define (reply-ok msg text)
  (ma-reply! msg (list :ok text)))

(define (reply-error msg text)
  (ma-reply! msg (list :error text)))

(define (on-signal term)
  #f)

(define *methods* '())
(define *default-method* #f)

(define (set-method! verb fn)
  (set! *methods* (cons (cons verb fn) *methods*)))

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
        (fn args msg)
      (if *default-method*
        (*default-method* verb args msg)
        (ma-reply! msg (list :error "unknown verb"))))))
