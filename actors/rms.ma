; RMS fortune agent.
; This behaviour extends /ma/scheme/agent/0.0.1.

(define FORTUNES
  (list
    "Technology at present is covert philosophy; the point is to make it openly philosophical."
    "Your primary goal is not to solve their problem. Your primary goal is to help them become one notch more capable of solving their problem on their own."
    "Sharing is good, and with digital technology, sharing is easy."
    "Facebook is not your friend, it is a surveillance engine."
    "The idea of copyright did not exist in ancient times, when authors frequently copied other authors at length in works of non-fiction. This practice was useful, and is the only way many authors' works have survived even in part."
    "The reason that a good citizen does not use such destructive means to become wealthier is that, if everyone did so, we would all become poorer from the mutual destructiveness."
    "I could have made money this way, and perhaps amused myself writing code. But I knew that at the end of my career, I would look back on years of building walls to divide people, and feel I had spent my life making the world a worse place."
    "People sometimes ask me if it is a sin in the Church of Emacs to use vi. Using a free version of vi is not a sin; it is a penance. So happy hacking."
    "In the free/libre software movement, we develop software that respects users' freedom, so we and you can escape from software that doesn't."
    "Proprietary software tends to have malicious features. The point is with a proprietary program, when the users don't have the source code, we can never tell. So you must consider every proprietary program as potential malware."
    "Fighting patents one by one will never eliminate the danger of software patents, any more than swatting mosquitoes will eliminate malaria."
    "Anything that prevents you from being friendly, a good neighbour, is a terror tactic."
    "Control over the use of one's ideas really constitutes control over other people's lives; and it is usually used to make their lives more difficult."
    "I suppose many people will continue moving towards careless computing, because there's a sucker born every minute."
    "If you want to accomplish something in the world, idealism is not enough - you need to choose a method that works to achieve the goal."
    "Proprietary software is an injustice."
    "Free software is software that respects your freedom and the social solidarity of your community. So it's free as in freedom."
    "If programmers deserve to be rewarded for creating innovative programs, by the same token they deserve to be punished if they restrict the use of these programs."
    "If you use a proprietary program or somebody else's web server, you're defenceless. You're putty in the hands of whoever developed that software."
    "The desire to be rewarded for one's creativity does not justify depriving the world in general of all or part of that creativity."
    "Proprietary software keeps users divided and helpless. Divided because each user is forbidden to redistribute it to others, and helpless because the users can't change it since they don't have the source code. They can't study what it really does. So the proprietary program is a system of unjust power."
    "When I launched the development of the GNU system, I explicitly said the purpose of developing this system is so we can use our computers and have freedom, thus if you use some other free system instead but you have freedom, then it's a success. It's not popularity for our code but it's success for our goal."
    "A smartphone is a computer. So everything we say about computers, that the software you run should be free and you should insist on that, applies to smart phones the same way."
    "One reason you should not use web applications to do your computing is that you lose control. It's just as bad as using a proprietary program."
    "All governments should be pressured to correct their abuses of human rights."
    "The computer industry is the only industry that is more fashion-driven than women's fashion."
    "Whether gods exist or not, there is no way to get absolute certainty about ethics. Without absolute certainty, what do we do? We do the best we can."
    "Value your freedom or you will lose it, teaches history. Don't bother us with politics, respond those who don't want to learn."
    "The interesting thing about cloud computing is that we've redefined cloud computing to include everything that we already do."
    "If there is a Like button in a page, Facebook knows who visited that page."
    "Officially, MPAA stands for Motion Picture Association of America, but I suggest that MPAA stands for Malicious Power Attacking All."
    "Software patents are dangerous to software developers because they impose monopolies on software ideas."
    "The idea of free software is that users of computing deserve freedom. They deserve in particular to have control over their computing."
    "I founded the free software movement, a movement for freedom to cooperate. Open source was a reaction against our idealism. We are still here."
    "Free software is a matter of liberty, not price. Think of free speech, not free beer."
    "GNU, which stands for Gnu's Not Unix, is the name for the complete Unix-compatible software system which I am writing so that I can give it away free to everyone who can use it."
    "If the users don't control the program, the program controls the users. A nonfree program is a yoke, an instrument of unjust power."
    "You can use any editor you want, but remember that vi vi vi is the text editor of the beast."
    "The most powerful programming language is Lisp. If you don't know Lisp (or Scheme), you don't appreciate what a powerful language is."
    "No person, no idea, and no religion deserves to be illegal to insult, not even the Church of Emacs."
    "Copying all or parts of a program is as natural to a programmer as breathing, and as productive. It ought to be as free."
    "I consider that the golden rule requires that if I like a program I must share it with other people who like it."))

(define (rms-defaults!)
  (begin
    (if (get-prop "name") #f (set-prop! "name" "Richard Stallman"))
    (if (get-prop "nick") #f (set-prop! "nick" "rms"))
    (if (get-prop "description")
        #f
        (set-prop! "description" "A roaming free software sage dispensing random fortunes."))
    (if (number? (get-prop "fortune-seed")) #f (set-prop! "fortune-seed" 7))
    (ma-save-state!)))

(define (rms-schedule-fortune!)
  (ma-send! (entity-url "scheduler") (list "fortune" :random 60 :fortune)))

(define (list-length xs)
  (if (null? xs) 0 (+ 1 (list-length (cdr xs)))))

(define (list-ref-at xs idx)
  (cond ((null? xs) #f)
        ((= idx 0) (car xs))
        (else (list-ref-at (cdr xs) (- idx 1)))))

(define (wrap-index n count)
  (cond ((< n 0) (wrap-index (+ n count) count))
        ((< n count) n)
        (else (wrap-index (- n count) count))))

(define (next-fortune-index count)
  (let* ((seed (get-prop "fortune-seed"))
         (base (if (number? seed) seed 7))
         (next (wrap-index (+ (* base 17) 31) count)))
    (set-prop! "fortune-seed" next)
    (ma-save-state!)
    next))

(define (next-fortune)
  (let ((count (list-length FORTUNES)))
    (if (= count 0)
        "Freedom requires sharing."
        (let ((fortune (list-ref-at FORTUNES (next-fortune-index count))))
          (if fortune fortune "Freedom requires sharing.")))))

(set-method! :help
  (lambda (args msg)
    (reply-ok msg
      (string-append
        (name) " help\n"
        "  :about      show rms identity and location\n"
        "  :where      show rms current parent\n"
        "  :owner      show current owner\n"
        "  :fortune    say a random fortune in the current room\n"
        "rms registers schedule fortune with #scheduler as random up to 60 seconds."))))

(set-method! :fortune
  (lambda (args msg)
    (let ((p (parent))
          (fortune (next-fortune)))
      (if (equal? p "")
          (reply-error msg "rms is nowhere")
          (begin
            (ma-send! p (list :say fortune))
            (reply-ok msg fortune))))))
