//! Integration tests for actor behaviours not exercised by the inline unit tests.
//!
//! Each test builds a fresh actor environment through the public crate API,
//! exercises one or more message verbs, and asserts on the resulting prop
//! state or reply terms.

use std::collections::HashMap;
use std::rc::Rc;

use ma_scheme_actor::value::Value;
use ma_scheme_actor::{eval_all, new_root_env};

// ── Shared helpers ────────────────────────────────────────────────────────

fn empty_state_cbor() -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&ciborium::Value::Map(Vec::new()), &mut bytes).unwrap();
    bytes
}

fn make_config(runtime: &str, self_did: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("runtime".to_string(), runtime.to_string());
    m.insert("self".to_string(), self_did.to_string());
    m
}

fn sample_msg(from: &str, to: &str, content: Value) -> Value {
    Value::Msg(Rc::new(ma_scheme_actor::msg::MsgRecord {
        id: "test-msg".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        created_at: 0,
        exp: 0,
        reply_to: None,
        msg_type: "application/vnd.ma.rpc.request".to_string(),
        content_type: "application/vnd.ma.term".to_string(),
        content,
    }))
}

fn rpc_msg(from: &str, to: &str, verb: &str) -> Value {
    sample_msg(from, to, Value::symbol(verb))
}

fn room_env() -> Rc<ma_scheme_actor::env::Env> {
    ma_scheme_actor::state::load_from_cbor(&empty_state_cbor()).unwrap();
    let env = new_root_env();
    ma_scheme_actor::state::install(&env);
    ma_scheme_actor::msg::install(&env);
    eval_all(include_str!("../stdlib.ma"), &env).unwrap();
    eval_all(include_str!("../actor.ma"), &env).unwrap();
    eval_all(include_str!("../state.ma"), &env).unwrap();
    eval_all(include_str!("../node.ma"), &env).unwrap();
    eval_all(include_str!("../../actors/room.ma"), &env).unwrap();
    // Recording stubs: capture sends and replies into numbered props.
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
        (define (test-agent-claim! actor nick)
          (remember-child!
            (make-map "actor" actor
                      "kind" "agent"
                      "protocol" "/ma/agent/0.0.1"
                      "parent" (canonical-actor (self))
                      "name" nick
                      "nick" nick
                      "description" "A test agent.")))
        (define (test-thing-claim! actor nick)
          (remember-child!
            (make-map "actor" actor
                      "kind" "thing"
                      "protocol" "/ma/thing/0.0.1"
                      "parent" (canonical-actor (self))
                      "name" nick
                      "nick" nick
                      "description" "A test thing.")))
        "#,
        &env,
    )
    .unwrap();
    env
}

fn exit_env() -> Rc<ma_scheme_actor::env::Env> {
    ma_scheme_actor::state::load_from_cbor(&empty_state_cbor()).unwrap();
    let env = new_root_env();
    ma_scheme_actor::state::install(&env);
    ma_scheme_actor::msg::install(&env);
    eval_all(include_str!("../stdlib.ma"), &env).unwrap();
    eval_all(include_str!("../actor.ma"), &env).unwrap();
    eval_all(include_str!("../state.ma"), &env).unwrap();
    eval_all(include_str!("../node.ma"), &env).unwrap();
    eval_all(include_str!("../../actors/exit.ma"), &env).unwrap();
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
        (define (ma-end) (set-prop! "ended" "yes"))
        "#,
        &env,
    )
    .unwrap();
    env
}

fn stdlib_env() -> Rc<ma_scheme_actor::env::Env> {
    ma_scheme_actor::state::load_from_cbor(&empty_state_cbor()).unwrap();
    let env = new_root_env();
    eval_all(include_str!("../stdlib.ma"), &env).unwrap();
    env
}

fn eval_str(src: &str, env: &Rc<ma_scheme_actor::env::Env>) -> String {
    match eval_all(src, env).unwrap() {
        Value::Str(s) => s.to_string(),
        other => panic!("expected string, got {other}"),
    }
}

fn eval_bool(src: &str, env: &Rc<ma_scheme_actor::env::Env>) -> bool {
    match eval_all(src, env).unwrap() {
        Value::Bool(b) => b,
        other => panic!("expected bool, got {other}"),
    }
}

fn eval_int(src: &str, env: &Rc<ma_scheme_actor::env::Env>) -> i64 {
    match eval_all(src, env).unwrap() {
        Value::Int(i) => i,
        other => panic!("expected int, got {other}"),
    }
}

// ── Room `:help` ──────────────────────────────────────────────────────────

#[test]
fn room_help_contains_room_name_and_key_commands() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#lounge"));
    eval_all(r#"(set-prop! "name" "The Lounge")"#, &env).unwrap();

    env.define(
        Rc::from("msg"),
        rpc_msg("did:ma:alice", "did:ma:runtime#lounge", ":test"),
    );
    eval_all("((find-method :help) '() msg)", &env).unwrap();

    // reply is [:ok "text..."] — extract and validate the text
    let text = eval_str("(car (cdr (get-prop \"reply-term:1\")))", &env);
    assert!(
        text.contains("The Lounge"),
        "help text should include the room name"
    );
    assert!(text.contains("look"), "help text should list 'look'");
    assert!(text.contains("say"), "help text should list 'say'");
    assert!(text.contains("exits?"), "help text should list 'exits?'");
    assert!(text.contains("dig"), "help text should list 'dig'");
}

// ── Room `:prop` ──────────────────────────────────────────────────────────

#[test]
fn room_prop_owner_can_set_and_reset_text_properties() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(set-prop! "owner" "did:ma:owner")"#, &env).unwrap();

    // Set a prop.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":prop"),
                Value::str("name"),
                Value::str("Grand Hall"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    // Verify prop was set: reply text names the key and node-name reflects it.
    let reply_text = eval_str("(car (cdr (get-prop \"reply-term:1\")))", &env);
    assert!(
        reply_text.contains("name"),
        "reply text should mention 'name', got: {reply_text}"
    );
    assert_eq!(eval_str("(get-prop \"name\")", &env), "Grand Hall");

    // Reset the prop (no value args).
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":prop"), Value::str("name")]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    // set-node-prop! with "" calls del-prop! so the key is absent, not set to ""
    assert_eq!(
        eval_all("(get-prop \"name\")", &env).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn room_prop_rejects_non_owner_and_missing_key() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(set-prop! "owner" "did:ma:owner")"#, &env).unwrap();

    // Non-owner is rejected.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:stranger",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":prop"),
                Value::str("name"),
                Value::str("Stolen"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );

    // Missing key is rejected.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":prop")]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

// ── Room `:emote` ─────────────────────────────────────────────────────────

#[test]
fn room_emote_broadcasts_typed_event_and_acknowledges() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
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
        sample_msg(
            "did:ma:bob",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":emote"),
                Value::str("waves"),
                Value::str("hello"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    // Emote is broadcast to occupants.
    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
    assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:bob");
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
        Value::symbol(":emote")
    );
    assert_eq!(
        eval_str("(car (cdr (cdr (get-prop \"sent-term:1\"))))", &env),
        "waves hello"
    );
    // Reply is :ok.
    assert_eq!(
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_emote_fails_for_speaker_without_event_ctx() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:unknown",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":emote"), Value::str("vanishes")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

// ── Room `:where?` ────────────────────────────────────────────────────────

#[test]
fn room_where_routes_query_to_movable_actor() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(test-thing-claim! "did:ma:runtime#lamp" "lamp")"#, &env).unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":where?"), Value::str("lamp")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:runtime#lamp"
    );
    assert_eq!(
        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
        Value::list(vec![Value::symbol(":where?")])
    );
}

#[test]
fn room_where_reports_unknown_token() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":where?"), Value::str("invisible")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    // Sends an error message back to the caller (not a reply).
    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:alice"
    );
}

// ── Room `:recycle` ───────────────────────────────────────────────────────

#[test]
fn room_recycle_forwards_to_known_movable() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(test-thing-claim! "did:ma:runtime#vase" "vase")"#, &env).unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":recycle"), Value::str("vase")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:runtime#vase"
    );
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
        Value::symbol(":recycle")
    );
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_recycle_rejects_unknown_token() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":recycle"), Value::str("ghost")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

// ── Room `:remove` ────────────────────────────────────────────────────────

#[test]
fn room_remove_owner_evicts_child_by_nick() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (set-prop! "owner" "did:ma:owner")
        (test-agent-claim! "did:ma:runtime#bot" "bot")
        "#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":remove"), Value::str("bot")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert!(eval_bool("(not (child-ctx \"did:ma:runtime#bot\"))", &env));
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_remove_rejects_non_owner_and_missing_args() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(set-prop! "owner" "did:ma:owner")"#, &env).unwrap();

    // Non-owner.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:stranger",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":remove"), Value::str("someone")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );

    // No args.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":remove")]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

// ── Room `:thing` alias management ────────────────────────────────────────

#[test]
fn room_thing_alias_set_and_cleared_by_owner() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(set-prop! "owner" "did:ma:owner")"#, &env).unwrap();
    eval_all(r#"(test-thing-claim! "did:ma:runtime#lamp" "lamp")"#, &env).unwrap();

    // Query existing alias.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":thing"), Value::str("lamp")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );

    // Owner clears the alias.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":thing"),
                Value::str("lamp"),
                Value::str(""),
            ]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert!(eval_bool("(not (thing-ref \"lamp\"))", &env));
}

// ── Exit `:traverse` ──────────────────────────────────────────────────────

#[test]
fn exit_traverse_unlocked_returns_target_ctx() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(
        r#"
        (set-prop! "parent"      "did:ma:runtime#room")
        (set-prop! "target-room" "did:ma:runtime#kitchen")
        (set-prop! "direction"   "north")
        "#,
        &env,
    )
    .unwrap();

    let traversal_ctx = eval_all(
        r#"(make-map "did" "did:ma:alice" "parent" "did:ma:runtime#room")"#,
        &env,
    )
    .unwrap();
    env.define(Rc::from("tctx"), traversal_ctx.clone());
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":traverse"), traversal_ctx]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    let result_ctx = eval_all("(car (cdr (get-prop \"reply-term:1\")))", &env).unwrap();
    assert!(matches!(result_ctx, Value::Map(_)));
    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"parent\")",
            &env
        ),
        "did:ma:runtime#kitchen"
    );
    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"direction\")",
            &env
        ),
        "north"
    );
}

#[test]
fn exit_traverse_locked_returns_blocked_ctx_at_source_room() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(
        r#"
        (set-prop! "parent"      "did:ma:runtime#room")
        (set-prop! "target-room" "did:ma:runtime#kitchen")
        (set-prop! "direction"   "north")
        (set-prop! "locked"      "true")
        "#,
        &env,
    )
    .unwrap();

    let traversal_ctx = eval_all(
        r#"(make-map "did" "did:ma:alice" "parent" "did:ma:runtime#room")"#,
        &env,
    )
    .unwrap();
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":traverse"), traversal_ctx]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    // Blocked: parent in reply ctx stays at the source room, not the target.
    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"parent\")",
            &env
        ),
        "did:ma:runtime#room"
    );
}

#[test]
fn exit_traverse_rejects_caller_not_source_room_and_not_traveller() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(
        r#"
        (set-prop! "parent"      "did:ma:runtime#room")
        (set-prop! "target-room" "did:ma:runtime#kitchen")
        (set-prop! "direction"   "north")
        "#,
        &env,
    )
    .unwrap();

    let traversal_ctx = eval_all(
        r#"(make-map "did" "did:ma:alice" "parent" "did:ma:runtime#room")"#,
        &env,
    )
    .unwrap();
    // Sender is neither the source room nor the traveller DID.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:intruder",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":traverse"), traversal_ctx]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

// ── Exit `:lock` / `:unlock` ──────────────────────────────────────────────

#[test]
fn exit_lock_unlock_only_allowed_from_source_room() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(r#"(set-prop! "parent" "did:ma:runtime#room")"#, &env).unwrap();

    // Source room may lock.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::symbol(":lock"),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert!(eval_bool("(locked?)", &env));

    // Intruder may not unlock.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:intruder",
            "did:ma:runtime#north-exit",
            Value::symbol(":unlock"),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":error")
    );
    assert!(eval_bool("(locked?)", &env));

    // Source room may unlock.
    env.define(
        Rc::from("msg3"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::symbol(":unlock"),
        ),
    );
    eval_all("(on-message msg3)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:3\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert!(eval_bool("(not (locked?))", &env));
}

// ── Exit `:locked?` ───────────────────────────────────────────────────────

#[test]
fn exit_locked_query_reflects_current_state() {
    let env = exit_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#gate"));

    env.define(
        Rc::from("msg"),
        rpc_msg("did:ma:anyone", "did:ma:runtime#gate", ":locked?"),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (cdr (get-prop \"reply-term:1\")))", &env).unwrap(),
        Value::str("false")
    );
}

// ── Exit `:where?` ────────────────────────────────────────────────────────

#[test]
fn exit_where_returns_its_parent_room() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(r#"(set-prop! "parent" "did:ma:runtime#room")"#, &env).unwrap();

    env.define(
        Rc::from("msg"),
        rpc_msg("did:ma:anyone", "did:ma:runtime#north-exit", ":where?"),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (cdr (get-prop \"reply-term:1\")))", &env).unwrap(),
        Value::str("did:ma:runtime#room")
    );
}

// ── Exit `:about` ─────────────────────────────────────────────────────────

#[test]
fn exit_about_returns_descriptive_text() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(
        r#"
        (set-prop! "parent"      "did:ma:runtime#room")
        (set-prop! "target-room" "did:ma:runtime#kitchen")
        (set-prop! "direction"   "north")
        "#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("msg"),
        rpc_msg("did:ma:anyone", "did:ma:runtime#north-exit", ":about"),
    );
    eval_all("(on-message msg)", &env).unwrap();

    let text = eval_str("(car (cdr (get-prop \"reply-term:1\")))", &env);
    assert!(
        text.contains("north"),
        "about text should mention direction"
    );
    assert!(
        text.contains("did:ma:runtime#kitchen"),
        "about text should mention target room"
    );
}

// ── Exit `:message` ───────────────────────────────────────────────────────

#[test]
fn exit_message_source_room_can_update_traveller_message() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(r#"(set-prop! "parent" "did:ma:runtime#room")"#, &env).unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":message"),
                Value::str("traveller"),
                Value::str("You slip through a secret door."),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert_eq!(
        eval_str("(get-prop \"traveller-message\")", &env),
        "You slip through a secret door."
    );
}

#[test]
fn exit_message_rejects_unknown_slot_and_non_source_caller() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(r#"(set-prop! "parent" "did:ma:runtime#room")"#, &env).unwrap();

    // Unknown slot.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":message"),
                Value::str("bogus"),
                Value::str("..."),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );

    // Non-source caller.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:intruder",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":message"),
                Value::str("traveller"),
                Value::str("nope"),
            ]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

// ── stdlib.ma helpers ─────────────────────────────────────────────────────

#[test]
fn stdlib_join_words_concatenates_with_spaces() {
    let env = stdlib_env();
    assert_eq!(
        eval_str(r#"(join-words (list "hello" "world"))"#, &env),
        "hello world"
    );
    assert_eq!(eval_str(r#"(join-words (list "one"))"#, &env), "one");
    assert_eq!(eval_str(r#"(join-words '())"#, &env), "");
}

#[test]
fn stdlib_list_ref_at_picks_by_index() {
    let env = stdlib_env();
    assert_eq!(
        eval_all(r#"(list-ref-at (list "a" "b" "c") 0)"#, &env).unwrap(),
        Value::str("a")
    );
    assert_eq!(
        eval_all(r#"(list-ref-at (list "a" "b" "c") 2)"#, &env).unwrap(),
        Value::str("c")
    );
    assert_eq!(
        eval_all(r#"(list-ref-at (list "a" "b") 5)"#, &env).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        eval_all(r#"(list-ref-at '() 0)"#, &env).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn stdlib_list_length_counts_elements() {
    let env = stdlib_env();
    assert_eq!(eval_int(r#"(list-length '())"#, &env), 0);
    assert_eq!(eval_int(r#"(list-length (list 1 2 3))"#, &env), 3);
}

#[test]
fn stdlib_ctx_shape_valid_requires_kind_parent_protocol() {
    let env = stdlib_env();

    // Valid ctx.
    assert!(eval_bool(
        r#"(ctx-shape-valid?
              (make-map "kind"     "thing"
                        "parent"   "did:ma:runtime#room"
                        "protocol" "/ma/thing/0.0.1"))"#,
        &env
    ));

    // Missing required fields are invalid.
    assert!(!eval_bool(
        r#"(ctx-shape-valid? (make-map "kind" "thing" "parent" "did:ma:runtime#room"))"#,
        &env
    ));
    assert!(!eval_bool(
        r#"(ctx-shape-valid? (make-map "kind" "unknown-kind" "parent" "x" "protocol" "/ma/thing/0.0.1"))"#,
        &env
    ));
    assert!(!eval_bool(r#"(ctx-shape-valid? (make-map))"#, &env));
}

#[test]
fn stdlib_actor_ctx_shape_requires_only_name() {
    let env = stdlib_env();

    // Full ctx with all optional fields is valid.
    assert!(eval_bool(
        r#"(actor-ctx-shape?
              (make-map "kind"        "thing"
                        "parent"      "did:ma:runtime#room"
                        "protocol"    "/ma/thing/0.0.1"
                        "name"        "Lamp"
                        "nick"        "lamp"
                        "description" "A warm desk lamp."))"#,
        &env
    ));

    // Only name — no nick or description — is still valid.
    assert!(eval_bool(
        r#"(actor-ctx-shape?
              (make-map "kind"     "thing"
                        "parent"   "did:ma:runtime#room"
                        "protocol" "/ma/thing/0.0.1"
                        "name"     "Lamp"))"#,
        &env
    ));

    // Missing name entirely is invalid.
    assert!(!eval_bool(
        r#"(actor-ctx-shape?
              (make-map "kind"        "thing"
                        "parent"      "did:ma:runtime#room"
                        "protocol"    "/ma/thing/0.0.1"
                        "nick"        "lamp"
                        "description" "A warm desk lamp."))"#,
        &env
    ));
}

#[test]
fn stdlib_unique_string_entries_deduplicates_preserving_order() {
    let env = stdlib_env();

    // string-entries uses cons so it reverses; unique-string-entries then deduplicates
    // in that reversed order → first unique from the end wins.
    assert_eq!(
        eval_all(
            r#"(unique-string-entries (list "a" "b" "a" "c" "b"))"#,
            &env
        )
        .unwrap(),
        Value::list(vec![Value::str("b"), Value::str("c"), Value::str("a")])
    );
    assert_eq!(
        eval_all(r#"(unique-string-entries '())"#, &env).unwrap(),
        Value::Nil
    );
    // Non-string entries are dropped.
    assert_eq!(
        eval_all(r#"(unique-string-entries (list "x" 42 "x"))"#, &env).unwrap(),
        Value::list(vec![Value::str("x")])
    );
}

#[test]
fn stdlib_non_empty_string_predicate() {
    let env = stdlib_env();
    assert!(eval_bool(r#"(non-empty-string? "hello")"#, &env));
    assert!(!eval_bool(r#"(non-empty-string? "")"#, &env));
    assert!(!eval_bool(r#"(non-empty-string? 42)"#, &env));
    assert!(!eval_bool(r#"(non-empty-string? #f)"#, &env));
}

#[test]
fn stdlib_list_append_concatenates_lists() {
    let env = stdlib_env();
    assert_eq!(
        eval_all(r#"(list-append (list 1 2) (list 3 4))"#, &env).unwrap(),
        Value::list(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4)
        ])
    );
    assert_eq!(
        eval_all(r#"(list-append '() (list 1))"#, &env).unwrap(),
        Value::list(vec![Value::Int(1)])
    );
    assert_eq!(
        eval_all(r#"(list-append (list 1) '())"#, &env).unwrap(),
        Value::list(vec![Value::Int(1)])
    );
}

#[test]
fn stdlib_arg_at_or_false_returns_element_or_false() {
    let env = stdlib_env();
    assert_eq!(
        eval_all(r#"(arg-at-or-false (list "a" "b") 0)"#, &env).unwrap(),
        Value::str("a")
    );
    assert_eq!(
        eval_all(r#"(arg-at-or-false (list "a" "b") 1)"#, &env).unwrap(),
        Value::str("b")
    );
    assert_eq!(
        eval_all(r#"(arg-at-or-false (list "a") 5)"#, &env).unwrap(),
        Value::Bool(false)
    );
}

// ── Ctx-text helper ───────────────────────────────────────────────────────

#[test]
fn stdlib_ctx_text_returns_string_or_false() {
    let env = stdlib_env();

    assert_eq!(
        eval_all(r#"(ctx-text (make-map "name" "Lamp") "name")"#, &env).unwrap(),
        Value::str("Lamp")
    );
    assert_eq!(
        eval_all(r#"(ctx-text (make-map) "name")"#, &env).unwrap(),
        Value::Bool(false)
    );
    // Non-string value yields false.
    assert_eq!(
        eval_all(r#"(ctx-text (make-map "n" 42) "n")"#, &env).unwrap(),
        Value::Bool(false)
    );
}
