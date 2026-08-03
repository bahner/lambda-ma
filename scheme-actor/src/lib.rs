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
        eval_all(include_str!("../node.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/room.ma"), &env).unwrap();
        eval_all(
                        r#"
                        (define (ma-send! target term) #f)
                        (define (ma-reply! msg term) #f)
                        (define (ma-save-state!) #f)
                        (define (ma-end) (set-prop! "ended" "yes"))
                        (define (test-actor-claim kind protocol actor nick description)
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" actor)
                                                    "kind" kind)
                                                "protocol" protocol)
                                            "parent" (canonical-actor (self)))
                                        "name" nick)
                                    "nick" nick)
                                "description" description))
                        (define (test-avatar-claim! actor nick)
                            (set-claim! actor (test-actor-claim "avatar" "/ma/avatar/0.0.1" actor nick "An avatar.")))
                        (define (test-avatar-claim-with-did! actor did nick)
                            (set-claim! actor
                                (map-set
                                    (test-actor-claim "avatar" "/ma/avatar/0.0.1" actor nick "An avatar.")
                                    "did" did)))
                        (define (test-agent-claim! actor nick)
                            (set-claim! actor (test-actor-claim "agent" "/ma/agent/0.0.1" actor nick "A test agent.")))
                        "#,
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
        eval_all(include_str!("../node.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/agent.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f) (define (ma-end) (set-prop! \"ended\" \"yes\"))",
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
        eval_all(include_str!("../node.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/thing.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f) (define (ma-end) (set-prop! \"ended\" \"yes\"))",
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
        eval_all(include_str!("../node.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/container.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f) (define (ma-end) (set-prop! \"ended\" \"yes\"))",
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
        eval_all(include_str!("../node.ma"), &env).unwrap();
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
                        (define (ma-entity-exists? actor) #f)
                        (define (ma-create-actor kind behaviour init fragment) fragment)
                        (define (ma-save-state!) #f)
                        "#,
                        &env,
                )
                .unwrap();
        env
    }

    fn root_actor_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../node.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/root.ma"), &env).unwrap();
        eval_all(
            r#"
            (define (ma-send! target term)
              (set-prop! "sent-target" target)
              (set-prop! "sent-term" term))
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
        eval_all(include_str!("../node.ma"), &env).unwrap();
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

    fn node_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all(include_str!("../node.ma"), &env).unwrap();
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
    fn reverse_reverses_proper_lists() {
        let env = new_root_env();

        assert_eq!(
            eval_all("(reverse (list 1 2 3))", &env).unwrap(),
            Value::list(vec![Value::Int(3), Value::Int(2), Value::Int(1)])
        );
        assert_eq!(eval_all("(reverse '())", &env).unwrap(), Value::Nil);
        assert!(eval_all("(reverse (cons 1 2))", &env).is_err());
        assert!(eval_all("(reverse! (list 1 2 3))", &env).is_err());
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
    fn actor_layer_does_not_expose_tree_methods() {
        let env = actor_env();

        assert!(eval_bool("(not (find-method :parent))", &env));
        assert!(eval_bool("(not (find-method :parent?))", &env));
        assert!(eval_bool("(not (find-method :child))", &env));
    }

    #[test]
    fn node_parent_admits_non_room_child_and_confirms_ctx() {
        let env = node_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#dog".to_string());
        config.insert("kind".to_string(), "/ma/agent/0.0.1".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        let collar_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#collar")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" "did:ma:runtime#dog")
                                    "name" "collar")
                                "nick" "Halsbånd")
                            "description" "Et rødt halsbånd.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("collar_ctx"), collar_ctx.clone());
        env.define(
            Rc::from("collar_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#collar",
                "did:ma:runtime#dog",
                Value::list(vec![Value::symbol(":parent"), collar_ctx.clone()]),
            )),
        );

        eval_all("(on-message collar_msg)", &env).unwrap();

        assert!(eval_bool(
            "(map? (child-ctx \"did:ma:runtime#collar\"))",
            &env
        ));
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#collar"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":child"), collar_ctx])
        );
    }

    #[test]
    fn node_child_commits_parent_and_notifies_both_parents() {
        let env = node_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#collar".to_string());
        config.insert("kind".to_string(), "/ma/thing/0.0.1".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        let committed_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#collar")
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "parent" "did:ma:runtime#dog")
                                    "name" "collar")
                                "nick" "Halsbånd")
                            "description" "Et rødt halsbånd.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#dog",
                "did:ma:runtime#collar",
                Value::list(vec![Value::symbol(":child"), committed_ctx]),
            )),
        );

        eval_all(
            r#"
                        (set-prop! "parent" "did:ma:runtime#inventory")
                        (set-prop! "name" "collar")
                        (set-prop! "nick" "Halsbånd")
                        (set-prop! "description" "Et rødt halsbånd.")
                        (on-message parent_msg)
                        "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#dog");
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#dog"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#inventory"
        );

        let first_sent_count = eval_int("(get-prop \"sent-count\")", &env);
        let first_reply_count = eval_int("(get-prop \"reply-count\")", &env);
        let authoritative_ctx = eval_all("(node-ctx)", &env).unwrap();
        env.define(
            Rc::from("authoritative_parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#dog",
                "did:ma:runtime#collar",
                Value::list(vec![Value::symbol(":child"), authoritative_ctx]),
            )),
        );

        eval_all("(on-message authoritative_parent_msg)", &env).unwrap();

        assert_eq!(
            eval_int("(get-prop \"sent-count\")", &env),
            first_sent_count
        );
        assert_eq!(
            eval_int("(get-prop \"reply-count\")", &env),
            first_reply_count + 1
        );
    }

    #[test]
    fn node_children_debug_returns_full_map_to_owner_and_filters_by_kind() {
        let env = node_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#room")),
        );
        env.define(
            Rc::from("other_msg"),
            Value::Msg(sample_msg("did:ma:other", "did:ma:runtime#room")),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (define avatar-ctx
              (make-map "actor" "did:ma:runtime#avatar"
                        "kind" "avatar"
                        "protocol" "/ma/avatar/0.0.1"
                        "parent" "did:ma:runtime#room"
                        "name" "Alice"
                        "nick" "Alice"
                        "description" "An avatar."))
            (define thing-ctx
              (make-map "actor" "did:ma:runtime#lamp"
                        "kind" "thing"
                        "protocol" "/ma/thing/0.0.1"
                        "parent" "did:ma:runtime#room"
                        "name" "lamp"
                        "nick" "Lamp"
                        "description" "A lamp."))
            (remember-child! avatar-ctx)
            (remember-child! thing-ctx)
            ((find-method :children?) '() owner_msg)
            ((find-method :children?) '() other_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(list-length (map-keys (children-map)))", &env), 2);
        assert_eq!(
            eval_int("(list-length (child-ctxs-by-kind \"avatar\"))", &env),
            1
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            eval_all("(list :ok (children-map))", &env).unwrap()
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner may inspect children"),
            ])
        );
        assert!(eval_bool("(not (has-prop? \"claims\"))", &env));
        assert!(eval_bool("(not (has-prop? \"things\"))", &env));
    }

    #[test]
    fn node_room_parent_is_locked_to_root_and_root_accepts_rooms() {
        let room = node_env();
        let mut room_config = std::collections::HashMap::new();
        room_config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        room_config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        room_config.insert("kind".to_string(), "/ma/room/0.0.1".to_string());
        crate::state::set_config(room_config);

        assert!(eval_bool(
            "(node-parent-admissible? \"did:ma:runtime#root\")",
            &room
        ));
        assert!(eval_bool(
            "(not (node-parent-admissible? \"did:ma:runtime#bag\"))",
            &room
        ));
        assert!(eval_bool(
            "(not (node-parent-admissible? \"did:ma:runtime#room\"))",
            &room
        ));

        let root = node_env();
        let mut root_config = std::collections::HashMap::new();
        root_config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        root_config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        root_config.insert("kind".to_string(), "/ma/root/0.0.1".to_string());
        crate::state::set_config(root_config);
        install_send_reply_recorders(&root);
        let room_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#room")
                                                "kind" "room")
                                            "protocol" "/ma/room/0.0.1")
                                        "parent" "did:ma:runtime#root")
                                    "name" "room")
                                "nick" "Room")
                            "description" "A room.")
                        "#,
            &root,
        )
        .unwrap();
        root.define(Rc::from("room_ctx"), room_ctx);
        root.define(
            Rc::from("room_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#root",
                eval_all("(list :parent room_ctx)", &root).unwrap(),
            )),
        );

        eval_all("(on-message room_msg)", &root).unwrap();

        assert!(eval_bool(
            "(map? (child-ctx \"did:ma:runtime#room\"))",
            &root
        ));
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &root),
            "did:ma:runtime#room"
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
                    Value::symbol(":move"),
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                    Value::symbol(":recycle"),
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
                    Value::symbol(":move"),
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                    Value::symbol(":recycle"),
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
                    Value::symbol(":move"),
                    Value::symbol(":claim"),
                    Value::symbol(":take"),
                    Value::symbol(":drop"),
                    Value::symbol(":recycle"),
                    Value::symbol(":fortune"),
                ])
            ])
        );
    }

    #[test]
    fn room_presence_uses_labels_and_keeps_who_avatar_only() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
        assert_eq!(eval_str("(who-text)", &env), "Who: none.");

        eval_all(
            r#"
                        (define avatar "did:ma:runtime#avatar1")
                        (set-label! avatar "Alice")
                        (set-claim! avatar
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" avatar)
                                                    "kind" "avatar")
                                                "protocol" "/ma/avatar/0.0.1")
                                            "parent" "did:ma:runtime#room")
                                        "name" "Alice")
                                    "nick" "Alice")
                                "description" "An avatar."))
                        (define rms "did:ma:runtime#rms")
                        (set-label! rms "rms")
                        (set-claim! rms
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" rms)
                                                    "kind" "agent")
                                                "protocol" "/ma/agent/0.0.1")
                                            "parent" "did:ma:runtime#room")
                                        "name" "Richard Stallman")
                                    "nick" "rms")
                                "description" "A roaming free software sage."))
                        (define lamp "did:ma:runtime#lamp")
                        (set-claim! lamp
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" lamp)
                                                    "kind" "thing")
                                                "protocol" "/ma/thing/0.0.1")
                                            "parent" "did:ma:runtime#room")
                                        "name" "lamp")
                                    "nick" "Lamp")
                                "description" "A brass lamp."))
                        "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(who-text)", &env), "Who: Alice");
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: Alice, rms");
        assert_eq!(eval_str("(things-text)", &env), "Things: Lamp");
        assert!(eval_bool("(movable-occupant? rms)", &env));
        assert!(!eval_bool("(movable-occupant? avatar)", &env));
        assert_eq!(
            eval_str("(movable-ref \"rms\")", &env),
            "did:ma:runtime#rms"
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
            (test-avatar-claim! bob "Bob")
                        (commit-avatar-entry! alice "did:ma:alice" #f "Alice" #f)
            (define (ctx-term-value term key)
              (let loop ((pairs (car (cdr term))))
                (cond ((null? pairs) #f)
                      ((equal? (car (car pairs)) key) (car (cdr (car pairs))))
                      (else (loop (cdr pairs))))))
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 4);
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
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:3\"))", &env).unwrap(),
            Value::symbol(":ctx")
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:4\"))", &env).unwrap(),
            Value::symbol(":ctx")
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

        let source_avatar = eval_str(
            r#"(avatar-for-did-in-runtime "did:ma:sky" "did:ma:did")"#,
            &env,
        );

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(&source_avatar, "did:ma:ma#cloud")),
        );
        env.define(Rc::from("source_avatar"), Value::str(&source_avatar));
        eval_all(
            r#"
                        ((find-method :enter)
                            (list (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set (make-map) "actor" "did:ma:did")
                                                        "avatar" source_avatar)
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
    fn room_direct_did_enter_requests_local_avatar_without_reply() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:ma".to_string());
        config.insert("self".to_string(), "did:ma:ma#cloud".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-entity-exists? actor) #f)
            (define (ma-create-actor kind behaviour init fragment)
              (set-prop! "created-kind" kind)
              (set-prop! "created-fragment" fragment)
              (entity-url fragment))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(did, "did:ma:ma#cloud")),
        );
        eval_all(
            r#"
            ((find-method :enter)
                            (list "Pondus")
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"created-kind\")", &env),
            "/ma/avatar/0.0.1"
        );
        assert_eq!(
            eval_str("(get-prop \"created-fragment\")", &env),
            eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env)
        );
        assert!(eval_bool("(not (has-prop? \"reply-count\"))", &env));
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
    }

    #[test]
    fn room_rejects_avatar_ctx_from_wrong_avatar() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:ma".to_string());
        config.insert("self".to_string(), "did:ma:ma#cloud".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
                        (define (ma-entity-exists? actor) #f)
                        (define (ma-create-actor kind init behaviour fragment)
                            (set-prop! "created-kind" kind)
                            (entity-url fragment))
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:sky#forged-avatar", "did:ma:ma#cloud")),
        );
        eval_all(
            r#"
                        ((find-method :enter)
                            (list
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:did")
                                                "avatar" "did:ma:sky#forged-avatar")
                                            "kind" "avatar")
                                        "room" "did:ma:ma#cloud")
                                    "nick" "Lars"))
                            msg)
                        "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool("(not (has-prop? \"created-kind\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("avatar enter ctx must come from the deterministic avatar for its DID"),
            ])
        );
    }

    #[test]
    fn room_look_includes_exits() {
        let env = room_env();

        assert!(eval_str("(room-text)", &env).ends_with("\nExits: none."));

        eval_all("(put-exit! \"north\" \"did:ma:runtime#north-exit\")", &env).unwrap();

        assert!(eval_str("(room-text)", &env).ends_with("\nExits: north"));
    }

    #[test]
    fn room_ctx_contains_protocol_rev_root_parent_and_visible_entries() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

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
                                "nick" "The Lamp")
                            "description" "A brass lamp.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("lamp_ctx"), lamp_ctx);

        eval_all(
            r#"
                        (set-prop! "ctx:rev" 7)
                        (set-thing! "The Lamp" "did:ma:runtime#lamp")
                        (set-claim! "did:ma:runtime#lamp" lamp_ctx)
                        (put-exit! "north" "did:ma:runtime#north-exit")
                        "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(ctx-text (room-ctx) \"protocol\")", &env),
            "/ma/room/0.0.1"
        );
        assert_eq!(
            eval_str("(ctx-text (room-ctx) \"parent\")", &env),
            "did:ma:runtime#root"
        );
        assert_eq!(eval_int("(map-ref (room-ctx) \"rev\" #f)", &env), 7);
        assert_eq!(
            eval_str(
                "(ctx-text (car (map-ref (room-ctx) \"things\" '())) \"actor\")",
                &env
            ),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (map-ref (room-ctx) \"exits\" '())) \"direction\")",
                &env
            ),
            "north"
        );
    }

    #[test]
    fn room_thing_alias_change_broadcasts_room_ctx_to_avatars() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (test-avatar-claim! "did:ma:runtime#avatar" "Avatar")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::symbol(":thing"),
            )),
        );

        eval_all(
            "((find-method :thing) (list \"Aladdins lampe\" \"did:ma:runtime#lamp\") msg)",
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(things-text)", &env), "Things: Aladdins lampe");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str("(ctx-text (car (map-ref (car (cdr (get-prop \"sent-term:1\"))) \"things\" '())) \"actor\")", &env),
            "did:ma:runtime#lamp"
        );
    }

    #[test]
    fn room_look_refreshes_callers_room_ctx_snapshot() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-prop! "ctx:rev" 4)
            (set-thing! "Aladdins lampe" "did:ma:runtime#lamp")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#avatar", "did:ma:runtime#room")),
        );

        eval_all("((find-method :look) '() msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_int(
                "(map-ref (car (cdr (get-prop \"sent-term:1\"))) \"rev\" 0)",
                &env
            ),
            5
        );
        assert_eq!(
            eval_str("(ctx-text (car (map-ref (car (cdr (get-prop \"sent-term:1\"))) \"things\" '())) \"nick\")", &env),
            "Aladdins lampe"
        );
    }

    #[test]
    fn room_look_with_argument_does_not_dispatch_to_visible_target() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#avatar", "did:ma:runtime#room")),
        );

        eval_all(
            r#"
            (put-exit! "north" "did:ma:runtime#north-exit")
            ((find-method :look) (list "north") msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Use your avatar to inspect visible things.")
            ])
        );
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
    fn room_owner_getter_prints_to_mediating_avatar() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (define owner-did "did:ma:owner")
            (define owner-avatar (avatar-for-did owner-did))
            (set-prop! "owner" owner-did)
            (test-avatar-claim-with-did! owner-avatar owner-did "Pondus")
            "#,
            &env,
        )
        .unwrap();
        let owner_avatar = eval_str("owner-avatar", &env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(&owner_avatar, "did:ma:runtime#room")),
        );

        eval_all("((find-method :owner) '() msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), owner_avatar);
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":print"), Value::str("did:ma:owner")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("did:ma:owner")])
        );
    }

    #[test]
    fn room_claim_from_avatar_ctx_uses_controlling_did() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (define owner-did "did:ma:owner")
            (define owner-avatar (avatar-for-did owner-did))
            (test-avatar-claim-with-did! owner-avatar owner-did "Pondus")
            "#,
            &env,
        )
        .unwrap();
        let owner_avatar = eval_str("owner-avatar", &env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(&owner_avatar, "did:ma:runtime#room")),
        );

        eval_all("((find-method :claim) '() msg)", &env).unwrap();

        assert_eq!(eval_str("(owner)", &env), "did:ma:owner");
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
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
            (test-agent-claim! "did:ma:runtime#rms" "rms")
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
            (test-avatar-claim! avatar "Alice")
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
        assert!(eval_bool(
            "(not (has-prop? \"label:did:ma:runtime#avatar\"))",
            &env
        ));

        eval_all("(test-avatar-claim! avatar \"Alice\")", &env).unwrap();

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
            (test-avatar-claim! donald-1 "Donald Duck")
            (test-avatar-claim! donald-2 "Donald Duck")
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
            (test-avatar-claim! avatar "Alice")
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
            (test-avatar-claim! donald-1 "Donald Duck")
            (test-avatar-claim! donald-2 "Donald Duck")
            (set-thing! "lamp" "did:ma:runtime#lamp")
            (put-exit! "down" "did:ma:runtime#down-exit")
            (define cloud-avatar "did:ma:runtime#cloud-avatar")
            (set-label! cloud-avatar "cloud")
            (test-avatar-claim! cloud-avatar "cloud")
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
                    "Ambiguous name: Donald Duck\noccupant Donald Duck = did:ma:runtime#donald1\noccupant Donald Duck = did:ma:runtime#donald2",
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
    fn room_did_lookup_prints_when_delegated_by_avatar() {
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
                        (test-avatar-claim! avatar "Avatar")
            (define duckie "did:ma:runtime#duckie")
            (set-label! duckie "Duckie")
            (test-avatar-claim! duckie "Duckie")
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

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
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
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("Duckie = did:ma:runtime#duckie"),
            ])
        );
    }

    #[test]
    fn room_broadcast_uses_avatar_presence_not_stale_occupants() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (define stale "did:ma:runtime#stale-avatar")
            (define present "did:ma:runtime#present-avatar")
            (define (ma-entity-exists? actor) #t)
            (set-label! stale "Gone")
            (set-label! present "Here")
            (set-prop! (presence-last-report-key stale) 1)
            (set-claim! present
                (map-set
                    (map-set
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set (make-map) "actor" present)
                                        "kind" "avatar")
                                    "protocol" "/ma/avatar/0.0.1")
                                "parent" "did:ma:runtime#room")
                            "name" "Here")
                        "nick" "Here")
                    "description" "An avatar."))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#duckie",
                "did:ma:runtime#room",
                Value::symbol(":say"),
            )),
        );

        eval_all("((find-method :say) (list \"quack\") msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#present-avatar"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("did:ma:runtime#duckie says: quack"),
            ])
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
        eval_all("(define (ma-entity-exists? actor) #t)", &env).unwrap();
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
        env.define(Rc::from("child_ctx"), child_ctx.clone());
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
    fn room_children_announcement_registers_dropped_container() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        let container_ctx = eval_all(
                        r#"
                        (map-set
                            (map-set
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
                                        "nick" "Vadsekk")
                                    "description" "A sturdy canvas bag.")
                                "rev" 2)
                            "contents" (make-map))
                        "#,
                        &env,
                )
                .unwrap();
        env.define(Rc::from("container_ctx"), container_ctx.clone());
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#bag",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":parent"), container_ctx]),
            )),
        );

        eval_all("(on-message parent_msg)", &env).unwrap();
        let first_sent_count = eval_int("(get-prop \"sent-count\")", &env);

        eval_all("(on-message parent_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(thing-ref \"Vadsekk\")", &env),
            "did:ma:runtime#bag"
        );
        assert_eq!(eval_str("(things-text)", &env), "Things: Vadsekk");
        assert_eq!(
            eval_int("(get-prop \"sent-count\")", &env),
            first_sent_count + 1
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:2\"))", &env).unwrap(),
            Value::symbol(":child")
        );
    }

    #[test]
    fn room_removes_departed_thing_by_actor_did_url() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);

        eval_all(
            r#"
            (set-thing! "Aladdins lampe" "did:ma:runtime#lamp")
            (set-claim! "did:ma:runtime#lamp"
                (map-set
                    (map-set
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                        "kind" "thing")
                                    "protocol" "/ma/thing/0.0.1")
                                "parent" "did:ma:runtime#inventory")
                            "name" "lamp")
                        "nick" "Aladdins lampe")
                    "description" "A warm brass lamp."))
            "#,
            &env,
        )
        .unwrap();

        let departed_ctx = eval_all(r#"(claim-ctx "did:ma:runtime#lamp")"#, &env).unwrap();
        env.define(
            Rc::from("departure_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":parent"), departed_ctx]),
            )),
        );

        eval_all("(on-message departure_msg)", &env).unwrap();

        assert!(eval_bool("(not (thing-ref \"Aladdins lampe\"))", &env));
        assert!(eval_bool("(not (claim-ctx \"did:ma:runtime#lamp\"))", &env));
        assert_eq!(eval_str("(things-text)", &env), "Things: none.");
    }

    #[test]
    fn room_repeated_agent_parent_ctx_is_idempotent() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all("(define (ma-entity-exists? actor) #t)", &env).unwrap();
        let child_ctx = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:runtime#duckie")
                                                "kind" "agent")
                                            "protocol" "/ma/scheme/agent/0.0.1")
                                        "parent" "did:ma:runtime#room")
                                    "name" "Rubber Duckie")
                                "nick" "Duckie")
                            "description" "A curious rubber duck.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("children_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#duckie",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":parent"), child_ctx.clone()]),
            )),
        );

        eval_all("(on-message children_msg)", &env).unwrap();
        let first_sent_count = eval_int("(get-prop \"sent-count\")", &env);
        let first_reply_count = eval_int("(get-prop \"reply-count\")", &env);
        let first_rev = eval_int("(get-prop \"ctx:rev\")", &env);

        eval_all("(on-message children_msg)", &env).unwrap();

        assert_eq!(
            eval_int("(get-prop \"sent-count\")", &env),
            first_sent_count + 1
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:3\"))", &env).unwrap(),
            Value::symbol(":child")
        );
        assert_eq!(
            eval_int("(get-prop \"reply-count\")", &env),
            first_reply_count + 1
        );
        assert_eq!(eval_int("(get-prop \"ctx:rev\")", &env), first_rev);
        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: Duckie");
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
            (test-avatar-claim! avatar "Avatar")
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
            (test-avatar-claim! avatar "Alice")
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
            (test-avatar-claim! avatar "Alice")
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
    fn room_presence_tick_saves_presence_challenge_state() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (define (ma-entity-exists? actor) #t)
            (define (ma-save-state!) (inc-prop! "save-count" 1))
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            (define avatar "did:ma:runtime#avatar")
            (set-label! avatar "Alice")
            (test-avatar-claim! avatar "Alice")
            (set-prop! "save-count" 0)
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#scheduler",
                "did:ma:runtime#room",
                Value::symbol(":presence-tick"),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"save-count\")", &env), 1);
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":report-parent"),
                Value::str("did:ma:runtime#room"),
                Value::Int(1),
                Value::str("ccd19035308b3ff6"),
            ])
        );
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

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 4);
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
            "did:ma:runtime#root"
        );
        assert!(eval_bool(
            r#"(let ((term (get-prop "sent-term:2")))
                 (and (equal? (car term) :parent)
                      (equal? (ctx-text (car (cdr term)) "actor") "did:ma:runtime#room")
                      (equal? (ctx-text (car (cdr term)) "parent") "did:ma:runtime#root")))"#,
            &env,
        ));
        assert_eq!(
            eval_str("(get-prop \"sent-target:3\")", &env),
            "did:ma:runtime#scheduler"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:3\")", &env).unwrap(),
            Value::list(vec![
                Value::str("presence"),
                Value::symbol(":interval"),
                Value::str("30s"),
                Value::symbol(":presence-tick"),
            ])
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:4\")", &env),
            "did:ma:runtime#root"
        );
        assert!(eval_bool(
            r#"(let ((term (get-prop "sent-term:4")))
                 (and (equal? (car term) :parent)
                      (equal? (ctx-text (car (cdr term)) "actor") "did:ma:runtime#room")
                      (equal? (ctx-text (car (cdr term)) "parent") "did:ma:runtime#root")))"#,
            &env,
        ));
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
                    Value::symbol(":recycle"),
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
            Value::list(vec![Value::symbol(":ok"), Value::Nil])
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
            "(not (method-member? :take (car (cdr (get-prop \"reply-term:1\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :drop (car (cdr (get-prop \"reply-term:1\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :enter (car (cdr (get-prop \"reply-term:1\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :go (car (cdr (get-prop \"reply-term:1\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :nick (car (cdr (get-prop \"reply-term:1\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(method-member? :owner (car (cdr (get-prop \"reply-term:2\"))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :name (car (cdr (get-prop \"reply-term:2\")))))",
            &env,
        ));
        assert!(eval_bool(
            "(not (method-member? :description (car (cdr (get-prop \"reply-term:2\")))))",
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
    fn room_parent_avatar_ctx_broadcasts_nick_change() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);

        eval_all(
            r#"
                        (define avatar "did:ma:runtime#avatar")
                        (define (ma-entity-exists? actor) #t)
                        (set-label! avatar "Bar")
                        (test-avatar-claim! avatar "Bar")
                        "#,
            &env,
        )
        .unwrap();

        env.define(
                        Rc::from("msg"),
                        Value::Msg(sample_term_msg(
                                "did:ma:runtime#avatar",
                                "did:ma:runtime#room",
                                Value::list(vec![
                                        Value::symbol(":parent"),
                                        eval_all(
                                                r#"
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set
                                                                    (map-set
                                                                        (map-set (make-map) "actor" "did:ma:runtime#avatar")
                                                                        "kind" "avatar")
                                                                    "protocol" "/ma/avatar/0.0.1")
                                                                "parent" "did:ma:runtime#room")
                                                            "name" "Foo")
                                                        "nick" "Foo")
                                                    "description" "An avatar.")
                                                "#,
                                                &env,
                                        )
                                        .unwrap(),
                                ]),
                        )),
                );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_str("(speaker-name avatar)", &env), "Foo");
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":ctx")
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Bar is now known as Foo."),
            ])
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn exit_about_with_ctx_prints_to_viewer() {
        let env = exit_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#north-exit",
            )),
        );

        eval_all(
            r#"
            (set-prop! "direction" "north")
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "target-room" "did:ma:runtime#kitchen")
            ((find-method :about)
              (list
                (map-set
                  (map-set
                    (map-set
                      (map-set (make-map) "actor" "did:ma:runtime#avatar")
                      "avatar" "did:ma:runtime#avatar")
                    "kind" "avatar")
                  "room" "did:ma:runtime#room"))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :print)",
            &env
        ));
        assert_eq!(
            eval_str("(car (cdr (get-prop \"sent-term:1\")))", &env),
            "exit north\nAn exit leading north.\nowner: (none)\nsource: did:ma:runtime#room\ntarget: did:ma:runtime#kitchen\ndirection: north\nlocked: false"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
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
            "(not (method-member? :owner? (car (cdr (get-prop \"reply-term:1\")))))",
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
            "(method-member? :owner? (car (cdr (get-prop \"reply-term:2\"))))",
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
        install_send_reply_recorders(&env);
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
                            (list (list (list :kind "agent")
                          (list :room "did:ma:runtime#room-a")))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#room-a");
        assert_eq!(eval_str("(pending-room)", &env), "");
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 3);
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#room-b"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:2\"))", &env).unwrap(),
            Value::symbol(":parent")
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#room-a"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:3\")", &env),
            "did:ma:runtime#room-a"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:3\"))", &env).unwrap(),
            Value::symbol(":parent")
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:3\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#room-a"
        );
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
                        (test-agent-claim! rms "rms")
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
                "(list :ctx (avatar-exit-ctx \"did:ma:runtime#rms\" #f))",
                &env
            )
            .unwrap(),
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
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
                        (set-prop! "inventory" (inventory-for-did (did)))
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
    fn avatar_look_visible_thing_uses_stored_room_ctx() {
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
                        (set-prop! "room-ctx"
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (make-map)
                                                "protocol" "/ma/room/0.0.1")
                                            "kind" "room")
                                        "actor" "did:ma:runtime#room")
                                    "rev" 1)
                                "things"
                                    (list
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                        "kind" "thing")
                                                    "name" "lamp")
                                                "nick" "north")
                                            "description" "A brass lamp."))))
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
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":look"), Value::str("did:ma:did")]),
        );
    }

    #[test]
    fn avatar_go_uses_stored_room_ctx_exit() {
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
        install_send_reply_recorders(&env);

        eval_all(
                        r#"
                        (set-prop! "did" "did:ma:did")
                        (set-prop! "room" "did:ma:runtime#room")
                        (set-prop! "room-ctx"
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (make-map)
                                                "protocol" "/ma/room/0.0.1")
                                            "kind" "room")
                                        "actor" "did:ma:runtime#room")
                                    "rev" 1)
                                "exits"
                                    (list
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set (make-map) "actor" "did:ma:runtime#north-exit")
                                                        "kind" "exit")
                                                    "direction" "north")
                                                "nick" "north")
                                            "description" "A narrow doorway."))))
                        "#,
                        &env,
                )
                .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
                &avatar,
                Value::list(vec![Value::symbol(":go"), Value::str("north")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"did\")",
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
        assert!(eval_bool(
            "(not (non-empty-string? (ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"inventory\")))",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"room\")",
                &env
            ),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn avatar_nick_emits_ctx_to_did_and_parent_room() {
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
        install_send_reply_recorders(&env);

        eval_all(
            r#"
                        (set-prop! "did" "did:ma:did")
                        (set-prop! "room" "did:ma:runtime#room")
                        (set-prop! "inventory" "did:ma:runtime#inventory")
                        (define (ma-entity-exists? actor) #f)
                        (define (ma-create-actor kind behaviour init fragment) fragment)
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
                &avatar,
                Value::list(vec![Value::symbol(":nick"), Value::str("Foo")]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_str("(nick)", &env), "Foo");
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), did);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#room"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :parent)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"actor\")",
                &env
            ),
            avatar
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"nick\")",
                &env
            ),
            "Foo"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn avatar_room_ctx_ignores_stale_revisions() {
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
                        (define (make-room-ctx rev name)
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (make-map)
                                                    "protocol" "/ma/room/0.0.1")
                                                "kind" "room")
                                            "actor" "did:ma:runtime#room")
                                        "parent" "did:ma:runtime#root")
                                    "rev" rev)
                                "name" name))
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#avatar",
                eval_all("(list :ctx (make-room-ctx 2 \"new\"))", &env).unwrap(),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#avatar",
                eval_all("(list :ctx (make-room-ctx 1 \"old\"))", &env).unwrap(),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(eval_int("(map-ref (stored-room-ctx) \"rev\" #f)", &env), 2);
        assert_eq!(
            eval_str("(ctx-text (stored-room-ctx) \"name\")", &env),
            "new"
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
    fn avatar_owner_query_with_args_delegates_to_room() {
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
            Value::symbol(":ok")
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":owner?"), Value::str("Shrugger")]),
        );
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
    fn avatar_owner_query_from_room_prints_owner_to_requester() {
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
            Value::symbol(":ok")
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#requester"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Owner: did:ma:did")
            ]),
        );
    }

    #[test]
    fn avatar_owner_queries_without_args_delegate_to_room() {
        let env = avatar_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());
        let avatar_id = eval_str(r#"(avatar-fragment "did:ma:did")"#, &env);
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
            Value::symbol(":ok")
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":owner?")])
        );
    }

    #[test]
    fn avatar_take_does_not_add_unconfirmed_inventory_tokens() {
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

        assert_eq!(eval_str("(inventory-text)", &env), "Inventory: empty.");

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

        assert_eq!(eval_str("(inventory-text)", &env), "Inventory: empty.");

        env.define(
            Rc::from("bad_take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("lampeTHISISABUG")]),
            )),
        );
        eval_all("(on-message bad_take_msg)", &env).unwrap();
        eval_all("(on-message inventory_msg)", &env).unwrap();

        assert_eq!(eval_str("(inventory-text)", &env), "Inventory: empty.");
    }

    #[test]
    fn avatar_inventory_waits_for_container_ctx_after_direct_actor_take() {
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
                Value::str(eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env)),
            ]),
        );
        assert_eq!(eval_str("(inventory-text)", &env), "Inventory: empty.");

        eval_all(
            r#"
                        (define satchel-ctx
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" "did:ma:runtime#satchel")
                                                    "kind" "thing")
                                                "protocol" "/ma/thing/0.0.1")
                                              "parent" (inventory-for-did "did:ma:did"))
                                        "name" "Lars'ers rygsæk")
                                    "nick" "Rygsækken")
                                "description" "A sturdy satchel."))
                        (define inventory-test-ctx
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set (make-map)
                                                                "protocol" "/ma/container/0.0.1")
                                                            "kind" "container")
                                                        "actor" (inventory-for-did "did:ma:did"))
                                                    "parent" (local-self))
                                                "rev" 1)
                                            "name" "Inventory")
                                        "nick" "inventory")
                                    "description" "A personal inventory container.")
                                "contents" (map-set (make-map) "did:ma:runtime#satchel" satchel-ctx)))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("children_msg"),
            Value::Msg(sample_term_msg(
                &eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env),
                "did:ma:runtime#avatar",
                eval_all("(list :parent inventory-test-ctx)", &env).unwrap(),
            )),
        );
        eval_all("(on-message children_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(inventory-text)", &env),
            "Inventory:\nRygsækken = did:ma:runtime#satchel",
        );
    }

    #[test]
    fn avatar_forwards_ctx_to_did_after_inventory_container_ctx_update() {
        let env = avatar_env();
        install_send_reply_recorders(&env);
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
                        (set-prop! "inventory" "did:ma:runtime#inventory")
                        (define lamp-ctx
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" "did:ma:runtime#lamp")
                                                    "kind" "thing")
                                                "protocol" "/ma/thing/0.0.1")
                                            "parent" "did:ma:runtime#inventory")
                                        "name" "lamp")
                                    "nick" "lamp")
                                "description" "A brass lamp."))
                        (define (container-ctx rev contents)
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set (make-map) "protocol" "/ma/container/0.0.1")
                                                            "kind" "container")
                                                        "actor" "did:ma:runtime#inventory")
                                                    "parent" (local-self))
                                                "rev" rev)
                                            "name" "Inventory")
                                        "nick" "inventory")
                                    "description" "A personal inventory container.")
                                "contents" contents))
                        (remember-inventory-ctx!
                            (container-ctx 1 (map-set (make-map) "did:ma:runtime#lamp" lamp-ctx)))
                        "#,
                        &env,
                )
                .unwrap();
        assert_eq!(
            eval_str("(inventory-text)", &env),
            "Inventory:\nlamp = did:ma:runtime#lamp"
        );

        env.define(
            Rc::from("ctx_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#inventory",
                &avatar,
                eval_all("(list :parent (container-ctx 2 (make-map)))", &env).unwrap(),
            )),
        );
        eval_all("(on-message ctx_msg)", &env).unwrap();

        assert_eq!(eval_str("(inventory-text)", &env), "Inventory: empty.");
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), did);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :ctx)",
            &env,
        ));
        assert_eq!(
            eval_str(
                "(ctx-value (car (cdr (get-prop \"sent-term:1\"))) :inv)",
                &env
            ),
            "did:ma:runtime#inventory"
        );

        env.define(
            Rc::from("stale_ctx_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#inventory",
                &avatar,
                eval_all("(list :parent (container-ctx 1 (make-map)))", &env).unwrap(),
            )),
        );
        eval_all("(on-message stale_ctx_msg)", &env).unwrap();

        assert!(eval_bool("(not (get-prop \"sent-target:2\"))", &env));
    }

    #[test]
    fn avatar_adopts_inventory_container_as_parent_via_child_ctx() {
        let env = avatar_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#avatar".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:did")
            (adopt-inventory! "did:ma:remote#inventory")
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:remote#inventory"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":child")
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:remote#inventory"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#avatar"
        );
    }

    #[test]
    fn avatar_look_carried_container_sends_container_look_for_controlling_did() {
        let env = avatar_env();
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config.clone());

        let avatar_id = eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env);
        let avatar_self = format!("did:ma:runtime#{avatar_id}");
        config.insert("self".to_string(), avatar_self);
        config.insert("id".to_string(), avatar_id);
        crate::state::set_config(config);

        eval_all(
            r#"
                        (set-prop! "did" "did:ma:did")
                        (set-prop! "room" "did:ma:runtime#room")
                        (set-prop! "inventory" "did:ma:runtime#inventory")
                        "#,
            &env,
        )
        .unwrap();

        eval_all(
                        r#"
                        (define satchel-ctx
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" "did:ma:runtime#satchel")
                                                    "kind" "container")
                                                "protocol" "/ma/container/0.0.1")
                                            "parent" "did:ma:runtime#inventory")
                                        "name" "Lars'ers rygsæk")
                                    "nick" "Rygsækken")
                                "description" "A sturdy satchel."))
                        (define inventory-test-ctx
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set (make-map)
                                                                "protocol" "/ma/container/0.0.1")
                                                            "kind" "container")
                                                        "actor" "did:ma:runtime#inventory")
                                                    "parent" (local-self))
                                                "rev" 1)
                                            "name" "Inventory")
                                        "nick" "inventory")
                                    "description" "A personal inventory container.")
                                "contents" (map-set (make-map) "did:ma:runtime#satchel" satchel-ctx)))
                        "#,
                        &env,
                )
                .unwrap();
        env.define(
            Rc::from("container_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#inventory",
                "did:ma:runtime#avatar",
                eval_all("(list :parent inventory-test-ctx)", &env).unwrap(),
            )),
        );
        eval_all("(on-message container_msg)", &env).unwrap();

        env.define(
            Rc::from("look_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":look"), Value::str("Rygsækken")]),
            )),
        );
        eval_all("(on-message look_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#satchel"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":look"), Value::str("did:ma:did")]),
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
        config.insert(
            "id".to_string(),
            eval_str(&format!(r#"(avatar-fragment "{did}")"#), &env),
        );
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
        let expected_inventory = eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env);

        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str(expected_inventory),
            ]),
        );
    }

    #[test]
    fn avatar_take_visible_actor_resolves_whole_visible_words() {
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
                        (set-prop! "inventory" (inventory-for-did (did)))
                        (define (entity-live? actor) #t)
                        (remember-room-ctx!
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set (make-map)
                                                                    "protocol" "/ma/room/0.0.1")
                                                                "kind" "room")
                                                            "actor" "did:ma:runtime#room")
                                                        "parent" "did:ma:runtime#root")
                                                    "rev" 1)
                                                "name" "Kitchen")
                                            "nick" "Kitchen")
                                        "description" "A practical kitchen.")
                                    "things"
                                        (list
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set (make-map)
                                                                    "actor" "did:ma:runtime#lamp")
                                                                "kind" "thing")
                                                            "protocol" "/ma/thing/0.0.1")
                                                        "name" "lamp")
                                                    "nick" "Aladdins lampe")
                                                "description" "A warm brass lamp.")))
                                "exits" '()))
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("Aladdins lampe")]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str(eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env)),
                eval_all("(room-ctx-ref \"Aladdins lampe\")", &env).unwrap(),
            ]),
        );

        env.define(
            Rc::from("take_aladdins_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("Aladdins")]),
            )),
        );
        eval_all("(on-message take_aladdins_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#lamp"
        );

        env.define(
            Rc::from("take_lampe_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("lampe")]),
            )),
        );
        eval_all("(on-message take_lampe_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(get-prop \"sent-target:3\")", &env),
            "did:ma:runtime#lamp"
        );

        env.define(
            Rc::from("take_lamp_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("lamp")]),
            )),
        );
        eval_all("(on-message take_lamp_msg)", &env).unwrap();
        assert_eq!(eval_str("(get-prop \"sent-target:4\")", &env), "did:ma:did");
        assert_eq!(
            eval_all("(get-prop \"sent-term:4\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Unknown visible agent or thing: lamp"),
            ]),
        );
    }

    #[test]
    fn thing_accepts_visible_take_ctx_from_owner_avatar() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        let avatar = format!(
            "did:ma:runtime#{}",
            eval_str(r#"(avatar-fragment "did:ma:owner")"#, &env)
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "Aladdins lampe")
            (set-prop! "description" "A warm brass lamp.")
            "#,
            &env,
        )
        .unwrap();

        let visible_ctx = eval_all("(node-ctx)", &env).unwrap();
        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                &avatar,
                "did:ma:runtime#lamp",
                Value::symbol(":take"),
            )),
        );
        env.define(Rc::from("visible_ctx"), visible_ctx);
        eval_all(
            r#"((find-method :take)
                (list "did:ma:owner" "did:ma:runtime#inventory" visible_ctx)
                take_msg)"#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#inventory"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":parent")
        );
    }

    #[test]
    fn avatar_take_visible_word_reports_ambiguous_matches() {
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
                        (define (visible-thing actor nick)
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map)
                                                    "actor" actor)
                                                "kind" "thing")
                                            "protocol" "/ma/thing/0.0.1")
                                        "name" actor)
                                    "nick" nick)
                                "description" nick))
                        (remember-room-ctx!
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set (make-map)
                                                                "protocol" "/ma/room/0.0.1")
                                                            "kind" "room")
                                                        "actor" "did:ma:runtime#room")
                                                    "parent" "did:ma:runtime#root")
                                                "rev" 1)
                                            "name" "Kitchen")
                                        "nick" "Kitchen")
                                    "description" "A practical kitchen.")
                                "things"
                                    (list
                                        (visible-thing "did:ma:runtime#red-lamp" "red lampe")
                                        (visible-thing "did:ma:runtime#blue-lamp" "blue lampe"))))
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("take_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":take"), Value::str("lampe")]),
            )),
        );
        eval_all("(on-message take_msg)", &env).unwrap();

        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:did");
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("Ambiguous visible agent or thing: lampe"),
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

        assert_eq!(eval_str("(node-parent)", &env), "");
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

        assert_eq!(eval_str("(node-parent)", &env), "");
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
    fn avatar_look_visible_container_uses_stored_room_ctx() {
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
        env.define(Rc::from("bag_ctx"), bag_ctx);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![Value::symbol(":look"), Value::str("Lars'ers rygsæk")]),
            )),
        );

        eval_all(
            r#"
                                                (set-prop! "did" "did:ma:did")
                                                (set-prop! "room" "did:ma:runtime#room")
                                                (set-prop! "room-ctx"
                                                    (map-set
                                                        (map-set
                                                            (map-set
                                                                (map-set
                                                                    (map-set
                                                                        (make-map)
                                                                        "protocol" "/ma/room/0.0.1")
                                                                    "kind" "room")
                                                                "actor" "did:ma:runtime#room")
                                                            "rev" 1)
                                                        "things" (list bag_ctx)))
                                                (on-message msg)
                        "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#bag"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":look"), Value::str("did:ma:did")]),
        );
    }

    #[test]
    fn agent_parent_take_keeps_child_until_committed_departure() {
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
        assert_eq!(
            eval_str("(children-text)", &env),
            "Children:\nThe Lamp = did:ma:runtime#lamp"
        );
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
                Value::str(eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env)),
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
                        (define duckie-ctx
                            (map-set
                (map-set
                  (map-set
                    (map-set
                      (map-set (make-map) "actor" "did:ma:runtime#duckie")
                      "kind" "agent")
                    "name" "duckie")
                  "nick" "Duckie")
                                "description" "A small duck."))
                        (remember-inventory-ctx!
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set (make-map)
                                                                "protocol" "/ma/container/0.0.1")
                                                            "kind" "container")
                                                        "actor" (inventory-for-did "did:ma:did"))
                                                    "parent" (local-self))
                                                "rev" 1)
                                            "name" "Inventory")
                                        "nick" "inventory")
                                    "description" "A personal inventory container.")
                                "contents" (map-set (make-map) "did:ma:runtime#duckie" duckie-ctx)))
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
                Value::str(eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env)),
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
                        (remember-inventory-ctx!
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set
                                                            (map-set (make-map)
                                                                "protocol" "/ma/container/0.0.1")
                                                            "kind" "container")
                                                        "actor" (inventory-for-did "did:ma:did"))
                                                    "parent" (local-self))
                                                "rev" 1)
                                            "name" "Inventory")
                                        "nick" "inventory")
                                    "description" "A personal inventory container.")
                                "contents"
                                    (map-set
                                        (map-set (make-map) "did:ma:runtime#lamp" lamp_ctx)
                                        "did:ma:runtime#bag" bag_ctx)))
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

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env)
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#lamp"),
                Value::str("did:ma:runtime#bag"),
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

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
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

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":drop"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#bag"),
                lamp_ctx,
            ])
        );
        assert_eq!(
            eval_str("(things-text)", &env),
            "Things: The Golden Lamp, Lars'ers rygsæk"
        );
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

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
        );
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

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#lamp"
        );
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
            Value::symbol(":parent"),
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
    fn thing_prop_update_announces_updated_ctx_to_parent() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#inventory")
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "lamp")
            (set-prop! "description" "A brass lamp.")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("prop_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#lamp",
                Value::list(vec![
                    Value::symbol(":prop"),
                    Value::str("nick"),
                    Value::str("shiny lamp"),
                ]),
            )),
        );
        eval_all("(on-message prop_msg)", &env).unwrap();

        assert_eq!(eval_str("(nick)", &env), "shiny lamp");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#inventory"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":parent"),
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"nick\")",
                &env
            ),
            "shiny lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("prop updated")]),
        );
    }

    #[test]
    fn room_ctx_prop_updates_announce_updated_ctx_to_root() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-init-prop! "parent" "did:ma:runtime#root")
            (set-init-prop! "name" "Harbour")
            (set-init-prop! "nick" "harbour")
            (set-init-prop! "description" "A quiet harbour.")
            "#,
            &env,
        )
        .unwrap();

        eval_all("(flush-ctx-prop-changes!)", &env).unwrap();
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        eval_all("(set-room-prop! \"name\" \"Old Harbour\")", &env).unwrap();
        eval_all("(flush-ctx-prop-changes!)", &env).unwrap();
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        eval_all("(set-room-prop! \"nick\" \"old harbour\")", &env).unwrap();
        eval_all("(flush-ctx-prop-changes!)", &env).unwrap();
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        eval_all(
            "(set-room-prop! \"description\" \"A weathered harbour.\")",
            &env,
        )
        .unwrap();
        eval_all("(flush-ctx-prop-changes!)", &env).unwrap();
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 3);
        for index in 1..=3 {
            assert_eq!(
                eval_str(&format!("(get-prop \"sent-target:{index}\")"), &env),
                "did:ma:runtime#root"
            );
            assert_eq!(
                eval_all(&format!("(car (get-prop \"sent-term:{index}\"))"), &env).unwrap(),
                Value::symbol(":parent")
            );
        }
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"name\")",
                &env
            ),
            "Old Harbour"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"nick\")",
                &env
            ),
            "old harbour"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:3\"))) \"description\")",
                &env
            ),
            "A weathered harbour."
        );
    }

    #[test]
    fn thing_recycle_requires_owner_via_parent_and_ends_entity() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_msg("did:ma:runtime#room", "did:ma:runtime#lamp")),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :recycle) (list "did:ma:intruder") parent_msg)
            ((find-method :recycle) (list "did:ma:owner") parent_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "");
        assert_eq!(eval_str("(get-prop \"ended\")", &env), "yes");
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":parent")
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
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"parent\")",
                &env
            ),
            ""
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 2);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner via current parent may recycle this thing"),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("recycled")])
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

        assert_eq!(eval_str("(node-parent)", &env), "");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
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
    fn container_parent_ctx_confirms_committed_child_ctx() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#inventory".to_string());
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
                                        "parent" "did:ma:runtime#inventory")
                                    "name" "lamp")
                                "nick" "Aladdins lampe")
                            "description" "A warm brass lamp.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("child_ctx"), child_ctx.clone());
        env.define(
            Rc::from("child_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#inventory",
                Value::list(vec![Value::symbol(":parent"), child_ctx.clone()]),
            )),
        );

        eval_all(
            r#"
                        (set-prop! "parent" "did:ma:runtime#avatar")
                        (on-message child_msg)
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
            Value::list(vec![Value::symbol(":child"), child_ctx])
        );
    }

    #[test]
    fn container_stops_when_current_parent_confirms_authoritative_ctx() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#inventory".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#avatar")
            (set-prop! "name" "Inventory")
            (set-prop! "nick" "inventory")
            (set-prop! "description" "A personal inventory container.")
            "#,
            &env,
        )
        .unwrap();
        let parent_ctx = eval_all(
            r#"(container-ctx-for-parent "did:ma:runtime#avatar")"#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#inventory",
                Value::list(vec![Value::symbol(":child"), parent_ctx]),
            )),
        );

        eval_all("(on-message parent_msg)", &env).unwrap();

        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn container_stops_when_current_parent_confirms_stale_ctx() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#inventory".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#avatar")
            (set-prop! "name" "Inventory")
            (set-prop! "nick" "inventory")
            (set-prop! "description" "A personal inventory container.")
            (set-prop! "ctx:rev" 8)
            "#,
            &env,
        )
        .unwrap();
        let stale_ctx = eval_all(
            r#"(map-set (container-ctx-for-parent "did:ma:runtime#avatar") "rev" 7)"#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("stale_ctx"), stale_ctx);
        env.define(
            Rc::from("parent_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#inventory",
                Value::symbol(":child"),
            )),
        );

        eval_all("((find-method :child) (list stale_ctx) parent_msg)", &env).unwrap();

        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        assert_eq!(eval_int("(container-ctx-rev)", &env), 8);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn thing_accepts_owner_avatar_drop_delegation() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (set-prop! "parent" "did:ma:runtime#inventory")
            "#,
            &env,
        )
        .unwrap();
        let avatar = eval_str(r#"(avatar-for-did "did:ma:did")"#, &env);
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_term_msg(
                &avatar,
                "did:ma:runtime#lamp",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            ((find-method :drop) (list "did:ma:did" "did:ma:runtime#room") avatar_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert!(eval_bool(
            "(let ((term (get-prop \"sent-term:1\"))) (and (equal? (car term) :parent) (equal? (map-ref (car (cdr term)) \"parent\" \"\") \"did:ma:runtime#room\")))",
            &env,
        ));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn thing_drop_cleans_stale_inventory_claim_by_did_url() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "Aladdins lampe")
            (set-prop! "description" "A warm brass lamp.")
            "#,
            &env,
        )
        .unwrap();
        let stale_ctx =
            eval_all(r#"(node-ctx-for-parent "did:ma:runtime#inventory")"#, &env).unwrap();
        env.define(Rc::from("stale_ctx"), stale_ctx);
        let avatar = eval_str(r#"(avatar-for-did "did:ma:did")"#, &env);
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_term_msg(
                &avatar,
                "did:ma:runtime#lamp",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            ((find-method :drop)
             (list "did:ma:did" "did:ma:runtime#room" stale_ctx)
             avatar_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#inventory"
        );
        assert!(eval_bool(
            r#"(let ((term (get-prop "sent-term:1")))
                 (and (equal? (car term) :parent)
                      (equal? (ctx-text (car (cdr term)) "actor") "did:ma:runtime#lamp")
                      (equal? (ctx-text (car (cdr term)) "parent") "did:ma:runtime#room")))"#,
            &env,
        ));
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#room"
        );
    }

    #[test]
    fn thing_repeated_drop_repairs_stale_inventory_parent() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "Aladdins lampe")
            (set-prop! "description" "A warm brass lamp.")
            "#,
            &env,
        )
        .unwrap();
        let stale_ctx =
            eval_all(r#"(node-ctx-for-parent "did:ma:runtime#inventory")"#, &env).unwrap();
        env.define(Rc::from("stale_ctx"), stale_ctx);
        env.define(
            Rc::from("inventory_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#inventory",
                "did:ma:runtime#lamp",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            ((find-method :drop)
             (list "did:ma:did" "did:ma:runtime#room" stale_ctx)
             inventory_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#inventory"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn container_parent_take_keeps_ctx_until_committed_departure() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#inventory".to_string());
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
                                        "parent" "did:ma:runtime#inventory")
                                    "name" "lamp")
                                "nick" "Aladdins lampe")
                            "description" "A warm brass lamp.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("child_ctx"), child_ctx.clone());
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#avatar",
                "did:ma:runtime#inventory",
                Value::symbol(":take"),
            )),
        );

        eval_all(
                        r#"
                        (set-prop! "parent" "did:ma:runtime#avatar")
                        (remember-content! child_ctx)
                        ((find-method :take) (list "did:ma:did" "did:ma:runtime#lamp" "did:ma:runtime#room" :drop) avatar_msg)
                        "#,
                        &env,
                )
                .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":drop"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#room"),
                child_ctx,
            ])
        );
        assert!(eval_bool(
            "(map? (child-ctx \"did:ma:runtime#lamp\"))",
            &env
        ));
    }

    #[test]
    fn container_look_can_present_contents_to_delegated_did() {
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
        env.define(Rc::from("child_ctx"), child_ctx);
        env.define(
            Rc::from("child_msg"),
            Value::Msg(sample_msg("did:ma:runtime#lamp", "did:ma:runtime#bag")),
        );
        env.define(
            Rc::from("avatar_msg"),
            Value::Msg(sample_msg("did:ma:runtime#avatar", "did:ma:runtime#bag")),
        );

        eval_all(
            r#"
                        (set-prop! "name" "bag")
                        (set-prop! "description" "A canvas bag.")
                        ((find-method :put-in) (list child_ctx) child_msg)
                        ((find-method :look) (list "did:ma:did") avatar_msg)
                        "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(get-prop \"sent-target:2\")", &env), "did:ma:did");
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":print"),
                Value::str("bag\nA canvas bag.\nContents:\nbrass lamp = did:ma:runtime#lamp"),
            ]),
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

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#room");
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
                Value::list(vec![Value::symbol(":child"), parent_ctx]),
            )),
        );

        eval_all("(on-message owner_msg)", &env).unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#room");
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

        assert_eq!(eval_str("(node-parent)", &env), "");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#bag"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn container_accepts_drop_delegation_from_inventory() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (set-prop! "parent" "did:ma:runtime#inventory")
            (set-prop! "name" "bag")
            (set-prop! "nick" "Vadsekk")
            (set-prop! "description" "A sturdy canvas bag.")
            "#,
            &env,
        )
        .unwrap();
        let carried_ctx = eval_all("(container-ctx)", &env).unwrap();
        env.define(Rc::from("carried_ctx"), carried_ctx);
        env.define(
            Rc::from("inventory_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#inventory",
                "did:ma:runtime#bag",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            ((find-method :drop)
                (list "did:ma:did" "did:ma:runtime#room" carried_ctx)
                inventory_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert!(eval_bool(
            "(let ((term (get-prop \"sent-term:1\"))) (and (equal? (car term) :parent) (equal? (map-ref (car (cdr term)) \"parent\" \"\") \"did:ma:runtime#room\")))",
            &env,
        ));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn container_commits_room_confirmation_and_notifies_inventory() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (set-prop! "parent" "did:ma:runtime#inventory")
            (set-prop! "name" "bag")
            (set-prop! "nick" "Vadsekk")
            (set-prop! "description" "A sturdy canvas bag.")
            "#,
            &env,
        )
        .unwrap();
        let room_ctx =
            eval_all(r#"(container-ctx-for-parent "did:ma:runtime#room")"#, &env).unwrap();
        env.define(Rc::from("room_ctx"), room_ctx);
        env.define(
            Rc::from("room_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#bag",
                Value::symbol(":child"),
            )),
        );

        eval_all("((find-method :child) (list room_ctx) room_msg)", &env).unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#room");
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#inventory"
        );
        assert!(eval_bool(
            r#"(let ((ctx (car (cdr (get-prop "sent-term:2")))))
                 (and (equal? (car (get-prop "sent-term:2")) :parent)
                      (equal? (ctx-text ctx "actor") "did:ma:runtime#bag")
                      (equal? (ctx-text ctx "parent") "did:ma:runtime#room")))"#,
            &env,
        ));

        let first_sent_count = eval_int("(get-prop \"sent-count\")", &env);
        let first_reply_count = eval_int("(get-prop \"reply-count\")", &env);
        let first_rev = eval_int("(container-ctx-rev)", &env);
        let authoritative_ctx = eval_all("(container-ctx)", &env).unwrap();
        env.define(
            Rc::from("authoritative_room_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#room",
                "did:ma:runtime#bag",
                Value::list(vec![Value::symbol(":child"), authoritative_ctx]),
            )),
        );

        eval_all(
            "((find-method :child) (list (container-ctx)) authoritative_room_msg)",
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_int("(get-prop \"sent-count\")", &env),
            first_sent_count
        );
        assert_eq!(
            eval_int("(get-prop \"reply-count\")", &env),
            first_reply_count + 1
        );
        assert_eq!(eval_int("(container-ctx-rev)", &env), first_rev);
    }

    #[test]
    fn container_repeated_drop_repairs_stale_inventory_parent() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#bag".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:did")
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "bag")
            (set-prop! "nick" "Vadsekk")
            (set-prop! "description" "A sturdy canvas bag.")
            "#,
            &env,
        )
        .unwrap();
        let stale_ctx = eval_all(
            r#"(container-ctx-for-parent "did:ma:runtime#inventory")"#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("stale_ctx"), stale_ctx);
        env.define(
            Rc::from("inventory_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#inventory",
                "did:ma:runtime#bag",
                Value::symbol(":drop"),
            )),
        );

        eval_all(
            r#"
            ((find-method :drop)
             (list "did:ma:did" "did:ma:runtime#room" stale_ctx)
             inventory_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#inventory"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#room"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("drop requested")])
        );
    }

    #[test]
    fn inventory_forgets_container_after_committed_room_ctx() {
        let env = container_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#inventory".to_string());
        crate::state::set_config(config);

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
                    "parent" "did:ma:runtime#inventory")
                  "name" "bag")
                "nick" "Vadsekk")
              "description" "A sturdy canvas bag.")
            "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("bag_ctx"), bag_ctx);
        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#avatar")
            (remember-child! bag_ctx)
            "#,
            &env,
        )
        .unwrap();
        let committed_ctx =
            eval_all(r#"(map-set bag_ctx "parent" "did:ma:runtime#room")"#, &env).unwrap();
        env.define(Rc::from("committed_ctx"), committed_ctx.clone());
        env.define(
            Rc::from("bag_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#bag",
                "did:ma:runtime#inventory",
                Value::list(vec![Value::symbol(":parent"), committed_ctx]),
            )),
        );

        eval_all("(on-message bag_msg)", &env).unwrap();

        assert!(eval_bool("(not (child-ctx \"did:ma:runtime#bag\"))", &env));
        assert_eq!(eval_str("(contents-text)", &env), "Contents: none.");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#avatar"
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
            Value::symbol(":parent"),
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
    fn avatar_children_registration_does_not_update_inventory_and_rejects_forgery() {
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
                Value::str("Inventory: empty."),
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
    fn avatar_drop_lookup_token_to_did_url_before_transfer() {
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
                                                                                                                                "parent" (inventory-for-did "did:ma:did"))
                  "name" "duckie")
                                "nick" "Aladdins lampe")
              "description" "A small duck.")
            "#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("child_ctx"), child_ctx);

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
                                &eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env),
                "did:ma:runtime#avatar",
                                eval_all(
                                        "(list :parent
                                             (map-set
                                                 (map-set
                                                     (map-set
                                                         (map-set
                                                             (map-set
                                                                 (map-set
                                                                     (map-set
                                                                         (map-set
                                                                             (map-set (make-map)
                                                                                 \"protocol\" \"/ma/container/0.0.1\")
                                                                             \"kind\" \"container\")
                                                                         \"actor\" (inventory-for-did \"did:ma:did\"))
                                                                     \"parent\" (local-self))
                                                                 \"rev\" 1)
                                                             \"name\" \"Inventory\")
                                                         \"nick\" \"inventory\")
                                                     \"description\" \"A personal inventory container.\")
                                                 \"contents\" (map-set (make-map) \"did:ma:runtime#duckie\" child_ctx)))",
                                        &env,
                                )
                                .unwrap(),
            )),
        );
        eval_all("(on-message adopt_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(inventory-text)", &env),
            "Inventory:\nAladdins lampe = did:ma:runtime#duckie",
        );

        env.define(
            Rc::from("drop_msg"),
            Value::Msg(sample_term_msg(
                did,
                "did:ma:runtime#avatar",
                Value::list(vec![
                    Value::symbol(":drop"),
                    Value::str("Aladdins"),
                    Value::str("lampe"),
                ]),
            )),
        );
        eval_all("(on-message drop_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:3\")", &env),
            eval_str(r#"(inventory-for-did "did:ma:did")"#, &env)
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:3\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":take"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#duckie"),
                Value::str("did:ma:runtime#room"),
                Value::symbol(":drop"),
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
            eval_str("(inventory-text)", &env),
            "Inventory:\nAladdins lampe = did:ma:runtime#duckie"
        );
    }

    #[test]
    fn avatar_drop_lookup_accepts_did_url_identity() {
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
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#duckie"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":drop"),
                Value::str("did:ma:did"),
                Value::str("did:ma:runtime#room"),
            ]),
        );
        assert!(eval_bool("(not (get-prop \"sent-term:2\"))", &env));
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
                Value::list(vec![Value::symbol(":child"), parent_ctx]),
            )),
        );
        eval_all("(on-message parent_msg)", &env).unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#mother");
        assert_eq!(eval_str("(nick)", &env), "new lamp");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#mother"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":parent"),
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#mother"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#father"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#mother"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok"),
        );

        eval_all("(on-message parent_msg)", &env).unwrap();
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 2);
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
                Value::list(vec![Value::symbol(":child"), parent_ctx]),
            )),
        );
        eval_all("(on-message forged_parent_msg)", &env).unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#father");
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("child ctx must name self and come from target parent"),
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
                Value::list(vec![Value::symbol(":child"), parent_ctx]),
            )),
        );
        eval_all("(on-message parent_msg)", &env).unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "did:ma:runtime#mother");
        assert_eq!(eval_str("(nick)", &env), "rms-on-tour");
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#mother"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":parent"),
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#father"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:2\"))", &env).unwrap(),
            Value::symbol(":parent"),
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok"),
        );

        eval_all("(on-message parent_msg)", &env).unwrap();
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 2);
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
                "(node-effective-did (list \"did:ma:did\" \"north\") msg)",
                &env
            ),
            "did:ma:did"
        );
        assert_eq!(
            eval_str(
                "(car (node-effective-args (list \"did:ma:did\" \"north\") msg))",
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
            "(set-init-prop! \"direction\" \"dør\")\n(set-init-prop! \"parent\" \"did:ma:runtime#source\")\n(set-init-prop! \"target-room\" \"did:ma:runtime#kitchen\")\n(ma-save-state!)\n"
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
            "(set-init-prop! \"direction\" \"north\")\n(set-init-prop! \"owner\" \"did:ma:owner\")\n(set-init-prop! \"parent\" \"did:ma:runtime#source\")\n(set-init-prop! \"target-room\" \"did:ma:runtime#kitchen\")\n(ma-save-state!)\n"
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
    fn room_move_forwards_avatar_ctx_to_exit() {
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
            (send-exit-ctx! "did:ma:runtime#avatar" "did:ma:did" "north" "did:ma:runtime#north-exit")
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
            "(equal? (car (get-prop \"sent-term:1\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"did\")",
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
        eval_all("(set-prop! \"parent\" \"did:ma:runtime#room\")", &env).unwrap();

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
    fn exit_ctx_returns_transformed_avatar_ctx_to_avatar() {
        let env = exit_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#north-exit".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "direction" "north")
            (set-prop! "parent" "did:ma:runtime#room")
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
                        ((find-method :ctx)
              (list (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (make-map)
                                                        "did" "did:ma:did")
                                                    "kind" "avatar")
                                                "avatar" "did:ma:runtime#avatar")
                      "room" "did:ma:runtime#room"))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target\")", &env),
            "did:ma:runtime#avatar"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term\")) :ctx)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"did\")",
                &env
            ),
            "did:ma:did"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term\"))) \"avatar\")",
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
    fn avatar_movement_enters_target_room() {
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
                        (set-prop! "room-ctx"
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (make-map)
                                                "protocol" "/ma/room/0.0.1")
                                            "kind" "room")
                                        "actor" "did:ma:runtime#room")
                                    "rev" 1)
                                "exits"
                                    (list
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set (make-map) "actor" "did:ma:runtime#north-exit")
                                                        "kind" "exit")
                                                    "direction" "north")
                                                "nick" "north")
                                            "description" "A narrow doorway."))))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#north-exit", &avatar)),
        );
        eval_all(
            r#"
                                 (let* ((ctx1 (map-set (make-map) "did" "did:ma:did"))
                   (ctx2 (map-set ctx1 "kind" "avatar"))
                   (ctx3 (map-set ctx2 "avatar" (local-self)))
                   (ctx4 (map-set ctx3 "room" "did:ma:runtime#kitchen"))
                   (ctx5 (map-set ctx4 "exit" "did:ma:runtime#north-exit"))
                   (ctx6 (map-set ctx5 "text" "You pass through the oak door.")))
              ((find-method :ctx) (list ctx6) msg))
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
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#kitchen"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :enter)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"did\")",
                &env
            ),
            did
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
            eval_str("(get-prop \"pending-room\")", &env),
            "did:ma:runtime#kitchen"
        );
    }

    #[test]
    fn avatar_committed_ctx_notifies_old_room_last() {
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
            (set-prop! "root" "did:ma:runtime#root")
            (set-prop! "room" "did:ma:runtime#old-room")
            (set-prop! "pending-room" "did:ma:runtime#new-room")
            (define (entity-live? actor) #t)
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#new-room", &avatar)),
        );
        eval_all(
            r#"
            ((find-method :ctx)
                            (list (list (list :kind "avatar")
                          (list :root "did:ma:runtime#root")
                          (list :avatar (local-self))
                          (list :room "did:ma:runtime#new-room")
                          (list :nick "Aletheia")))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(room)", &env), "did:ma:runtime#new-room");
        assert!(eval_bool("(not (has-prop? \"pending-room\"))", &env));
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), did);
        assert_eq!(
            eval_str(
                "(ctx-value (car (cdr (get-prop \"sent-term:1\"))) :inv)",
                &env
            ),
            eval_str(&format!(r#"(inventory-for-did "{did}")"#), &env)
        );
        assert_eq!(
            eval_str(
                "(ctx-value (car (cdr (get-prop \"sent-term:1\"))) :room)",
                &env
            ),
            "did:ma:runtime#new-room"
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:runtime#old-room"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:2\"))", &env).unwrap(),
            Value::symbol(":parent")
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#new-room"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:2\"))) \"nick\")",
                &env
            ),
            "Aletheia"
        );
    }

    #[test]
    fn avatar_movement_enters_cross_runtime_target_room() {
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
                        (set-prop! "room-ctx"
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (make-map)
                                                "protocol" "/ma/room/0.0.1")
                                            "kind" "room")
                                        "actor" "did:ma:sky#construct")
                                    "rev" 1)
                                "exits"
                                    (list
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set
                                                        (map-set (make-map) "actor" "did:ma:sky#cloud-exit")
                                                        "kind" "exit")
                                                    "direction" "cloud")
                                                "nick" "cloud")
                                            "description" "A cloudward passage."))))
            "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:sky#cloud-exit", &avatar)),
        );
        eval_all(
            r#"
                                                (let ((ctx (map-set
                                                                            (map-set
                                                                                (map-set
                                                                                    (map-set
                                                                                        (map-set (make-map) "did" "did:ma:did")
                                                                                        "avatar" (local-self))
                                                                                    "kind" "avatar")
                                                                                "room" "did:ma:ma#cloud")
                                                                            "exit" "did:ma:sky#cloud-exit")))
                                                    ((find-method :ctx) (list ctx) msg))
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:ma#cloud"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :enter)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"did\")",
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
            (send-exit-ctx! "did:ma:runtime#avatar" "did:ma:did" "north" "did:ma:runtime#dead-exit")
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
                "(list :ctx (avatar-exit-ctx \"did:ma:runtime#avatar\" \"did:ma:did\"))",
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
    fn avatar_start_initialises_deterministic_inventory_without_pending_state() {
        let env = avatar_env();
        let did = "did:ma:owner";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#avatar".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "did" "did:ma:owner")
            (define (entity-live? actor) #f)
            (define (ma-create-actor kind behaviour init fragment)
              (set-prop! "created-kind" kind)
              (set-prop! "created-fragment" fragment)
              fragment)
            (on-signal (list :start))
            "#,
            &env,
        )
        .unwrap();

        let expected_fragment = eval_str(&format!(r#"(inventory-fragment "{did}")"#), &env);
        assert_eq!(
            eval_str("(get-prop \"inventory\")", &env),
            format!("did:ma:runtime#{expected_fragment}")
        );
        assert_eq!(
            eval_str("(get-prop \"created-kind\")", &env),
            "/ma/container/0.0.1"
        );
        assert_eq!(
            eval_str("(get-prop \"created-fragment\")", &env),
            expected_fragment
        );
        assert!(eval_bool("(not (has-prop? \"pending-take\"))", &env));
    }

    #[test]
    fn avatar_start_preserves_supplied_cross_runtime_inventory() {
        let env = avatar_env();
        let inventory = "did:ma:source-runtime#inventory";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:target-runtime".to_string());
        config.insert(
            "self".to_string(),
            "did:ma:target-runtime#avatar".to_string(),
        );
        crate::state::set_config(config);

        eval_all(
            &format!(
                r#"
                (define (ma-create-actor kind behaviour init fragment)
                  (set-prop! "created-fragment" fragment)
                  fragment)
                (set-prop! "inventory" "{inventory}")
                (on-signal (list :start))
                "#
            ),
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(get-prop \"inventory\")", &env), inventory);
        assert!(!eval_bool("(has-prop? \"created-fragment\")", &env));
    }

    #[test]
    fn root_entry_forwards_supplied_cross_runtime_inventory() {
        let env = root_actor_env();
        let inventory = "did:ma:source-runtime#inventory";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:target-runtime".to_string());
        config.insert("self".to_string(), "did:ma:target-runtime#root".to_string());
        crate::state::set_config(config);

        assert_eq!(
            eval_str(
                &format!(r#"(requested-inventory (list "" "" "{inventory}"))"#),
                &env,
            ),
            inventory
        );

        eval_all(
            &format!(
                r#"
                (define (ma-entity-exists? actor) #t)
                (ensure-avatar "did:ma:owner" "Owner" "did:ma:target-runtime#room" "{inventory}")
                "#
            ),
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_all(
                "(car (cdr (cdr (cdr (cdr (get-prop \"sent-term\"))))))",
                &env
            )
            .unwrap(),
            Value::str(inventory)
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
            (test-avatar-claim! owner-avatar "me")
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
