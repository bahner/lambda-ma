; Stationary rubber duck actor.
; It can speak in its room, but it does not move and is not in root's location registry.

(define (room) (get-prop "room"))

(define (join-words words)
  (cond ((null? words) "")
        ((null? (cdr words)) (car words))
        (else (string-append (car words) " " (join-words (cdr words))))))

(set-method! :quack
  (lambda (args msg)
    (ma-send! (room) (list :say "kvakk"))))

(set-method! :say
  (lambda (args msg)
    (ma-send! (room) (list :say (join-words args)))))

(set-method! :emote
  (lambda (args msg)
    (ma-send! (room) (list :emote (join-words args)))))
