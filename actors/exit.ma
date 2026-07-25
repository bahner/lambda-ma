; Locked exit actor.
; Exits are traversal entities owned by rooms or by root for world entry.

(define (target-room) (get-prop "target-room"))
(define (source-room) (get-prop "source-room"))
(define (direction) (get-prop "direction"))
(define (runtime) (ma-get-config-key "runtime"))
(define (canonical-actor actor)
  (if (and actor (string-prefix? "#" actor)) (string-append (runtime) actor) actor))
(define (same-actor? a b)
  (equal? (canonical-actor a) (canonical-actor b)))

(set-method! :fill
  (lambda (args msg)
    (if (same-actor? (msg-from msg) (source-room))
        (ma-end)
        #f)))

(set-method! :traverse
  (lambda (args msg)
    (let ((avatar (car args))
          (source-room (if (or (null? (cdr args))) #f (car (cdr args))))
          (user (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args)))))
          (nick (if (or (null? (cdr args)) (null? (cdr (cdr args))) (null? (cdr (cdr (cdr args))))) #f (car (cdr (cdr (cdr args))))))
          (target (target-room)))
      (if target
          (begin
          (ma-send! (canonical-actor avatar) (list :print (string-append "You go " (direction) ".")))
            (if user
            (ma-send! (canonical-actor target) (list :enter user (canonical-actor avatar) (canonical-actor source-room) nick))
            (ma-send! (canonical-actor target) (list :enter (canonical-actor avatar) (canonical-actor source-room)))))
          (ma-send! (canonical-actor avatar) (list :print "This exit leads nowhere."))))))

(set-method! :traverse-agent
  (lambda (args msg)
    (let ((agent (car args))
          (source-room (if (or (null? (cdr args))) #f (car (cdr args))))
          (target (target-room)))
      (if target
          (ma-send! (canonical-actor agent) (list :enter-room (canonical-actor target) (canonical-actor source-room)))
          #f))))
