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

    fn remember_exit(
        env: &Rc<Env>,
        actor: &str,
        direction: &str,
        target_room: &str,
        target_name: &str,
    ) {
        env.define(Rc::from("exit-actor"), Value::str(actor));
        env.define(Rc::from("exit-direction"), Value::str(direction));
        env.define(Rc::from("exit-target-room"), Value::str(target_room));
        env.define(Rc::from("exit-target-name"), Value::str(target_name));
        eval_all(
            r#"
            (remember-child!
              (make-map "actor" exit-actor
                        "kind" "exit"
                        "protocol" "/ma/exit/0.0.1"
                        "parent" "did:ma:runtime#room"
                        "name" exit-direction
                        "nick" exit-direction
                        "description" "An exit."
                        "direction" exit-direction
                        "target-room" exit-target-room
                        "target-name" exit-target-name))
            "#,
            env,
        )
        .unwrap();
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

    fn house_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../../actors/house.ma"), &env).unwrap();
        eval_all(
            "(define (ma-send! target term) #f) (define (ma-reply! msg term) #f) (define (ma-save-state!) #f)",
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

    fn runtime_ctx_actor_env() -> Rc<Env> {
        crate::state::load_from_cbor(&empty_state_cbor()).unwrap();
        let env = new_root_env();
        crate::state::install(&env);
        crate::msg::install(&env);
        eval_all(include_str!("../stdlib.ma"), &env).unwrap();
        eval_all(include_str!("../actor.ma"), &env).unwrap();
        eval_all(include_str!("../state.ma"), &env).unwrap();
        eval_all("(define (ma-save-state!) #f)", &env).unwrap();
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
            eval_all("(list :child (child-ack-ctx collar_ctx))", &env).unwrap()
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
    fn node_forge_creates_child_and_replies_with_new_entity_url() {
        let env = node_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (define (ma-create-actor kind behaviour init fragment)
              (set-prop! "forge-kind" kind)
              (set-prop! "forge-behaviour" (if behaviour behaviour "none"))
              (set-prop! "forge-init" init)
              (set-prop! "forge-fragment" (if fragment fragment "none"))
              "newthing1")
            "#,
            &env,
        )
        .unwrap();
        let ctx = eval_all(
            r#"(map-set (map-set (map-set (make-map) "kind" "/ma/thing/0.0.1") "name" "Lamp") "owner" "did:ma:sneaky")"#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("forge_ctx"), ctx.clone());
        env.define(
            Rc::from("forge_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":forge"), ctx]),
            )),
        );

        eval_all("(on-message forge_msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"forge-kind\")", &env),
            "/ma/thing/0.0.1"
        );
        assert_eq!(eval_str("(get-prop \"forge-behaviour\")", &env), "none");
        assert_eq!(eval_str("(get-prop \"forge-fragment\")", &env), "none");
        let init = eval_str("(get-prop \"forge-init\")", &env);
        assert!(init.contains("(set-init-prop! \"name\" \"Lamp\")"));
        assert!(init.contains("(set-init-prop! \"owner\" \"did:ma:owner\")"));
        assert!(init.contains("(set-init-prop! \"parent\" \"did:ma:runtime#room\")"));
        assert!(!init.contains("did:ma:sneaky"));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("did:ma:runtime#newthing1")
            ])
        );
    }

    #[test]
    fn node_forge_rejects_ctx_missing_kind_or_name() {
        let env = node_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        let ctx = eval_all(r#"(map-set (make-map) "kind" "/ma/thing/0.0.1")"#, &env).unwrap();
        env.define(Rc::from("forge_ctx"), ctx.clone());
        env.define(
            Rc::from("forge_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":forge"), ctx]),
            )),
        );

        eval_all("(on-message forge_msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("usage: :forge <ctx with kind, name>")
            ])
        );
    }

    #[test]
    fn node_forge_refuses_when_max_children_reached() {
        let env = node_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);
        eval_all(r#"(set-prop! "max-children" "0")"#, &env).unwrap();
        let ctx = eval_all(
            r#"(map-set (map-set (make-map) "kind" "/ma/thing/0.0.1") "name" "Lamp")"#,
            &env,
        )
        .unwrap();
        env.define(Rc::from("forge_ctx"), ctx.clone());
        env.define(
            Rc::from("forge_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":forge"), ctx]),
            )),
        );

        eval_all("(on-message forge_msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("forge refused: max-children limit reached")
            ])
        );
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
    fn duck_registers_distinct_quack_and_duck_schedules() {
        let env = duck_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        crate::state::set_config(config);
        install_send_reply_recorders(&env);

        eval_all(
            r#"
            (duck-schedule-quack!)
            (duck-schedule-duck!)
            (duck-schedule-quack!)
            (duck-schedule-duck!)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::str("quack"),
                Value::symbol(":random"),
                Value::Int(600),
                Value::symbol(":quack"),
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::str("duck"),
                Value::symbol(":random"),
                Value::Int(600),
                Value::symbol(":duck"),
            ])
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
                    Value::symbol(":set-parent"),
                    Value::symbol(":hold"),
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
                    Value::symbol(":set-parent"),
                    Value::symbol(":hold"),
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
                    Value::symbol(":set-parent"),
                    Value::symbol(":hold"),
                    Value::symbol(":recycle"),
                    Value::symbol(":fortune"),
                ])
            ])
        );
    }

    #[test]
    fn room_direct_did_enter_commits_child_ctx_and_replies() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let did = "did:ma:did";
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:ma".to_string());
        config.insert("self".to_string(), "did:ma:ma#cloud".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
                        (set-prop! "ctx"
                            (make-map "runtime" "did:ma:ma"
                                                "root" "did:ma:ma#root"
                                                "house" "did:ma:world#house44"
                                                "scheduler" "did:ma:ma#scheduler"
                                                "rev" 1))
                        "#,
            &env,
        )
        .unwrap();

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg(did, "did:ma:ma#cloud")),
        );
        env.define(Rc::from("did"), Value::str(did));
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
            eval_str("(ctx-text (did-ctx did) \"parent\")", &env),
            "did:ma:ma#cloud"
        );
        assert_eq!(eval_str("(ctx-text (did-ctx did) \"actor\")", &env), did);
        assert!(eval_bool("(map-has-key? (children-map) did)", &env));
        assert_eq!(eval_str("(speaker-name did)", &env), "Pondus");
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(eval_str("(who-text)", &env), "Who: Pondus");
        assert_eq!(
            eval_str("(get-prop \"sent-target:2\")", &env),
            "did:ma:world#house44"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :did-ctx)",
            &env
        ));
    }

    #[test]
    fn room_who_replies_with_cbor_encodable_child_derived_map() {
        let env = room_env();
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (set-did-ctx! "did:ma:duckie"
              (make-map "did" "did:ma:duckie"
                        "parent" "did:ma:runtime#construct"
                        "name" "Duckie"
                        "rev" 5))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:caller", "did:ma:runtime#construct")),
        );

        eval_all("((find-method :who?) '() msg)", &env).unwrap();

        let reply = eval_all("(get-prop \"reply-term:1\")", &env).unwrap();
        crate::cbor::encode(&reply).unwrap();
        assert!(eval_bool(
            "(map? (car (cdr (get-prop \"reply-term:1\"))))",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (map-ref (car (cdr (get-prop \"reply-term:1\"))) \"did:ma:duckie\") \"name\")",
                &env
            ),
            "Duckie"
        );
    }

    #[test]
    fn room_direct_did_entry_broadcasts_typed_arrival() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-did-ctx! "did:ma:bob"
              (make-map "did" "did:ma:bob"
                        "parent" "did:ma:runtime#room"
                        "name" "Bob"
                        "nick" "Bob"
                        "description" "A visitor."
                        "rev" 1))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:alice", "did:ma:runtime#room")),
        );

        eval_all("((find-method :enter) (list \"Alice\") msg)", &env).unwrap();

        assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:bob");
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :arrive)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"did\")",
                &env
            ),
            "did:ma:alice"
        );
    }

    #[test]
    fn house_replaces_did_parent_after_notifying_previous_room() {
        let env = house_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:ma#spa", "did:ma:ma#house")),
        );
        eval_all(
            r#"
                (set-prop! "did-ctxs"
                  (map-set (make-map) "did:ma:alice"
                  (make-map "did" "did:ma:alice"
                      "parent" "did:ma:ma#lobby"
                          "name" "Alice"
                          "nick" "Alice"
                          "description" "A visitor."
                          "rev" 1)))
                ((find-method :did-ctx)
                  (list "did:ma:alice"
                  (make-map "did" "did:ma:alice"
                      "parent" "did:ma:ma#spa"
                          "name" "Alice"
                          "nick" "Alice"
                          "description" "A visitor."
                          "rev" 2))
                  msg)
                "#,
            &env,
        )
        .unwrap();
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:ma#lobby"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :leave)",
            &env
        ));
        assert_eq!(
            eval_str("(car (cdr (get-prop \"sent-term:1\")))", &env),
            "did:ma:alice"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (map-ref (get-prop \"did-ctxs\") \"did:ma:alice\") \"parent\")",
                &env
            ),
            "did:ma:ma#spa"
        );
    }

    #[test]
    fn house_rejects_did_ctx_with_a_mismatched_identity() {
        let env = house_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:ma#spa", "did:ma:ma#house")),
        );
        eval_all(
            r#"
                (set-prop! "did-ctxs" (make-map))
                ((find-method :did-ctx)
                  (list "did:ma:alice"
                    (make-map "did" "did:ma:bob"
                          "parent" "did:ma:ma#spa"
                          "name" "Alice"
                          "nick" "Alice"
                          "description" "A visitor."
                          "rev" 1))
                  msg)
                "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool(
            "(equal? (car (get-prop \"reply-term:1\")) :error)",
            &env
        ));
        assert!(!eval_bool(
            "(map-has-key? (house-map \"did-ctxs\") \"did:ma:alice\")",
            &env
        ));
    }

    #[test]
    fn house_keys_entity_ctx_by_full_sender_without_duplicate_identity() {
        let env = house_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:world#lamp", "did:ma:ma#house")),
        );
        eval_all(
            r#"
                ((find-method :entity-ctx)
                  (list
                    (make-map "kind" "thing"
                          "protocol" "/ma/thing/0.0.1"
                          "name" "Lamp"
                          "nick" "lamp"
                          "description" "A lamp."
                          "parent" "did:ma:world#room"
                          "rev" 1))
                  msg)
                "#,
            &env,
        )
        .unwrap();
        assert_eq!(
            eval_str(
                "(ctx-text (map-ref (get-prop \"entity-ctxs\") \"did:ma:world#lamp\") \"kind\")",
                &env
            ),
            "thing"
        );
        assert!(!eval_bool(
            "(map-has-key? (map-ref (get-prop \"entity-ctxs\") \"did:ma:world#lamp\") \"actor\")",
            &env
        ));
    }

    #[test]
    fn room_only_accepts_targeted_did_leave_from_house() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
                        r#"
                                                (set-prop! "ctx"
                                                    (make-map "runtime" "did:ma:runtime"
                                                                        "root" "did:ma:runtime#root"
                                                                        "house" "did:ma:world#house44"
                                                                        "scheduler" "did:ma:runtime#scheduler"
                                                                        "rev" 1))
                                                (set-did-ctx! "did:ma:alice"
                                                    (make-map "parent" "did:ma:runtime#room"
                                                              "name" "Alice"
                                                              "nick" "Alice"
                                                              "description" "A visitor."
                                                              "rev" 1))
                        "#,
                        &env,
                )
                .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:world#house44", "did:ma:runtime#room")),
        );
        eval_all("((find-method :leave) (list \"did:ma:alice\") msg)", &env).unwrap();
        assert_eq!(
            eval_bool("(member-string? \"did:ma:alice\" (did-occupants))", &env),
            false
        );
    }

    #[test]
    fn room_ctx_includes_structured_exits() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        assert!(eval_bool("(null? (map-ref (room-ctx) \"exits\"))", &env));

                eval_all(
                        r#"
                        (remember-child!
                            (make-map "actor" "did:ma:runtime#north-exit"
                                                "kind" "exit"
                                                "protocol" "/ma/exit/0.0.1"
                                                "parent" "did:ma:runtime#room"
                                                "name" "north"
                                                "nick" "north"
                                                "description" "An exit leading north."
                                                "direction" "north"
                                                "target-room" "did:ma:runtime#kitchen"))
                        "#,
                        &env,
                )
                .unwrap();

        assert_eq!(
            eval_str(
                "(ctx-text (car (map-ref (room-ctx) \"exits\")) \"direction\")",
                &env
            ),
            "north"
        );
    }

    #[test]
    fn room_queries_reply_without_printing() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:alice", "did:ma:runtime#room")),
        );

        eval_all("((find-method :look) '() msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert!(eval_bool(
            "(equal? (car (get-prop \"reply-term:1\")) :ok)",
            &env
        ));
        assert!(eval_bool(
            "(map? (car (cdr (get-prop \"reply-term:1\"))))",
            &env
        ));
        assert!(eval_bool(
            "(map-has-key? (car (cdr (get-prop \"reply-term:1\"))) \"exits\")",
            &env
        ));
        assert!(eval_bool("(not (has-prop? \"sent-count\"))", &env));
    }

    #[test]
    fn room_speech_events_are_typed_and_acknowledged() {
        let env = room_env();
        install_send_reply_recorders(&env);
        eval_all(
            r#"
            (set-did-ctx! "did:ma:alice"
                            (make-map "did" "did:ma:alice"
                                                "parent" "did:ma:runtime#room"
                        "name" "Alice"
                        "nick" "Alice"
                        "description" "A visitor."
                        "rev" 1))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:alice", "did:ma:runtime#room")),
        );

        eval_all("((find-method :say) (list \"hello\") msg)", &env).unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :say)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"did\")",
                &env
            ),
            "did:ma:alice"
        );
        assert_eq!(
            eval_str("(car (cdr (cdr (get-prop \"sent-term:1\"))))", &env),
            "hello"
        );
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert!(eval_bool("(equal? (get-prop \"reply-term:1\") :ok)", &env));
    }

    #[test]
    fn room_announces_actor_child_departure() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (remember-child!
              (make-map "actor" "did:ma:runtime#duckie"
                        "kind" "thing"
                        "protocol" "/ma/thing/0.0.1"
                        "parent" "did:ma:runtime#room"
                        "name" "duckie"
                        "nick" "Duckie"
                        "description" "A duck."))
                        (set-did-ctx! "did:ma:alice"
                            (make-map "parent" "did:ma:runtime#room"
                                                "name" "Alice"
                                                "nick" "Alice"
                                                "description" "A visitor."
                                                "rev" 1))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#duckie", "did:ma:runtime#room")),
        );
        eval_all(
            r#"
            ((find-method :parent)
              (list (make-map "actor" "did:ma:runtime#duckie"
                              "kind" "thing"
                              "protocol" "/ma/thing/0.0.1"
                              "parent" "did:ma:runtime#other"
                              "name" "duckie"
                              "nick" "Duckie"
                              "description" "A duck."))
              msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :leave)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#duckie"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :child)",
            &env
        ));
        assert!(eval_bool(
            "(not (map-has-key? (children-map) \"did:ma:runtime#duckie\"))",
            &env
        ));
    }

    #[test]
    fn room_announces_visible_actor_arrival() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-did-ctx! "did:ma:alice"
              (make-map "parent" "did:ma:runtime#room"
                        "name" "Alice"
                        "nick" "Alice"
                        "description" "A visitor."
                        "rev" 1))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#duckie", "did:ma:runtime#room")),
        );
        eval_all(
            r#"
            ((find-method :parent)
              (list (make-map "actor" "did:ma:runtime#duckie"
                              "kind" "thing"
                              "protocol" "/ma/thing/0.0.1"
                              "parent" "did:ma:runtime#room"
                              "name" "duckie"
                              "nick" "Duckie"
                              "description" "A duck."))
              msg)
            "#,
            &env,
        )
        .unwrap();

        // The arrival goes to Alice, then the room refreshes every child ctx:
        // Alice's direct DID entry and duckie both receive one :child.
        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 3);
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :arrive)",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
                &env
            ),
            "did:ma:runtime#duckie"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:2\")) :child)",
            &env
        ));
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:3\")) :child)",
            &env
        ));
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
                        "#,
            &env,
        )
        .unwrap();
        remember_exit(
            &env,
            "did:ma:runtime#north-exit",
            "north",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );

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
            (test-agent-claim! "did:ma:runtime#rms" "rms")
            (on-event :leave-occupant '() msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(occupants-text)", &env), "Occupants: none.");
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
    fn room_removes_recycled_thing_with_empty_parent_departure_ctx() {
        // Regression test: recycle sends a departure ctx with an explicitly
        // empty "parent" (there is no new parent - the thing is terminating),
        // not a real did-url like the take/drop reparenting flow. The room
        // must still forget the thing.
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
                                "parent" "did:ma:runtime#room")
                            "name" "lamp")
                        "nick" "Aladdins lampe")
                    "description" "A warm brass lamp."))
            "#,
            &env,
        )
        .unwrap();

        let departed_ctx = eval_all(
            r#"(map-set (claim-ctx "did:ma:runtime#lamp") "parent" "")"#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("recycle_departure_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":parent"), departed_ctx]),
            )),
        );

        eval_all("(on-message recycle_departure_msg)", &env).unwrap();

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
    fn room_announces_root_on_init_and_start() {
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
                        (set-prop! "ctx"
                            (make-map "runtime" "did:ma:runtime"
                                                "root" "did:ma:runtime#root"
                                                "house" "did:ma:world#house44"
                                                "scheduler" "did:ma:runtime#scheduler"
                                                "rev" 1))
            (on-signal :init)
            (on-signal :start)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#root"
        );
        assert!(eval_bool(
            r#"(let ((term (get-prop "sent-term:1")))
                 (and (equal? (car term) :parent)
                      (equal? (ctx-text (car (cdr term)) "actor") "did:ma:runtime#room")
                      (equal? (ctx-text (car (cdr term)) "parent") "did:ma:runtime#root")))"#,
            &env,
        ));
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
                    Value::symbol(":set-parent"),
                    Value::symbol(":hold"),
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
    fn unowned_container_without_recovery_secret_can_be_claimed() {
        let env = container_env();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#bag")),
        );

        eval_all("((find-method :claim) '() msg)", &env).unwrap();

        assert_eq!(eval_str("(owner)", &env), "did:ma:owner");
    }

    #[test]
    fn already_claimed_thing_explains_how_to_reclaim() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#lamp")),
        );
        eval_all("(set-owner! \"did:ma:owner\")", &env).unwrap();

        eval_all("((find-method :claim) '() msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("already claimed. Reclaim with :claim <secret>")
            ])
        );
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
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 1);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner may recycle this thing"),
            ])
        );
    }

    #[test]
    fn thing_owner_can_recycle_directly_without_parent_proxy() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#lamp")),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "parent" "did:ma:runtime#room")
            ((find-method :recycle) '() owner_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(node-parent)", &env), "");
        assert_eq!(eval_str("(get-prop \"ended\")", &env), "yes");
        assert!(eval_bool("(not (has-prop? \"reply-count\"))", &env));
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
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :set-parent) (list "did:ma:runtime#room") owner_msg)
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
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("set-parent requested")
            ])
        );
    }

    #[test]
    fn thing_unowned_can_be_reparented_without_claiming_it() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("stranger_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:stranger",
                "did:ma:runtime#lamp",
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :set-parent) (list "did:ma:runtime#inventory") stranger_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_all("(get-prop \"owner\")", &env).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("set-parent requested")
            ])
        );
    }

    #[test]
    fn thing_unowned_can_be_held_directly_by_a_stranger() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("stranger_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:stranger",
                "did:ma:runtime#lamp",
                Value::symbol(":hold"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :hold) '() stranger_msg)
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool("(not (get-prop \"owner\"))", &env));
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:stranger"
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("hold requested")])
        );
    }

    #[test]
    fn thing_owned_by_another_can_still_be_held_by_anyone_in_room() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("stranger_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:stranger",
                "did:ma:runtime#lamp",
                Value::symbol(":hold"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :hold) '() stranger_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(get-prop \"owner\")", &env), "did:ma:owner");
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("hold requested")])
        );
    }

    #[test]
    fn thing_owned_by_another_can_still_be_dropped_by_its_current_holder() {
        // Parenting != ownership: whoever currently holds/carries a thing
        // (its parent) may relocate it further, even if someone else owns
        // it — regression test for the removed "only owner may set-parent
        // this actor" gate that used to block exactly this.
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("holder_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:holder",
                "did:ma:runtime#lamp",
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "parent" "did:ma:holder")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            ((find-method :set-parent) (list "did:ma:runtime#room") holder_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(get-prop \"owner\")", &env), "did:ma:owner");
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("set-parent requested")
            ])
        );
    }

    #[test]
    fn thing_hold_refuses_and_resyncs_when_caller_not_in_cached_room() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("stranger_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:stranger",
                "did:ma:runtime#lamp",
                Value::symbol(":hold"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            (set-parent-ctx!
              (make-map "actor" "did:ma:runtime#lamp"
                        "parent" "did:ma:runtime#room"
                        "parent-ctx" (make-map "kind" "room"
                                               "who" (make-map "did:ma:other" #t))))
            ((find-method :hold) '() stranger_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("hold refused: not in the same room, try again")
            ])
        );
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#room"
        );
        assert!(eval_bool(
            "(equal? (car (get-prop \"sent-term:1\")) :parent)",
            &env
        ));
    }

    #[test]
    fn thing_hold_succeeds_when_caller_is_in_cached_room() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("stranger_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:stranger",
                "did:ma:runtime#lamp",
                Value::symbol(":hold"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "parent" "did:ma:runtime#room")
            (set-prop! "name" "lamp")
            (set-prop! "nick" "brass lamp")
            (set-prop! "description" "A warm brass lamp.")
            (set-parent-ctx!
              (make-map "actor" "did:ma:runtime#lamp"
                        "parent" "did:ma:runtime#room"
                        "parent-ctx" (make-map "kind" "room"
                                               "who" (make-map "did:ma:stranger" #t))))
            ((find-method :hold) '() stranger_msg)
            "#,
            &env,
        )
        .unwrap();

        assert!(eval_bool("(not (get-prop \"owner\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("hold requested")])
        );
    }

    #[test]
    fn room_drop_is_a_capacity_precheck_only() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:avatar", "did:ma:runtime#room")),
        );

        eval_all("((find-method :drop) '() msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
        assert!(eval_bool("(not (get-prop \"sent-count\"))", &env));

        eval_all(
            r#"
            (set-prop! "max-children" "0")
            ((find-method :drop) '() msg)
            "#,
            &env,
        )
        .unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("drop refused: room is full")
            ])
        );

        eval_all("((find-method :drop) (list \"extra\") msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":error"), Value::str("usage: :drop")])
        );
    }

    #[test]
    fn container_lock_allows_owner_or_current_secret_to_unlock() {
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
        env.define(
            Rc::from("guest_msg"),
            Value::Msg(sample_msg("did:ma:guest", "did:ma:runtime#bag")),
        );

        eval_all(
            r#"
            ((find-method :lock) '() owner_msg)
            ((find-method :claim) '() owner_msg)
            ((find-method :lock) (list "supersecret") owner_msg)
            ((find-method :contents?) '() owner_msg)
            ((find-method :unlock) (list "wrong") guest_msg)
            ((find-method :unlock) (list "supersecret") guest_msg)
            ((find-method :lock) (list "newsecret") owner_msg)
            ((find-method :unlock) (list "supersecret") guest_msg)
            ((find-method :unlock) (list "newsecret") guest_msg)
            ((find-method :lock) '() owner_msg)
            ((find-method :contents?) '() owner_msg)
            ((find-method :unlock) '() owner_msg)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(eval_str("(get-prop \"owner\")", &env), "did:ma:owner");
        assert_eq!(eval_str("(get-prop \"lock-secret\")", &env), "newsecret");
        assert_eq!(eval_int("(get-prop \"reply-count\")", &env), 12);
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner may lock this container")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("claimed")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("locked")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:4\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("The container is locked.")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:5\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner or lock secret may unlock this container")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:7\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("locked")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:8\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner or lock secret may unlock this container")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:9\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("unlocked")])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:11\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("The container is locked.")
            ])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:12\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::str("unlocked")])
        );
    }

    #[test]
    fn container_contents_reflect_admitted_and_departed_children() {
        // :put-in/:take-from no longer exist — items admit themselves into a
        // container via the generic :parent/:child handshake (:set-parent on
        // the item side), and :contents? is just container.ma's contents-map
        // alias for node.ma's shared children-map. This exercises that wiring
        // directly, without re-testing the handshake itself (already covered
        // by the node_* admission/departure tests).
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
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#bag",
                Value::list(vec![Value::symbol(":parent"), child_ctx.clone()]),
            )),
        );

        eval_all("(on-message parent_msg)", &env).unwrap();
        eval_all("((find-method :contents?) '() parent_msg)", &env).unwrap();

        let departure_ctx = eval_all(
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
                                        "parent" "")
                  "name" "lamp")
                "nick" "brass lamp")
              "description" "A warm brass lamp.")
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("departure_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#lamp",
                "did:ma:runtime#bag",
                Value::list(vec![Value::symbol(":parent"), departure_ctx]),
            )),
        );
        eval_all("(on-message departure_msg)", &env).unwrap();
        eval_all("((find-method :contents?) '() parent_msg)", &env).unwrap();

        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::list(vec![child_ctx])])
        );
        assert_eq!(
            eval_all("(get-prop \"reply-term:4\")", &env).unwrap(),
            Value::list(vec![Value::symbol(":ok"), Value::list(vec![])])
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
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            ((find-method :set-parent)
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
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-prop! "name" "bag")
            (set-prop! "nick" "canvas bag")
            (set-prop! "description" "A sturdy canvas bag.")
            ((find-method :set-parent) (list "did:ma:runtime#room") owner_msg)
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
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("set-parent requested")
            ])
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
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            ((find-method :set-parent)
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
            Value::list(vec![
                Value::symbol(":ok"),
                Value::str("set-parent requested")
            ])
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
                Value::symbol(":set-parent"),
            )),
        );

        eval_all(
            r#"
            ((find-method :set-parent)
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
        remember_exit(
            &env,
            "did:ma:runtime#north-exit",
            "north",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );
        assert_eq!(eval_str("(random-exit-direction)", &env), "north");
    }

    #[test]
    fn room_exit_lookup_accepts_non_ascii_direction() {
        let env = room_env();
        remember_exit(
            &env,
            "did:ma:runtime#door-exit",
            "dør",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );

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

        remember_exit(
            &env,
            "did:ma:runtime#north-exit",
            "north",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );

        assert_eq!(eval_str("(exits-text)", &env), "Exits: north");
        assert_eq!(
            eval_str("(exit-target \"north\")", &env),
            "did:ma:runtime#north-exit"
        );
    }

    #[test]
    fn room_start_migrates_legacy_exit_properties() {
        let env = room_env();
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);

        eval_all(
            r#"
            (set-prop! "exits" (make-map "north" "did:ma:runtime#north-exit"))
            (set-prop! "exit-target:north" "did:ma:runtime#kitchen")
            (set-prop! "exit-target-name:north" "Kitchen")
            (on-signal :start)
            "#,
            &env,
        )
        .unwrap();

        assert_eq!(
            eval_str("(exit-target \"north\")", &env),
            "did:ma:runtime#north-exit"
        );
        assert_eq!(
            eval_str("(exit-room-target \"north\")", &env),
            "did:ma:runtime#kitchen"
        );
        assert_eq!(
            eval_str("(ctx-text (exit-ctx \"north\") \"target-name\")", &env),
            "Kitchen"
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
            eval_str("(exit-init \"dør\" \"did:ma:runtime#kitchen\" #f)", &env),
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
            eval_str("(exit-init \"north\" \"did:ma:runtime#kitchen\" #f)", &env),
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
            (define (ma-send! target term)
              (inc-prop! "sent-count" 1)
              (set-prop! (string-append "sent-target:" (number->string (get-prop "sent-count"))) target)
              (set-prop! (string-append "sent-term:" (number->string (get-prop "sent-count"))) term))
            "#,
            &env,
        )
        .unwrap();
        remember_exit(
            &env,
            "did:ma:runtime#north-exit",
            "north",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );

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
    fn room_removes_exit_from_children() {
        let env = room_env();
        remember_exit(
            &env,
            "did:ma:runtime#north-exit",
            "north",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );
        eval_all("(forget-child! \"did:ma:runtime#north-exit\")", &env).unwrap();

        assert_eq!(eval_str("(exits-text)", &env), "Exits: none.");
        assert!(eval_bool("(not (exit-target \"north\"))", &env));
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
    fn room_fill_broadcasts_typed_event() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (set-did-ctx! "did:ma:owner"
              (make-map "did" "did:ma:owner"
                        "parent" "did:ma:runtime#room"
                        "name" "Owner"
                        "nick" "Owner"
                        "description" "A visitor."
                        "rev" 1))
            (set-did-ctx! "did:ma:observer"
              (make-map "did" "did:ma:observer"
                        "parent" "did:ma:runtime#room"
                        "name" "Observer"
                        "nick" "Observer"
                        "description" "A visitor."
                        "rev" 1))
            "#,
            &env,
        )
        .unwrap();
        remember_exit(
            &env,
            "did:ma:runtime#north-exit",
            "north",
            "did:ma:runtime#kitchen",
            "Kitchen",
        );
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:owner", "did:ma:runtime#room")),
        );

        eval_all("((find-method :fill) (list \"north\") msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#north-exit"
        );
        eval_all(
            r#"
            (define (fill-event-term index)
              (if (> index (get-prop "sent-count"))
                  #f
                  (let ((term (get-prop (string-append "sent-term:" (number->string index)))))
                                        (if (and (pair? term)
                                                         (equal? (car term) :fill)
                                                         (pair? (cdr term))
                                                         (map? (car (cdr term))))
                        term
                        (fill-event-term (+ index 1))))))
            "#,
            &env,
        )
        .unwrap();
        assert!(eval_bool(
            "(let ((term (fill-event-term 1))) (and term (map? (car (cdr term)))))",
            &env
        ));
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (fill-event-term 1))) \"did\")",
                &env
            ),
            "did:ma:owner"
        );
        assert_eq!(
            eval_str("(car (cdr (cdr (fill-event-term 1))))", &env),
            "north"
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
    fn room_remembers_named_dig_target_for_idempotency() {
        let env = room_env();
        remember_exit(
            &env,
            "did:ma:runtime#door-exit",
            "dør",
            "did:ma:runtime#kitchen",
            "køkken",
        );
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
    fn room_dig_links_only_full_did_urls() {
        let env = room_env();

        assert_eq!(
            eval_str(
                r#"(existing-room-target "did:ma:runtime#kitchen")"#,
                &env
            ),
            "did:ma:runtime#kitchen"
        );
        assert!(eval_bool(
            r#"(not (existing-room-target "did:ma:runtime"))"#,
            &env
        ));
        assert!(eval_bool(
            r#"(not (existing-room-target "bar"))"#,
            &env
        ));
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
    fn root_orphan_for_live_actor_forwards_owner_and_old_parent() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        config.insert("root".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (define (ma-entity-exists? actor)
              (equal? actor "did:ma:runtime#lamp"))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:owner",
                "did:ma:runtime#root",
                Value::list(vec![
                    Value::symbol(":orphan"),
                    Value::str("did:ma:runtime#lamp"),
                    Value::str("from"),
                    Value::str("did:ma:other#room"),
                ]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":orphan-root"),
                Value::str("did:ma:owner"),
                Value::str("did:ma:other#room"),
            ]),
        );
    }

    #[test]
    fn root_ctx_uses_only_configured_full_service_did_urls() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        config.insert("house".to_string(), "did:ma:world#house44".to_string());
        config.insert(
            "scheduler".to_string(),
            "did:ma:runtime#scheduler".to_string(),
        );
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:caller", "did:ma:runtime#root")),
        );

        eval_all("((find-method :ctx?) '() msg)", &env).unwrap();

        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"runtime\")",
                &env
            ),
            "did:ma:runtime"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"root\")",
                &env
            ),
            "did:ma:runtime#root"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"house\")",
                &env
            ),
            "did:ma:world#house44"
        );

        let mut fragment_config = std::collections::HashMap::new();
        fragment_config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        fragment_config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        fragment_config.insert("house".to_string(), "#house".to_string());
        fragment_config.insert(
            "scheduler".to_string(),
            "did:ma:runtime#scheduler".to_string(),
        );
        crate::state::set_config(fragment_config);

        eval_all("((find-method :ctx?) '() msg)", &env).unwrap();

        assert_eq!(
            eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
            Value::symbol(":error")
        );
    }

    #[test]
    fn root_enter_replies_with_configured_start_room() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        config.insert("start".to_string(), "did:ma:runtime#construct".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:caller", "did:ma:runtime#root")),
        );

        eval_all("((find-method :enter?) '() msg)", &env).unwrap();

        assert_eq!(
            eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
            Value::symbol(":ok")
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#construct"
        );
    }

    #[test]
    fn root_enter_errors_without_a_configured_start_room() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:caller", "did:ma:runtime#root")),
        );

        eval_all("((find-method :enter?) '() msg)", &env).unwrap();

        assert_eq!(
            eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
            Value::symbol(":error")
        );
    }

    #[test]
    fn root_registers_local_full_actor_and_sends_its_runtime_ctx() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        config.insert("house".to_string(), "did:ma:world#house44".to_string());
        config.insert(
            "scheduler".to_string(),
            "did:ma:runtime#scheduler".to_string(),
        );
        crate::state::set_config(config);
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_msg("did:ma:runtime#lamp", "did:ma:runtime#root")),
        );

        eval_all("((find-method :register) '() msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#lamp"
        );
        assert_eq!(
            eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
            Value::symbol(":ctx")
        );
        assert!(eval_bool(
            "(map-ref (get-prop \"subscribers\") \"did:ma:runtime#lamp\" #f)",
            &env
        ));
    }

    #[test]
    fn actor_accepts_runtime_ctx_only_from_its_full_root_did_url() {
        let env = runtime_ctx_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (define runtime-ctx-test
              (map-set
                (map-set
                  (map-set
                    (map-set
                      (map-set (make-map) "runtime" "did:ma:runtime")
                      "root" "did:ma:runtime#root")
                    "house" "did:ma:world#house44")
                  "scheduler" "did:ma:runtime#scheduler")
                "rev" 1))
            "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#root",
                "did:ma:runtime#lamp",
                Value::list(vec![
                    Value::symbol(":ctx"),
                    eval_all("runtime-ctx-test", &env).unwrap(),
                ]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
            Value::symbol(":ok")
        );
        assert_eq!(
            eval_str("(ctx-text (get-prop \"ctx\") \"house\")", &env),
            "did:ma:world#house44"
        );

        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:other#root",
                "did:ma:runtime#lamp",
                Value::list(vec![
                    Value::symbol(":ctx"),
                    eval_all("runtime-ctx-test", &env).unwrap(),
                ]),
            )),
        );
        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
            Value::symbol(":error")
        );
    }

    #[test]
    fn root_orphan_for_unavailable_actor_sends_signed_repair_to_named_parent() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        config.insert("root".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        eval_all("(define (ma-entity-exists? actor) #f)", &env).unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:other#room",
                "did:ma:runtime#root",
                Value::list(vec![
                    Value::symbol(":orphan"),
                    Value::str("did:ma:runtime#lamp"),
                    Value::str("from"),
                    Value::str("did:ma:other#room"),
                ]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:other#room"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"kind\")",
                &env
            ),
            "orphan"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#root"
        );
    }

    #[test]
    fn room_owner_can_evict_unreachable_orphan_child_idempotently() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        config.insert("root".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
            (set-prop! "owner" "did:ma:owner")
            (test-agent-claim! "did:ma:other#orphan" "orphan")
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
                    Value::symbol(":orphan"),
                    Value::str("did:ma:other#orphan"),
                ]),
            )),
        );

        eval_all("(on-message msg) (on-message msg)", &env).unwrap();

        assert!(eval_bool("(not (child-ctx \"did:ma:other#orphan\"))", &env));
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn live_thing_accepts_root_orphan_adoption_only_for_its_owner() {
        let env = thing_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#lamp".to_string());
        config.insert("root".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        eval_all(
            r#"
                        (set-prop! "owner" "did:ma:owner")
                        (set-prop! "parent" "did:ma:other#room")
                        "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("owner_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#root",
                "did:ma:runtime#lamp",
                Value::list(vec![
                    Value::symbol(":orphan-root"),
                    Value::str("did:ma:owner"),
                    Value::str("did:ma:other#room"),
                ]),
            )),
        );
        env.define(
            Rc::from("other_msg"),
            Value::Msg(sample_term_msg(
                "did:ma:runtime#root",
                "did:ma:runtime#lamp",
                Value::list(vec![
                    Value::symbol(":orphan-root"),
                    Value::str("did:ma:other"),
                    Value::str("did:ma:other#room"),
                ]),
            )),
        );

        eval_all("(on-message owner_msg)", &env).unwrap();
        assert_eq!(
            eval_str("(get-prop \"sent-target:1\")", &env),
            "did:ma:runtime#root"
        );
        assert_eq!(
            eval_str(
                "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"parent\")",
                &env
            ),
            "did:ma:runtime#root"
        );

        eval_all("(on-message other_msg)", &env).unwrap();
        assert_eq!(
            eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
            Value::list(vec![
                Value::symbol(":error"),
                Value::str("only owner may orphan this actor")
            ]),
        );
    }

    #[test]
    fn root_orphans_lists_only_live_movable_children() {
        let env = root_actor_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#root".to_string());
        config.insert("root".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        eval_all(
                        r#"
                        (define (ma-entity-exists? actor)
                            (equal? actor "did:ma:runtime#lamp"))
                        (define (root-child actor kind protocol)
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set
                                                    (map-set (make-map) "actor" actor)
                                                    "kind" kind)
                                                "protocol" protocol)
                                            "parent" (local-self))
                                        "name" actor)
                                    "nick" actor)
                                "description" actor))
                        (remember-child! (root-child "did:ma:runtime#lamp" "thing" "/ma/thing/0.0.1"))
                        (remember-child! (root-child "did:ma:runtime#room" "room" "/ma/room/0.0.1"))
                        (remember-child! (root-child "did:ma:runtime#dead" "container" "/ma/container/0.0.1"))
                        "#,
                        &env,
                )
                .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:anyone",
                "did:ma:runtime#root",
                Value::symbol(":orphans?"),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert_eq!(
            eval_all("(car (cdr (get-prop \"reply-term:1\")))", &env).unwrap(),
            Value::list(vec![
                eval_all("(child-ctx \"did:ma:runtime#lamp\")", &env).unwrap()
            ]),
        );
    }

    #[test]
    fn room_accepts_root_signed_repair_for_unavailable_movable_child() {
        let env = room_env();
        install_send_reply_recorders(&env);
        let mut config = std::collections::HashMap::new();
        config.insert("runtime".to_string(), "did:ma:runtime".to_string());
        config.insert("self".to_string(), "did:ma:runtime#room".to_string());
        config.insert("root".to_string(), "did:ma:runtime#root".to_string());
        crate::state::set_config(config);
        eval_all(
            "(test-agent-claim! \"did:ma:other#orphan\" \"orphan\")",
            &env,
        )
        .unwrap();
        let repair = eval_all(
            r#"
                        (map-set
                            (map-set
                                (map-set
                                    (map-set
                                        (map-set
                                            (map-set
                                                (map-set (make-map) "actor" "did:ma:other#orphan")
                                                "kind" "orphan")
                                            "protocol" "/ma/orphan/0.0.1")
                                        "parent" "did:ma:other#root")
                                    "name" "orphan")
                                "nick" "orphan")
                            "description" "An unavailable orphaned actor.")
                        "#,
            &env,
        )
        .unwrap();
        env.define(
            Rc::from("msg"),
            Value::Msg(sample_term_msg(
                "did:ma:other#root",
                "did:ma:runtime#room",
                Value::list(vec![Value::symbol(":parent"), repair]),
            )),
        );

        eval_all("(on-message msg)", &env).unwrap();

        assert!(eval_bool("(not (child-ctx \"did:ma:other#orphan\"))", &env));
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
