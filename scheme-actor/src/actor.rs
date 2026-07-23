//! Entity-management primitives that cross to the runtime host — currently
//! just `ma-create-actor`, wired to the `ma_create_entity` host function
//! (ma-runtime-v1.md §14.4). Phase 5 territory, same category as
//! `ma_send`/`ma_reply` (not yet wired) — this one lands first as a
//! temporary, pragmatic path to parameterised entity creation (a reusable
//! `behaviour` template plus a per-instance `:init` payload) without
//! requiring a `#root` orchestrator to exist first: any kind that
//! declares `ma_create_entity` in its `host_functions` (e.g.
//! `/ma/scheme/actor/0.0.1`) can call this directly from its own script.

#[cfg(target_arch = "wasm32")]
mod host {
    use extism_pdk::*;

    #[host_fn]
    extern "ExtismHost" {
        fn ma_create_entity(input: Vec<u8>) -> Vec<u8>;
        fn ma_entity_exists(input: Vec<u8>) -> Vec<u8>;
        fn ma_derived_id(input: Vec<u8>) -> Vec<u8>;
    }

    /// `input` is CBOR-encoded `{"kind": text, "behaviour": text/null,
    /// "init": bytes/null}` (ma-runtime-v1.md §14.4's `ma_create_entity`).
    /// Returns the CBOR-encoded fragment string the runtime generated.
    pub fn create_entity(input: &[u8]) -> Result<Vec<u8>, String> {
        unsafe { ma_create_entity(input.to_vec()) }.map_err(|e| e.to_string())
    }

    /// `input` is raw UTF-8: `fragment`, `#fragment`, or a local DID-URL.
    /// Returns raw UTF-8 `true` or `false`.
    pub fn entity_exists(input: &str) -> Result<bool, String> {
        let out =
            unsafe { ma_entity_exists(input.as_bytes().to_vec()) }.map_err(|e| e.to_string())?;
        match std::str::from_utf8(&out).map(str::trim) {
            Ok("true") => Ok(true),
            Ok("false") => Ok(false),
            Ok(other) => Err(format!(
                "ma_entity_exists returned invalid boolean: {other}"
            )),
            Err(e) => Err(format!("ma_entity_exists returned invalid UTF-8: {e}")),
        }
    }

    /// `input` is CBOR-encoded `{"context": text, "hint": text, "bytes": int}`.
    /// Returns raw UTF-8 lower-hex.
    pub fn derived_id(input: &[u8]) -> Result<String, String> {
        let out = unsafe { ma_derived_id(input.to_vec()) }.map_err(|e| e.to_string())?;
        String::from_utf8(out).map_err(|e| format!("ma_derived_id returned invalid UTF-8: {e}"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    pub fn create_entity(_input: &[u8]) -> Result<Vec<u8>, String> {
        Err(
            "ma_create_entity is only available compiled to wasm32 (no host to call natively)"
                .to_string(),
        )
    }

    pub fn entity_exists(_input: &str) -> Result<bool, String> {
        Err(
            "ma_entity_exists is only available compiled to wasm32 (no host to call natively)"
                .to_string(),
        )
    }

    pub fn derived_id(_input: &[u8]) -> Result<String, String> {
        Err(
            "ma_derived_id is only available compiled to wasm32 (no host to call natively)"
                .to_string(),
        )
    }
}

use std::rc::Rc;

use crate::env::Env;
use crate::value::{EvalError, EvalResult, Value};

const ROOT_KIND: &str = "/ma/root/0.0.1";
const ROOM_KIND: &str = "/ma/room/0.0.1";
const AVATAR_KIND: &str = "/ma/avatar/0.0.1";

/// Register `ma-create-actor` into `env`.
pub fn install(env: &Rc<Env>) {
    env.define(
        Rc::from("ma-create-actor"),
        Value::Builtin("ma-create-actor", b_ma_create_actor),
    );
    env.define(
        Rc::from("ma-entity-exists?"),
        Value::Builtin("ma-entity-exists?", b_ma_entity_exists),
    );
    env.define(
        Rc::from("ma-derived-id"),
        Value::Builtin("ma-derived-id", b_ma_derived_id),
    );
}

/// `(ma-derived-id context hint bytes)` — runtime-scoped deterministic ID.
fn b_ma_derived_id(args: &[Value]) -> EvalResult<Value> {
    let kind = crate::state::config_value("kind");
    if !matches!(
        kind.as_deref(),
        Some(ROOT_KIND) | Some(ROOM_KIND) | Some(AVATAR_KIND)
    ) {
        return Err(EvalError::new(
            "ma-derived-id: only root, room, and avatar actors may derive runtime IDs",
        ));
    }

    let [context, hint, bytes] = args else {
        return Err(EvalError::new(format!(
            "ma-derived-id: expected exactly 3 arguments, got {}",
            args.len()
        )));
    };
    let Value::Str(context) = context else {
        return Err(EvalError::new(format!(
            "ma-derived-id: context must be a string, found {}",
            context.type_name()
        )));
    };
    let Value::Str(hint) = hint else {
        return Err(EvalError::new(format!(
            "ma-derived-id: hint must be a string, found {}",
            hint.type_name()
        )));
    };
    let Value::Int(bytes) = bytes else {
        return Err(EvalError::new(format!(
            "ma-derived-id: bytes must be an integer, found {}",
            bytes.type_name()
        )));
    };
    if !(1..=32).contains(bytes) {
        return Err(EvalError::new("ma-derived-id: bytes must be in 1..=32"));
    }

    let cbor = ciborium::Value::Map(vec![
        (
            ciborium::Value::Text("context".to_string()),
            ciborium::Value::Text(context.to_string()),
        ),
        (
            ciborium::Value::Text("hint".to_string()),
            ciborium::Value::Text(hint.to_string()),
        ),
        (
            ciborium::Value::Text("bytes".to_string()),
            ciborium::Value::Integer((*bytes).into()),
        ),
    ]);
    let mut buf = Vec::new();
    ciborium::ser::into_writer(&cbor, &mut buf)
        .map_err(|e| EvalError::new(format!("ma-derived-id: CBOR encode failed: {e}")))?;
    host::derived_id(&buf)
        .map(|id| Value::Str(Rc::from(id.as_str())))
        .map_err(|e| EvalError::new(format!("ma-derived-id: {e}")))
}

/// `(ma-entity-exists? actor)` — true if `actor` names a live local entity.
fn b_ma_entity_exists(args: &[Value]) -> EvalResult<Value> {
    let [target] = args else {
        return Err(EvalError::new(format!(
            "ma-entity-exists?: expected exactly 1 argument, got {}",
            args.len()
        )));
    };
    let Value::Str(target) = target else {
        return Err(EvalError::new(format!(
            "ma-entity-exists?: target must be a string, found {}",
            target.type_name()
        )));
    };
    host::entity_exists(target)
        .map(Value::Bool)
        .map_err(|e| EvalError::new(format!("ma-entity-exists?: {e}")))
}

/// `(ma-create-actor kind behaviour init)` — requests creation of a new
/// entity via the `ma_create_entity` host function.
///
/// - `kind` (string, required) — protocol ID, e.g. `"/ma/scheme/actor/0.0.1"`.
/// - `behaviour` (string or `#f`) — `"/ipfs/<cid>"`/`"/ipns/<key>"`
///   reference to the new entity's own behaviour source, if the kind
///   declares a `behaviour` dialect. `#f` (or omitting any content) means
///   no per-entity behaviour.
/// - `init` (string or `#f`) — raw ma-scheme source text delivered
///   verbatim as the `:init` signal's payload on the new entity's very
///   first load (ma-scheme-v1.md §3.3) — evaluated directly, top-to-bottom,
///   never dispatched to a script-defined `on-signal`. `#f` means no
///   creation payload.
///
/// Returns the new entity's generated fragment (a string) on success.
/// Actual plugin loading happens after the current dispatch returns
/// (ma-runtime-v1.md's `ma_create_entity` semantics) — a successful return
/// here means the request was queued, not that the entity is live yet.
fn b_ma_create_actor(args: &[Value]) -> EvalResult<Value> {
    let (kind, behaviour, init, fragment_hint) = match args {
        [kind, behaviour, init] => (kind, behaviour, init, None),
        [kind, behaviour, init, hint] => (kind, behaviour, init, Some(hint)),
        _ => {
            return Err(EvalError::new(format!(
                "ma-create-actor: expected 3 or 4 arguments (kind behaviour init [fragment-hint]), got {}",
                args.len()
            )));
        }
    };

    let Value::Str(kind) = kind else {
        return Err(EvalError::new(format!(
            "ma-create-actor: kind must be a string, found {}",
            kind.type_name()
        )));
    };
    let behaviour = as_optional_string("ma-create-actor", "behaviour", behaviour)?;
    let init = as_optional_string("ma-create-actor", "init", init)?;
    let fragment_hint = match fragment_hint {
        Some(h) => as_optional_string("ma-create-actor", "fragment-hint", h)?,
        None => None,
    };

    let mut cbor_entries = vec![
        (
            ciborium::Value::Text("kind".to_string()),
            ciborium::Value::Text(kind.to_string()),
        ),
        (
            ciborium::Value::Text("behaviour".to_string()),
            match &behaviour {
                Some(s) => ciborium::Value::Text(s.clone()),
                None => ciborium::Value::Null,
            },
        ),
        (
            ciborium::Value::Text("init".to_string()),
            match &init {
                Some(s) => ciborium::Value::Bytes(s.as_bytes().to_vec()),
                None => ciborium::Value::Null,
            },
        ),
    ];
    if let Some(hint) = &fragment_hint {
        cbor_entries.push((
            ciborium::Value::Text("fragment_hint".to_string()),
            ciborium::Value::Text(hint.clone()),
        ));
    }
    let cbor = ciborium::Value::Map(cbor_entries);
    let mut buf = Vec::new();
    ciborium::ser::into_writer(&cbor, &mut buf)
        .map_err(|e| EvalError::new(format!("ma-create-actor: CBOR encode failed: {e}")))?;

    let out =
        host::create_entity(&buf).map_err(|e| EvalError::new(format!("ma-create-actor: {e}")))?;
    let fragment: String = ciborium::de::from_reader(out.as_slice()).map_err(|e| {
        EvalError::new(format!(
            "ma-create-actor: decoding fragment reply failed: {e}"
        ))
    })?;
    Ok(Value::Str(Rc::from(fragment.as_str())))
}

/// `#f` (or, permissively, `'()`) means "not given"; a string is passed
/// through; anything else is a type error. Mirrors `get-prop`'s existing
/// "absent" convention (`Value::Bool(false)`) elsewhere in this crate.
fn as_optional_string(fname: &str, argname: &str, v: &Value) -> EvalResult<Option<String>> {
    match v {
        Value::Str(s) => Ok(Some(s.to_string())),
        Value::Bool(false) | Value::Nil => Ok(None),
        other => Err(EvalError::new(format!(
            "{fname}: {argname} must be a string or #f, found {}",
            other.type_name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Env;
    use std::collections::HashMap;

    fn env_with_actor() -> Rc<Env> {
        let env = Env::new_root();
        install(&env);
        env
    }

    fn set_kind(kind: &str) {
        let mut config = HashMap::new();
        config.insert("kind".to_string(), kind.to_string());
        crate::state::set_config(config);
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let env = env_with_actor();
        let err =
            crate::eval_all(r#"(ma-create-actor "/ma/scheme/actor/0.0.1")"#, &env).unwrap_err();
        assert!(err.0.contains("expected 3 or 4 arguments"), "{}", err.0);
    }

    #[test]
    fn kind_must_be_a_string() {
        let env = env_with_actor();
        let err = crate::eval_all("(ma-create-actor 42 #f #f)", &env).unwrap_err();
        assert!(err.0.contains("kind must be a string"), "{}", err.0);
    }

    #[test]
    fn behaviour_and_init_accept_string_or_false() {
        let env = env_with_actor();
        let err = crate::eval_all("(ma-create-actor \"/k\" 42 #f)", &env).unwrap_err();
        assert!(
            err.0.contains("behaviour must be a string or #f"),
            "{}",
            err.0
        );

        let err = crate::eval_all("(ma-create-actor \"/k\" #f 42)", &env).unwrap_err();
        assert!(err.0.contains("init must be a string or #f"), "{}", err.0);
    }

    #[test]
    fn derived_id_validates_arguments() {
        let env = env_with_actor();
        set_kind(ROOT_KIND);
        let err = crate::eval_all(r#"(ma-derived-id "ctx" "hint")"#, &env).unwrap_err();
        assert!(err.0.contains("expected exactly 3 arguments"), "{}", err.0);

        let err = crate::eval_all(r#"(ma-derived-id 42 "hint" 8)"#, &env).unwrap_err();
        assert!(err.0.contains("context must be a string"), "{}", err.0);

        let err = crate::eval_all(r#"(ma-derived-id "ctx" 42 8)"#, &env).unwrap_err();
        assert!(err.0.contains("hint must be a string"), "{}", err.0);

        let err = crate::eval_all(r#"(ma-derived-id "ctx" "hint" "8")"#, &env).unwrap_err();
        assert!(err.0.contains("bytes must be an integer"), "{}", err.0);

        let err = crate::eval_all(r#"(ma-derived-id "ctx" "hint" 0)"#, &env).unwrap_err();
        assert!(err.0.contains("bytes must be in 1..=32"), "{}", err.0);
    }

    #[test]
    fn derived_id_is_limited_to_root_room_and_avatar() {
        let env = env_with_actor();
        set_kind("/ma/thing/0.0.1");
        let err = crate::eval_all(r#"(ma-derived-id "ctx" "hint" 8)"#, &env).unwrap_err();
        assert!(
            err.0
                .contains("only root, room, and avatar actors may derive runtime IDs"),
            "{}",
            err.0
        );
    }

    #[test]
    fn derived_id_is_available_to_avatar() {
        let env = env_with_actor();
        set_kind(AVATAR_KIND);
        let err = crate::eval_all(r#"(ma-derived-id "ctx" "hint" 8)"#, &env).unwrap_err();
        assert!(
            err.0
                .contains("ma_derived_id is only available compiled to wasm32"),
            "{}",
            err.0
        );
    }

    /// On the native (non-wasm32) target there is no real Extism host to
    /// call, so a fully well-formed invocation still errors — but the
    /// error must come from the host-call boundary itself (proving CBOR
    /// encoding and argument validation all succeeded), not from
    /// argument/type checking.
    #[test]
    fn well_formed_call_reaches_the_host_boundary() {
        let env = env_with_actor();
        let err = crate::eval_all(
            r#"(ma-create-actor "/ma/scheme/actor/0.0.1" "/ipfs/bafyabc" "(set-prop! \"description\" \"My batcave\")")"#,
            &env,
        )
        .unwrap_err();
        assert!(
            err.0.contains("only available compiled to wasm32"),
            "{}",
            err.0
        );
    }

    #[test]
    fn behaviour_and_init_may_both_be_omitted() {
        let env = env_with_actor();
        let err = crate::eval_all(r#"(ma-create-actor "/ma/scheme/actor/0.0.1" #f #f)"#, &env)
            .unwrap_err();
        // Still hits the (stubbed) host boundary, not an argument error.
        assert!(
            err.0.contains("only available compiled to wasm32"),
            "{}",
            err.0
        );
    }

    #[test]
    fn well_formed_derived_id_reaches_the_host_boundary() {
        let env = env_with_actor();
        set_kind(ROOT_KIND);
        let err = crate::eval_all(
            r#"(ma-derived-id "ma entity-fragment v1" "did:ma:k51user" 8)"#,
            &env,
        )
        .unwrap_err();
        assert!(
            err.0.contains("only available compiled to wasm32"),
            "{}",
            err.0
        );
    }
}
