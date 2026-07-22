; Duck agent.
; This behaviour extends /ma/scheme/agent/0.0.1.

(define (duck-defaults!)
  (begin
    (if (get-prop "name") #f (set-prop! "name" "Rubber Duckie™"))
    (if (get-prop "nick") #f (set-prop! "nick" "Duckie"))
    (if (get-prop "description")
        #f
        (set-prop! "description" "A curious rubber duck that waddles around and quacks because one is never alone with a rubber duck"))
    (ma-save-state!)))

(define (runtime-started-at)
  (let ((value (ma-get-config-key "started_at")))
    (if value value "")))

(define (scheduled-this-runtime? key)
  (equal? (get-prop key) (runtime-started-at)))

(define (mark-scheduled! key)
  (begin
    (set-prop! key (runtime-started-at))
    (ma-save-state!)))

(define (duck-schedule-quack!)
  (let ((key "schedule:quack:started-at"))
    (if (scheduled-this-runtime? key)
        #f
        (begin
          (mark-scheduled! key)
          (ma-send! (entity-url "scheduler") (list "quack" :random 600 :quack))))))

(define (duck-say msg text)
  (let ((p (parent)))
    (if (equal? p "")
        (reply-error msg "duck is nowhere")
        (begin
          (ma-send! p (list :say text))
          (reply-ok msg text)))))

(set-method! :help
  (lambda (args msg)
    (reply-ok msg
      (string-append
        (name) " help\n"
        "  :about      show duck identity and location\n"
        "  :where      show duck current parent\n"
        "  :owner      show current owner\n"
        "  :duck       say a duck line in the current room\n"
        "  :quack      say quack in the current room"))))

(set-method! :duck
  (lambda (args msg)
    (duck-say msg "A duck waddles through the room. It looks busy.")))

(set-method! :quack
  (lambda (args msg)
    (duck-say msg "quack")))

(duck-defaults!)
