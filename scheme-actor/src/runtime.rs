//! Runtime-crossing primitives: messaging and explicit state persistence
//! (ma-scheme-v1.md §9-§10), wired to the reference runtime host functions.

#[cfg(target_arch = "wasm32")]
mod host {
    use extism_pdk::*;

    #[host_fn]
    extern "ExtismHost" {
        fn ma_send(input: Vec<u8>) -> Vec<u8>;
        fn ma_reply(input: Vec<u8>) -> Vec<u8>;
        fn ma_set_state(input: Vec<u8>) -> Vec<u8>;
        fn ma_set_behaviour(input: Vec<u8>) -> Vec<u8>;
    }

    pub fn send(input: &[u8]) -> Result<(), String> {
        unsafe { ma_send(input.to_vec()) }
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn reply(input: &[u8]) -> Result<(), String> {
        unsafe { ma_reply(input.to_vec()) }
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn set_state(input: &[u8]) -> Result<(), String> {
        unsafe { ma_set_state(input.to_vec()) }
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn set_behaviour(input: &str) -> Result<(), String> {
        unsafe { ma_set_behaviour(input.as_bytes().to_vec()) }
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    pub fn send(_input: &[u8]) -> Result<(), String> {
        Err("ma_send is only available compiled to wasm32 (no host to call natively)".to_string())
    }

    pub fn reply(_input: &[u8]) -> Result<(), String> {
        Err("ma_reply is only available compiled to wasm32 (no host to call natively)".to_string())
    }

    pub fn set_state(_input: &[u8]) -> Result<(), String> {
        Err(
            "ma_set_state is only available compiled to wasm32 (no host to call natively)"
                .to_string(),
        )
    }

    pub fn set_behaviour(_input: &str) -> Result<(), String> {
        Err(
            "ma_set_behaviour is only available compiled to wasm32 (no host to call natively)"
                .to_string(),
        )
    }
}

use std::rc::Rc;

use ciborium::Value as Cbor;

use crate::env::Env;
use crate::value::{EvalError, EvalResult, Value};

const CONTENT_TYPE_TERM: &str = "application/vnd.ma.term";

/// Register runtime-crossing primitives into `env`.
pub fn install(env: &Rc<Env>) {
    env.define(Rc::from("ma-send!"), Value::Builtin("ma-send!", b_ma_send));
    env.define(
        Rc::from("ma-reply!"),
        Value::Builtin("ma-reply!", b_ma_reply),
    );
    env.define(
        Rc::from("ma-save-state!"),
        Value::Builtin("ma-save-state!", b_ma_save_state),
    );
    env.define(
        Rc::from("ma-set-behaviour!"),
        Value::Builtin("ma-set-behaviour!", b_ma_set_behaviour),
    );
}

fn b_ma_send(args: &[Value]) -> EvalResult<Value> {
    let [target, term] = args else {
        return Err(EvalError::new(format!(
            "ma-send!: expected exactly 2 arguments (target term), got {}",
            args.len()
        )));
    };
    let Value::Str(target) = target else {
        return Err(EvalError::new(format!(
            "ma-send!: target must be a string, found {}",
            target.type_name()
        )));
    };

    let input = encode_send_envelope(target, term)?;
    host::send(&input).map_err(|e| EvalError::new(format!("ma-send!: {e}")))?;
    Ok(Value::Nil)
}

fn b_ma_reply(args: &[Value]) -> EvalResult<Value> {
    let [msg, term] = args else {
        return Err(EvalError::new(format!(
            "ma-reply!: expected exactly 2 arguments (msg term), got {}",
            args.len()
        )));
    };
    let Value::Msg(msg) = msg else {
        return Err(EvalError::new(format!(
            "ma-reply!: first argument must be a msg record, found {}",
            msg.type_name()
        )));
    };

    let input = encode_reply_request(msg, term)?;
    host::reply(&input).map_err(|e| EvalError::new(format!("ma-reply!: {e}")))?;
    Ok(Value::Nil)
}

fn b_ma_save_state(args: &[Value]) -> EvalResult<Value> {
    if !args.is_empty() {
        return Err(EvalError::new(format!(
            "ma-save-state!: expected exactly 0 arguments, got {}",
            args.len()
        )));
    }

    let state = crate::state::dump_to_cbor()?;
    host::set_state(&state).map_err(|e| EvalError::new(format!("ma-save-state!: {e}")))?;
    Ok(Value::Nil)
}

fn b_ma_set_behaviour(args: &[Value]) -> EvalResult<Value> {
    let [reference] = args else {
        return Err(EvalError::new(format!(
            "ma-set-behaviour!: expected exactly 1 argument, got {}",
            args.len()
        )));
    };
    let Value::Str(reference) = reference else {
        return Err(EvalError::new(format!(
            "ma-set-behaviour!: reference must be a string, found {}",
            reference.type_name()
        )));
    };

    host::set_behaviour(reference)
        .map_err(|e| EvalError::new(format!("ma-set-behaviour!: {e}")))?;
    Ok(Value::Nil)
}

fn encode_send_envelope(target: &str, term: &Value) -> EvalResult<Vec<u8>> {
    let content = crate::cbor::encode(term)?;
    encode_cbor(&Cbor::Map(vec![
        (Cbor::Text("to".to_string()), Cbor::Text(target.to_string())),
        (
            Cbor::Text("content_type".to_string()),
            Cbor::Text(CONTENT_TYPE_TERM.to_string()),
        ),
        (Cbor::Text("content".to_string()), Cbor::Bytes(content)),
        (Cbor::Text("reply_to".to_string()), Cbor::Null),
    ]))
}

fn encode_reply_request(msg: &crate::msg::MsgRecord, term: &Value) -> EvalResult<Vec<u8>> {
    let content = crate::cbor::encode(term)?;
    encode_cbor(&Cbor::Map(vec![
        (
            Cbor::Text("msg".to_string()),
            Cbor::Map(vec![
                (Cbor::Text("id".to_string()), Cbor::Text(msg.id.clone())),
                (Cbor::Text("from".to_string()), Cbor::Text(msg.from.clone())),
            ]),
        ),
        (
            Cbor::Text("content_type".to_string()),
            Cbor::Text(CONTENT_TYPE_TERM.to_string()),
        ),
        (Cbor::Text("content".to_string()), Cbor::Bytes(content)),
    ]))
}

fn encode_cbor(value: &Cbor) -> EvalResult<Vec<u8>> {
    let mut out = Vec::new();
    ciborium::ser::into_writer(value, &mut out)
        .map_err(|e| EvalError::new(format!("runtime primitive CBOR encode error: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_get<'a>(map: &'a [(Cbor, Cbor)], key: &str) -> Option<&'a Cbor> {
        map.iter().find_map(|(k, v)| match k {
            Cbor::Text(s) if s == key => Some(v),
            _ => None,
        })
    }

    fn decode(bytes: &[u8]) -> Cbor {
        ciborium::de::from_reader(bytes).unwrap()
    }

    fn sample_msg() -> crate::msg::MsgRecord {
        crate::msg::MsgRecord {
            id: "msg-1".to_string(),
            from: "did:ma:alice".to_string(),
            to: "did:ma:bob#room".to_string(),
            created_at: 0,
            exp: 0,
            reply_to: None,
            msg_type: "application/vnd.ma.rpc.request".to_string(),
            content_type: CONTENT_TYPE_TERM.to_string(),
            content: Value::symbol(":ping"),
        }
    }

    #[test]
    fn send_envelope_matches_runtime_shape() {
        let bytes = encode_send_envelope("did:ma:bob#room", &Value::symbol(":wave")).unwrap();
        let Cbor::Map(map) = decode(&bytes) else {
            panic!("expected map");
        };
        assert_eq!(
            map_get(&map, "to"),
            Some(&Cbor::Text("did:ma:bob#room".to_string()))
        );
        assert_eq!(
            map_get(&map, "content_type"),
            Some(&Cbor::Text(CONTENT_TYPE_TERM.to_string()))
        );
        assert_eq!(map_get(&map, "reply_to"), Some(&Cbor::Null));
        assert_eq!(map_get(&map, "message_type"), None);
        let Some(Cbor::Bytes(content)) = map_get(&map, "content") else {
            panic!("missing content bytes");
        };
        assert_eq!(
            crate::cbor::decode(content).unwrap(),
            Value::symbol(":wave")
        );
    }

    #[test]
    fn reply_request_uses_minimal_msg_ref() {
        let bytes =
            encode_reply_request(&sample_msg(), &Value::list(vec![Value::symbol(":ok")])).unwrap();
        let Cbor::Map(map) = decode(&bytes) else {
            panic!("expected map");
        };
        let Some(Cbor::Map(msg)) = map_get(&map, "msg") else {
            panic!("missing msg map");
        };
        assert_eq!(map_get(msg, "id"), Some(&Cbor::Text("msg-1".to_string())));
        assert_eq!(
            map_get(msg, "from"),
            Some(&Cbor::Text("did:ma:alice".to_string()))
        );
        assert_eq!(map_get(msg, "to"), None);
        assert_eq!(
            map_get(&map, "content_type"),
            Some(&Cbor::Text(CONTENT_TYPE_TERM.to_string()))
        );
    }

    #[test]
    fn native_builtins_reach_host_boundary_after_validation() {
        let env = Env::new_root();
        crate::state::install(&env);
        install(&env);

        let err = crate::eval_all("(ma-send! \"did:ma:bob#room\" :ping)", &env).unwrap_err();
        assert!(err.0.contains("ma_send is only available"), "{}", err.0);

        crate::eval_all("(set-prop! \"hp\" 10)", &env).unwrap();
        let err = crate::eval_all("(ma-save-state!)", &env).unwrap_err();
        assert!(
            err.0.contains("ma_set_state is only available"),
            "{}",
            err.0
        );
    }

    #[test]
    fn reply_requires_msg_record() {
        let env = Env::new_root();
        install(&env);
        let err = crate::eval_all("(ma-reply! 42 :pong)", &env).unwrap_err();
        assert!(
            err.0.contains("first argument must be a msg record"),
            "{}",
            err.0
        );
    }
}
