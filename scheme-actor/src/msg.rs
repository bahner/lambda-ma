//! The `msg` record (ma-scheme-v1.md §4) — read-only, host-provided.

use std::rc::Rc;

use crate::value::{EvalError, EvalResult, Value};

/// A single incoming message, decoded from the plugin ABI's `CastInput`
/// (ma-runtime-v1.md §14.3). Read-only from ma-scheme's perspective — a
/// script has no way to mutate it; there is no `set-msg!` of any kind.
pub struct MsgRecord {
    pub id: String,
    pub from: String,
    pub to: String,
    pub created_at: i64,
    /// Absolute Unix-epoch-seconds expiry timestamp (`0` = never expires).
    /// Field name matches the canonical wire format
    /// (ma-messaging-format-v1.md §2, `exp`) and `ma_core::Message.exp` /
    /// `rust-ma-runtime::LocalMessage.exp` exactly — not spelled out as
    /// `expires`.
    pub exp: i64,
    /// Message ID this is a reply to, if any.
    pub reply_to: Option<String>,
    /// Routing/dispatch category MIME type, e.g. `"application/vnd.ma.rpc.request"`.
    pub msg_type: String,
    /// Payload format MIME type, e.g. `"application/vnd.ma.term"`.
    pub content_type: String,
    /// The message body, already decoded per §6.
    pub content: Value,
}

/// Register the `msg` accessors (§4) into `env`. Not part of the core
/// builtins module (§8) — these are ma-scheme-actor specific, added
/// alongside the props/state primitives (§9) once a `msg` value exists to
/// operate on.
pub fn install(env: &std::rc::Rc<crate::env::Env>) {
    macro_rules! def {
        ($name:literal, $f:expr) => {
            env.define(Rc::from($name), Value::Builtin($name, $f));
        };
    }
    def!("msg-id", b_msg_id);
    def!("msg-from", b_msg_from);
    def!("msg-to", b_msg_to);
    def!("msg-created-at", b_msg_created_at);
    def!("msg-exp", b_msg_exp);
    def!("msg-reply-to", b_msg_reply_to);
    def!("msg-type", b_msg_type);
    def!("msg-content-type", b_msg_content_type);
    def!("msg-content", b_msg_content);
    def!("msg?", b_msg_p);
}

fn as_msg<'a>(name: &str, args: &'a [Value]) -> EvalResult<&'a Rc<MsgRecord>> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "{name}: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    match &args[0] {
        Value::Msg(m) => Ok(m),
        other => Err(EvalError::new(format!(
            "{name}: expected a msg record, found {}",
            other.type_name()
        ))),
    }
}

fn b_msg_id(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::str(as_msg("msg-id", args)?.id.clone()))
}

fn b_msg_from(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::str(as_msg("msg-from", args)?.from.clone()))
}

fn b_msg_to(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::str(as_msg("msg-to", args)?.to.clone()))
}

fn b_msg_created_at(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Int(as_msg("msg-created-at", args)?.created_at))
}

fn b_msg_exp(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Int(as_msg("msg-exp", args)?.exp))
}

fn b_msg_reply_to(args: &[Value]) -> EvalResult<Value> {
    match &as_msg("msg-reply-to", args)?.reply_to {
        Some(id) => Ok(Value::str(id.clone())),
        None => Ok(Value::Bool(false)),
    }
}

fn b_msg_type(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::str(as_msg("msg-type", args)?.msg_type.clone()))
}

fn b_msg_content_type(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::str(
        as_msg("msg-content-type", args)?.content_type.clone(),
    ))
}

fn b_msg_content(args: &[Value]) -> EvalResult<Value> {
    Ok(as_msg("msg-content", args)?.content.clone())
}

fn b_msg_p(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "msg?: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    Ok(Value::Bool(matches!(&args[0], Value::Msg(_))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Rc<MsgRecord> {
        Rc::new(MsgRecord {
            id: "msg-1".to_string(),
            from: "did:ma:alice".to_string(),
            to: "did:ma:bob#room".to_string(),
            created_at: 0,
            exp: 0,
            reply_to: None,
            msg_type: "application/vnd.ma.rpc.request".to_string(),
            content_type: "application/vnd.ma.term".to_string(),
            content: Value::symbol(":ping"),
        })
    }

    fn env_with_msg() -> (std::rc::Rc<crate::env::Env>, Value) {
        let env = crate::env::Env::new_root();
        install(&env);
        (env, Value::Msg(sample()))
    }

    #[test]
    fn accessors_read_expected_fields() {
        let (env, msg) = env_with_msg();
        env.define(Rc::from("m"), msg);
        assert_eq!(
            crate::eval_all("(msg-id m)", &env).unwrap(),
            Value::str("msg-1")
        );
        assert_eq!(
            crate::eval_all("(msg-from m)", &env).unwrap(),
            Value::str("did:ma:alice")
        );
        assert_eq!(
            crate::eval_all("(msg-to m)", &env).unwrap(),
            Value::str("did:ma:bob#room")
        );
        assert_eq!(
            crate::eval_all("(msg-reply-to m)", &env).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            crate::eval_all("(msg-type m)", &env).unwrap(),
            Value::str("application/vnd.ma.rpc.request")
        );
        assert_eq!(
            crate::eval_all("(msg-content m)", &env).unwrap(),
            Value::symbol(":ping")
        );
        assert_eq!(
            crate::eval_all("(msg? m)", &env).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            crate::eval_all("(msg? 42)", &env).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn accessor_on_non_msg_is_an_error() {
        let (env, _msg) = env_with_msg();
        env.define(Rc::from("m"), Value::Int(1));
        assert!(crate::eval_all("(msg-id m)", &env).is_err());
    }
}
