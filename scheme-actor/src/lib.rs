//! ma-actor — generic ma-scheme host, `/ma/scheme/actor/0.0.1` (ma-scheme-v1.md).
//!
//! Two Wasm exports, always: `on_message` for incoming messages,
//! `on_signal` for every runtime-originated lifecycle event
//! (`:set-state`/`:set-behaviour`/`:init`/`:start`/`:shutdown`, §3). This
//! collapses what was previously five separately-named lifecycle exports
//! into one — see `lifecycle::on_signal` for the dispatch logic.

pub mod actor;
pub mod builtins;
pub mod cbor;
pub mod env;
pub mod eval;
pub mod include;
pub mod lifecycle;
pub mod msg;
pub mod parser;
pub mod runtime;
pub mod state;
pub mod value;

use std::rc::Rc;

use env::Env;
use eval::eval;
use parser::Parser;
use value::{EvalResult, Value};

/// Build a fresh environment with every core builtin (§8) installed only
/// — no props/msg/config. Used directly by tests exercising just the
/// parser/evaluator; the lifecycle's own `new_full_env` (in
/// `lifecycle.rs`) additionally installs props/config/msg builtins.
pub fn new_root_env() -> Rc<Env> {
    let env = Env::new_root();
    builtins::install(&env);
    env
}

/// Parse and evaluate every top-level form in `src` against `env`, in
/// order, returning the value of the last one (or `Nil` if `src` is
/// empty). Used by tests; production `:set-behaviour`/`:init` handling
/// goes through `lifecycle`'s own `eval_with_includes` instead (which
/// additionally expands top-level `ma-include-ipfs` forms, §11.1).
pub fn eval_all(src: &str, env: &Rc<Env>) -> EvalResult<Value> {
    let forms = Parser::parse_all(src)?;
    let mut result = Value::Nil;
    for form in &forms {
        result = eval(form, env)?;
    }
    Ok(result)
}

use extism_pdk::*;

#[plugin_fn]
pub fn on_signal(input: Vec<u8>) -> FnResult<()> {
    lifecycle::on_signal(&input)?;
    Ok(())
}

#[plugin_fn]
pub fn on_message(input: Vec<u8>) -> FnResult<Vec<u8>> {
    let msg = lifecycle::decode_cast_input(&input)?;
    // Return value is ignored by the runtime (§3) regardless of whether
    // on-message is defined; outbound communication happens via
    // ma-send!/ma-reply!, not via this return value.
    lifecycle::on_message(msg)?;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::Value as Cbor;
    use std::rc::Rc;

    fn run(src: &str) -> Value {
        let env = new_root_env();
        eval_all(src, &env).unwrap()
    }

    fn empty_state_cbor() -> Vec<u8> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&Cbor::Map(Vec::new()), &mut bytes).unwrap();
        bytes
    }

    fn room_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/room.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f)",
            &env,
        )
        .unwrap();
        env
    }

    fn agent_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/agent.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f)",
            &env,
        )
        .unwrap();
        env
    }

    fn duck_env() -> Rc<Env> {
        let env = agent_env();
        eval_all(include_str!("../../actors/duck.ma"), &env).unwrap();
        env
    }

    fn rms_env() -> Rc<Env> {
        let env = agent_env();
        eval_all(include_str!("../../actors/rms.ma"), &env).unwrap();
        env
    }

    fn install_send_reply_recorders(env: &Rc<Env>) {
        eval_all(
            r#"
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            (define (ma-reply! msg term)
              (inc-prop! "reply-count" 1)
              (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
            "#,
            env,
        )
        .unwrap();
    }

    fn avatar_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/avatar.ma"), &env).unwrap();
        eval_all(
                        r#"
                        (define (ma-send! target term)
                            (inc-prop! "sent-count" 1)
                            (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
                            (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
                        (define (ma-reply! msg term)
                            (inc-prop! "reply-count" 1)
                            (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
                        (define (ma-save-state!) #f)
                        "#,
                        &env,
                )
                .unwrap();
        env
    }

    fn exit_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/exit.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-end) (set-prop! \"ended\" \"yes\"))",
            &env,
        )
        .unwrap();
        env
    }

    fn sample_term_msg(from: &str, to: &str, content: Value) -> Rc<crate::msg::MsgRecord> {
        Rc::new(crate::msg::MsgRecord {
            id: "msg-1".to_string(),
            from: from.to_string(),
            to: to.to_string(),
            created_at: 0,
            exp: 0,
            reply_to: None,
            msg_type: "application/vnd.ma.rpc.request".to_string(),
            content_type: "application/vnd.ma.term".to_string(),
            content,
        })
    }

    fn sample_msg(from: &str, to: &str) -> Rc<crate::msg::MsgRecord> {
        sample_term_msg(from, to, Value::symbol(":test"))
    }

    fn eval_str(src: &str, env: &Rc<Env>) -> String {
        match eval_all(src, env).unwrap() {
            Value::Str(s) => s.to_string(),
            other => panic!("expected string, got {other}"),
        }
    }

    fn eval_bool(src: &str, env: &Rc<Env>) -> bool {
        match eval_all(src, env).unwrap() {
            Value::Bool(value) => value,
            other => panic!("expected bool, got {other}"),
        }
    }

    fn eval_int(src: &str, env: &Rc<Env>) -> i64 {
        match eval_all(src, env).unwrap() {
            Value::Int(value) => value,
            other => panic!("expected int, got {other}"),
        }
    }

    #[test]
    fn lambda_ma_actor_files_parse() {
        for (name, source) in [
            ("avatar.ma", include_str!("../../actors/avatar.ma")),
            ("room.ma", include_str!("../../actors/room.ma")),
            ("root.ma", include_str!("../../actors/root.ma")),
            ("exit.ma", include_str!("../../actors/exit.ma")),
            ("agent.ma", include_str!("../../actors/agent.ma")),
            ("rms.ma", include_str!("../../actors/rms.ma")),
            ("duck.ma", include_str!("../../actors/duck.ma")),
            ("thing.ma", include_str!("../../actors/thing.ma")),
        ] {
            Parser::parse_all(source).unwrap_or_else(|err| panic!("{name}: {err}"));
        }
    }

    #[test]
    fn actor_behaviour_method_is_generic() {
        let env = agent_env();
        assert!(eval_bool("(procedure? (find-method :behaviour))", &env));

        let source = include_str!("../actor.ma");
        assert!(source.contains("(get-prop \"owner\")"));
        assert!(source.contains("(msg-from-owner? actor-owner msg)"));
    }

    #[test]
    fn actor_on_message_does_not_tail_trampoline_msg_record() {
        let env = agent_env();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#construct",
                "did:ma:runtime#duckie",
                Value::list(vec![
                    Value::symbol(":parent-report"),
                    Value::str("did:ma:runtime#duckie"),
                    Value::str("did:ma:runtime#construct"),
                    Value::Int(1),
                    Value::str("n1"),
                ]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();
    }

    #[test]
    fn duck_on_message_parent_report_uses_current_actor_dispatch() {
        let env = duck_env();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#construct",
                "did:ma:runtime#duckie",
                Value::list(vec![
                    Value::symbol(":parent-report"),
                    Value::str("did:ma:runtime#duckie"),
                    Value::str("did:ma:runtime#construct"),
                    Value::Int(1),
                    Value::str("n1"),
                ]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();
    }

    #[test]
    fn duck_on_message_parent_report_atom_uses_current_actor_dispatch() {
        let env = duck_env();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#construct",
                "did:ma:runtime#duckie",
                Value::symbol(":parent-report"),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();
    }

    #[test]
    fn duck_quack_speaks_in_room_with_silent_ack() {
        let env = duck_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#duckie")),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#construct")
            ((find-method :quack) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#construct"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":say"), Value::str("quack")])
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("")])
        );
    }

    #[test]
    fn rms_fortune_speaks_in_room_with_silent_ack() {
        let env = rms_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#rms")),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#construct")
            ((find-method :fortune) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#construct"
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("")])
        );
    }

    #[test]
    fn room_presence_uses_labels_and_keeps_who_avatar_only() {
        let env = room_env();
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
        assert_eq!(eval_str("(who-text)", &env), "Who: none.");

        eval_all(
            r#"
                        (define avatar "did:ma:runtime#avatar1")
                        (set-label! avatar "Alice")
                        (add-occupant! avatar)
                        (add-avatar-occupant! avatar)
                        (define rms "did:ma:runtime#rms")
                        (set-label! rms "rms")
                        (set-prop! (claim-key rms)
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set (make-map) "kind" "agent")
                                        "name" "Richard Stallman")
                                    "nick" "rms")
                                "description" "A roaming free software sage."))
                        (add-occupant! rms)
                        "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(who-text)", &env), "Who: Alice");
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: rms, Alice");
        assert!(eval_bool("(movable-occupant? rms)", &env));
        assert!(!eval_bool("(movable-occupant? avatar)", &env));
        assert_eq!(
            eval_str("(movable-ref \"rms\")", &env),
            "did:ma:runtime#rms"
        );
    }

    #[test]
    fn room_nick_broadcast_preserves_old_nick() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert(
            "self".to_string(),
            "did:ma:runtime#construct".to_string(),
        );
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(
                "did:ma:other#avatar1",
                "did:ma:runtime#construct",
            )),
        );

        eval_all(
            r#"
            (define avatar "did:ma:other#avatar1")
            (set-label! avatar "Atlas")
            (add-avatar-presence! avatar)
            ((find-method :nick) (list "Aletheia") msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Atlas is now known as Aletheia."),
            ])
        );
        assert_eq!(eval_str("(speaker-name avatar)", &env), "Aletheia");
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
    }

    #[test]
    fn room_arrival_broadcast_skips_arriving_avatar() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert(
            "self".to_string(),
            "did:ma:runtime#construct".to_string(),
        );
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(
                "did:ma:runtime#alice",
                "did:ma:runtime#construct",
            )),
        );

        eval_all(
            r#"
                        (define (ctx-term-value term key)
                            (let loop ((pairs (car (cdr term))))
                                (cond ((null? pairs) #f)
                                            ((equal? (car (car pairs)) key) (car (cdr (car pairs))))
                                            (else (loop (cdr pairs))))))
            (define (ma-entity-exists? actor) #t)
            (set-prop! "name" "The Garden")
            (define bob "did:ma:runtime#bob")
            (set-label! bob "Bob")
            (add-avatar-presence! bob)
            ((find-method :enter) (list "did:ma:runtime#alice" #f "Alice") msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:runtime#bob");
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":print"), Value::str("Alice arrives.")])
        );
        assert_eq!(eval_str("(get-prop \"sent-target:2\")", &env), "did:ma:runtime#alice");
        assert_eq!(
            eval_str("(ctx-term-value (get-prop \"sent-term:2\") :text)", &env),
            "The Garden"
        );
    }

    #[test]
    fn room_ignores_foreign_legacy_avatar_entry() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:target".to_string());
        config.insert("self".to_string(), "did:ma:target#construct".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(
                "did:ma:source#avatar",
                "did:ma:target#construct",
            )),
        );

        eval_all(
            "((find-method :enter) (list \"did:ma:source#avatar\" #f \"Lars\") msg)",
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
        assert_eq!(eval_str("(who-text)", &env), "Who: none.");
    }

    #[test]
    fn room_cross_runtime_avatar_ctx_requests_local_avatar() {
        let env = room_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:ma".to_string());
        config.insert("self".to_string(), "did:ma:ma#cloud".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
                        (define (ma-entity-exists? actor) #f)
                        (define (ma-create-actor kind init behaviour fragment)
                            (set-prop! "created-kind" kind)
                            (set-prop! "created-init" init)
                            (set-prop! "created-fragment" fragment)
                            (entity-url fragment))
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:sky#avatar", "did:ma:ma#cloud")),
        );
        eval_all(
            r#"
                        ((find-method :enter)
                            (list (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set (make-map) "actor" "did:ma:user")
                                                        "avatar" "did:ma:sky#avatar")
                                                    "kind" "avatar")
                                                "room" "did:ma:ma#cloud")
                                            "nick" "Lars"))
                            msg)
                        "#,
            &env,
        )
        .unwrap();

        let expected_fragment = eval_str(&format!(r#"(avatar-fragment "{user}")"#), &env);
        assert_eq!(
            eval_str("(get-prop \"created-kind\")", &env),
            "/ma/avatar/0.0.1"
        );
        assert_eq!(
            eval_str("(get-prop \"created-fragment\")", &env),
            expected_fragment
        );
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
    }

    #[test]
    fn room_look_includes_exits() {
        let env = room_env();

        assert!(eval_str("(room-text)", &env).starts_with("“This is the Construct."));
        assert!(!eval_str("(room-text)", &env).starts_with("Construct\n"));
        assert!(eval_str("(room-text)", &env).ends_with("\nExits: none."));

        eval_all("(put-exit! \"north\" \"did:ma:runtime#north-exit\")", &env).unwrap();

        assert!(eval_str("(room-text)", &env).ends_with("\nExits: north"));
    }

    #[test]
    fn stdlib_recognises_direct_and_avatar_owner_messages() {
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();

        let owner = "did:ma:owner";
        let runtime = "did:ma:runtime";
        eval_all(&format!(r#"(define (runtime) "{runtime}")"#), &env).unwrap();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), runtime.to_string());
        crate::state::set_config(config);

        let avatar = eval_str(&format!(r#"(avatar-for-user "{owner}")"#), &env);

        env.define(
            Rc::from("direct_msg"),
            Value::Msg(sample_msg(owner, "did:ma:runtime#room")),
        );
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_msg(&avatar, "did:ma:runtime#room")),
        );
        env.define(
            Rc::from("other_msg"),
            Value::Msg(sample_msg("did:ma:other", "did:ma:runtime#room")),
        );

        assert!(eval_bool(
            r#"(msg-from-owner? "did:ma:owner" direct_msg)"#,
            &env
        ));
        assert!(eval_bool(
            r#"(msg-from-owner? "did:ma:owner" avatar_msg)"#,
            &env
        ));
        assert!(!eval_bool(
            r#"(msg-from-owner? "did:ma:owner" other_msg)"#,
            &env
        ));
    }

    #[test]
    fn stdlib_canonicalises_actor_refs() {
        let env = new_root_env();
        crate::state::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(r#"(define (runtime) "did:ma:runtime")"#, &env).unwrap();

        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config);

        assert_eq!(
            eval_str(r#"(entity-url "room")"#, &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str(r##"(canonical-actor "#room")"##, &env),
            "did:ma:runtime#room"
        );
        assert!(eval_bool(
            r##"(same-actor? "#room" "did:ma:runtime#room")"##,
            &env
        ));
        assert!(!eval_bool(r##"(local-actor-ref? "#room")"##, &env));
        assert!(eval_bool(
            r##"(local-actor-ref? "did:ma:runtime#room")"##,
            &env
        ));
        assert!(!eval_bool(
            r##"(local-actor-ref? "did:ma:other#room")"##,
            &env
        ));
        assert!(eval_bool(r##"(did-url? "did:ma:runtime#room")"##, &env));
        assert!(!eval_bool(r##"(did-url? "did:ma:runtime")"##, &env));
    }

    #[test]
    fn stdlib_validates_ctx_maps() {
        let env = new_root_env();
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();

        assert!(eval_bool(
            r#"(valid-ctx? (map-set (map-set (make-map) "kind" "avatar") "room" "did:ma:runtime#room"))"#,
            &env
        ));
        assert!(eval_bool(
            r#"(valid-ctx? (map-set (map-set (make-map) "kind" "agent") "text" "You arrive."))"#,
            &env
        ));
        assert!(!eval_bool(
            r#"(valid-ctx? (map-set (make-map) "kind" "dragon"))"#,
            &env
        ));
        assert!(!eval_bool(
            r#"(actor-ctx? (map-set (map-set (map-set (make-map) "kind" "thing") "name" "stone") "nick" "stone"))"#,
            &env
        ));
        assert!(eval_bool(
            r#"(thing-ctx? (map-set (map-set (map-set (map-set (make-map) "kind" "thing") "name" "stone") "nick" "stone") "description" "A small stone."))"#,
            &env
        ));
        assert!(eval_bool(
            r#"(room-ctx? (map-set (map-set (make-map) "kind" "avatar") "room" "did:ma:runtime#room"))"#,
            &env
        ));
        assert!(!eval_bool(
            r#"(agent-ctx? (map-set (map-set (map-set (map-set (make-map) "kind" "thing") "name" "stone") "nick" "stone") "description" "A small stone."))"#,
            &env
        ));
    }

    #[test]
    fn room_owner_transfer_uses_ownership_errors() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:other",
                "did:ma:runtime#room",
                Value::symbol(":owner"),
            )),
        );

        eval_all("((find-method :owner) (list \"did:ma:new\") msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:other"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Only this room's owner can transfer ownership."),
            ])
        );
    }

    #[test]
    fn room_leave_occupant_canonicalises_sender() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#old-room".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#rms", "did:ma:runtime#old-room")),
        );

        eval_all(
            r#"
            (set-label! "did:ma:runtime#rms" "rms")
            (add-occupant! "did:ma:runtime#rms")
            (on-event :leave-occupant '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
    }

    #[test]
    fn room_owner_can_remove_avatar_presence_by_label() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (define avatar "did:ma:runtime#avatar")
            (set-label! avatar "Alice")
            (add-avatar-presence! avatar)
            (presence-touch! avatar 1)
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::symbol(":remove"),
            )),
        );

        eval_all("((find-method :remove) (list \"Alice\") msg)", &env).unwrap();

        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
        assert_eq!(eval_str("(who-text)", &env), "Who: none.");
        assert_eq!(
            eval_str("(get-prop \"label:did:ma:runtime#avatar\")", &env),
            "Alice"
        );

        eval_all("(add-avatar-presence! avatar)", &env).unwrap();

        assert_eq!(eval_str("(who-text)", &env), "Who: Alice");
    }

    #[test]
    fn room_remove_rejects_ambiguous_nick_and_accepts_did_url() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (define (ma-reply! msg term)
              (inc-prop! "reply-count" 1)
              (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
                        (define (ma-entity-exists? actor) #t)
            (set-prop! "owner" "did:ma:owner")
            (define donald-1 "did:ma:runtime#donald1")
            (define donald-2 "did:ma:runtime#donald2")
            (set-label! donald-1 "Donald Duck")
            (set-label! donald-2 "Donald Duck")
            (add-avatar-presence! donald-1)
            (add-avatar-presence! donald-2)
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::symbol(":remove"),
            )),
        );

        eval_all(
            "((find-method :remove) (list \"Donald\" \"Duck\") msg)",
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(who-text)", &env),
            "Who: Donald Duck, Donald Duck"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("Ambiguous occupant nick: Donald Duck. Use a DID or DID-URL."),
            ])
        );

        eval_all(
            "((find-method :remove) (list \"did:ma:runtime#donald1\") msg)",
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(who-text)", &env), "Who: Donald Duck");
    }

    #[test]
    fn room_owner_can_list_visible_dids() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (define (ma-reply! msg term)
              (inc-prop! "reply-count" 1)
              (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
            (set-prop! "owner" "did:ma:owner")
            (define avatar "did:ma:runtime#avatar")
            (set-label! avatar "Alice")
            (add-avatar-presence! avatar)
            (set-thing! "lamp" "did:ma:runtime#lamp")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::symbol(":dids?"),
            )),
        );

        eval_all("((find-method :dids?) '() msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("DIDs:\nAlice = did:ma:runtime#avatar\nlamp = did:ma:runtime#lamp"),
            ])
        );
    }

    #[test]
    fn room_did_lookup_is_visible_and_explicit() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (define (ma-reply! msg term)
              (inc-prop! "reply-count" 1)
              (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
            (define donald-1 "did:ma:runtime#donald1")
            (define donald-2 "did:ma:runtime#donald2")
            (set-label! donald-1 "Donald Duck")
            (set-label! donald-2 "Donald Duck")
            (add-avatar-presence! donald-1)
            (add-avatar-presence! donald-2)
            (set-thing! "lamp" "did:ma:runtime#lamp")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:user",
                "did:ma:runtime#room",
                Value::symbol(":did?"),
            )),
        );

        eval_all("((find-method :did?) (list \"lamp\") msg)", &env).unwrap();
        eval_all("((find-method :did?) (list \"Donald\" \"Duck\") msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("lamp = did:ma:runtime#lamp"),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("Ambiguous occupant nick: Donald Duck. Ask for a DID or DID-URL."),
            ])
        );
    }

    #[test]
    fn room_did_lookup_prints_when_delegated_by_avatar() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
                        (define (ma-entity-exists? actor) #t)
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
                        (define avatar "did:ma:runtime#avatar")
                        (set-label! avatar "Avatar")
                        (add-avatar-presence! avatar)
            (define duckie "did:ma:runtime#duckie")
            (set-label! duckie "Duckie")
            (add-avatar-presence! duckie)
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#room",
                Value::symbol(":did?"),
            )),
        );

        eval_all("((find-method :did?) (list \"Duckie\") msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Duckie = did:ma:runtime#duckie"),
            ])
        );
    }

    #[test]
    fn room_parent_report_mismatch_removes_occupant() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room-a".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-entity-exists? actor) #t)
            (define avatar "did:ma:runtime#avatar")
            (set-label! avatar "Alice")
            (add-avatar-presence! avatar)
            (presence-request! avatar 1 "n1")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#room-a",
                Value::list(vec![
                    Value::symbol(":parent-report"),
                    Value::str("did:ma:runtime#avatar"),
                    Value::str("did:ma:runtime#room-b"),
                    Value::Int(1),
                    Value::str("n1"),
                ]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
        assert_eq!(eval_str("(who-text)", &env), "Who: none.");
    }

    #[test]
    fn room_presence_tick_removes_unresponsive_occupant_after_timeout() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-entity-exists? actor) #t)
            (define avatar "did:ma:runtime#avatar")
            (set-label! avatar "Alice")
            (add-avatar-presence! avatar)
            (set-prop! "presence:tick" 10)
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#room",
                Value::symbol(":presence-tick"),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
    }

    #[test]
    fn room_registers_presence_schedule_on_init_and_start() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        config.insert("started_at".to_string(), "123".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
                        (define (ma-entity-exists? actor) #t)
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            (on-signal :init)
            (set-prop! "schedule:presence:started-at" "old-runtime")
            (on-signal :start)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#scheduler"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::str("presence"),
                Value::symbol(":interval"),
                Value::str("30s"),
                Value::symbol(":presence-tick"),
            ])
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#scheduler"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::str("presence"),
                Value::symbol(":interval"),
                Value::str("30s"),
                Value::symbol(":presence-tick"),
            ])
        );
    }

    #[test]
    fn room_child_alive_init_notifies_parent() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#child".to_string());
        config.insert("parent".to_string(), "parent".to_string());
        config.insert("started_at".to_string(), "123".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            (set-prop! "schedule:presence:started-at" "123")
            (set-prop! "child-alive-nonce" "nonce-1")
            (set-prop! "child-alive-direction" "dør")
            (notify-child-alive!)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#parent"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":child-alive"),
                Value::str("did:ma:runtime#child"),
                Value::str("/ma/room/0.0.1"),
                Value::str("nonce-1"),
                Value::str("dør"),
            ])
        );
    }

    #[test]
    fn agent_commits_parent_from_pending_room_ctx() {
        let env = agent_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#rms".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room-b")
            (enter "did:ma:runtime#room-a")
            "#,
            &env,
        )
        .unwrap();
        assert_eq!(eval_str("(pending-room)", &env), "did:ma:runtime#room-a");

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#room-a", "did:ma:runtime#rms")),
        );
        eval_all(
            r#"
            ((find-method :ctx)
              (list (list (list :protocol LAMBDA_CTX_PROTOCOL)
                          (list :kind "agent")
                          (list :room "did:ma:runtime#room-a")))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(parent)", &env), "did:ma:runtime#room-a");
        assert_eq!(eval_str("(pending-room)", &env), "");
    }

    #[test]
    fn unowned_agent_without_recovery_secret_can_be_claimed() {
        let env = agent_env();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#duckie")),
        );

        eval_all("((find-method :claim) '() msg)", &env).unwrap();

        assert_eq!(eval_str("(owner)", &env), "did:ma:owner");
    }

    #[test]
    fn agent_does_not_route_new_move_while_entry_pending() {
        let env = agent_env();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#rms")),
        );
        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room-b")
            (set-prop! "pending-room" "did:ma:runtime#room-a")
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1))
            ((find-method :move) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
    }

    #[test]
    fn avatar_normalises_incoming_command_verb_only() {
        let env = avatar_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{user}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:user")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                user,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":Look")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();
        assert!(eval_bool(
            r#"(equal? (get-prop "sent-term:1") (list :look))"#,
            &env
        ));

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                user,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":Say"), Value::str("Hello THERE")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();
        assert!(eval_bool(
            r#"(equal? (get-prop "sent-term:2") (list :say "Hello THERE"))"#,
            &env
        ));
    }

    #[test]
    fn avatar_forwards_look_arguments_to_room() {
        let env = avatar_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{user}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:user")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                user,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":look"), Value::str("north")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":look"), Value::str("north")]),
        );
    }

    #[test]
    fn avatar_forwards_unknown_room_verbs_without_owner_delegation() {
        let env = avatar_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{user}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:user")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                user,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":dids?")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":dids?")]),
        );
    }

    #[test]
    fn agent_treats_same_runtime_did_url_as_local_actor_caller() {
        let env = agent_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#room", "did:ma:runtime#rms")),
        );

        assert_eq!(
            eval_str(
                "(effective-user (list \"did:ma:user\" \"north\") msg)",
                &env
            ),
            "did:ma:user"
        );
        assert_eq!(
            eval_str(
                "(car (effective-args (list \"did:ma:user\" \"north\") msg))",
                &env
            ),
            "north"
        );
    }

    #[test]
    fn room_move_selects_an_available_exit() {
        let env = room_env();
        assert!(eval_bool("(not (random-exit-direction))", &env));
        eval_all("(put-exit! \"north\" \"did:ma:runtime#north-exit\")", &env).unwrap();
        assert_eq!(eval_str("(random-exit-direction)", &env), "north");
    }

    #[test]
    fn room_exit_lookup_accepts_non_ascii_direction() {
        let env = room_env();
        eval_all("(put-exit! \"dør\" \"did:ma:runtime#door-exit\")", &env).unwrap();

        assert_eq!(eval_str("(exits-text)", &env), "Exits: dør");
        assert_eq!(
            eval_str("(exit-target \"dør\")", &env),
            "did:ma:runtime#door-exit"
        );
    }

    #[test]
    fn room_exit_listing_is_direction_only() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (put-exit! "north" "did:ma:runtime#north-exit")
            (set-prop! "exit-target:north" "did:ma:runtime#kitchen")
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(exits-text)", &env), "Exits: north");
        assert_eq!(
            eval_str("(exit-target \"north\")", &env),
            "did:ma:runtime#north-exit"
        );
    }

    #[test]
    fn room_exit_init_persists_exit_state() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#source".to_string());
        crate::state::set_config(config);

        assert_eq!(
            eval_str("(exit-init \"dør\" \"did:ma:runtime#kitchen\")", &env),
            "(set-prop! \"direction\" \"dør\")\n(set-prop! \"source-room\" \"did:ma:runtime#source\")\n(set-prop! \"target-room\" \"did:ma:runtime#kitchen\")\n(ma-save-state!)\n"
        );
    }

    #[test]
    fn room_exit_init_passes_owner_to_exit_actor() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#source".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(exit-init \"north\" \"did:ma:runtime#kitchen\")", &env),
            "(set-prop! \"direction\" \"north\")\n(set-prop! \"owner\" \"did:ma:owner\")\n(set-prop! \"source-room\" \"did:ma:runtime#source\")\n(set-prop! \"target-room\" \"did:ma:runtime#kitchen\")\n(ma-save-state!)\n"
        );
    }

    #[test]
    fn room_exit_command_forwards_to_exit_actor() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (put-exit! "north" "did:ma:runtime#north-exit")
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::list(vec![
                    Value::symbol(":exit"),
                    Value::str("north"),
                    Value::symbol(":lock"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":lock")]),
        );

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::list(vec![
                    Value::symbol(":exit"),
                    Value::str("north"),
                    Value::symbol(":unlock"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":unlock")]),
        );
    }

    #[test]
    fn room_traversal_forwards_minimal_ctx_to_exit() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
                        (define (ma-entity-exists? actor) #t)
            (traverse-exit! "did:ma:runtime#avatar" "did:ma:user" "north" "did:ma:runtime#north-exit")
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :traverse)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:user"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"avatar\")",
                &env
            ),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"room\")",
                &env
            ),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"kind\")",
                &env
            ),
            "avatar"
        );
    }

    #[test]
    fn room_go_from_direct_user_did_moves_the_users_avatar() {
        let env = room_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (put-exit! "north" "did:ma:other#north-exit")
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                user,
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":go"), Value::str("north")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        let expected_avatar = eval_str(&format!(r#"(avatar-for-user "{user}")"#), &env);
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:other#north-exit"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :traverse)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            user
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"avatar\")",
                &env
            ),
            expected_avatar
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"kind\")",
                &env
            ),
            "avatar"
        );
    }

    #[test]
    fn room_go_rejects_foreign_delegated_avatar_call() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:target".to_string());
        config.insert("self".to_string(), "did:ma:target#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (put-exit! "cloud" "did:ma:target#cloud-exit")
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            (define (ma-reply! msg term)
              (inc-prop! "reply-count" 1)
              (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:source#avatar",
                "did:ma:target#room",
                Value::list(vec![
                    Value::symbol(":go"),
                    Value::str("did:ma:user"),
                    Value::str("cloud"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert!(eval_bool(
            "(equal? (car (get-prop \"reply-term:1\")) :ok)",
            &env
        ));
    }

    #[test]
    fn room_go_no_exit_acknowledges_direct_user_call() {
        let env = room_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            (define (ma-reply! msg term)
              (inc-prop! "reply-count" 1)
              (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                user,
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":go"), Value::str("cloud")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        let expected_avatar = eval_str(&format!(r#"(avatar-for-user "{user}")"#), &env);
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            expected_avatar
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("")])
        );
    }

    #[test]
    fn room_remove_exit_clears_topology_state() {
        let env = room_env();
        eval_all(
            r#"
            (put-exit! "north" "did:ma:runtime#north-exit")
            (set-prop! "exit-target:north" "did:ma:runtime#kitchen")
            (set-prop! "exit-target-name:north" "Kitchen")
            (remove-exit! "north")
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(exits-text)", &env), "Exits: none.");
        assert!(eval_bool("(not (exit-target \"north\"))", &env));
        assert!(eval_bool("(not (get-prop \"exit-target:north\"))", &env));
        assert!(eval_bool(
            "(not (get-prop \"exit-target-name:north\"))",
            &env
        ));
    }

    #[test]
    fn exit_fill_only_allows_source_room_to_end_it() {
        let env = exit_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#north-exit".to_string());
        crate::state::set_config(config);
        eval_all("(set-prop! \"source-room\" \"did:ma:runtime#room\")", &env).unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#intruder",
                "did:ma:runtime#north-exit",
                Value::symbol(":fill"),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();
        assert!(eval_bool("(not (get-prop \"ended\"))", &env));

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#north-exit",
                Value::symbol(":fill"),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();
        assert_eq!(eval_str("(get-prop \"ended\")", &env), "yes");
    }

    #[test]
    fn exit_traverse_returns_transformed_ctx_to_source_room() {
        let env = exit_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#north-exit".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "direction" "north")
            (set-prop! "source-room" "did:ma:runtime#room")
            (set-prop! "target-room" "did:ma:runtime#kitchen")
            (set-prop! "traveller-message" "You pass through the oak door.")
            (define (ma-send! target term)
              (set-prop! "sent-target" target)
              (set-prop! "sent-term" term))
                        (define (ma-reply! msg term)
                            (inc-prop! "reply-count" 1)
                            (set-prop! (string-append "reply-term:" (number->string (get-prop "reply-count"))) term))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#north-exit",
            )),
        );
        eval_all(
            r#"
            ((find-method :traverse)
              (list (map-set
                      (map-set
                        (map-set (make-map) "actor" "did:ma:runtime#avatar")
                        "kind" "avatar")
                      "room" "did:ma:runtime#room"))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target\")", &env),
            "did:ma:runtime#room"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term\")) :traversed)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"kind\")",
                &env
            ),
            "avatar"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"room\")",
                &env
            ),
            "did:ma:runtime#kitchen"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"text\")",
                &env
            ),
            "You pass through the oak door."
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"exit\")",
                &env
            ),
            "did:ma:runtime#north-exit"
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("")])
        );
    }

    #[test]
    fn avatar_ctx_forwards_movement_ctx_to_user() {
        let env = avatar_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{user}")"#), &env);
        let avatar = format!("did:ma:runtime#{avatar_id}");
        config.insert("self".to_string(), avatar.clone());
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:user")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#room", &avatar)),
        );
        eval_all(
            r#"
                                                ((find-method :ctx)
                                                        (list (map-set
                                                                            (map-set
                                                                                (map-set
                                                                                    (map-set
                                                                                        (map-set (make-map) "actor" "did:ma:user")
                                                                                        "kind" "avatar")
                                                                                    "avatar" (local-self))
                                                                                "room" "did:ma:runtime#kitchen")
                                                                            "text" "You pass through the oak door."))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), user);
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("You pass through the oak door.")
            ]),
        );
        assert_eq!(eval_str("(get-prop \"sent-target:2\")", &env), user);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"actor\")",
                &env
            ),
            user
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"kind\")",
                &env
            ),
            "avatar"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"avatar\")",
                &env
            ),
            avatar
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"room\")",
                &env
            ),
            "did:ma:runtime#kitchen"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"text\")",
                &env
            ),
            "You pass through the oak door."
        );
        assert_eq!(
            eval_str("(get-prop \"pending-room\")", &env),
            "did:ma:runtime#kitchen"
        );
    }

    #[test]
    fn avatar_ctx_forwards_cross_runtime_movement_ctx_to_user() {
        let env = avatar_env();
        let user = "did:ma:user";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:sky".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{user}")"#), &env);
        let avatar = format!("did:ma:sky#{avatar_id}");
        config.insert("self".to_string(), avatar.clone());
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:user")
            (set-prop! "room" "did:ma:sky#construct")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:sky#construct", &avatar)),
        );
        eval_all(
            r#"
                        (let ((ctx (map-set
                                                 (map-set
                                                     (map-set
                                                         (map-set (make-map) "actor" "did:ma:user")
                                                         "avatar" (local-self))
                                                     "kind" "avatar")
                                                 "room" "did:ma:ma#cloud")))
                            ((find-method :ctx) (list ctx) msg))
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), user);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            user
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"avatar\")",
                &env
            ),
            avatar
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"room\")",
                &env
            ),
            "did:ma:ma#cloud"
        );
        assert_eq!(
            eval_str("(get-prop \"pending-room\")", &env),
            "did:ma:ma#cloud"
        );
    }

    #[test]
    fn agent_ctx_enters_target_room_itself() {
        let env = agent_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#rms".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room")
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#room", "did:ma:runtime#rms")),
        );
        eval_all(
            r#"
                        ((find-method :ctx)
                            (list (map-set
                                            (map-set
                                                (map-set (make-map) "actor" (self))
                                                "kind" "agent")
                                            "room" "did:ma:runtime#kitchen"))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":leave-occupant")]),
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#kitchen"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :enter)",
            &env
        ));
        assert_eq!(eval_str("(pending-room)", &env), "did:ma:runtime#kitchen");
    }

    #[test]
    fn room_heals_dead_local_exit_before_traversal() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "exit-target:north" "did:ma:runtime#kitchen")
            (put-exit! "north" "did:ma:runtime#dead-exit")
                        (define (ma-entity-exists? actor)
                            (equal? actor "did:ma:runtime#avatar"))
            (define (ma-create-actor kind behaviour init fragment)
              (set-prop! "created-fragment" fragment)
              fragment)
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
                            (set-prop! "sent-target" target)
                            (set-prop! "sent-term" term))
            (traverse-exit! "did:ma:runtime#avatar" "did:ma:user" "north" "did:ma:runtime#dead-exit")
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"created-fragment\")", &env),
            eval_str("(exit-fragment \"north\")", &env)
        );
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target\")", &env),
            eval_str("(entity-url (exit-fragment \"north\"))", &env)
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term\")", &env).unwrap(),
            eval_all(
                "(list :traverse (movement-ctx \"did:ma:runtime#avatar\" \"did:ma:user\"))",
                &env
            )
            .unwrap(),
        );
        assert_ne!(
            eval_str("(exit-target \"north\")", &env),
            "did:ma:runtime#dead-exit"
        );
    }

    #[test]
    fn room_remembers_named_dig_target_for_idempotency() {
        let env = room_env();
        eval_all(
            "(remember-exit-target! \"dør\" \"did:ma:runtime#kitchen\" \"køkken\")",
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(remembered-new-room-target \"dør\" \"køkken\")", &env),
            "did:ma:runtime#kitchen"
        );
        assert!(eval_bool(
            "(not (remembered-new-room-target \"dør\" \"stue\"))",
            &env
        ));
    }

    #[test]
    fn room_dig_waits_for_new_room_child_alive_callback() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#source".to_string());
        crate::state::set_config(config);

        let owner_avatar = eval_str(r#"(avatar-for-user "did:ma:user")"#, &env);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:user")
            (define owner-avatar (avatar-for-user "did:ma:user"))
            (set-label! owner-avatar "me")
            (add-avatar-presence! owner-avatar)
                        (define (ma-entity-exists? actor) (same-actor? actor owner-avatar))
            (define (ma-create-actor kind behaviour init fragment)
              (inc-prop! "created-count" 1)
              (set-prop! (string-append "created-kind:" (number->string (get-prop "created-count"))) kind)
              (set-prop! (string-append "created-init:" (number->string (get-prop "created-count"))) init)
              (if fragment fragment "random-room"))
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                &owner_avatar,
                "did:ma:runtime#source",
                Value::list(vec![
                    Value::symbol(":dig"),
                    Value::str("dør"),
                    Value::str("to"),
                    Value::str("køkken"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"created-count\")", &env), 1);
        assert!(eval_bool(
            &format!(r#"(member-entry? "{owner_avatar}" (occupants))"#),
            &env
        ));
        assert!(eval_bool("(not (exit-target \"dør\"))", &env));
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        let target_room = eval_str("(get-prop \"pending-new-room:dør\")", &env);
        assert!(target_room.starts_with("did:ma:runtime#"));
        assert!(eval_str("(get-prop \"created-init:1\")", &env).contains("child-alive-nonce"));
        assert!(eval_str("(get-prop \"created-init:1\")", &env).contains("notify-child-alive!"));

        let nonce = eval_str("(get-prop \"pending-new-room-nonce:dør\")", &env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                &target_room,
                "did:ma:runtime#source",
                Value::list(vec![
                    Value::symbol(":child-alive"),
                    Value::str(target_room.clone()),
                    Value::str("/ma/room/0.0.1"),
                    Value::str(nonce),
                    Value::str("dør"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"created-count\")", &env), 2);
        assert_eq!(
            eval_str("(exit-target \"dør\")", &env),
            eval_str("(entity-url (exit-fragment \"dør\"))", &env)
        );
        assert_eq!(
            eval_str("(get-prop \"exit-target:dør\")", &env),
            target_room
        );
        assert!(eval_bool("(not (get-prop \"pending-new-room:dør\"))", &env));
        assert!(eval_int("(get-prop \"sent-count\")", &env) >= 2);
    }

    #[test]
    fn room_named_dig_fragments_are_deterministic() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:k51runtime".to_string());
        config.insert("self".to_string(), "#source".to_string());
        crate::state::set_config(config);

        let kitchen = eval_str("(named-room-fragment \"dør\" \"køkken\")", &env);
        assert_eq!(kitchen.len(), 16);
        assert!(kitchen.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            kitchen,
            eval_str("(named-room-fragment \"dør\" \"køkken\")", &env)
        );
        assert_ne!(
            kitchen,
            eval_str("(named-room-fragment \"vindu\" \"køkken\")", &env)
        );
        assert_ne!(
            kitchen,
            eval_str("(named-room-fragment \"dør\" \"stue\")", &env)
        );

        let exit = eval_str("(exit-fragment \"dør\")", &env);
        assert_eq!(exit.len(), 16);
        assert_eq!(exit, eval_str("(exit-fragment \"dør\")", &env));
        assert_ne!(exit, kitchen);
    }

    #[test]
    fn room_ctx_terms_use_fully_qualified_actor_refs() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:k51target".to_string());
        config.insert("self".to_string(), "#construct".to_string());
        config.insert("root".to_string(), "#root".to_string());
        crate::state::set_config(config);
        eval_all(
            r##"(define (ctx-term-value term key)
    (let loop ((pairs (car (cdr term))))
        (cond ((null? pairs) #f)
                    ((equal? (car (car pairs)) key) (car (cdr (car pairs))))
                    (else (loop (cdr pairs))))))
(define avatar-ctx (avatar-room-ctx "#alice" "Alice" ""))"##,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(ctx-term-value avatar-ctx :root)", &env),
            "did:ma:k51target#root"
        );
        assert_eq!(
            eval_str("(ctx-term-value avatar-ctx :avatar)", &env),
            "did:ma:k51target#alice"
        );
        assert_eq!(
            eval_str("(ctx-term-value avatar-ctx :room)", &env),
            "did:ma:k51target#construct"
        );
    }

    #[test]
    fn random_builtin_returns_integer_in_range() {
        let env = new_root_env();
        let value = eval_int("(random 3)", &env);
        assert!((0..3).contains(&value));
        assert!(eval_all("(random 0)", &env).is_err());
        assert!(eval_all("(random \"3\")", &env).is_err());
    }

    #[test]
    fn blake3_hashes_strings_as_lower_hex() {
        let env = new_root_env();

        assert_eq!(
            eval_str("(blake3 \"abc\")", &env),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        assert_eq!(eval_str("(blake3 \"abc\" 8)", &env), "6437b3ac38465133");
        assert!(eval_all("(blake3 42)", &env).is_err());
        assert!(eval_all("(blake3 \"abc\" 0)", &env).is_err());
        assert!(eval_all("(blake3 \"abc\" 33)", &env).is_err());
    }

    #[test]
    fn room_reconcile_does_not_add_unlabelled_callers() {
        let env = room_env();
        eval_all(
            "(reconcile-caller-occupant! \"did:ma:runtime#raw-avatar\")",
            &env,
        )
        .unwrap();
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
    }

    #[test]
    fn arithmetic() {
        assert_eq!(run("(+ 1 2 3)"), Value::Int(6));
        assert_eq!(run("(- 10 3 2)"), Value::Int(5));
        assert_eq!(run("(* 2 3 4)"), Value::Int(24));
        assert_eq!(run("(/ 10 2)"), Value::Float(5.0));
    }

    #[test]
    fn integer_arithmetic_overflow_is_an_error() {
        let env = new_root_env();
        assert!(eval_all("(+ 9223372036854775807 1)", &env).is_err());
        assert!(eval_all("(- -9223372036854775808)", &env).is_err());
        assert!(eval_all("(* 9223372036854775807 2)", &env).is_err());
    }

    #[test]
    fn comparisons() {
        assert_eq!(run("(= 1 1 1)"), Value::Bool(true));
        assert_eq!(run("(< 1 2 3)"), Value::Bool(true));
        assert_eq!(run("(< 1 3 2)"), Value::Bool(false));
    }

    #[test]
    fn define_and_lookup() {
        assert_eq!(run("(define x 42) x"), Value::Int(42));
    }

    #[test]
    fn if_special_form() {
        assert_eq!(run("(if #t 1 2)"), Value::Int(1));
        assert_eq!(run("(if #f 1 2)"), Value::Int(2));
        assert_eq!(run("(if (= 1 2) 1)"), Value::Nil);
    }

    #[test]
    fn cond_special_form() {
        assert_eq!(
            run("(cond ((= 1 2) :no) ((= 1 1) :yes) (#t :fallback))"),
            Value::symbol(":yes")
        );
        assert_eq!(
            run("(cond ((= 1 2) :no) (else :fallback))"),
            Value::symbol(":fallback")
        );
        assert_eq!(run("(cond ((= 1 2) :no))"), Value::Nil);
    }

    #[test]
    fn cond_else_must_be_last() {
        let env = new_root_env();
        let err = eval_all("(cond (else :fallback) (#t :unreachable))", &env)
            .expect_err("non-final else clause should fail");
        assert_eq!(err.to_string(), "cond: else clause must be last");
    }

    #[test]
    fn when_and_unless() {
        assert_eq!(run("(when #t 1 2 3)"), Value::Int(3));
        assert_eq!(run("(when #f 1 2 3)"), Value::Nil);
        assert_eq!(run("(unless #f 1 2 3)"), Value::Int(3));
        assert_eq!(run("(unless #t 1 2 3)"), Value::Nil);
    }

    #[test]
    fn and_or() {
        assert_eq!(run("(and 1 2 3)"), Value::Int(3));
        assert_eq!(run("(and 1 #f 3)"), Value::Bool(false));
        assert_eq!(run("(or #f #f 3)"), Value::Int(3));
        assert_eq!(run("(or #f #f #f)"), Value::Bool(false));
    }

    #[test]
    fn lambda_and_application() {
        assert_eq!(run("((lambda (x y) (+ x y)) 3 4)"), Value::Int(7));
    }

    #[test]
    fn define_function_sugar() {
        assert_eq!(
            run("(define (square x) (* x x)) (square 5)"),
            Value::Int(25)
        );
    }

    #[test]
    fn let_forms() {
        assert_eq!(run("(let ((x 1) (y 2)) (+ x y))"), Value::Int(3));
        assert_eq!(run("(let* ((x 1) (y (+ x 1))) (+ x y))"), Value::Int(3));
        assert_eq!(
            run("(let loop ((n 3) (acc 0)) (if (= n 0) acc (loop (- n 1) (+ acc n))))"),
            Value::Int(6)
        );
        assert_eq!(
            run("(let loop ((f (lambda (x) x))) (procedure? f))"),
            Value::Bool(true)
        );
    }

    #[test]
    fn letrec_mutual_recursion() {
        let src = r#"
            (letrec ((even? (lambda (n) (if (= n 0) #t (odd? (- n 1)))))
                     (odd?  (lambda (n) (if (= n 0) #f (even? (- n 1))))))
              (even? 10))
        "#;
        assert_eq!(run(src), Value::Bool(true));
    }

    #[test]
    fn duplicate_bindings_are_rejected() {
        let env = new_root_env();
        assert!(eval_all("(lambda (x x) x)", &env).is_err());
        assert!(eval_all("(define (f x x) x)", &env).is_err());
        assert!(eval_all("(let ((x 1) (x 2)) x)", &env).is_err());
        assert!(eval_all("(let loop ((x 1) (x 2)) x)", &env).is_err());
        assert!(eval_all("(letrec ((x 1) (x 2)) x)", &env).is_err());
        assert_eq!(
            eval_all("(let* ((x 1) (x 2)) x)", &env).unwrap(),
            Value::Int(2)
        );
    }

    #[test]
    fn let_forms_require_a_body() {
        let env = new_root_env();
        assert!(eval_all("(let ((x 1)))", &env).is_err());
        assert!(eval_all("(let* ((x 1)))", &env).is_err());
        assert!(eval_all("(letrec ((x 1)))", &env).is_err());
    }

    #[test]
    fn set_bang_mutates_lexical_binding() {
        assert_eq!(run("(define x 1) (set! x (+ x 1)) x"), Value::Int(2));
    }

    #[test]
    fn quote_produces_inert_data() {
        assert_eq!(run("(car '(1 2 3))"), Value::Int(1));
    }

    #[test]
    fn malformed_special_forms_reject_extra_operands() {
        let env = new_root_env();
        assert!(eval_all("(quote a b)", &env).is_err());
        assert!(eval_all("(if #t 1 2 3)", &env).is_err());
        assert!(eval_all("(define x 1 2)", &env).is_err());
        assert!(eval_all("(define x)", &env).is_err());
        assert!(eval_all("(define x 1) (set! x 2 3)", &env).is_err());
    }

    #[test]
    fn deep_tail_recursion_does_not_overflow_stack() {
        // Required by ma-scheme-v1.md §7: self-tail-calls must run in O(1)
        // host stack. 1,000,000 iterations would blow a naively recursive
        // Rust implementation's stack; a trampoline handles it trivially.
        let src = r#"
            (define (count-down n)
              (if (= n 0) :done (count-down (- n 1))))
            (count-down 1000000)
        "#;
        assert_eq!(run(src), Value::symbol(":done"));
    }

    #[test]
    fn string_and_type_builtins() {
        assert_eq!(run(r#"(string-append "foo" "bar")"#), Value::str("foobar"));
        assert_eq!(run(r#"(string-length "føø")"#), Value::Int(3));
        assert_eq!(run(r#"(string-empty? "")"#), Value::Bool(true));
        assert_eq!(run(r##"(string-prefix? "#" "#room")"##), Value::Bool(true));
        assert_eq!(
            run(r##"(string-prefix? "#" "did:ma:abc#room")"##),
            Value::Bool(false)
        );
        assert_eq!(
            run(r#"(string-suffix? "room" "did:ma:abc#room")"#),
            Value::Bool(true)
        );
        assert_eq!(
            run(r##"(string-contains? "#" "did:ma:abc#room")"##),
            Value::Bool(true)
        );
        assert_eq!(run(r#"(substring "føøbar" 1 4)"#), Value::str("øøb"));
        assert_eq!(run(r#"(string-trim "  hi\n")"#), Value::str("hi"));
        assert_eq!(
            run(r#"(string-split "a,b,c" ",")"#),
            Value::list(vec![Value::str("a"), Value::str("b"), Value::str("c")])
        );
        assert_eq!(
            run(r#"(string-join (list "a" "b" "c") "/")"#),
            Value::str("a/b/c")
        );
        assert_eq!(run(r#"(string-upcase "abcæ")"#), Value::str("ABCÆ"));
        assert_eq!(run(r#"(string-downcase "ABCÆ")"#), Value::str("abcæ"));
        assert_eq!(run(r#"(char-upcase "æ")"#), Value::str("Æ"));
        assert_eq!(run(r#"(char-downcase "Æ")"#), Value::str("æ"));
        assert_eq!(run("(number->string 42)"), Value::str("42"));
        assert_eq!(run(r#"(string->number "42")"#), Value::Int(42));
        assert_eq!(run("(symbol->string ':Look)"), Value::str(":Look"));
        assert_eq!(run(r#"(string->symbol ":look")"#), Value::symbol(":look"));
        assert_eq!(run("(string? \"x\")"), Value::Bool(true));
        assert_eq!(run("(number? 1)"), Value::Bool(true));
        assert_eq!(run("(symbol? 'x)"), Value::Bool(true));
        assert_eq!(run("(map? (make-map))"), Value::Bool(true));
        assert_eq!(run("(procedure? car)"), Value::Bool(true));
    }

    #[test]
    fn map_builtins() {
        assert_eq!(
            run(r#"(map-ref (make-map "a" 1 "b" 2) "a")"#),
            Value::Int(1)
        );
        assert_eq!(
            run(r#"(map-ref (make-map) "missing" "fallback")"#),
            Value::str("fallback")
        );
        assert_eq!(
            run(r#"(map-has-key? (make-map "a" 1) "a")"#),
            Value::Bool(true)
        );
        assert_eq!(
            run(r#"(map-keys (make-map "b" 2 "a" 1))"#),
            Value::list(vec![Value::str("a"), Value::str("b")])
        );
        assert_eq!(
            run(r#"(map-values (make-map "b" 2 "a" 1))"#),
            Value::list(vec![Value::Int(1), Value::Int(2)])
        );
        assert_eq!(
            run(r#"(map-ref (map-set (make-map "a" 1) "a" 9) "a")"#),
            Value::Int(9)
        );
        assert_eq!(
            run(r#"(map-has-key? (map-delete (make-map "a" 1) "a") "a")"#),
            Value::Bool(false)
        );
        assert_eq!(
            run(r#"(map-ref (alist->map (map->alist (make-map "a" 1))) "a")"#),
            Value::Int(1)
        );
        assert_eq!(
            run(r#"(map-ref (make-map "a" 1 "a" 2) "a")"#),
            Value::Int(2)
        );
    }

    #[test]
    fn equal_p_deep_comparison() {
        assert_eq!(run("(equal? '(1 2 3) '(1 2 3))"), Value::Bool(true));
        assert_eq!(run("(equal? '(1 2 3) '(1 2 4))"), Value::Bool(false));
    }
}
