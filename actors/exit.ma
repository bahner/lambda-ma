; Locked exit actor.
; Exits are traversal entities owned by rooms or by root for world entry.

(define (target-room) (get-prop "target-room"))
(define (direction) (get-prop "direction"))

(set-method! :traverse
  (lambda (args msg)
    (let ((avatar (car args))
          (user (if (or (null? (cdr args)) (null? (cdr (cdr args)))) #f (car (cdr (cdr args)))))
          (nick (if (or (null? (cdr args)) (null? (cdr (cdr args))) (null? (cdr (cdr (cdr args))))) #f (car (cdr (cdr (cdr args))))))
          (target (target-room)))
      (if target
          (begin
            (ma-send! avatar (list :print (string-append "You go " (direction) ".")))
            (if user
                (ma-send! target (list :enter user avatar (ma-get-config-key "self") nick))
                (ma-send! target (list :enter avatar (ma-get-config-key "self")))))
          (ma-send! avatar (list :print "This exit leads nowhere."))))))
