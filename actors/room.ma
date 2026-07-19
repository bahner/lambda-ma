; Locked room actor.
; Rooms own exits and local room policy. Avatars act through their current room.

(define ROOM_KIND "/ma/room/0.0.1")
(define EXIT_KIND "/ma/exit/0.0.1")

(define (self) (ma-get-config-key "self"))
(define (runtime) (ma-get-config-key "runtime"))
(define (root) (ma-get-config-key "root"))
(define (entity-url fragment) (string-append (runtime) "#" fragment))

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(define (member? x xs)
  (cond ((null? xs) #f)
        ((equal? x (car xs)) #t)
        (else (member? x (cdr xs)))))

(define (occupants)
  (let ((xs (get-prop "occupants")))
    (if xs xs '())))

(define (add-occupant! avatar)
  (if (member? avatar (occupants))
      #f
      (set-prop! "occupants" (cons avatar (occupants)))))

(define (remove-one x xs)
  (cond ((null? xs) '())
        ((equal? x (car xs)) (remove-one x (cdr xs)))
        (else (cons (car xs) (remove-one x (cdr xs))))))

(define (remove-occupant! avatar)
  (set-prop! "occupants" (remove-one avatar (occupants))))

(define (label-key actor) (string-append "label:" actor))

(define (avatar-ref entry)
  (if (pair? entry) (car entry) entry))

(define (avatar-label entry)
  (if (and (pair? entry) (pair? (cdr entry))) (car (cdr entry)) #f))

(define (avatar-refs entries)
  (if (null? entries)
      '()
      (cons (avatar-ref (car entries)) (avatar-refs (cdr entries)))))

(define (store-labels! entries)
  (if (null? entries)
      #f
      (let ((avatar (avatar-ref (car entries)))
            (label (avatar-label (car entries))))
        (if label (set-prop! (label-key avatar) label) #f)
        (store-labels! (cdr entries)))))

(define (speaker-name actor)
  (let ((label (get-prop (label-key actor))))
    (if label label actor)))

(define (room-name)
  (let ((name (get-prop "name")))
    (if name name "A Room")))

(define (room-description)
  (let ((description (get-prop "description")))
    (if description description "You are in a room.")))

(define (room-text)
  (string-append (room-name) "\n" (room-description)))

(define (names-of actors)
  (cond ((null? actors) "")
        ((null? (cdr actors)) (speaker-name (car actors)))
        (else (string-append (speaker-name (car actors)) ", " (names-of (cdr actors))))))

(define (from-root? msg)
  (equal? (msg-from msg) (root)))

(define (on-event event args msg)
  (cond ((equal? event :join-avatar)
         (let ((avatar (car args)))
           (add-occupant! avatar)
           (ma-save-state!)
           (broadcast (string-append (speaker-name avatar) " arrives."))))
        ((equal? event :leave-avatar)
         (let ((avatar (car args)))
           (remove-occupant! avatar)
           (ma-save-state!)
           (broadcast (string-append (speaker-name avatar) " leaves."))))
        (else #f)))

(define (broadcast text)
  (let loop ((xs (occupants)))
    (if (null? xs)
        #f
        (begin
          (ma-send! (car xs) (list :print text))
          (loop (cdr xs))))))

(define (exit-key direction) (string-append "exit:" direction))

(define (room-init) #f)

(define (exit-init direction target-room)
  (string-append
    "(set-prop! \"direction\" \"" direction "\")\n"
    "(set-prop! \"target-room\" \"" target-room "\")"))

(set-method! :join-avatar
  (lambda (args msg)
    (if (from-root? msg)
        (on-event :join-avatar args msg)
        #f)))

(set-method! :leave-avatar
  (lambda (args msg)
    (if (from-root? msg)
        (on-event :leave-avatar args msg)
        #f)))

(set-method! :ctx
  (lambda (args msg)
    (if (from-root? msg)
        (let ((kind (car args))
              (payload (car (cdr args))))
          (if (equal? kind :avatars)
              (begin
                (set-prop! "occupants" (avatar-refs payload))
                (store-labels! payload)
                (ma-save-state!))
              #f))
        #f)))

(set-method! :look
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (ma-send! avatar (list :print (room-text))))))

(set-method! :exits
  (lambda (args msg)
    (let ((avatar (msg-from msg)))
      (ma-send! avatar (list :print "Exits are whatever has been dug from here.")))))

(set-method! :say
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args)))
      (broadcast (string-append (speaker-name speaker) " says: " text)))))

(set-method! :emote
  (lambda (args msg)
    (let ((speaker (msg-from msg))
          (text (join-words args)))
      (broadcast (string-append (speaker-name speaker) " " text)))))

(set-method! :dig
  (lambda (args msg)
    (let ((avatar (msg-from msg))
          (direction (if (null? args) "out" (car args))))
      (add-occupant! avatar)
      (let* ((room-fragment (ma-create-actor ROOM_KIND #f (room-init)))
             (target-room (entity-url room-fragment))
             (exit-fragment (ma-create-actor EXIT_KIND #f (exit-init direction target-room)))
             (exit (entity-url exit-fragment)))
        (set-prop! (exit-key direction) exit)
        (ma-save-state!)
        (broadcast (string-append avatar " digs " direction "."))
        (ma-send! avatar (list :print (string-append "You dig " direction " and open a new exit.")))))))

(set-method! :go
  (lambda (args msg)
    (let ((avatar (msg-from msg))
          (direction (if (null? args) "out" (car args))))
      (let ((exit (get-prop (exit-key direction))))
        (if exit
            (ma-send! exit (list :traverse avatar (self)))
            (ma-send! avatar (list :print (string-append "No exit " direction "."))))))))

(set-method! :enter-avatar
  (lambda (args msg)
    (let ((avatar (car args)))
      (ma-send! (root) (list :arrived avatar (self))))))
