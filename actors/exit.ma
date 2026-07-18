; Locked exit actor.
; Exits are traversal entities owned by rooms or by root for world entry.

(define (target-room) (get-prop "target-room"))
(define (direction) (get-prop "direction"))

(set-method! :traverse
  (lambda (args msg)
    (let ((avatar (car args))
          (target (target-room)))
      (if target
          (begin
            (ma-send! avatar (list :print (string-append "You go " (direction) ".")))
            (ma-send! target (list :enter-avatar avatar (ma-get-config-key "self"))))
          (ma-send! avatar (list :print "This exit leads nowhere."))))))
