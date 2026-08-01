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

    fn thing_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/thing.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f)",
            &env,
        )
        .unwrap();
        env
    }

    fn container_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/container.ma"), &env).unwrap();
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

    fn actor_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f)",
            &env,
        )
        .unwrap();
        env
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
    fn actor_metadata_methods_are_generic() {
        let env = actor_env();
        let mut config = std::collections::HashMap::new();
        config.insert("id".to_string(), "lamp".to_string());
        config.insert("kind".to_string(), "/ma/thing/0.0.1".to_string());
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#lamp")),
        );
        env.define(
            Rc::from("other_msg"),
            Value::Msg(sample_msg("did:ma:other", "did:ma:runtime#lamp")),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            ((find-method :name) '() owner_msg)
            ((find-method :description) '() owner_msg)
            ((find-method :kind?) '() owner_msg)
            ((find-method :name) (list "Brass" "Lamp") owner_msg)
            ((find-method :description) (list "A" "warm" "desk" "lamp") owner_msg)
            ((find-method :kind?) (list "/ma/other/0.0.1") owner_msg)
            ((find-method :name) (list "Stolen") other_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 7);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("(none)")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("(none)")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("/ma/thing/0.0.1")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:4\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("Brass Lamp")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:5\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("A warm desk lamp")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:6\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("kind is read-only")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:7\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner may set name")
            ])
        );
        assert_eq!(eval_str("(get-prop \"name\")", &env), "Brass Lamp");
        assert_eq!(
            eval_str("(get-prop \"description\")", &env),
            "A warm desk lamp"
        );
    }

    #[test]
    fn actor_metadata_setter_accepts_controlling_did() {
        let env = actor_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#avatar")),
        );

        eval_all(
            r#"
            (set-prop! "did" "did:ma:owner")
            ((find-method :name) (list "Pondus") msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("Pondus")])
        );
        assert_eq!(eval_str("(get-prop \"name\")", &env), "Pondus");
    }

    #[test]
    fn reply_ok_helpers_split_ack_from_payload() {
        let env = agent_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:caller", "did:ma:actor")),
        );

        eval_all("(reply-ok msg)", &env).unwrap();
        eval_all("(reply-ok-with msg \"payload\")", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("payload")])
        );
        assert!(eval_all("(reply-ok msg \"payload\")", &env).is_err());
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
    fn duck_on_message_parent_query_uses_inherited_actor_handler_args_first() {
        let env = duck_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#duckie",
                Value::symbol(":parent?"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#construct")
            (on-message msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("did:ma:runtime#construct")
            ])
        );
    }

    #[test]
    fn thing_on_message_parent_query_uses_inherited_actor_handler_args_first() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#lamp",
                Value::symbol(":parent?"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#construct")
            (on-message msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("did:ma:runtime#construct")
            ])
        );
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
            Value::symbol(":ok")
        );
    }

    #[test]
    fn duck_duck_emotes_in_room_with_silent_ack() {
        let env = duck_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#duckie")),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#construct")
            ((find-method :duck) '() msg)
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
            Value::list(vec![
                Value::symbol(":emote"),
                Value::str("waddles through the room. It looks busy.")
            ])
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn duck_cmds_include_duck_specific_verbs() {
        let env = duck_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#duckie")),
        );

        eval_all("((find-method :cmds?) '() msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::list(vec![
                    Value::symbol(":go"),
                    Value::symbol(":move"),
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                    Value::symbol(":duck"),
                    Value::symbol(":quack"),
                ])
            ])
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
            Value::symbol(":ok")
        );
    }

    #[test]
    fn specialised_agents_register_cmds() {
        let duck = duck_env();
        install_send_reply_recorders(&duck);
        duck.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#duckie")),
        );
        eval_all("((find-method :cmds?) '() msg)", &duck).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &duck).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::list(vec![
                    Value::symbol(":go"),
                    Value::symbol(":move"),
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                    Value::symbol(":duck"),
                    Value::symbol(":quack"),
                ])
            ])
        );

        let rms = rms_env();
        install_send_reply_recorders(&rms);
        rms.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#rms")),
        );
        eval_all("((find-method :cmds?) '() msg)", &rms).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &rms).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::list(vec![
                    Value::symbol(":go"),
                    Value::symbol(":move"),
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                    Value::symbol(":fortune"),
                ])
            ])
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
        config.insert("self".to_string(), "did:ma:runtime#construct".to_string());
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
    fn room_entry_tells_entering_avatar_the_room_name() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:target".to_string());
        config.insert("self".to_string(), "did:ma:target#kitchen".to_string());
        config.insert("root".to_string(), "did:ma:target#root".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);

        eval_all(
            r#"
            (define (ma-entity-exists? actor) #t)
            (define bob "did:ma:target#bob")
            (define alice "did:ma:target#alice")
            (set-prop! "name" "Kitchen")
            (set-label! bob "Bob")
            (add-avatar-presence! bob)
            (commit-avatar-entry! alice #f "Alice")
            (define (ctx-term-value term key)
              (let loop ((pairs (car (cdr term))))
                (cond ((null? pairs) #f)
                      ((equal? (car (car pairs)) key) (car (cdr (car pairs))))
                      (else (loop (cdr pairs))))))
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:target#bob"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":print"), Value::str("Alice arrives.")])
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:target#alice"
        );
        assert_eq!(
            eval_str("(ctx-term-value (get-prop \"sent-term:2\") :text)", &env),
            "Kitchen"
        );
    }

    #[test]
    fn room_cross_runtime_avatar_ctx_requests_local_avatar() {
        let env = room_env();
        let did = "did:ma:did";
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
                                                        (map-set (make-map) "actor" "did:ma:did")
                                                        "avatar" "did:ma:sky#avatar")
                                                    "kind" "avatar")
                                                "room" "did:ma:ma#cloud")
                                            "nick" "Lars"))
                            msg)
                        "#,
            &env,
        )
        .unwrap();

        let expected_fragment = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
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

        let avatar = eval_str(&format!(r#"(avatar-for-did "{owner}")"#), &env);

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
        assert!(eval_bool(
            r##"(valid-did-url? "did:ma:runtime#room")"##,
            &env
        ));
        assert!(!eval_bool(r##"(valid-did-url? "did:ma:runtime")"##, &env));
        assert!(eval_bool(r##"(valid-did? "did:ma:did")"##, &env));
        assert!(!eval_bool(r##"(valid-did? "did:ma:runtime#room")"##, &env));
    }

    #[test]
    fn room_owner_transfer_uses_ownership_errors() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
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

        assert!(eval_bool("(not (get-prop \"sent-count\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("Only this room's owner can transfer ownership."),
            ])
        );
    }

    #[test]
    fn room_owner_getter_replies_with_raw_did() {
        let env = room_env();
        install_send_reply_recorders(&env);
        eval_all(r#"(set-prop! "owner" "did:ma:owner")"#, &env).unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#room")),
        );

        eval_all("((find-method :owner) '() msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("did:ma:owner")])
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
            (put-exit! "down" "did:ma:runtime#down-exit")
            (define cloud-avatar "did:ma:runtime#cloud-avatar")
            (set-label! cloud-avatar "cloud")
            (add-avatar-presence! cloud-avatar)
            (set-thing! "cloud" "did:ma:runtime#cloud-thing")
            (put-exit! "cloud" "did:ma:runtime#cloud-exit")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:did",
                "did:ma:runtime#room",
                Value::symbol(":did?"),
            )),
        );

        eval_all("((find-method :did?) (list \"lamp\") msg)", &env).unwrap();
        eval_all("((find-method :did?) (list \"down\") msg)", &env).unwrap();
        eval_all("((find-method :did?) (list \"Donald\" \"Duck\") msg)", &env).unwrap();
        eval_all("((find-method :did?) (list \"cloud\") msg)", &env).unwrap();
        eval_all("((find-method :did?) (list \"exit\" \"cloud\") msg)", &env).unwrap();
        eval_all("((find-method :did?) (list \"thing\" \"cloud\") msg)", &env).unwrap();
        eval_all(
            "((find-method :did?) (list \"occupant\" \"cloud\") msg)",
            &env,
        )
        .unwrap();

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
                Value::symbol(":ok"),
                Value::str("down = did:ma:runtime#down-exit"),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str(
                    "Ambiguous name: Donald Duck\noccupant Donald Duck = did:ma:runtime#donald2\noccupant Donald Duck = did:ma:runtime#donald1",
                ),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:4\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str(
                    "Ambiguous name: cloud\nexit cloud = did:ma:runtime#cloud-exit\nthing cloud = did:ma:runtime#cloud-thing\noccupant cloud = did:ma:runtime#cloud-avatar",
                ),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:5\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("exit cloud = did:ma:runtime#cloud-exit"),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:6\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("thing cloud = did:ma:runtime#cloud-thing"),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:7\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("occupant cloud = did:ma:runtime#cloud-avatar"),
            ])
        );
    }

    #[test]
    fn room_did_lookup_replies_when_delegated_by_avatar() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (define (ma-entity-exists? actor) #t)
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

        assert!(eval_bool("(not (get-prop \"sent-count\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("Duckie = did:ma:runtime#duckie"),
            ])
        );
    }

    #[test]
    fn room_drop_accepts_avatar_resolved_carried_actor() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                    (map-set (make-map) "actor" "did:ma:runtime#duckie")
                    "kind" "thing")
                  "name" "duckie")
                "nick" "Duckie")
              "description" "A small duck.")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#room",
                Value::symbol(":drop"),
            )),
        );
        env.define(Rc::from("child_ctx"), child_ctx.clone());

        eval_all(
            r#"((find-method :drop) (list "did:ma:did" "did:ma:runtime#duckie" child_ctx) msg)"#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":drop-thing"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#duckie"),
                Value::str("did:ma:runtime#room"),
                child_ctx,
            ]),
        );
    }

    #[test]
    fn room_children_announcement_registers_dropped_thing() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        let child_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" "did:ma:runtime#room")
                                    "name" "lamp")
                                "nick" "The Lamp")
                            "description" "A brass lamp.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("children_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":child"), child_ctx]),
            )),
        );

        eval_all("(on-message children_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(thing-ref \"The Lamp\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (claim-ctx \"did:ma:runtime#lamp\") \"parent\")",
                &env
            ),
            "did:ma:runtime#room"
        );
        assert_eq!(eval_str("(things-text)", &env), "Things: The Lamp");
    }

    #[test]
    fn room_owner_query_resolves_visible_actor_and_asks_it_to_print_owner() {
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
            (define avatar "did:ma:runtime#avatar")
            (set-label! avatar "Avatar")
            (add-avatar-presence! avatar)
            (set-thing! "Shrugger" "did:ma:runtime#0d2b5070fc6c3412")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#room",
                Value::symbol(":owner?"),
            )),
        );

        eval_all("((find-method :owner?) (list \"Shrugger\") msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#0d2b5070fc6c3412"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":owner?"),
                Value::str("did:ma:runtime#avatar"),
                Value::str("Shrugger"),
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
    fn movable_and_exit_methods_are_categorised() {
        let thing = thing_env();
        install_send_reply_recorders(&thing);
        thing.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#lamp")),
        );
        eval_all(
            r#"
            ((find-method :cmds?) '() msg)
            ((find-method :rpcs?) '() msg)
            ((find-method :metas?) '() msg)
            "#,
            &thing,
        )
        .unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &thing).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::list(vec![
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                ])
            ])
        );
        assert!(eval_bool(
            "(not (method-member? :parent (car (cdr (get-prop \"reply-term:2\")))))",
            &thing,
        ));
        assert!(eval_bool(
            "(method-member? :parent (car (cdr (get-prop \"reply-term:3\"))))",
            &thing,
        ));

        let exit = exit_env();
        install_send_reply_recorders(&exit);
        exit.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#exit")),
        );
        eval_all("((find-method :cmds?) '() msg)", &exit).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &exit).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::list(vec![Value::symbol(":traverse")])
            ])
        );
    }

    #[test]
    fn room_methods_are_categorised() {
        let env = room_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#room")),
        );
        eval_all(
            r#"
            ((find-method :cmds?) '() msg)
            ((find-method :rpcs?) '() msg)
            ((find-method :metas?) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool(
            "(method-member? :look (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :say (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :owner (car (cdr (get-prop \"reply-term:2\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :did? (car (cdr (get-prop \"reply-term:2\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :parent-report (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :child (car (cdr (get-prop \"reply-term:3\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :parent-report (car (cdr (get-prop \"reply-term:3\")))))",
            &env,
        ));
    }

    #[test]
    fn avatar_ack_only_proxies_are_not_public_rpcs() {
        let env = avatar_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#avatar")),
        );

        eval_all(
            r#"
            ((find-method :rpcs?) '() msg)
            ((find-method :cmds?) '() msg)
            ((find-method :metas?) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool(
            "(method-member? :ctx? (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :did? (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :dids? (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :prop (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :owner (car (cdr (get-prop \"reply-term:1\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :owner? (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :did? (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :dids? (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :prop (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :here? (car (cdr (get-prop \"reply-term:2\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :exit-message (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :here? (car (cdr (get-prop \"reply-term:1\")))))",
            &env,
        ));
        assert!(eval_bool("(not (find-method :exit-message))", &env,));
        assert!(eval_bool(
            "(not (method-member? :owner (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :owner? (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :child (car (cdr (get-prop \"reply-term:3\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :enter-room (car (cdr (get-prop \"reply-term:3\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :print (car (cdr (get-prop \"reply-term:3\")))))",
            &env,
        ));
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
    fn room_direct_agent_move_uses_agent_ctx() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#construct".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#rms", "did:ma:runtime#construct")),
        );

        eval_all(
            r#"
            (define rms "did:ma:runtime#rms")
            (set-label! rms "rms")
            (set-claim! rms
              (map-set
                (map-set
                  (map-set
                    (map-set (make-map) "kind" "agent")
                    "name" "Richard Stallman")
                  "nick" "rms")
                "description" "A roaming free software sage."))
            (add-occupant! rms)
            (put-exit! "north" "did:ma:runtime#north-exit")
            (define (ma-entity-exists? actor) #t)
            ((find-method :move) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            eval_all(
                "(list :traverse (movement-ctx \"did:ma:runtime#rms\" #f))",
                &env
            )
            .unwrap(),
        );
    }

    #[test]
    fn avatar_normalises_incoming_command_verb_only() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
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
                did,
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
    fn avatar_here_is_command_that_reports_current_room() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":here?")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("You are in did:ma:runtime#room."),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn avatar_forwards_look_arguments_to_room() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
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
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
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
    fn avatar_owner_query_with_args_is_usage_error() {
        let env = avatar_env();
        install_send_reply_recorders(&env);
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":owner?"), Value::str("Shrugger")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":error"), Value::str("usage: :owner?")]),
        );
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
    }

    #[test]
    fn avatar_mediates_owner_to_room_without_owner_method() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool("(not (find-method :owner))", &env));

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":owner"),
                    Value::str("did:ma:new-owner"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":owner"),
                Value::str("did:ma:new-owner")
            ]),
        );
    }

    #[test]
    fn avatar_owner_query_from_room_with_args_is_usage_error() {
        let env = avatar_env();
        install_send_reply_recorders(&env);
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":owner?"),
                    Value::str("did:ma:runtime#requester"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":error"), Value::str("usage: :owner?")]),
        );
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
    }

    #[test]
    fn avatar_owner_queries_without_args_return_avatar_owner() {
        let env = avatar_env();
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "owner" "did:ma:owner")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:did", "did:ma:runtime#avatar")),
        );

        eval_all(
            r#"
            ((find-method :owner?) '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool("(not (find-method :owner))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("did:ma:owner")])
        );
    }

    #[test]
    fn avatar_inventory_tracks_take_and_drop_tokens() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("lamp")]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();

        env.define(
            Rc::from("inventory_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":i")]),
            )),
        );
        eval_all("(on-message inventory_msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Inventory:\nlamp")
            ]),
        );

        env.define(
            Rc::from("drop_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":drop"), Value::str("lamp")]),
            )),
        );
        eval_all("(on-message drop_msg)", &env).unwrap();
        eval_all("(on-message inventory_msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"sent-term:4\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Inventory:\nlamp")
            ]),
        );
    }

    #[test]
    fn avatar_inventory_waits_for_child_ctx_after_direct_actor_take() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        let avatar_self = format!("did:ma:runtime#{avatar_id}");
        config.insert("self".to_string(), avatar_self.clone());
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":take"),
                    Value::str("did:ma:runtime#satchel"),
                ]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#satchel"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str(avatar_self),
            ]),
        );
        assert_eq!(eval_str("(inventory-text)", &env), "Inventory: empty.");

        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#satchel")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" (local-self))
                  "name" "Lars'ers rygsæk")
                "nick" "Rygsækken")
              "description" "A sturdy satchel.")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("children_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#satchel",
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":child"), child_ctx]),
            )),
        );
        eval_all("(on-message children_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(inventory-text)", &env),
            "Inventory:\nRygsækken = did:ma:runtime#satchel",
        );
    }

    #[test]
    fn avatar_direct_take_uses_avatar_actor_not_controlling_did_as_parent() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());
        config.insert("self".to_string(), did.to_string());
        config.insert("id".to_string(), eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env));
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":take"),
                    Value::str("did:ma:runtime#satchel"),
                ]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();
        let expected_avatar = eval_str(&format!(r#"(avatar-for-did "{did}")"#), &env);

        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str(expected_avatar),
            ]),
        );
    }

    #[test]
    fn container_owner_can_take_orphan_through_local_avatar() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#bag",
                Value::symbol(":take"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "bag")
            (set-prop! "nick" "canvas bag")
            (set-prop! "description" "A sturdy canvas bag.")
            ((find-method :take) (list "did:ma:owner" "did:ma:runtime#avatar") avatar_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(parent)", &env), "");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("take requested")])
        );
    }

    #[test]
    fn agent_owner_can_take_orphan_through_local_avatar() {
        let env = agent_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#duckie".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#duckie",
                Value::symbol(":take"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "duckie")
            (set-prop! "nick" "Duckie")
            (set-prop! "description" "A small duck.")
            ((find-method :take) (list "did:ma:owner" "did:ma:runtime#avatar") avatar_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(parent)", &env), "");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("take requested")])
        );
    }

    #[test]
    fn agent_parent_take_picks_up_carried_child() {
        let env = agent_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#duckie".to_string());
        crate::state::set_config(config);
        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                    (map-set (make-map) "actor" "did:ma:runtime#lamp")
                    "kind" "thing")
                  "name" "lamp")
                "nick" "The Lamp")
              "description" "A brass lamp.")
            "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("child_ctx"), child_ctx.clone());
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#duckie",
                Value::symbol(":take"),
            )),
        );

        eval_all(
            r#"
            (remember-child! child_ctx)
            ((find-method :take) (list "did:ma:owner" "lamp") avatar_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:owner"),
                Value::str("did:ma:runtime#avatar"),
                child_ctx,
            ]),
        );
        assert_eq!(eval_str("(children-text)", &env), "Children: none.");
    }

    #[test]
    fn avatar_take_from_routes_to_explicit_parent() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":take"),
                    Value::str("lamp"),
                    Value::str("from"),
                    Value::str("did:ma:runtime#satchel"),
                ]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#satchel"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str("lamp"),
            ]),
        );
    }

    #[test]
    fn avatar_take_from_resolves_carried_parent_name() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            (remember-child!
              (map-set
                (map-set
                  (map-set
                    (map-set
                      (map-set (make-map) "actor" "did:ma:runtime#duckie")
                      "kind" "agent")
                    "name" "duckie")
                  "nick" "Duckie")
                "description" "A small duck."))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":take"),
                    Value::str("lamp"),
                    Value::str("from"),
                    Value::str("duckie"),
                ]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#duckie"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str("lamp"),
            ]),
        );
    }

        #[test]
        fn avatar_put_in_routes_carried_ctx_to_container() {
                let env = avatar_env();
                let did = "did:ma:did";
                let mut config = std::collections::HashMap::new();
                config.insert("runtime".to_string(), "did:ma:runtime".to_string());
                crate::state::set_config(config.clone());

                let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
                config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
                config.insert("id".to_string(), avatar_id);
                crate::state::set_config(config);

                eval_all(
                        r#"
                        (set-prop! "did" "did:ma:did")
                        (set-prop! "room" "did:ma:runtime#room")
                        "#,
                        &env,
                )
                .unwrap();
                let lamp_ctx = eval_all(
                        r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" (local-self))
                                    "name" "lamp")
                                "nick" "The Lamp")
                            "description" "A warm brass lamp.")
                        "#,
                        &env,
                )
                .unwrap();
                let bag_ctx = eval_all(
                        r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#bag")
                                                "kind" "container")
                                            "protocol" "/ma/container/0.0.1")
                                        "parent" (local-self))
                                    "name" "Lars'ers rygsæk")
                                "nick" "Lars'ers rygsæk")
                            "description" "A sturdy satchel.")
                        "#,
                        &env,
                )
                .unwrap();
                env.define(Rc::from("lamp_ctx"), lamp_ctx.clone());
                env.define(Rc::from("bag_ctx"), bag_ctx);

                eval_all(
                        r#"
                        (remember-child! lamp_ctx)
                        (remember-child! bag_ctx)
                        "#,
                        &env,
                )
                .unwrap();

                env.define(
                        Rc::from("put_msg"),
                        Value::Msg(sample_term_msg(
                                did,
                                "did:ma:runtime#avatar",
                                Value::list(vec![
                                        Value::symbol(":put"),
                                        Value::str("The Lamp"),
                                        Value::str("in"),
                                        Value::str("Lars'ers rygsæk"),
                                ]),
                        )),
                );
                eval_all("(on-message put_msg)", &env).unwrap();

                assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:runtime#lamp");
                assert_eq!(
                        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
                    Value::list(vec![
                        Value::symbol(":drop"),
                        Value::str("did:ma:did"),
                        Value::str("did:ma:runtime#bag"),
                        lamp_ctx.clone(),
                    ])
                );
                assert_eq!(
                        eval_str("(inventory-text)", &env),
                    "Inventory:\nLars'ers rygsæk = did:ma:runtime#bag\nThe Lamp = did:ma:runtime#lamp"
                );
        }

                                #[test]
                                fn avatar_put_in_delegates_visible_container_to_room() {
                                        let env = avatar_env();
                                        let did = "did:ma:did";
                                        let mut config = std::collections::HashMap::new();
                                        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
                                        crate::state::set_config(config.clone());

                                        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
                                        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
                                        config.insert("id".to_string(), avatar_id);
                                        crate::state::set_config(config);

                                        eval_all(
                                                r#"
                                                (set-prop! "did" "did:ma:did")
                                                (set-prop! "room" "did:ma:runtime#room")
                                                "#,
                                                &env,
                                        )
                                        .unwrap();

                                        env.define(
                                                Rc::from("put_msg"),
                                                Value::Msg(sample_term_msg(
                                                        did,
                                                        "did:ma:runtime#avatar",
                                                        Value::list(vec![
                                                                Value::symbol(":put"),
                                                                Value::str("The Golden Lamp"),
                                                                Value::str("in"),
                                                                Value::str("Lars'ers rygsæk"),
                                                        ]),
                                                )),
                                        );
                                        eval_all("(on-message put_msg)", &env).unwrap();

                                        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:runtime#room");
                                        assert_eq!(
                                                eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
                                                Value::list(vec![
                                                        Value::symbol(":put"),
                                                        Value::str("did:ma:did"),
                                                        Value::str("The Golden Lamp"),
                                                        Value::str("in"),
                                                        Value::str("Lars'ers rygsæk"),
                                                ])
                                        );
                                }

                                #[test]
                                fn room_put_visible_thing_delegates_transfer_to_avatar() {
                                        let env = room_env();
                                        let mut config = std::collections::HashMap::new();
                                        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
                                        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
                                        crate::state::set_config(config);
                                        install_send_reply_recorders(&env);

                                        let lamp_ctx = eval_all(
                                                r#"
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set
                                                                    (map-set
                                                                        (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                                        "kind" "thing")
                                                                    "protocol" "/ma/thing/0.0.1")
                                                                "parent" "did:ma:runtime#room")
                                                            "name" "lamp")
                                                        "nick" "The Golden Lamp")
                                                    "description" "A warm golden lamp.")
                                                "#,
                                                &env,
                                        )
                                        .unwrap();
                                        let bag_ctx = eval_all(
                                                r#"
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set
                                                                    (map-set
                                                                        (map-set (make-map) "actor" "did:ma:runtime#bag")
                                                                        "kind" "container")
                                                                    "protocol" "/ma/container/0.0.1")
                                                                "parent" "did:ma:runtime#room")
                                                            "name" "Lars'ers rygsæk")
                                                        "nick" "Lars'ers rygsæk")
                                                    "description" "A sturdy satchel.")
                                                "#,
                                                &env,
                                        )
                                        .unwrap();
                                        env.define(Rc::from("lamp_ctx"), lamp_ctx.clone());
                                        env.define(Rc::from("bag_ctx"), bag_ctx);
                                        env.define(
                                                Rc::from("put_msg"),
                                                Value::Msg(sample_term_msg(
                                                        "did:ma:runtime#avatar",
                                                        "did:ma:runtime#room",
                                                        Value::symbol(":put"),
                                                )),
                                        );

                                        eval_all(
                                                r#"
                                                (set-thing! "The Golden Lamp" "did:ma:runtime#lamp")
                                                (set-claim! "did:ma:runtime#lamp" lamp_ctx)
                                                (set-thing! "Lars'ers rygsæk" "did:ma:runtime#bag")
                                                (set-claim! "did:ma:runtime#bag" bag_ctx)
                                                ((find-method :put) (list "did:ma:did" "The Golden Lamp" "in" "Lars'ers rygsæk") put_msg)
                                                "#,
                                                &env,
                                        )
                                        .unwrap();

                                        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:runtime#lamp");
                                        assert_eq!(
                                                eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
                                                Value::list(vec![
                                                        Value::symbol(":drop"),
                                                        Value::str("did:ma:did"),
                                                        Value::str("did:ma:runtime#bag"),
                                                        lamp_ctx,
                                                ])
                                        );
                                        assert_eq!(eval_str("(things-text)", &env), "Things: Lars'ers rygsæk, The Golden Lamp");
                                }

                #[test]
                fn room_put_reports_not_visible_for_missing_tokens() {
                    let env = room_env();
                    let mut config = std::collections::HashMap::new();
                    config.insert("runtime".to_string(), "did:ma:runtime".to_string());
                    config.insert("self".to_string(), "did:ma:runtime#room".to_string());
                    crate::state::set_config(config);
                    install_send_reply_recorders(&env);
                    env.define(
                        Rc::from("put_msg"),
                        Value::Msg(sample_term_msg(
                            "did:ma:runtime#avatar",
                            "did:ma:runtime#room",
                            Value::symbol(":put"),
                        )),
                    );

                    eval_all(
                        r#"
                        ((find-method :put) (list "did:ma:did" "The Golden Lamp" "in" "Lars'ers rygsæk") put_msg)
                        "#,
                        &env,
                    )
                    .unwrap();

                    assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:runtime#avatar");
                    assert_eq!(
                        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
                        Value::list(vec![
                            Value::symbol(":print"),
                            Value::str("You cannot see The Golden Lamp."),
                        ])
                    );
                }

                #[test]
                fn avatar_put_thing_transfers_visible_item_into_container() {
                    let env = avatar_env();
                    let did = "did:ma:did";
                    let mut config = std::collections::HashMap::new();
                    config.insert("runtime".to_string(), "did:ma:runtime".to_string());
                    crate::state::set_config(config.clone());

                    let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
                    config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
                    config.insert("id".to_string(), avatar_id);
                    crate::state::set_config(config);

                    eval_all(
                        r#"
                        (set-prop! "did" "did:ma:did")
                        (set-prop! "room" "did:ma:runtime#room")
                        "#,
                        &env,
                    )
                    .unwrap();
                    let lamp_ctx = eval_all(
                        r#"
                        (map-set
                          (map-set
                            (map-set
                              (map-set
                                                                (map-set
                                                                    (map-set
                                                                        (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                                        "kind" "thing")
                                                                    "protocol" "/ma/thing/0.0.1")
                                                                "parent" (local-self))
                              "name" "lamp")
                            "nick" "The Golden Lamp")
                          "description" "A warm golden lamp.")
                        "#,
                        &env,
                    )
                    .unwrap();

                    env.define(
                        Rc::from("put_thing_msg"),
                        Value::Msg(sample_term_msg(
                            "did:ma:runtime#room",
                            "did:ma:runtime#avatar",
                            Value::list(vec![
                                Value::symbol(":put-thing"),
                                Value::str("did:ma:did"),
                                Value::str("did:ma:runtime#lamp"),
                                Value::str("did:ma:runtime#bag"),
                                lamp_ctx.clone(),
                            ]),
                        )),
                    );
                    eval_all("(on-message put_thing_msg)", &env).unwrap();

                    assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:runtime#lamp");
                    assert_eq!(
                        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
                        Value::list(vec![
                            Value::symbol(":drop"),
                            Value::str("did:ma:did"),
                            Value::str("did:ma:runtime#bag"),
                            lamp_ctx,
                        ])
                    );
                }

    #[test]
    fn thing_announces_children_ctx_to_parent_on_start() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#avatar")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            (on-signal :start)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":child"),
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"kind\")",
                &env
            ),
            "thing"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"nick\")",
                &env
            ),
            "brass lamp"
        );
    }

    #[test]
    fn thing_owner_can_drop_orphan_into_room() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#lamp",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :drop) (list "did:ma:runtime#room") owner_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(parent)", &env), "");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str("(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn container_lock_blocks_contents_and_preserves_message() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#bag")),
        );

        eval_all(
            r#"
            ((find-method :lock) (list "The" "bag" "is" "tied" "shut.") owner_msg)
            ((find-method :contents?) '() owner_msg)
            ((find-method :unlock) '() owner_msg)
            ((find-method :lock) '() owner_msg)
            ((find-method :contents?) '() owner_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(get-prop \"owner\")", &env), "did:ma:owner");
        assert_eq!(
            eval_str("(get-prop \"locked-message\")", &env),
            "The bag is tied shut."
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 5);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("locked")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("The bag is tied shut.")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("unlocked")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:5\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("The bag is tied shut.")
            ])
        );
    }

    #[test]
    fn container_put_in_stores_ctx_and_take_from_returns_ctx() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);
        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                    (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" "did:ma:runtime#bag")
                  "name" "lamp")
                "nick" "brass lamp")
              "description" "A warm brass lamp.")
            "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("child_ctx"), child_ctx.clone());
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_msg("did:ma:runtime#lamp", "did:ma:runtime#bag")),
        );

        eval_all(
            r#"
            ((find-method :put-in) (list child_ctx) parent_msg)
            ((find-method :contents?) '() parent_msg)
            ((find-method :take-from) (list "brass lamp") parent_msg)
            ((find-method :contents?) '() parent_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 4);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("put in")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("Contents:\nbrass lamp = did:ma:runtime#lamp")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), child_ctx])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:4\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("Contents: none.")])
        );
    }

    #[test]
    fn container_take_still_moves_the_whole_container() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_msg("did:ma:runtime#room", "did:ma:runtime#bag")),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "bag")
            (set-prop! "nick" "canvas bag")
            (set-prop! "description" "A sturdy canvas bag.")
            ((find-method :take) (list "did:ma:owner" "did:ma:runtime#avatar") parent_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(parent)", &env), "did:ma:runtime#room");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"kind\")",
                &env
            ),
            "container"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("take requested")])
        );
    }

    #[test]
    fn container_owner_can_recover_orphan_parent_once() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "bag")
            (set-prop! "nick" "canvas bag")
            (set-prop! "description" "A sturdy canvas bag.")
            "#,
            &env,
        )
        .unwrap();
        let parent_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#bag")
                                                "kind" "container")
                                            "protocol" "/ma/container/0.0.1")
                                        "parent" "did:ma:runtime#room")
                  "name" "bag")
                "nick" "canvas bag")
              "description" "A sturdy canvas bag.")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#bag",
                Value::list(vec![Value::symbol(":parent"), parent_ctx]),
            )),
        );

        eval_all("(on-message owner_msg)", &env).unwrap();

        assert_eq!(eval_str("(parent)", &env), "did:ma:runtime#room");
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok"),
        );
    }

    #[test]
    fn container_owner_can_drop_orphan_into_room() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#bag",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "bag")
            (set-prop! "nick" "canvas bag")
            (set-prop! "description" "A sturdy canvas bag.")
            ((find-method :drop) (list "did:ma:runtime#room") owner_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(parent)", &env), "");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str("(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")", &env),
            "did:ma:runtime#bag"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn agent_announces_children_ctx_to_parent_on_start() {
        let env = agent_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#rms".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#avatar")
            (set-prop! "name" "Richard Stallman")
            (set-prop! "nick" "rms")
            (set-prop! "description" "A roaming free software sage.")
            (on-signal :start)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":child"),
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#rms"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"kind\")",
                &env
            ),
            "agent"
        );
    }

    #[test]
    fn avatar_children_registration_updates_inventory_and_rejects_forgery() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(r#"(set-prop! "did" "did:ma:did")"#, &env).unwrap();
        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                                                "parent" (local-self))
                  "name" "lamp")
                "nick" "brass lamp")
              "description" "A warm brass lamp.")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("adopt_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":child"), child_ctx.clone()]),
            )),
        );
        eval_all("(on-message adopt_msg)", &env).unwrap();

        env.define(
            Rc::from("inventory_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":i")]),
            )),
        );
        eval_all("(on-message inventory_msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Inventory:\nbrass lamp = did:ma:runtime#lamp"),
            ]),
        );

        env.define(
            Rc::from("forged_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#forger",
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":child"), child_ctx]),
            )),
        );
        eval_all("(on-message forged_msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("child ctx actor must match sender"),
            ]),
        );
    }

    #[test]
    fn avatar_drop_resolves_registered_inventory_ctx() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();
        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#duckie")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                                                "parent" (local-self))
                  "name" "duckie")
                "nick" "Duckie")
              "description" "A small duck.")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("Duckie")]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();

        env.define(
            Rc::from("adopt_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#duckie",
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":child"), child_ctx.clone()]),
            )),
        );
        eval_all("(on-message adopt_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(inventory-text)", &env),
            "Inventory:\nDuckie = did:ma:runtime#duckie",
        );

        env.define(
            Rc::from("drop_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":drop"),
                    Value::str("did:ma:runtime#duckie"),
                ]),
            )),
        );
        eval_all("(on-message drop_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:3\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:3\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":drop"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#duckie"),
                child_ctx,
            ]),
        );

        env.define(
            Rc::from("inventory_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":i")]),
            )),
        );
        eval_all("(on-message inventory_msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"sent-term:4\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Inventory:\nDuckie = did:ma:runtime#duckie")
            ]),
        );
    }

    #[test]
    fn avatar_drop_thing_accepts_current_room_call() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        config.insert("self".to_string(), format!("did:ma:runtime#{avatar_id}"));
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (set-prop! "room" "did:ma:runtime#room")
            "#,
            &env,
        )
        .unwrap();
        let child_ctx = eval_all(
            r#"
            (map-set
              (map-set
                (map-set
                  (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#duckie")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" "did:ma:runtime#avatar")
                                    "name" "duckie")
                "nick" "Duckie")
              "description" "A small duck.")
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("drop_thing_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":drop-thing"),
                    Value::str("did:ma:did"),
                    Value::str("did:ma:runtime#duckie"),
                    Value::str("did:ma:runtime#room"),
                    child_ctx.clone(),
                ]),
            )),
        );
        eval_all("(on-message drop_thing_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#duckie"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":drop"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#room"),
                child_ctx,
            ]),
        );
    }

    #[test]
    fn thing_accepts_parent_ctx_from_target_parent_and_notifies_old_parent() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
                        (set-prop! "parent" "did:ma:runtime#father")
                        (set-prop! "owner" "did:ma:owner")
                        "#,
            &env,
        )
        .unwrap();
        let parent_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                    "parent" "did:ma:runtime#mother")
                                "name" "lamp")
                                "nick" "new lamp")
                            "description" "Freshly transferred.")
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#mother",
                "did:ma:runtime#lamp",
                Value::list(vec![Value::symbol(":parent"), parent_ctx]),
            )),
        );
        eval_all("(on-message parent_msg)", &env).unwrap();

        assert_eq!(eval_str("(parent)", &env), "did:ma:runtime#mother");
        assert_eq!(eval_str("(nick)", &env), "new lamp");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#father"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":child"),
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#mother"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok"),
        );
    }

    #[test]
    fn agent_rejects_parent_ctx_from_non_parent() {
        let env = agent_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#rms".to_string());
        crate::state::set_config(config);

        eval_all(r#"(set-prop! "parent" "did:ma:runtime#father")"#, &env).unwrap();
        let parent_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#rms")
                                                "kind" "agent")
                                            "protocol" "/ma/scheme/agent/0.0.1")
                                        "parent" "did:ma:runtime#mother")
                                    "name" "Richard Stallman")
                                "nick" "rms")
                            "description" "A roaming free software sage.")
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("forged_parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#stranger",
                "did:ma:runtime#rms",
                Value::list(vec![Value::symbol(":parent"), parent_ctx]),
            )),
        );
        eval_all("(on-message forged_parent_msg)", &env).unwrap();

        assert_eq!(eval_str("(parent)", &env), "did:ma:runtime#father");
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("parent ctx must come from target parent"),
            ]),
        );
    }

    #[test]
    fn agent_accepts_parent_ctx_from_target_parent_and_notifies_old_parent() {
        let env = agent_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#rms".to_string());
        crate::state::set_config(config);

        eval_all(r#"(set-prop! "parent" "did:ma:runtime#father")"#, &env).unwrap();
        let parent_ctx = eval_all(
            r#"
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set
                                                                    (map-set
                                                                        (map-set (make-map) "actor" "did:ma:runtime#rms")
                                                                        "kind" "agent")
                                                                    "protocol" "/ma/scheme/agent/0.0.1")
                                                                "parent" "did:ma:runtime#mother")
                                                            "name" "Richard Stallman")
                                                        "nick" "rms-on-tour")
                                                    "description" "Travelling under a new parent.")
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#mother",
                "did:ma:runtime#rms",
                Value::list(vec![Value::symbol(":parent"), parent_ctx]),
            )),
        );
        eval_all("(on-message parent_msg)", &env).unwrap();

        assert_eq!(eval_str("(parent)", &env), "did:ma:runtime#mother");
        assert_eq!(eval_str("(nick)", &env), "rms-on-tour");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#father"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":child"),
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok"),
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
            eval_str("(effective-did (list \"did:ma:did\" \"north\") msg)", &env),
            "did:ma:did"
        );
        assert_eq!(
            eval_str(
                "(car (effective-args (list \"did:ma:did\" \"north\") msg))",
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
            eval_str("(get-prop \"sent-target:3\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:3\")", &env).unwrap(),
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
            (traverse-exit! "did:ma:runtime#avatar" "did:ma:did" "north" "did:ma:runtime#north-exit")
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
            "did:ma:did"
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
    fn room_go_from_direct_did_moves_the_did_avatar() {
        let env = room_env();
        let did = "did:ma:did";
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
                did,
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":go"), Value::str("north")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        let expected_avatar = eval_str(&format!(r#"(avatar-for-did "{did}")"#), &env);
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
            did
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
                    Value::str("did:ma:did"),
                    Value::str("cloud"),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert!(eval_bool("(equal? (get-prop \"reply-term:1\") :ok)", &env));
    }

    #[test]
    fn room_go_no_exit_acknowledges_direct_did_call() {
        let env = room_env();
        let did = "did:ma:did";
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
                did,
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":go"), Value::str("cloud")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        let expected_avatar = eval_str(&format!(r#"(avatar-for-did "{did}")"#), &env);
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            expected_avatar
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
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
            Value::symbol(":ok")
        );
    }

    #[test]
    fn avatar_ctx_forwards_movement_ctx_to_principal_did() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        let avatar = format!("did:ma:runtime#{avatar_id}");
        config.insert("self".to_string(), avatar.clone());
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
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
                                                                                        (map-set (make-map) "actor" "did:ma:did")
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
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), did);
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("You pass through the oak door.")
            ]),
        );
        assert_eq!(eval_str("(get-prop \"sent-target:2\")", &env), did);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"actor\")",
                &env
            ),
            did
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
    fn avatar_ctx_forwards_cross_runtime_movement_ctx_to_principal_did() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:sky".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        let avatar = format!("did:ma:sky#{avatar_id}");
        config.insert("self".to_string(), avatar.clone());
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
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
                                                         (map-set (make-map) "actor" "did:ma:did")
                                                         "avatar" (local-self))
                                                     "kind" "avatar")
                                                 "room" "did:ma:ma#cloud")))
                            ((find-method :ctx) (list ctx) msg))
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), did);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            did
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
                                            (map-set
                                                (map-set (make-map) "actor" (self))
                                                "kind" "agent")
                                            "room" "did:ma:runtime#kitchen")
                                      "text" "You go north."))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#kitchen"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :enter)",
            &env
        ));
        assert_eq!(
            eval_str("(get-prop \"last-message\")", &env),
            "You go north."
        );
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
            (traverse-exit! "did:ma:runtime#avatar" "did:ma:did" "north" "did:ma:runtime#dead-exit")
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
                "(list :traverse (movement-ctx \"did:ma:runtime#avatar\" \"did:ma:did\"))",
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

        let owner_avatar = eval_str(r#"(avatar-for-did "did:ma:did")"#, &env);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (define owner-avatar (avatar-for-did "did:ma:did"))
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
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), owner_avatar);
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
