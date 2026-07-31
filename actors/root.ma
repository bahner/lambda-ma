; Locked world root / avatar factory actor.
; Root is a known factory only; avatars own ctx, nick, and room state.

(define (local-self) (canonical-actor (self)))

; Entry target resolution. Root only chooses a configured start room; it does
; not infer or create fallback rooms.
(define (configured-start-room)
  (let ((configured (ma-get-config-key "start")))
    (if configured configured (get-prop "start"))))

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
  (if (entity-live? requested) (canonical-actor requested) (ensure-start-room)))

(define (requested-nick args)
  (cond ((null? args) #f)
        ((delegated-enter? args)
         (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args)))))
        ((equal? (car args) "") (if (null? (cdr args)) #f (car (cdr args))))
        ((null? (cdr args)) #f)
        (else (car (cdr args)))))

(define (delegated-enter? args)
  (and (not (null? args)) (string-prefix? "did:ma:" (car args))))

(define (entry-did args msg)
  (if (delegated-enter? args) (car args) (msg-from msg)))

; Avatar creation is asynchronous. The init code asks the room to admit the
; avatar; the room later sends committed ctx back to the avatar.
(define (avatar-init did nick room)
  (let ((n (nick-or-default nick))
        (r (local-self))
        (avatar (avatar-for-did did))
        (target-room (canonical-actor room)))
    (string-append
      "(set-prop! \"did\" \"" did "\")\n"
      "(set-prop! \"root\" \"" r "\")\n"
      "(set-prop! \"nick\" \"" n "\")\n"
      "(ma-save-state!)\n"
      "(ma-send! \"" target-room "\" (list :enter \"" avatar "\" #f \"" n "\"))\n")))

(define (ensure-avatar did nick room)
  (let ((avatar (avatar-for-did did)))
    (if (entity-live? avatar)
        (begin
          (ma-send! (canonical-actor avatar) (list :enter-room (canonical-actor room) did (nick-or-default nick)))
          avatar)
        (entity-url (ma-create-actor AVATAR_KIND #f (avatar-init did nick room) (avatar-fragment did))))))

; Public entry methods.
(set-method! :enter
  (lambda (args msg)
    (let* ((did (entry-did args msg))
           (room (entry-room (requested-room args)))
           (nick (nick-or-default (requested-nick args)))
           (avatar (ensure-avatar did nick room)))
      (ma-reply! msg (list :ok avatar)))))

(set-method! :avatar?
  (lambda (args msg)
    (let* ((did (msg-from msg))
           (room (ensure-start-room))
           (avatar (ensure-avatar did #f room)))
      (ma-reply! msg (list :ok avatar)))))
