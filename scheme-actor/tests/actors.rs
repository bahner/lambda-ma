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
        (define (test-did-presence! did ctx)
          (remember-child!
            (map-set
              (map-set
                (map-set ctx "actor" did)
                "kind" "agent")
              "protocol" "/ma/agent/0.0.1")))
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
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::symbol(":ok")
    );
    // Verify the prop was set: node-name reflects it.
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
        eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
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
        (test-did-presence! "did:ma:bob"
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
fn room_direct_did_entry_enables_say_and_emote() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));

    env.define(
        Rc::from("enter_msg"),
        rpc_msg("did:ma:alice", "did:ma:runtime#room", ":enter"),
    );
    eval_all("(on-message enter_msg)", &env).unwrap();

    assert_eq!(
        eval_str("(ctx-text (child-ctx \"did:ma:alice\") \"actor\")", &env),
        "did:ma:alice"
    );
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );

    env.define(
        Rc::from("say_msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":say"), Value::str("hello")]),
        ),
    );
    eval_all("(on-message say_msg)", &env).unwrap();

    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 3);
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:3\"))", &env).unwrap(),
        Value::symbol(":say")
    );
    assert_eq!(
        eval_str("(car (cdr (cdr (get-prop \"sent-term:3\"))))", &env),
        "hello"
    );

    env.define(
        Rc::from("emote_msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":emote"), Value::str("dances")]),
        ),
    );
    eval_all("(on-message emote_msg)", &env).unwrap();

    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 4);
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:4\"))", &env).unwrap(),
        Value::symbol(":emote")
    );
    assert_eq!(
        eval_str("(car (cdr (cdr (get-prop \"sent-term:4\"))))", &env),
        "dances"
    );
    assert_eq!(
        eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_enriches_child_speech_for_every_present_actor() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (test-did-presence! "did:ma:alice"
          (make-map "did" "did:ma:alice"
                    "parent" "did:ma:runtime#room"
                    "name" "Alice"
                    "nick" "Alice"
                    "description" "A visitor."
                    "rev" 1))
        (test-agent-claim! "did:ma:runtime#duckie" "Duckie")
        "#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("say_msg"),
        sample_msg(
            "did:ma:runtime#duckie",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":say"), Value::str("quack")]),
        ),
    );
    eval_all("(on-message say_msg)", &env).unwrap();

    // The duckie's speech reaches every present actor: Alice and the duckie itself.
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:alice"
    );
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
        Value::symbol(":say")
    );
    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"sent-term:1\"))) \"actor\")",
            &env
        ),
        "did:ma:runtime#duckie"
    );
    assert_eq!(
        eval_str("(car (cdr (cdr (get-prop \"sent-term:1\"))))", &env),
        "quack"
    );
    assert_eq!(
        eval_str("(get-prop \"sent-target:2\")", &env),
        "did:ma:runtime#duckie"
    );
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:2\"))", &env).unwrap(),
        Value::symbol(":say")
    );

    env.define(
        Rc::from("emote_msg"),
        sample_msg(
            "did:ma:runtime#duckie",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":emote"), Value::str("dances")]),
        ),
    );
    eval_all("(on-message emote_msg)", &env).unwrap();

    assert_eq!(
        eval_str("(get-prop \"sent-target:3\")", &env),
        "did:ma:alice"
    );
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:3\"))", &env).unwrap(),
        Value::symbol(":emote")
    );
    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"sent-term:3\"))) \"actor\")",
            &env
        ),
        "did:ma:runtime#duckie"
    );
    assert_eq!(
        eval_str("(car (cdr (cdr (get-prop \"sent-term:3\"))))", &env),
        "dances"
    );
    assert_eq!(
        eval_str("(get-prop \"sent-target:4\")", &env),
        "did:ma:runtime#duckie"
    );
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:4\"))", &env).unwrap(),
        Value::symbol(":emote")
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

// ── Room `:narrate` ───────────────────────────────────────────────────────

#[test]
fn room_narrate_requires_owner_and_broadcasts() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (set-prop! "owner" "did:ma:alice")
        (test-did-presence! "did:ma:bob"
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

    // A non-owner is refused before anything is broadcast.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:bob",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":narrate"),
                Value::str("a cat"),
                Value::str("walks"),
                Value::str("past"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(get-prop \"sent-count\")", &env).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );

    // The owner's narration is broadcast subject-less to every child.
    env.define(
        Rc::from("narrate_msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":narrate"),
                Value::str("a cat"),
                Value::str("walks"),
                Value::str("past"),
                Value::str("the"),
                Value::str("doorway"),
            ]),
        ),
    );
    eval_all("(on-message narrate_msg)", &env).unwrap();

    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
    assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:bob");
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
        Value::symbol(":narrate")
    );
    // The term is subject-less: just the verb and the joined text.
    assert_eq!(
        eval_str("(car (cdr (get-prop \"sent-term:1\")))", &env),
        "a cat walks past the doorway"
    );
    // Reply is :ok.
    assert_eq!(
        eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_narrate_to_targets_one_child() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (set-prop! "owner" "did:ma:alice")
        (test-did-presence! "did:ma:bob"
          (make-map "did" "did:ma:bob"
                    "parent" "did:ma:runtime#room"
                    "name" "Bob"
                    "nick" "Bob"
                    "description" "A visitor."
                    "rev" 1))
        (test-agent-claim! "did:ma:runtime#duckie" "Duckie")
        "#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":narrate-to"),
                Value::str("did:ma:bob"),
                Value::str("you"),
                Value::str("hear"),
                Value::str("a"),
                Value::str("creak"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    // Exactly one send, to the target only, with the subject-less :narrate event.
    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 1);
    assert_eq!(eval_str("(get-prop \"sent-target:1\")", &env), "did:ma:bob");
    assert_eq!(
        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
        Value::list(vec![
            Value::symbol(":narrate"),
            Value::str("you hear a creak")
        ])
    );
    assert_eq!(
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_narrate_accepts_only_designated_trigger() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (set-prop! "owner" "did:ma:alice")
        (set-prop! "trigger" "did:ma:runtime#trigger")
        (test-agent-claim! "did:ma:runtime#trigger" "Trigger")
        (test-agent-claim! "did:ma:runtime#impostor" "Impostor")
        "#,
        &env,
    )
    .unwrap();

    // A forged trigger-kind child that is not the designated one is refused.
    env.define(
        Rc::from("bad_msg"),
        sample_msg(
            "did:ma:runtime#impostor",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":narrate"), Value::str("fake")]),
        ),
    );
    eval_all("(on-message bad_msg)", &env).unwrap();
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );

    // The designated trigger may narrate (broadcast to both children).
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#trigger",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":narrate"),
                Value::str("the cat walks past"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(eval_int("(get-prop \"sent-count\")", &env), 2);
    assert_eq!(
        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
        Value::list(vec![
            Value::symbol(":narrate"),
            Value::str("the cat walks past")
        ])
    );
    assert_eq!(
        eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_narrate_accepts_scheduler() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(set-prop! "owner" "did:ma:alice")"#, &env).unwrap();

    // The runtime scheduler may narrate (time-based narration); no children, no sends.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#scheduler",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":narrate"),
                Value::str("the wind howls"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(
        eval_all("(get-prop \"sent-count\")", &env).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::symbol(":ok")
    );
}

#[test]
fn room_trigger_reports_designated_trigger() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(r#"(set-prop! "trigger" "did:ma:runtime#trigger")"#, &env).unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:bob",
            "did:ma:runtime#room",
            Value::symbol(":trigger?"),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::list(vec![
            Value::symbol(":ok"),
            Value::str("did:ma:runtime#trigger")
        ])
    );
}

// ── Trigger actor ──────────────────────────────────────────────────────────

fn trigger_env() -> Rc<ma_scheme_actor::env::Env> {
    ma_scheme_actor::state::load_from_cbor(&empty_state_cbor()).unwrap();
    let env = new_root_env();
    ma_scheme_actor::state::install(&env);
    ma_scheme_actor::msg::install(&env);
    eval_all(include_str!("../stdlib.ma"), &env).unwrap();
    eval_all(include_str!("../actor.ma"), &env).unwrap();
    eval_all(include_str!("../state.ma"), &env).unwrap();
    eval_all(include_str!("../node.ma"), &env).unwrap();
    eval_all(include_str!("../../actors/trigger.ma"), &env).unwrap();
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
        (define (test-trigger-env!)
          (set-prop! "parent" "did:ma:runtime#room")
          (set-prop! "owner" "did:ma:alice")
          (set-prop! "parent-ctx"
            (make-map "children"
              (make-map
                "did:ma:alice" (make-map "actor" "did:ma:alice" "nick" "Alice" "name" "Alice" "kind" "agent" "protocol" "/ma/agent/0.0.1")
                "did:ma:bob" (make-map "actor" "did:ma:bob" "nick" "Bob" "name" "Bob" "kind" "agent" "protocol" "/ma/agent/0.0.1")))))
        "#,
        &env,
    )
    .unwrap();
    eval_all("(test-trigger-env!)", &env).unwrap();
    env
}

fn count_msg(
    env: &Rc<ma_scheme_actor::env::Env>,
    from: &str,
    did: &str,
    name: &str,
    delta: i64,
) -> Value {
    let _ = env;
    sample_msg(
        from,
        "did:ma:runtime#room",
        Value::list(vec![
            Value::symbol(":count"),
            Value::str(did),
            Value::str(name),
            Value::Int(delta),
        ]),
    )
}

#[test]
fn trigger_count_self_and_owner_manage_counters() {
    let env = trigger_env();

    // Bob counts his own jokes (self-authorised).
    env.define(
        Rc::from("msg"),
        count_msg(&env, "did:ma:bob", "did:ma:bob", "num_jokes", 1),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(
        eval_int("(counter-value \"did:ma:bob\" \"num_jokes\")", &env),
        1
    );

    // The owner sets an absolute value (override).
    env.define(
        Rc::from("set_msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":set"),
                Value::str("did:ma:bob"),
                Value::str("karma_points"),
                Value::Int(200),
            ]),
        ),
    );
    eval_all("(on-message set_msg)", &env).unwrap();
    assert_eq!(
        eval_int("(counter-value \"did:ma:bob\" \"karma_points\")", &env),
        200
    );

    // A negative delta on the owner's behalf adjusts down.
    env.define(
        Rc::from("adjust_msg"),
        count_msg(&env, "did:ma:alice", "did:ma:bob", "karma_points", -500),
    );
    eval_all("(on-message adjust_msg)", &env).unwrap();
    assert_eq!(
        eval_int("(counter-value \"did:ma:bob\" \"karma_points\")", &env),
        -300
    );

    // Self-reset.
    env.define(
        Rc::from("reset_msg"),
        sample_msg(
            "did:ma:bob",
            "did:ma:runtime#room",
            Value::list(vec![
                Value::symbol(":reset"),
                Value::str("did:ma:bob"),
                Value::str("num_jokes"),
            ]),
        ),
    );
    eval_all("(on-message reset_msg)", &env).unwrap();
    assert_eq!(
        eval_int("(counter-value \"did:ma:bob\" \"num_jokes\")", &env),
        0
    );

    // The full counter map is queryable by any room child.
    env.define(
        Rc::from("query_msg"),
        sample_msg(
            "did:ma:bob",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":counters")]),
        ),
    );
    eval_all("(on-message query_msg)", &env).unwrap();
    let reply = eval_all("(car (cdr (get-prop \"reply-term:5\")))", &env).unwrap();
    let expected = eval_all(
        r#"(make-map "did:ma:bob" (make-map "karma_points" -300))"#,
        &env,
    )
    .unwrap();
    assert_eq!(reply, expected);
}

#[test]
fn trigger_room_counter_accepts_any_present_child() {
    let env = trigger_env();

    // Bob (present in the room) increments the room's own shared counter.
    env.define(
        Rc::from("msg"),
        count_msg(&env, "did:ma:bob", "did:ma:runtime#room", "visits", 1),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(
        eval_int("(counter-value \"did:ma:runtime#room\" \"visits\")", &env),
        1
    );
}

#[test]
fn trigger_rejects_unauthorised_count() {
    let env = trigger_env();

    // A stranger who is not a child, not the owner, and not the did itself.
    env.define(
        Rc::from("msg"),
        count_msg(&env, "did:ma:mallory", "did:ma:bob", "num_jokes", 1),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(
        eval_int("(counter-value \"did:ma:bob\" \"num_jokes\")", &env),
        0
    );
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

#[test]
fn trigger_rule_narrates_on_count_threshold() {
    let env = trigger_env();

    // The owner installs a rule: on "num_jokes" count 2 → narrate.
    let content = eval_all(
        r#"(list :trigger "joke-2" "num_jokes" "count" 2 :narrate "To vitser! Haha.")"#,
        &env,
    )
    .unwrap();
    env.define(
        Rc::from("rule_msg"),
        sample_msg("did:ma:alice", "did:ma:runtime#room", content),
    );
    eval_all("(on-message rule_msg)", &env).unwrap();
    assert_eq!(
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::symbol(":ok")
    );

    // One joke: no fire yet.
    env.define(
        Rc::from("c1"),
        count_msg(&env, "did:ma:bob", "did:ma:bob", "num_jokes", 1),
    );
    eval_all("(on-message c1)", &env).unwrap();
    assert_eq!(
        eval_all("(get-prop \"sent-count\")", &env).unwrap(),
        Value::Bool(false)
    );

    // Two jokes: the rule fires a room-wide narration.
    env.define(
        Rc::from("c2"),
        count_msg(&env, "did:ma:bob", "did:ma:bob", "num_jokes", 1),
    );
    eval_all("(on-message c2)", &env).unwrap();
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:runtime#room"
    );
    assert_eq!(
        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
        Value::list(vec![
            Value::symbol(":narrate"),
            Value::str("To vitser! Haha.")
        ])
    );
}

#[test]
fn trigger_rule_narrates_to_scope_did_with_template() {
    let env = trigger_env();

    // Per-identity rule: every 2nd arrival salutes the arriver personally,
    // with the counter value and name substituted into the text.
    let content = eval_all(
        r#"(list :trigger "salute" :arrive "every" 2 :narrate_to
             "Velkommen" "tilbake," (list :string-append (list :did-name) "!")
             "Du" "har" "vært" "her" (list :did-counter :arrive) "ganger.")"#,
        &env,
    )
    .unwrap();
    env.define(
        Rc::from("rule_msg"),
        sample_msg("did:ma:alice", "did:ma:runtime#room", content),
    );
    eval_all("(on-message rule_msg)", &env).unwrap();

    let alice_ctx = eval_all(
        r#"(make-map "actor" "did:ma:alice" "nick" "Alice" "kind" "agent" "protocol" "/ma/agent/0.0.1")"#,
        &env,
    )
    .unwrap();
    let arrive = || {
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":arrive"), alice_ctx.clone()]),
        )
    };

    env.define(Rc::from("a1"), arrive());
    eval_all("(on-message a1)", &env).unwrap();
    assert_eq!(
        eval_all("(get-prop \"sent-count\")", &env).unwrap(),
        Value::Bool(false)
    );

    env.define(Rc::from("a2"), arrive());
    eval_all("(on-message a2)", &env).unwrap();
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:runtime#room"
    );
    assert_eq!(
        eval_all("(get-prop \"sent-term:1\")", &env).unwrap(),
        Value::list(vec![
            Value::symbol(":narrate-to"),
            Value::str("did:ma:alice"),
            Value::str("Velkommen tilbake, Alice! Du har vært her 2 ganger.")
        ])
    );
}

#[test]
fn trigger_automatically_counts_room_events() {
    let env = trigger_env();
    let bob_ctx = eval_all(
        r#"(make-map "actor" "did:ma:bob" "nick" "Bob" "kind" "agent" "protocol" "/ma/agent/0.0.1")"#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("say_msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":say"), bob_ctx, Value::str("hello")]),
        ),
    );
    eval_all("(on-message say_msg)", &env).unwrap();
    assert_eq!(eval_int("(counter-value \"did:ma:bob\" \":say\")", &env), 1);
}

#[test]
fn trigger_rejects_invalid_rule() {
    let env = trigger_env();

    // A rule with an unknown template function must be rejected at setup.
    let content = eval_all(
        r#"(list :trigger "bad" "x" "count" 3 :narrate (list :bogus))"#,
        &env,
    )
    .unwrap();
    env.define(
        Rc::from("rule_msg"),
        sample_msg("did:ma:alice", "did:ma:runtime#room", content),
    );
    eval_all("(on-message rule_msg)", &env).unwrap();
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );
    assert_eq!(eval_int("(list-length (trigger-rules))", &env), 0);
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
fn room_remove_owner_evicts_child_by_lowercase_nick_when_visible_label_is_mixed_case() {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (set-prop! "owner" "did:ma:owner")
        (test-agent-claim! "did:ma:runtime#duckie" "Duckie")
        "#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#room",
            Value::list(vec![Value::symbol(":remove"), Value::str("duckie")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert!(eval_bool(
        "(not (child-ctx \"did:ma:runtime#duckie\"))",
        &env
    ));
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
fn exit_traverse_locked_allows_traveller_with_matching_secret() {
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
        (set-prop! "lock-secret" "open sesame")
        "#,
        &env,
    )
    .unwrap();

    let traversal_ctx = eval_all(
        r#"(make-map "did" "did:ma:alice" "parent" "did:ma:runtime#room")"#,
        &env,
    )
    .unwrap();

    // Wrong secret: still blocked at the source room.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":traverse"),
                traversal_ctx.clone(),
                Value::str("wrong"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"parent\")",
            &env
        ),
        "did:ma:runtime#room"
    );

    // Matching secret: traverses to the target room.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:alice",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":traverse"),
                traversal_ctx,
                Value::str("open sesame"),
            ]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"reply-term:2\"))) \"parent\")",
            &env
        ),
        "did:ma:runtime#kitchen"
    );
}

#[test]
fn exit_traverse_locked_narrates_refusal_to_traveller() {
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
        (set-prop! "locked-msg-text" "None shall pass!")
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
            "did:ma:alice",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":traverse"), traversal_ctx]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    // The refusal is narrated to the traveller as an in-world :say event,
    // not an inbox message; the traverse reply stays the structured channel.
    assert_eq!(
        eval_str("(get-prop \"sent-target:1\")", &env),
        "did:ma:alice"
    );
    assert_eq!(
        eval_all("(car (get-prop \"sent-term:1\"))", &env).unwrap(),
        Value::symbol(":say")
    );
    assert_eq!(
        eval_all("(car (cdr (cdr (get-prop \"sent-term:1\"))))", &env).unwrap(),
        Value::str("None shall pass!")
    );
    // Still blocked at the source room.
    assert_eq!(
        eval_str(
            "(ctx-text (car (cdr (get-prop \"reply-term:1\"))) \"parent\")",
            &env
        ),
        "did:ma:runtime#room"
    );
}

#[test]
fn exit_prop_locked_msg_text_settable_by_source_room_or_owner() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(
        r#"
        (set-prop! "parent" "did:ma:runtime#room")
        (set-prop! "owner" "did:ma:owner")
        "#,
        &env,
    )
    .unwrap();

    // Source room sets the refusal message.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":prop"),
                Value::str("locked-msg-text"),
                Value::str("None shall pass!"),
            ]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(get-prop \"reply-term:1\")", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert_eq!(eval_str("(locked-msg-text)", &env), "None shall pass!");

    // The room owner (the avatar, a bare DID) may also set it directly — the
    // command from the bug report: `prop <exit> locked-msg-text …`.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:owner",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":prop"),
                Value::str("locked-msg-text"),
                Value::str("Abandon hope"),
                Value::str("all ye"),
                Value::str("who enter"),
            ]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(get-prop \"reply-term:2\")", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert_eq!(
        eval_str("(locked-msg-text)", &env),
        "Abandon hope all ye who enter"
    );

    // A passer-by may not edit exit props.
    env.define(
        Rc::from("msg3"),
        sample_msg(
            "did:ma:intruder",
            "did:ma:runtime#north-exit",
            Value::list(vec![
                Value::symbol(":prop"),
                Value::str("locked-msg-text"),
                Value::str("hacked"),
            ]),
        ),
    );
    eval_all("(on-message msg3)", &env).unwrap();

    assert_eq!(
        eval_all("(get-prop \"reply-term:3\")", &env).unwrap(),
        Value::list(vec![
            Value::symbol(":error"),
            Value::str("not authorised to edit props")
        ])
    );
    assert_eq!(
        eval_str("(locked-msg-text)", &env),
        "Abandon hope all ye who enter"
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

#[test]
fn exit_lock_secret_allows_non_owner_unlock_with_matching_secret() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(r#"(set-prop! "parent" "did:ma:runtime#room")"#, &env).unwrap();

    // Source room locks with a secret.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":lock"), Value::str("open sesame")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert!(eval_bool("(locked?)", &env));
    assert_eq!(eval_str("(lock-secret)", &env), "open sesame");

    // A traveller without the secret may not unlock.
    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:traveller",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":unlock"), Value::str("wrong")]),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":error")
    );
    assert!(eval_bool("(locked?)", &env));

    // A traveller holding the secret may unlock.
    env.define(
        Rc::from("msg3"),
        sample_msg(
            "did:ma:traveller",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":unlock"), Value::str("open sesame")]),
        ),
    );
    eval_all("(on-message msg3)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:3\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert!(eval_bool("(not (locked?))", &env));
}

#[test]
fn exit_bare_lock_preserves_existing_secret() {
    let env = exit_env();
    let mut config = make_config("did:ma:runtime", "did:ma:runtime#north-exit");
    config.insert("kind".to_string(), "/ma/exit/0.0.1".to_string());
    ma_scheme_actor::state::set_config(config);
    eval_all(r#"(set-prop! "parent" "did:ma:runtime#room")"#, &env).unwrap();

    // Lock with a secret, then re-lock bare: the stored secret must survive.
    env.define(
        Rc::from("msg"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::list(vec![Value::symbol(":lock"), Value::str("sesame")]),
        ),
    );
    eval_all("(on-message msg)", &env).unwrap();
    assert_eq!(eval_str("(lock-secret)", &env), "sesame");

    env.define(
        Rc::from("msg2"),
        sample_msg(
            "did:ma:runtime#room",
            "did:ma:runtime#north-exit",
            Value::symbol(":lock"),
        ),
    );
    eval_all("(on-message msg2)", &env).unwrap();

    assert_eq!(
        eval_all("(car (get-prop \"reply-term:2\"))", &env).unwrap(),
        Value::symbol(":ok")
    );
    assert!(eval_bool("(locked?)", &env));
    assert_eq!(eval_str("(lock-secret)", &env), "sesame");
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

// ── Roll-call ─────────────────────────────────────────────────────────────
//
// `begin` opens an empty shadow map, every reporting child lands in it, and
// `end` swaps it in wholesale. A child that never reports is simply one that
// never reappeared in the new tree.

/// Room owned by `did:ma:owner`, holding Alice and Bob, with a `test-child-ctx`
/// helper for building the ctx a child sends back with `:parent`.
fn roll_call_env() -> Rc<ma_scheme_actor::env::Env> {
    let env = room_env();
    ma_scheme_actor::state::set_config(make_config("did:ma:runtime", "did:ma:runtime#room"));
    eval_all(
        r#"
        (set-prop! "owner" "did:ma:owner")
        (test-agent-claim! "did:ma:alice" "Alice")
        (test-agent-claim! "did:ma:bob" "Bob")
        (define (test-child-ctx actor nick)
          (make-map "actor" actor
                    "did" actor
                    "kind" "agent"
                    "protocol" "/ma/agent/0.0.1"
                    "parent" (canonical-actor (self))
                    "name" nick
                    "nick" nick
                    "description" "A test agent."))
        "#,
        &env,
    )
    .unwrap();
    env.define(
        Rc::from("owner_msg"),
        rpc_msg("did:ma:owner", "did:ma:runtime#room", ":roll-call"),
    );
    env.define(
        Rc::from("alice_msg"),
        rpc_msg("did:ma:alice", "did:ma:runtime#room", ":parent"),
    );
    env
}

fn has_child(env: &Rc<ma_scheme_actor::env::Env>, actor: &str) -> bool {
    eval_bool(&format!("(map-has-key? (children-map) \"{actor}\")"), env)
}

fn begin_roll_call(env: &Rc<ma_scheme_actor::env::Env>) {
    eval_all("((find-method :roll-call) (list \"begin\") owner_msg)", env).unwrap();
}

fn end_roll_call(env: &Rc<ma_scheme_actor::env::Env>) {
    eval_all("((find-method :roll-call) (list \"end\") owner_msg)", env).unwrap();
}

/// Alice reports, Bob stays silent, so only Alice survives the swap.
#[test]
fn roll_call_end_keeps_reporting_children_and_drops_silent_ones() {
    let env = roll_call_env();

    begin_roll_call(&env);
    assert!(eval_bool("(roll-call-active?)", &env));
    // Nothing is pruned until end.
    assert!(has_child(&env, "did:ma:alice"));
    assert!(has_child(&env, "did:ma:bob"));

    eval_all(
        r#"((find-method :parent) (list (test-child-ctx "did:ma:alice" "Alice")) alice_msg)"#,
        &env,
    )
    .unwrap();

    end_roll_call(&env);

    assert!(!eval_bool("(roll-call-active?)", &env));
    assert!(has_child(&env, "did:ma:alice"), "Alice reported in");
    assert!(!has_child(&env, "did:ma:bob"), "Bob never reported in");
}

/// The swap installs the ctx the child just sent, not the one already stored.
#[test]
fn roll_call_end_installs_freshly_reported_ctx() {
    let env = roll_call_env();
    assert_eq!(
        eval_str(r#"(ctx-text (child-ctx "did:ma:alice") "nick")"#, &env),
        "Alice"
    );

    begin_roll_call(&env);
    eval_all(
        r#"((find-method :parent) (list (test-child-ctx "did:ma:alice" "Trinity")) alice_msg)"#,
        &env,
    )
    .unwrap();
    end_roll_call(&env);

    assert_eq!(
        eval_str(r#"(ctx-text (child-ctx "did:ma:alice") "nick")"#, &env),
        "Trinity",
        "survivors carry the ctx reported during the roll-call, not stale state"
    );
}

/// Someone who arrives mid-window is captured by remember-child! and survives.
#[test]
fn roll_call_end_keeps_child_that_arrived_during_the_window() {
    let env = roll_call_env();

    begin_roll_call(&env);
    eval_all(r#"(test-agent-claim! "did:ma:carol" "Carol")"#, &env).unwrap();
    end_roll_call(&env);

    assert!(
        has_child(&env, "did:ma:carol"),
        "an arrival during the window must not be evicted by the swap"
    );
}

/// A child that reports and then leaves must not be resurrected by the swap.
#[test]
fn roll_call_end_does_not_resurrect_child_that_left_during_the_window() {
    let env = roll_call_env();

    begin_roll_call(&env);
    eval_all(
        r#"((find-method :parent) (list (test-child-ctx "did:ma:alice" "Alice")) alice_msg)"#,
        &env,
    )
    .unwrap();
    eval_all(r#"(forget-child! "did:ma:alice")"#, &env).unwrap();
    end_roll_call(&env);

    assert!(
        !has_child(&env, "did:ma:alice"),
        "a departure during the window must survive the swap"
    );
}

/// Nobody answering is a transport failure, not an empty room.
#[test]
fn roll_call_end_without_any_answers_keeps_the_live_children() {
    let env = roll_call_env();

    begin_roll_call(&env);
    end_roll_call(&env);

    assert!(has_child(&env, "did:ma:alice"));
    assert!(has_child(&env, "did:ma:bob"));
    assert!(!eval_bool("(roll-call-active?)", &env));
    let reply = eval_str("(car (cdr (get-prop \"reply-term:2\")))", &env);
    assert!(
        reply.contains("no answers"),
        "the caller should be told the swap was skipped, got {reply:?}"
    );
}

/// The runtime scheduler dispatches the timed end under its own fragment.
#[test]
fn roll_call_end_accepts_the_runtime_scheduler_as_caller() {
    let env = roll_call_env();
    begin_roll_call(&env);
    eval_all(
        r#"((find-method :parent) (list (test-child-ctx "did:ma:alice" "Alice")) alice_msg)"#,
        &env,
    )
    .unwrap();

    env.define(
        Rc::from("scheduler_msg"),
        rpc_msg(
            "did:ma:runtime#scheduler",
            "did:ma:runtime#room",
            ":roll-call",
        ),
    );
    eval_all(
        "((find-method :roll-call) (list \"end\") scheduler_msg)",
        &env,
    )
    .unwrap();

    assert!(!eval_bool("(roll-call-active?)", &env));
    assert!(has_child(&env, "did:ma:alice"));
    assert!(!has_child(&env, "did:ma:bob"));
}

/// A stranger may neither start nor end a roll-call.
#[test]
fn roll_call_rejects_unauthorised_caller() {
    let env = roll_call_env();
    env.define(
        Rc::from("stranger_msg"),
        rpc_msg("did:ma:mallory", "did:ma:runtime#room", ":roll-call"),
    );

    eval_all(
        "((find-method :roll-call) (list \"begin\") stranger_msg)",
        &env,
    )
    .unwrap();

    assert!(!eval_bool("(roll-call-active?)", &env));
    assert_eq!(
        eval_all("(car (get-prop \"reply-term:1\"))", &env).unwrap(),
        Value::symbol(":error")
    );
}

/// `begin 10` arrives from CBOR as an integer, and schedules a "10s" end.
#[test]
fn roll_call_begin_accepts_integer_seconds_and_schedules_the_end() {
    let env = roll_call_env();

    eval_all(
        "((find-method :roll-call) (list \"begin\" 10) owner_msg)",
        &env,
    )
    .unwrap();

    assert!(eval_bool("(roll-call-active?)", &env));
    // Children are pinged first; the scheduler request is the final send.
    let last = eval_str(
        "(get-prop (string-append \"sent-target:\" (number->string (get-prop \"sent-count\"))))",
        &env,
    );
    assert_eq!(last, "did:ma:runtime#scheduler");
    let spec = eval_str(
        "(car (cdr (cdr (get-prop (string-append \"sent-term:\" (number->string (get-prop \"sent-count\")))))))",
        &env,
    );
    assert_eq!(spec, "10s");
}
