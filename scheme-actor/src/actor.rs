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
    }

    /// `input` is CBOR-encoded `{"kind": text, "behaviour": text/null,
    /// "init": bytes/null}` (ma-runtime-v1.md §14.4's `ma_create_entity`).
    /// Returns the CBOR-encoded fragment string the runtime generated.
    pub fn create_entity(input: &[u8]) -> Result<Vec<u8>, String> {
        unsafe { ma_create_entity(input.to_vec()) }.map_err(|e| e.to_string())
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
}

use std::rc::Rc;

use crate::env::Env;
use crate::value::{EvalError, EvalResult, Value};

/// Register `ma-create-actor` into `env`.
pub fn install(env: &Rc<Env>) {
    env.define(
        Rc::from("ma-create-actor"),
        Value::Builtin("ma-create-actor", b_ma_create_actor),
    );
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
    let [kind, behaviour, init] = args else {
        return Err(EvalError::new(format!(
            "ma-create-actor: expected exactly 3 arguments (kind behaviour init), got {}",
            args.len()
        )));
    };

    let Value::Str(kind) = kind else {
        return Err(EvalError::new(format!(
            "ma-create-actor: kind must be a string, found {}",
            kind.type_name()
        )));
    };
    let behaviour = as_optional_string("ma-create-actor", "behaviour", behaviour)?;
    let init = as_optional_string("ma-create-actor", "init", init)?;

    let cbor = ciborium::Value::Map(vec![
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
    ]);
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

    fn env_with_actor() -> Rc<Env> {
        let env = Env::new_root();
        install(&env);
        env
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let env = env_with_actor();
        let err =
            crate::eval_all(r#"(ma-create-actor "/ma/scheme/actor/0.0.1")"#, &env).unwrap_err();
        assert!(err.0.contains("expected exactly 3 arguments"), "{}", err.0);
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
}
