; ma-scheme stdlib.
;
; Generic Scheme helpers shared by higher lambda-ma libraries. This layer must
; not depend on actor host functions, runtime configuration, or persisted state.

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(define (list-append left right)
  (if (null? left)
      right
      (cons (car left) (list-append (cdr left) right))))

(define (list-length xs)
  (if (null? xs) 0 (+ 1 (list-length (cdr xs)))))

(define (list-ref-at xs idx)
  (cond ((null? xs) #f)
        ((= idx 0) (car xs))
        (else (list-ref-at (cdr xs) (- idx 1)))))

(define (arg-at-or-false args index)
  (cond ((null? args) #f)
        ((= index 0) (car args))
        (else (arg-at-or-false (cdr args) (- index 1)))))

(define (non-empty-string? value)
  (and (string? value) (not (equal? value ""))))

(define (empty-string->false value)
  (if (equal? value "") #f value))

(define (default-nick) "avatar")

(define (nick-or-default nick)
  (if (non-empty-string? nick) nick (default-nick)))

(define (ctx-text ctx key)
  (let ((value (map-ref ctx key #f)))
    (if (string? value) value #f)))

(define (ctx-optional-text-valid? ctx key)
  (let ((value (map-ref ctx key #f)))
    (or (not value) (string? value))))

(define (ctx-kind-valid? kind)
  (or (equal? kind "avatar")
      (equal? kind "thing")
      (equal? kind "agent")))

(define (valid-ctx? ctx)
  (and (map? ctx)
       (ctx-kind-valid? (ctx-text ctx "kind"))
       (ctx-optional-text-valid? ctx "actor")
       (ctx-optional-text-valid? ctx "avatar")
       (ctx-optional-text-valid? ctx "did")
       (ctx-optional-text-valid? ctx "root")
      (ctx-optional-text-valid? ctx "parent")
       (ctx-optional-text-valid? ctx "room")
       (ctx-optional-text-valid? ctx "name")
       (ctx-optional-text-valid? ctx "nick")
       (ctx-optional-text-valid? ctx "description")
       (ctx-optional-text-valid? ctx "text")
      (ctx-optional-text-valid? ctx "exit")
      (ctx-optional-text-valid? ctx "direction")))

(define (ctx-kind? ctx kind)
  (and (valid-ctx? ctx)
       (equal? (ctx-text ctx "kind") kind)))

(define (actor-ctx? ctx)
  (and (valid-ctx? ctx)
       (non-empty-string? (ctx-text ctx "name"))
       (non-empty-string? (ctx-text ctx "nick"))
       (non-empty-string? (ctx-text ctx "description"))))

(define (avatar-ctx? ctx)
  (ctx-kind? ctx "avatar"))

(define (agent-ctx? ctx)
  (and (actor-ctx? ctx)
       (equal? (ctx-text ctx "kind") "agent")))

(define (thing-ctx? ctx)
  (and (actor-ctx? ctx)
       (equal? (ctx-text ctx "kind") "thing")))

(define (room-ctx? ctx)
  (and (valid-ctx? ctx)
       (non-empty-string? (ctx-text ctx "room"))))

(define (string-entries xs)
  (let loop ((rest xs) (acc '()))
    (cond ((null? rest) acc)
          ((string? (car rest)) (loop (cdr rest) (cons (car rest) acc)))
          (else (loop (cdr rest) acc)))))

(define (member-string? entry xs)
  (cond ((null? xs) #f)
        ((equal? entry (car xs)) #t)
        (else (member-string? entry (cdr xs)))))

(define (unique-string-entries xs)
  (let loop ((rest (string-entries xs)) (acc '()))
    (cond ((null? rest) acc)
          ((member-string? (car rest) acc) (loop (cdr rest) acc))
          (else (loop (cdr rest) (list-append acc (list (car rest))))))))
