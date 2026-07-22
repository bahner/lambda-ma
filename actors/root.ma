; Locked world root / avatar factory actor.
; Root is a known factory only; avatars own ctx, nick, and room state.

(define AVATAR_KIND "/ma/avatar/0.0.1")
(define LAMBDA_CTX_PROTOCOL "/ma/lambda/ctx/0.0.1")
(define ENTITY_FRAGMENT_CONTEXT "ma entity-fragment v1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))
(define (default-nick) "avatar")

(define (nick-or-default nick)
  (if nick nick (default-nick)))

(define (configured-start-room)
  (let ((configured (ma-get-config-key "start")))
    (if configured configured (get-prop "start"))))

(define (entity-live? actor)
  (and actor (ma-entity-exists? actor)))

(define (ensure-start-room)
  (let ((start (configured-start-room)))
    (if (entity-live? start)
        (if (equal? (get-prop "start") start)
            start
            (begin
              (set-prop! "start" start)
              (ma-save-state!)
              start))
        (error "entry start room is not configured"))))

(define (requested-room args)
  (cond ((null? args) #f)
        ((delegated-enter? args)
         (if (or (null? (cdr args)) (equal? (car (cdr args)) "")) #f (car (cdr args))))
        ((equal? (car args) "") #f)
        (else (car args))))

(define (entry-room requested)
  (if (entity-live? requested) requested (ensure-start-room)))

(define (requested-nick args)
  (cond ((null? args) #f)
        ((delegated-enter? args)
         (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args)))))
        ((equal? (car args) "") (if (null? (cdr args)) #f (car (cdr args))))
        ((null? (cdr args)) #f)
        (else (car (cdr args)))))

(define (delegated-enter? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (entry-user args msg)
  (if (delegated-enter? args) (car args) (msg-from msg)))

(define (avatar-fragment user)
  (ma-derived-id ENTITY_FRAGMENT_CONTEXT user 8))

(define (avatar-for-user user)
  (entity-url (avatar-fragment user)))

(define (avatar-init user nick room)
  (let ((n (nick-or-default nick))
        (r (self)))
    (string-append
      "(set-prop! \"user\" \"" user "\")\n"
      "(set-prop! \"root\" \"" r "\")\n"
      "(set-prop! \"nick\" \"" n "\")\n"
      "(ma-send! \"" room "\" (list :enter (ma-get-config-key \"self\") #f \"" n "\"))\n")))

(define (ensure-avatar user nick room)
  (let ((avatar (avatar-for-user user)))
    (if (entity-live? avatar)
        (begin
          (ma-send! avatar (list :enter-room room))
          avatar)
        (entity-url (ma-create-actor AVATAR_KIND #f (avatar-init user nick room) user)))))

(set-method! :enter
  (lambda (args msg)
    (let* ((user (entry-user args msg))
           (room (entry-room (requested-room args)))
           (nick (nick-or-default (requested-nick args)))
           (avatar (ensure-avatar user nick room)))
      (ma-reply! msg (list :ok avatar)))))

(set-method! :avatar?
  (lambda (args msg)
    (let* ((user (msg-from msg))
           (room (ensure-start-room))
           (avatar (ensure-avatar user #f room)))
      (ma-reply! msg (list :ok avatar)))))
