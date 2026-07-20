//! Entity state table and props primitives (ma-scheme-v1.md §9), plus the
//! read-only `ma-get-config-key` lookup (§9.1).
//!
//! The state table lives in a `thread_local` (Wasm is single-threaded, and
//! an Extism plugin instance is long-lived across dispatches — exactly
//! like the module-level `plugin = MyActor()` singleton pattern used by
//! the Python actors). `get-prop`/`set-prop!`/etc. only ever mutate this
//! in-memory table; nothing is durably persisted until `(ma-save-state!)`
//! is called (§9), which flushes this table through the runtime host.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ciborium::Value as Cbor;

use crate::env::Env;
use crate::value::{EvalError, EvalResult, Value};

thread_local! {
    static PROPS: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
    /// Well-known, per-entity read-only config keys (§9.1) — `self` and
    /// `fragment` at minimum. Populated once at load time from the Extism
    /// plugin config (`extism_pdk::config::get`).
    static CONFIG: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// Replace the live config lookup table. Called once per load (Phase 5's
/// lifecycle wiring reads `config::get("self")` etc. and passes them
/// here) — kept as a plain setter so this module has no direct
/// `extism_pdk` dependency of its own.
pub fn set_config(entries: HashMap<String, String>) {
    CONFIG.with(|c| *c.borrow_mut() = entries);
}

/// Replace the entire in-memory props table — used by `set_state` (§3.1)
/// to restore persisted state at load time.
pub fn load_from_cbor(bytes: &[u8]) -> EvalResult<()> {
    let cbor: Cbor = ciborium::de::from_reader(bytes)
        .map_err(|e| EvalError::new(format!("state CBOR decode error: {e}")))?;
    let Cbor::Map(entries) = cbor else {
        return Err(EvalError::new("persisted state must be a CBOR map"));
    };
    let mut table = HashMap::with_capacity(entries.len());
    for (k, v) in entries {
        let Cbor::Text(key) = k else {
            return Err(EvalError::new("persisted state keys must be strings"));
        };
        table.insert(key, crate::cbor::decode_cbor_value(&v)?);
    }
    PROPS.with(|p| *p.borrow_mut() = table);
    Ok(())
}

/// Snapshot the entire in-memory props table as CBOR bytes — used by
/// `ma-save-state!` (Phase 5) to persist via `ma_set_state`.
pub fn dump_to_cbor() -> EvalResult<Vec<u8>> {
    let entries: Vec<(Cbor, Cbor)> = PROPS.with(|p| -> EvalResult<_> {
        p.borrow()
            .iter()
            .map(|(k, v)| Ok((Cbor::Text(k.clone()), crate::cbor::encode_cbor_value(v)?)))
            .collect()
    })?;
    let mut out = Vec::new();
    ciborium::ser::into_writer(&Cbor::Map(entries), &mut out)
        .map_err(|e| EvalError::new(format!("state CBOR encode error: {e}")))?;
    Ok(out)
}

/// Register the props primitives (§9) and `ma-get-config-key` (§9.1).
/// Deliberately unprefixed for props, per the naming convention in the
/// spec's Abstract — these are internal to the entity's own data, not a
/// "reach out to the runtime" the way `ma-`-prefixed calls are.
pub fn install(env: &Rc<Env>) {
    macro_rules! def {
        ($name:literal, $f:expr) => {
            env.define(Rc::from($name), Value::Builtin($name, $f));
        };
    }
    def!("get-prop", b_get_prop);
    def!("set-prop!", b_set_prop);
    def!("inc-prop!", b_inc_prop);
    def!("del-prop!", b_del_prop);
    def!("has-prop?", b_has_prop);
    def!("ma-get-config-key", b_get_config_key);
}

fn as_key(name: &str, v: &Value) -> EvalResult<String> {
    match v {
        Value::Str(s) => Ok(s.to_string()),
        other => Err(EvalError::new(format!(
            "{name}: key must be a string, found {}",
            other.type_name()
        ))),
    }
}

fn b_get_prop(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "get-prop: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    let key = as_key("get-prop", &args[0])?;
    Ok(PROPS.with(|p| p.borrow().get(&key).cloned().unwrap_or(Value::Bool(false))))
}

fn b_set_prop(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 2 {
        return Err(EvalError::new(format!(
            "set-prop!: expected exactly 2 arguments, got {}",
            args.len()
        )));
    }
    let key = as_key("set-prop!", &args[0])?;
    let value = args[1].clone();
    PROPS.with(|p| p.borrow_mut().insert(key, value));
    Ok(Value::Nil)
}

fn b_inc_prop(args: &[Value]) -> EvalResult<Value> {
    if args.is_empty() || args.len() > 2 {
        return Err(EvalError::new(format!(
            "inc-prop!: expected 1 or 2 arguments, got {}",
            args.len()
        )));
    }
    let key = as_key("inc-prop!", &args[0])?;
    let amount = match args.get(1) {
        Some(Value::Int(i)) => *i,
        Some(other) => {
            return Err(EvalError::new(format!(
                "inc-prop!: amount must be an integer, found {}",
                other.type_name()
            )))
        }
        None => 1,
    };
    let new_value = PROPS.with(|p| -> EvalResult<i64> {
        let mut table = p.borrow_mut();
        let current = match table.get(&key) {
            Some(Value::Int(i)) => *i,
            Some(other) => {
                return Err(EvalError::new(format!(
                    "inc-prop!: existing value at {key:?} is not an integer: {}",
                    other.type_name()
                )))
            }
            None => 0,
        };
        let updated = current
            .checked_add(amount)
            .ok_or_else(|| EvalError::new("inc-prop!: integer overflow"))?;
        table.insert(key.clone(), Value::Int(updated));
        Ok(updated)
    })?;
    Ok(Value::Int(new_value))
}

fn b_del_prop(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "del-prop!: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    let key = as_key("del-prop!", &args[0])?;
    PROPS.with(|p| p.borrow_mut().remove(&key));
    Ok(Value::Nil)
}

fn b_has_prop(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "has-prop?: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    let key = as_key("has-prop?", &args[0])?;
    Ok(Value::Bool(PROPS.with(|p| p.borrow().contains_key(&key))))
}

fn b_get_config_key(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "ma-get-config-key: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    let key = as_key("ma-get-config-key", &args[0])?;
    Ok(CONFIG.with(|c| {
        c.borrow()
            .get(&key)
            .cloned()
            .map_or(Value::Bool(false), Value::str)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // PROPS/CONFIG are thread_locals shared by every test in this module;
    // run everything in this module through a single test so ordering
    // can't cause cross-test interference (cargo runs tests in threads,
    // but thread_local means each *thread* gets its own copy — still,
    // keep it simple and explicit rather than relying on that).
    #[test]
    fn props_and_config_lifecycle() {
        let env = Env::new_root();
        crate::builtins::install(&env);
        install(&env);

        // No props yet: get-prop returns #f, has-prop? is false.
        assert_eq!(
            crate::eval_all("(get-prop \"hp\")", &env).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            crate::eval_all("(has-prop? \"hp\")", &env).unwrap(),
            Value::Bool(false)
        );

        // set-prop! then get-prop round-trips.
        crate::eval_all("(set-prop! \"hp\" 10)", &env).unwrap();
        assert_eq!(
            crate::eval_all("(get-prop \"hp\")", &env).unwrap(),
            Value::Int(10)
        );
        assert_eq!(
            crate::eval_all("(has-prop? \"hp\")", &env).unwrap(),
            Value::Bool(true)
        );

        // inc-prop! with and without an explicit amount.
        assert_eq!(
            crate::eval_all("(inc-prop! \"hp\")", &env).unwrap(),
            Value::Int(11)
        );
        assert_eq!(
            crate::eval_all("(inc-prop! \"hp\" 5)", &env).unwrap(),
            Value::Int(16)
        );
        assert_eq!(
            crate::eval_all("(inc-prop! \"fresh\" 3)", &env).unwrap(),
            Value::Int(3)
        );

        // del-prop! removes it.
        crate::eval_all("(del-prop! \"hp\")", &env).unwrap();
        assert_eq!(
            crate::eval_all("(has-prop? \"hp\")", &env).unwrap(),
            Value::Bool(false)
        );

        // ma-get-config-key reflects whatever set_config populated.
        let mut cfg = HashMap::new();
        cfg.insert("self".to_string(), "did:ma:tester#room".to_string());
        set_config(cfg);
        assert_eq!(
            crate::eval_all("(ma-get-config-key \"self\")", &env).unwrap(),
            Value::str("did:ma:tester#room")
        );
        assert_eq!(
            crate::eval_all("(ma-get-config-key \"missing\")", &env).unwrap(),
            Value::Bool(false)
        );

        // dump/load round-trip through CBOR.
        crate::eval_all("(set-prop! \"name\" \"dog\")", &env).unwrap();
        let dumped = dump_to_cbor().unwrap();
        PROPS.with(|p| p.borrow_mut().clear());
        assert_eq!(
            crate::eval_all("(has-prop? \"name\")", &env).unwrap(),
            Value::Bool(false)
        );
        load_from_cbor(&dumped).unwrap();
        assert_eq!(
            crate::eval_all("(get-prop \"name\")", &env).unwrap(),
            Value::str("dog")
        );
        assert_eq!(
            crate::eval_all("(get-prop \"fresh\")", &env).unwrap(),
            Value::Int(3)
        );

        // Maps are ordinary persisted state values.
        crate::eval_all(
            r#"(set-prop! "exits" (make-map "north" "did:ma:tester#exit"))"#,
            &env,
        )
        .unwrap();
        let dumped = dump_to_cbor().unwrap();
        PROPS.with(|p| p.borrow_mut().clear());
        load_from_cbor(&dumped).unwrap();
        assert_eq!(
            crate::eval_all(r#"(map-ref (get-prop "exits") "north")"#, &env).unwrap(),
            Value::str("did:ma:tester#exit")
        );

        // Reset shared thread_local state so other tests in this binary
        // (if any come to depend on a clean slate) aren't affected.
        PROPS.with(|p| p.borrow_mut().clear());
    }

    #[test]
    fn inc_prop_on_non_integer_is_an_error() {
        let env = Env::new_root();
        install(&env);
        crate::eval_all("(set-prop! \"name\" \"dog\")", &env).unwrap();
        assert!(crate::eval_all("(inc-prop! \"name\")", &env).is_err());
        PROPS.with(|p| p.borrow_mut().clear());
    }

    #[test]
    fn inc_prop_integer_overflow_is_an_error() {
        let env = Env::new_root();
        install(&env);
        crate::eval_all("(set-prop! \"n\" 9223372036854775807)", &env).unwrap();
        assert!(crate::eval_all("(inc-prop! \"n\")", &env).is_err());
        PROPS.with(|p| p.borrow_mut().clear());
    }

    #[test]
    fn load_from_cbor_rejects_non_map() {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&Cbor::Integer(1.into()), &mut bytes).unwrap();
        assert!(load_from_cbor(&bytes).is_err());
    }
}
