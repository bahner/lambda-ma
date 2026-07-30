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

(define (string-entries xs)
  (let loop ((rest xs) (acc '()))
    (cond ((null? rest) acc)
          ((string? (car rest)) (loop (cdr rest) (cons (car rest) acc)))
          (else (loop (cdr rest) acc)))))
