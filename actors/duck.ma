; Duck agent.
; This behaviour extends /ma/scheme/agent/0.0.1.

; Defaults fill in only missing inherited agent state.
(define (duck-defaults!)
  (begin
    (if (get-prop "name") #f (set-init-prop! "name" "Rubber Duckie™"))
    (if (get-prop "nick") #f (set-init-prop! "nick" "Duckie"))
    (if (get-prop "description")
        #f
        (set-init-prop! "description" "A curious rubber duck that waddles around and quacks because one is never alone with a rubber duck"))
    (ma-save-state!)))

(define (duck-schedule-quack!)
  (let ((key "schedule:quack:started-at"))
    (if (scheduled-this-runtime? key)
        #f
        (begin
          (mark-scheduled! key)
          (ma-send! (entity-url "scheduler") (list "quack" :random 600 :quack))))))

; Duck-specific room speech and action.
(define (duck-say msg text)
  (let ((p (node-parent)))
    (if (equal? p "")
        (reply-error msg "duck is nowhere")
        (begin
          (ma-send! p (list :say text))
          (reply-ok msg)))))

(define (duck-emote msg text)
  (let ((p (node-parent)))
    (if (equal? p "")
        (reply-error msg "duck is nowhere")
        (begin
          (ma-send! p (list :emote text))
          (reply-ok msg)))))

; Public methods added on top of the generic agent behaviour.
(set-rpc-method! :help
  (lambda (args msg)
    (reply-ok-with msg
      (string-append
        (name) " help\n"
        "  :about      show duck identity and location\n"
        "  :where?     show duck current parent\n"
        "  :owner      show current owner\n"
        "  :duck       say a duck line in the current room\n"
        "  :quack      say quack in the current room"))))

(set-cmd-method! :duck
  (lambda (args msg)
    (duck-emote msg "waddles through the room. It looks busy.")))

(set-cmd-method! :quack
  (lambda (args msg)
    (duck-say msg "quack")))

(duck-defaults!)
