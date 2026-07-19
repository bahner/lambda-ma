//! The lifecycle (ma-scheme-v1.md §3) wired to the two Extism exports,
//! `on_message` and `on_signal`, and `CastInput` decoding (ma-runtime-v1.md
//! §14.3) for `on_message`.

use std::cell::RefCell;
use std::rc::Rc;

use ciborium::Value as Cbor;

use crate::env::Env;
use crate::eval::eval;
use crate::msg::MsgRecord;
use crate::parser::Parser;
use crate::value::{EvalError, EvalResult, Value};

/// The real `ma_ipfs_include` host function (ma-runtime-v1.md §14.2.2),
/// used by `ma-include-ipfs` (ma-scheme-v1.md §11.1) to resolve a literal
/// `#/ipfs/<cid>`/`#/ipns/<key>` reference to UTF-8 source text.
///
/// wasm32-only: an `extern "ExtismHost"` block declares a real Wasm
/// import, which the native (`cargo test`) target has nothing to link
/// against. Kept behind `cfg` rather than making every caller injectable
/// in production, so the plugin_fn entry points stay simple; tests that
/// need to exercise the expansion algorithm itself use
/// `include::expand_top_level` directly with a fake fetch closure (see
/// `include.rs`'s own tests) rather than going through this module.
#[cfg(target_arch = "wasm32")]
mod host_ipfs {
    use crate::value::{EvalError, EvalResult};
    use extism_pdk::*;

    #[host_fn]
    extern "ExtismHost" {
        fn ma_ipfs_include(reference: String) -> Vec<u8>;
    }

    pub fn fetch(reference: &str) -> EvalResult<String> {
        let bytes = unsafe { ma_ipfs_include(reference.to_string()) }
            .map_err(|e| EvalError::new(format!("ma_ipfs_include: {e}")))?;
        String::from_utf8(bytes).map_err(|e| {
            EvalError::new(format!("ma_ipfs_include: response is not valid UTF-8: {e}"))
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod host_ipfs {
    use crate::value::{EvalError, EvalResult};

    pub fn fetch(_reference: &str) -> EvalResult<String> {
        Err(EvalError::new(
            "ma_ipfs_include is only available compiled to wasm32 (no host to call natively)",
        ))
    }
}

/// Read the well-known, runtime-assigned config keys (ma-runtime-v1.md
/// §14.3 — `self`/`id`/`kind`/`cid`/`behaviour`/`runtime`/`iroh_node_id`/
/// `started_at`/`parent`) out of the Extism plugin's own config map and
/// hand them to `state::set_config` so `ma-get-config-key` (§9.1) can
/// read them. Absent keys (e.g. `behaviour`/`parent` for an entity with
/// neither) are simply not inserted. Idempotent and cheap — safe to call
/// on every `on_signal` invocation rather than tracking "already loaded"
/// state of its own.
#[cfg(target_arch = "wasm32")]
fn load_config() {
    const KEYS: &[&str] = &[
        "self",
        "id",
        "kind",
        "cid",
        "behaviour",
        "runtime",
        "iroh_node_id",
        "started_at",
        "parent",
        "root",
        "start",
        "i18n",
        "did_document_publishing_interval_secs",
        "did_document_publishing_timeout_secs",
        "did_document_publishing_lifetime_hours",
        "ipns_publish_lifetime_hours",
        "ipns_publish_resolve",
        "ipns_publish_allow_offline",
        "kubo_rpc_url",
        "kubo_key_alias",
        "log_level",
        "log_level_stdout",
        "did_resolver_positive_ttl_secs",
        "did_resolver_negative_ttl_secs",
        "log_file",
        "ipv6_enable",
    ];
    let mut map = std::collections::HashMap::new();
    for key in KEYS {
        if let Ok(Some(value)) = extism_pdk::config::get(key) {
            map.insert((*key).to_string(), value);
        }
    }
    crate::state::set_config(map);
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config() {
    // No real Extism host to read config from natively; tests populate
    // `state::CONFIG` directly via `state::set_config` instead.
}

/// Parse `src`, expand every top-level `ma-include-ipfs` form (§11.1)
/// using the real host fetch, and evaluate the fully-expanded forms in
/// order against `env`. Used by both `:set-behaviour` (§3.2) and `:init`
/// (§3.3) handling — both are one-shot, load-time-only evaluations of
/// host-given text, exactly where top-level-only `ma-include-ipfs`
/// expansion is meant to run.
fn eval_with_includes(src: &str, env: &Rc<Env>) -> EvalResult<()> {
    let forms = Parser::parse_all(src)?;
    let expanded = crate::include::expand_top_level(forms, &mut host_ipfs::fetch)?;
    for form in &expanded {
        eval(form, env)?;
    }
    Ok(())
}

thread_local! {
    /// The script's live environment, populated while handling
    /// `:set-behaviour` (and further extended in place while handling
    /// `:init`, per §3.3 — both evaluate into the *same* environment).
    /// `None` until the first of those runs; lazily created with just the
    /// builtins installed if a kind somehow dispatches before either has
    /// run.
    static SCRIPT_ENV: RefCell<Option<Rc<Env>>> = const { RefCell::new(None) };
}

/// Fresh environment with every builtin this host provides: core (§8),
/// props/config (§9/§9.1), `msg` accessors (§4), and `ma-create-actor`
/// (entity creation — a pragmatic temporary path, see `crate::actor`).
pub fn new_full_env() -> Rc<Env> {
    let env = Env::new_root();
    crate::builtins::install(&env);
    crate::state::install(&env);
    crate::msg::install(&env);
    crate::runtime::install(&env);
    crate::actor::install(&env);
    env
}

fn get_or_init_env() -> Rc<Env> {
    SCRIPT_ENV.with(|e| {
        let mut slot = e.borrow_mut();
        if slot.is_none() {
            *slot = Some(new_full_env());
        }
        Rc::clone(slot.as_ref().unwrap())
    })
}

/// Look up `name` in the script environment and, if it is bound to a
/// callable (lambda or builtin), apply it to `args`. Returns `Ok(None)`
/// if `name` is unbound — this is not an error at this level; callers
/// decide what an absent hook means (`on_message` replies `:error`,
/// `on-signal`'s `:start`/`:shutdown` handling is silently a no-op per
/// §3.4/§3.6).
fn call_if_defined(name: &str, args: &[Value]) -> EvalResult<Option<Value>> {
    let env = get_or_init_env();
    let Ok(f) = env.lookup(name) else {
        return Ok(None);
    };
    match f {
        Value::Lambda(_) | Value::Builtin(..) => {
            let call_expr = Value::list(
                std::iter::once(Value::symbol(name))
                    .chain(args.iter().cloned().map(quote_wrap))
                    .collect(),
            );
            Ok(Some(eval(&call_expr, &env)?))
        }
        other => Err(EvalError::new(format!(
            "{name}: bound to a non-procedure value ({}), cannot call it",
            other.type_name()
        ))),
    }
}

/// Wrap an already-evaluated `Value` in `(quote ...)` so it can be spliced
/// back into a freshly-built call expression without being re-evaluated
/// (e.g. a `msg` record must be passed through as-is, not interpreted as
/// code).
fn quote_wrap(v: Value) -> Value {
    Value::list(vec![Value::symbol("quote"), v])
}

/// `:set-state` — host-mechanical, restores persisted state.
fn handle_set_state(state_bytes: &[u8]) -> EvalResult<()> {
    if state_bytes.is_empty() {
        return Ok(());
    }
    crate::state::load_from_cbor(state_bytes)
}

/// `:set-behaviour` — host-mechanical, parses and evaluates the resolved
/// behaviour text top-to-bottom into a fresh environment. Expands any
/// top-level `ma-include-ipfs` forms first (§11.1).
fn handle_set_behaviour(text: &str) -> EvalResult<()> {
    let env = new_full_env();
    eval_with_includes(text, &env)?;
    SCRIPT_ENV.with(|e| *e.borrow_mut() = Some(env));
    Ok(())
}

/// `:init` — host-mechanical, evaluates the creation payload as ma-scheme
/// source directly into the *same* environment `:set-behaviour` already
/// populated (falls back to a fresh one if `:set-behaviour` never fired,
/// e.g. a kind with no behaviour dialect at all — not this kind's normal
/// case, but kept robust). Expands any top-level `ma-include-ipfs` forms
/// in the payload first (§11.1), same as `:set-behaviour`.
fn handle_init(payload: &str) -> EvalResult<()> {
    if payload.is_empty() {
        return Ok(());
    }
    let env = get_or_init_env();
    eval_with_includes(payload, &env)?;
    Ok(())
}

/// `on_signal` — the single Wasm export (besides `on_message`) for every
/// runtime-originated lifecycle event (ma-scheme-v1.md §3). Decodes the
/// incoming CBOR term (a bare atom, or a two-element `[atom, data]`
/// array — exactly the same shape as a message dispatch term, §6) and
/// dispatches:
///
/// - `:set-state`/`:set-behaviour`/`:init` are handled entirely by this
///   host's own fixed logic (above) — never looked up as a script
///   function. `:set-behaviour` in particular *cannot* be dispatched to a
///   script handler: the script's own definitions don't exist until this
///   very call finishes evaluating the given text.
/// - Anything else (`:start`, `:shutdown`, or any future signal this
///   specification does not yet define) looks up and calls a single,
///   optional script-defined `on-signal` function with the term, exactly
///   the way `on_message` looks up and calls `on-message`. A script that
///   doesn't define `on-signal`, or whose `on-signal` doesn't recognize a
///   given atom, silently does nothing — there is no fallback of any kind.
pub fn on_signal(input: &[u8]) -> EvalResult<()> {
    load_config();

    let cbor: Cbor = ciborium::de::from_reader(input)
        .map_err(|e| EvalError::new(format!("on_signal: CBOR decode error: {e}")))?;

    let (verb, data): (String, Option<Cbor>) = match cbor {
        Cbor::Text(s) => (s, None),
        Cbor::Array(ref items) if items.len() == 2 => match &items[0] {
            Cbor::Text(s) => (s.clone(), Some(items[1].clone())),
            _ => return Err(EvalError::new("on_signal: term head must be a text atom")),
        },
        _ => {
            return Err(EvalError::new(
                "on_signal: term must be a bare atom or a two-element [atom, data] array",
            ))
        }
    };

    let bytes_data = |signal: &str| -> EvalResult<Vec<u8>> {
        match &data {
            Some(Cbor::Bytes(b)) => Ok(b.clone()),
            _ => Err(EvalError::new(format!(
                "on_signal: {signal} requires bytes data"
            ))),
        }
    };
    let utf8_data = |signal: &str| -> EvalResult<String> {
        String::from_utf8(bytes_data(signal)?).map_err(|e| {
            EvalError::new(format!("on_signal: {signal} data is not valid UTF-8: {e}"))
        })
    };

    match verb.as_str() {
        ":set-state" => handle_set_state(&bytes_data(":set-state")?),
        ":set-behaviour" => handle_set_behaviour(&utf8_data(":set-behaviour")?),
        ":init" => handle_init(&utf8_data(":init")?),
        _ => {
            let term = match data {
                None => Value::symbol(verb),
                Some(d) => Value::list(vec![
                    Value::symbol(verb),
                    crate::cbor::decode_cbor_value(&d)?,
                ]),
            };
            call_if_defined("on-signal", &[term])?;
            Ok(())
        }
    }
}

/// `on_message` (§3.5) — required script hook. Returns `Ok(None)` if the
/// script does not define `on-message` at all — the caller (the Extism
/// export) decides how to surface that (§3.5: reply `[:error, "no
/// behaviour configured"]` if a reply is expected, otherwise drop it
/// silently).
pub fn on_message(msg: Rc<MsgRecord>) -> EvalResult<Option<Value>> {
    call_if_defined("on-message", &[Value::Msg(msg)])
}

/// Decode a `CastInput` (ma-runtime-v1.md §14.3) into a [`MsgRecord`].
///
/// `created_at`/`exp` (Unix epoch seconds; `exp == 0` means never) and
/// `message_type` are all required fields on the wire — the reference
/// runtime's `PluginMsg` restored `created_at`/`exp` once the primary
/// guest target became this Rust host (they were previously stripped only
/// to work around a Python/`cbor2`-specific decode bug that does not apply
/// here).
pub fn decode_cast_input(bytes: &[u8]) -> EvalResult<Rc<MsgRecord>> {
    let cbor: Cbor = ciborium::de::from_reader(bytes)
        .map_err(|e| EvalError::new(format!("CastInput CBOR decode error: {e}")))?;
    let Cbor::Map(top) = cbor else {
        return Err(EvalError::new("CastInput must be a CBOR map"));
    };
    let msg_cbor = find_map_entry(&top, "msg")
        .ok_or_else(|| EvalError::new("CastInput missing 'msg' field"))?;
    let Cbor::Map(msg_fields) = msg_cbor else {
        return Err(EvalError::new("CastInput.msg must be a CBOR map"));
    };

    let text_field = |name: &str| -> EvalResult<String> {
        match find_map_entry(msg_fields, name) {
            Some(Cbor::Text(s)) => Ok(s.clone()),
            Some(_) => Err(EvalError::new(format!("CastInput.msg.{name} must be text"))),
            None => Err(EvalError::new(format!("CastInput.msg missing '{name}'"))),
        }
    };
    let optional_text_field = |name: &str| -> EvalResult<Option<String>> {
        match find_map_entry(msg_fields, name) {
            Some(Cbor::Text(s)) => Ok(Some(s.clone())),
            Some(Cbor::Null) | None => Ok(None),
            Some(_) => Err(EvalError::new(format!(
                "CastInput.msg.{name} must be text or null"
            ))),
        }
    };
    let int_field = |name: &str| -> EvalResult<i64> {
        match find_map_entry(msg_fields, name) {
            Some(Cbor::Integer(i)) => {
                crate::cbor::integer_to_i64(*i, &format!("CastInput.msg.{name}"))
            }
            Some(_) => Err(EvalError::new(format!(
                "CastInput.msg.{name} must be an integer"
            ))),
            None => Err(EvalError::new(format!("CastInput.msg missing '{name}'"))),
        }
    };

    let content_bytes = match find_map_entry(msg_fields, "content") {
        Some(Cbor::Bytes(b)) => b.clone(),
        Some(_) => return Err(EvalError::new("CastInput.msg.content must be bytes")),
        None => return Err(EvalError::new("CastInput.msg missing 'content'")),
    };
    let content = crate::cbor::decode(&content_bytes)?;

    Ok(Rc::new(MsgRecord {
        id: text_field("id")?,
        from: text_field("from")?,
        to: text_field("to")?,
        created_at: int_field("created_at")?,
        exp: int_field("exp")?,
        reply_to: optional_text_field("reply_to")?,
        msg_type: text_field("message_type")?,
        content_type: text_field("content_type")?,
        content,
    }))
}

fn find_map_entry<'a>(entries: &'a [(Cbor, Cbor)], key: &str) -> Option<&'a Cbor> {
    entries.iter().find_map(|(k, v)| match k {
        Cbor::Text(s) if s == key => Some(v),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        SCRIPT_ENV.with(|e| *e.borrow_mut() = None);
        crate::state::set_config(std::collections::HashMap::new());
    }

    /// Build a minimal CastInput CBOR payload matching the reference
    /// runtime's `PluginMsg` shape (ma-runtime-v1.md §14.3), with a
    /// ma-scheme-encoded `content` (§6: a colon-symbol here, `:ping`).
    fn sample_cast_input() -> Vec<u8> {
        let content = crate::cbor::encode(&Value::symbol(":ping")).unwrap();
        let msg = Cbor::Map(vec![
            (Cbor::Text("id".into()), Cbor::Text("msg-1".into())),
            (Cbor::Text("from".into()), Cbor::Text("did:ma:alice".into())),
            (
                Cbor::Text("to".into()),
                Cbor::Text("did:ma:bob#room".into()),
            ),
            (Cbor::Text("created_at".into()), Cbor::Integer(0.into())),
            (Cbor::Text("exp".into()), Cbor::Integer(0.into())),
            (Cbor::Text("reply_to".into()), Cbor::Null),
            (
                Cbor::Text("message_type".into()),
                Cbor::Text("application/vnd.ma.rpc.request".into()),
            ),
            (
                Cbor::Text("content_type".into()),
                Cbor::Text("application/vnd.ma.term".into()),
            ),
            (Cbor::Text("content".into()), Cbor::Bytes(content)),
        ]);
        let top = Cbor::Map(vec![(Cbor::Text("msg".into()), msg)]);
        let mut out = Vec::new();
        ciborium::ser::into_writer(&top, &mut out).unwrap();
        out
    }

    /// Encode a bare-atom signal term (e.g. `:start`), matching the wire
    /// shape `on_signal` expects.
    fn atom_signal(verb: &str) -> Vec<u8> {
        let mut out = Vec::new();
        ciborium::ser::into_writer(&Cbor::Text(verb.to_string()), &mut out).unwrap();
        out
    }

    /// Encode a `[atom, bytes]` signal term (e.g. `:set-state`/
    /// `:set-behaviour`/`:init`), matching the wire shape `on_signal`
    /// expects.
    fn data_signal(verb: &str, data: &[u8]) -> Vec<u8> {
        let term = Cbor::Array(vec![
            Cbor::Text(verb.to_string()),
            Cbor::Bytes(data.to_vec()),
        ]);
        let mut out = Vec::new();
        ciborium::ser::into_writer(&term, &mut out).unwrap();
        out
    }

    #[test]
    fn full_signal_lifecycle() {
        reset();

        // :set-state before any behaviour is loaded: restores persisted
        // props (§3.1). Empty bytes is a no-op (fresh entity) — sent here
        // as a genuinely empty state blob.
        on_signal(&data_signal(":set-state", b"")).unwrap();

        // :set-behaviour (§3.2): defines on-signal/on-message and a
        // helper, all sharing one environment.
        let behaviour_text = br#"
            (define (on-signal term)
              (cond ((equal? term :start) (set-prop! "started" #t))
                    ((equal? term :shutdown) (set-prop! "shutdown" #t))))
            (define (on-message m)
              (inc-prop! "received")
              (set-prop! "last-content-type" (msg-content-type m)))
            "#;
        on_signal(&data_signal(":set-behaviour", behaviour_text)).unwrap();

        // :init (§3.3): host-mechanical, evaluates straight into the SAME
        // environment :set-behaviour populated — a defined helper from
        // :set-behaviour must already be visible, and anything :init
        // defines must survive into later stages too.
        on_signal(&data_signal(":init", br#"(set-prop! "name" "fido")"#)).unwrap();
        {
            let env = get_or_init_env();
            assert_eq!(
                crate::eval_all("(get-prop \"name\")", &env).unwrap(),
                Value::str("fido")
            );
        }

        // :start (§3.4): calls the script's on-signal with the :start term.
        on_signal(&atom_signal(":start")).unwrap();
        {
            let env = get_or_init_env();
            assert_eq!(
                crate::eval_all("(get-prop \"started\")", &env).unwrap(),
                Value::Bool(true)
            );
        }

        // on_message (§3.5): decodes a CastInput and calls on-message.
        let msg = decode_cast_input(&sample_cast_input()).unwrap();
        assert_eq!(msg.content, Value::symbol(":ping"));
        on_message(msg).unwrap();
        {
            let env = get_or_init_env();
            assert_eq!(
                crate::eval_all("(get-prop \"received\")", &env).unwrap(),
                Value::Int(1)
            );
            assert_eq!(
                crate::eval_all("(get-prop \"last-content-type\")", &env).unwrap(),
                Value::str("application/vnd.ma.term")
            );
        }

        // :shutdown (§3.6): calls the script's on-signal with the
        // :shutdown term.
        on_signal(&atom_signal(":shutdown")).unwrap();
        {
            let env = get_or_init_env();
            assert_eq!(
                crate::eval_all("(get-prop \"shutdown\")", &env).unwrap(),
                Value::Bool(true)
            );
        }

        reset();
    }

    #[test]
    fn start_and_shutdown_are_noop_when_on_signal_undefined() {
        reset();
        on_signal(&data_signal(":set-behaviour", b"(define (on-message m) m)")).unwrap();
        assert!(on_signal(&atom_signal(":start")).is_ok());
        assert!(on_signal(&atom_signal(":shutdown")).is_ok());
        reset();
    }

    #[test]
    fn set_state_is_never_dispatched_to_the_scripts_on_signal() {
        // Even if a script defines on-signal, :set-state must never reach
        // it — it is host-mechanical only, handled entirely inside
        // handle_set_state. Uses a real (non-empty-bytes) CBOR map so the
        // early-return-on-empty-bytes path in handle_set_state isn't what
        // makes this test pass trivially.
        reset();
        on_signal(&data_signal(
            ":set-behaviour",
            br#"(define (on-signal term) (inc-prop! "on-signal-calls" 1))"#,
        ))
        .unwrap();
        let mut state_bytes = Vec::new();
        ciborium::ser::into_writer(&Cbor::Map(vec![]), &mut state_bytes).unwrap();
        on_signal(&data_signal(":set-state", &state_bytes)).unwrap();
        let env = get_or_init_env();
        assert_eq!(
            crate::eval_all("(has-prop? \"on-signal-calls\")", &env).unwrap(),
            Value::Bool(false)
        );
        reset();
    }

    #[test]
    fn on_message_returns_none_when_undefined() {
        reset();
        on_signal(&data_signal(":set-behaviour", b"(define x 1)")).unwrap();
        let msg = decode_cast_input(&sample_cast_input()).unwrap();
        assert_eq!(on_message(msg).unwrap(), None);
        reset();
    }

    #[test]
    fn on_signal_rejects_malformed_term() {
        let mut not_a_term = Vec::new();
        ciborium::ser::into_writer(&Cbor::Integer(1.into()), &mut not_a_term).unwrap();
        assert!(on_signal(&not_a_term).is_err());
    }

    #[test]
    fn decode_cast_input_rejects_missing_msg_field() {
        let top = Cbor::Map(vec![]);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&top, &mut bytes).unwrap();
        assert!(decode_cast_input(&bytes).is_err());
    }

    #[test]
    fn decode_cast_input_rejects_out_of_range_timestamps() {
        let mut content = Vec::new();
        ciborium::ser::into_writer(&Cbor::Text(":ping".into()), &mut content).unwrap();
        let too_large = ciborium::value::Integer::try_from(i128::from(i64::MAX) + 1).unwrap();
        let msg = Cbor::Map(vec![
            (Cbor::Text("id".into()), Cbor::Text("msg-1".into())),
            (Cbor::Text("from".into()), Cbor::Text("did:ma:alice".into())),
            (
                Cbor::Text("to".into()),
                Cbor::Text("did:ma:bob#room".into()),
            ),
            (Cbor::Text("created_at".into()), Cbor::Integer(too_large)),
            (Cbor::Text("exp".into()), Cbor::Integer(0.into())),
            (Cbor::Text("reply_to".into()), Cbor::Null),
            (
                Cbor::Text("message_type".into()),
                Cbor::Text("application/vnd.ma.rpc.request".into()),
            ),
            (
                Cbor::Text("content_type".into()),
                Cbor::Text("application/vnd.ma.term".into()),
            ),
            (Cbor::Text("content".into()), Cbor::Bytes(content)),
        ]);
        let top = Cbor::Map(vec![(Cbor::Text("msg".into()), msg)]);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&top, &mut bytes).unwrap();

        assert!(decode_cast_input(&bytes).is_err());
    }

    #[test]
    fn decode_cast_input_rejects_malformed_reply_to() {
        let content = crate::cbor::encode(&Value::symbol(":ping")).unwrap();
        let msg = Cbor::Map(vec![
            (Cbor::Text("id".into()), Cbor::Text("msg-1".into())),
            (Cbor::Text("from".into()), Cbor::Text("did:ma:alice".into())),
            (
                Cbor::Text("to".into()),
                Cbor::Text("did:ma:bob#room".into()),
            ),
            (Cbor::Text("created_at".into()), Cbor::Integer(0.into())),
            (Cbor::Text("exp".into()), Cbor::Integer(0.into())),
            (Cbor::Text("reply_to".into()), Cbor::Integer(1.into())),
            (
                Cbor::Text("message_type".into()),
                Cbor::Text("application/vnd.ma.rpc.request".into()),
            ),
            (
                Cbor::Text("content_type".into()),
                Cbor::Text("application/vnd.ma.term".into()),
            ),
            (Cbor::Text("content".into()), Cbor::Bytes(content)),
        ]);
        let top = Cbor::Map(vec![(Cbor::Text("msg".into()), msg)]);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&top, &mut bytes).unwrap();

        assert!(decode_cast_input(&bytes).is_err());
    }
}
