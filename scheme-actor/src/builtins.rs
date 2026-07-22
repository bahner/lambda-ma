//! Core builtins (ma-scheme-v1.md §8) — intentionally small; anything else
//! is either user-written ma-scheme or a convention prelude (§15), never
//! grown here without a version bump.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::env::Env;
use crate::value::{EvalError, EvalResult, Value};

/// Register every core builtin into `env` (the root/global environment).
pub fn install(env: &Rc<Env>) {
    macro_rules! def {
        ($name:literal, $f:expr) => {
            env.define(Rc::from($name), Value::Builtin($name, $f));
        };
    }

    // Arithmetic
    def!("+", b_add);
    def!("-", b_sub);
    def!("*", b_mul);
    def!("/", b_div);
    def!("random", b_random);

    // Comparison
    def!("=", b_num_eq);
    def!("<", b_lt);
    def!(">", b_gt);
    def!("<=", b_le);
    def!(">=", b_ge);

    // Boolean
    def!("not", b_not);

    // Pairs/lists
    def!("cons", b_cons);
    def!("car", b_car);
    def!("cdr", b_cdr);
    def!("list", b_list);
    def!("null?", b_null_p);
    def!("pair?", b_pair_p);

    // Type predicates
    def!("string?", b_string_p);
    def!("number?", b_number_p);
    def!("boolean?", b_boolean_p);
    def!("symbol?", b_symbol_p);
    def!("map?", b_map_p);
    def!("procedure?", b_procedure_p);

    // Strings
    def!("string-append", b_string_append);
    def!("string-prefix?", b_string_prefix_p);
    def!("number->string", b_number_to_string);
    def!("string->number", b_string_to_number);

    // Maps
    def!("make-map", b_make_map);
    def!("map-ref", b_map_ref);
    def!("map-set", b_map_set);
    def!("map-delete", b_map_delete);
    def!("map-has-key?", b_map_has_key_p);
    def!("map-keys", b_map_keys);
    def!("map-values", b_map_values);
    def!("map->alist", b_map_to_alist);
    def!("alist->map", b_alist_to_map);

    // Equality
    def!("equal?", b_equal_p);
}

fn as_f64(v: &Value) -> EvalResult<f64> {
    match v {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(x) => Ok(*x),
        other => Err(EvalError::new(format!(
            "expected a number, found {}: {other}",
            other.type_name()
        ))),
    }
}

/// If every argument is an integer, keep integer arithmetic exact;
/// otherwise promote to float. Standard numeric-tower-lite behaviour.
fn all_ints(args: &[Value]) -> bool {
    args.iter().all(|v| matches!(v, Value::Int(_)))
}

fn b_add(args: &[Value]) -> EvalResult<Value> {
    if all_ints(args) {
        let mut sum: i64 = 0;
        for a in args {
            let Value::Int(i) = a else { unreachable!() };
            sum = sum
                .checked_add(*i)
                .ok_or_else(|| EvalError::new("+: integer overflow"))?;
        }
        Ok(Value::Int(sum))
    } else {
        let mut sum = 0.0;
        for a in args {
            sum += as_f64(a)?;
        }
        Ok(Value::Float(sum))
    }
}

fn b_sub(args: &[Value]) -> EvalResult<Value> {
    if args.is_empty() {
        return Err(EvalError::new("-: expected at least 1 argument"));
    }
    if all_ints(args) {
        let Value::Int(first) = args[0] else {
            unreachable!()
        };
        if args.len() == 1 {
            return first
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| EvalError::new("-: integer overflow"));
        }
        let mut acc = first;
        for a in &args[1..] {
            let Value::Int(i) = a else { unreachable!() };
            acc = acc
                .checked_sub(*i)
                .ok_or_else(|| EvalError::new("-: integer overflow"))?;
        }
        Ok(Value::Int(acc))
    } else {
        let first = as_f64(&args[0])?;
        if args.len() == 1 {
            return Ok(Value::Float(-first));
        }
        let mut acc = first;
        for a in &args[1..] {
            acc -= as_f64(a)?;
        }
        Ok(Value::Float(acc))
    }
}

fn b_mul(args: &[Value]) -> EvalResult<Value> {
    if all_ints(args) {
        let mut prod: i64 = 1;
        for a in args {
            let Value::Int(i) = a else { unreachable!() };
            prod = prod
                .checked_mul(*i)
                .ok_or_else(|| EvalError::new("*: integer overflow"))?;
        }
        Ok(Value::Int(prod))
    } else {
        let mut prod = 1.0;
        for a in args {
            prod *= as_f64(a)?;
        }
        Ok(Value::Float(prod))
    }
}

fn b_div(args: &[Value]) -> EvalResult<Value> {
    if args.is_empty() {
        return Err(EvalError::new("/: expected at least 1 argument"));
    }
    let first = as_f64(&args[0])?;
    if args.len() == 1 {
        if first == 0.0 {
            return Err(EvalError::new("/: division by zero"));
        }
        return Ok(Value::Float(1.0 / first));
    }
    let mut acc = first;
    for a in &args[1..] {
        let d = as_f64(a)?;
        if d == 0.0 {
            return Err(EvalError::new("/: division by zero"));
        }
        acc /= d;
    }
    Ok(Value::Float(acc))
}

thread_local! {
    static RANDOM_STATE: Cell<u64> = const { Cell::new(0) };
}

fn mix_seed(mut state: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x1000_0000_01b3);
    }
    state
}

fn random_seed() -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325;
    for key in ["self", "runtime", "started_at", "iroh_node_id", "id"] {
        if let Some(value) = crate::state::config_value(key) {
            seed = mix_seed(seed, key.as_bytes());
            seed = mix_seed(seed, value.as_bytes());
        }
    }
    if seed == 0 {
        1
    } else {
        seed
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn next_random_u64() -> u64 {
    RANDOM_STATE.with(|state| {
        let current = match state.get() {
            0 => random_seed(),
            value => value,
        };
        let next = current.wrapping_add(0x9e37_79b9_7f4a_7c15);
        state.set(next);
        splitmix64(next)
    })
}

fn random_below(n: u64) -> u64 {
    if n == 1 {
        return 0;
    }
    let zone = u64::MAX - (u64::MAX % n);
    loop {
        let value = next_random_u64();
        if value < zone {
            return value % n;
        }
    }
}

fn b_random(args: &[Value]) -> EvalResult<Value> {
    let upper = one_arg("random", args)?;
    let Value::Int(n) = upper else {
        return Err(EvalError::new(format!(
            "random: expected an integer upper bound, found {}",
            upper.type_name()
        )));
    };
    if *n <= 0 {
        return Err(EvalError::new("random: upper bound must be > 0"));
    }
    let n = u64::try_from(*n).map_err(|_| EvalError::new("random: upper bound too large"))?;
    let value = random_below(n);
    i64::try_from(value)
        .map(Value::Int)
        .map_err(|_| EvalError::new("random: result overflow"))
}

fn numeric_chain(args: &[Value], op: fn(f64, f64) -> bool) -> EvalResult<Value> {
    if args.len() < 2 {
        return Err(EvalError::new("expected at least 2 arguments"));
    }
    for w in args.windows(2) {
        let a = as_f64(&w[0])?;
        let b = as_f64(&w[1])?;
        if !op(a, b) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn b_num_eq(args: &[Value]) -> EvalResult<Value> {
    numeric_chain(args, |a, b| a == b)
}
fn b_lt(args: &[Value]) -> EvalResult<Value> {
    numeric_chain(args, |a, b| a < b)
}
fn b_gt(args: &[Value]) -> EvalResult<Value> {
    numeric_chain(args, |a, b| a > b)
}
fn b_le(args: &[Value]) -> EvalResult<Value> {
    numeric_chain(args, |a, b| a <= b)
}
fn b_ge(args: &[Value]) -> EvalResult<Value> {
    numeric_chain(args, |a, b| a >= b)
}

fn b_not(args: &[Value]) -> EvalResult<Value> {
    let v = one_arg("not", args)?;
    Ok(Value::Bool(!v.is_truthy()))
}

fn one_arg<'a>(name: &str, args: &'a [Value]) -> EvalResult<&'a Value> {
    if args.len() != 1 {
        return Err(EvalError::new(format!(
            "{name}: expected exactly 1 argument, got {}",
            args.len()
        )));
    }
    Ok(&args[0])
}

fn two_args<'a>(name: &str, args: &'a [Value]) -> EvalResult<(&'a Value, &'a Value)> {
    if args.len() != 2 {
        return Err(EvalError::new(format!(
            "{name}: expected exactly 2 arguments, got {}",
            args.len()
        )));
    }
    Ok((&args[0], &args[1]))
}

fn b_cons(args: &[Value]) -> EvalResult<Value> {
    let (a, b) = two_args("cons", args)?;
    Ok(Value::cons(a.clone(), b.clone()))
}

fn b_car(args: &[Value]) -> EvalResult<Value> {
    one_arg("car", args)?.car()
}

fn b_cdr(args: &[Value]) -> EvalResult<Value> {
    one_arg("cdr", args)?.cdr()
}

fn b_list(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::list(args.to_vec()))
}

fn b_null_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(one_arg("null?", args)?.is_nil()))
}

fn b_pair_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(one_arg("pair?", args)?.is_pair()))
}

fn b_string_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(matches!(
        one_arg("string?", args)?,
        Value::Str(_)
    )))
}

fn b_number_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(matches!(
        one_arg("number?", args)?,
        Value::Int(_) | Value::Float(_)
    )))
}

fn b_boolean_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(matches!(
        one_arg("boolean?", args)?,
        Value::Bool(_)
    )))
}

fn b_symbol_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(matches!(
        one_arg("symbol?", args)?,
        Value::Symbol(_)
    )))
}

fn b_map_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(matches!(one_arg("map?", args)?, Value::Map(_))))
}

fn b_procedure_p(args: &[Value]) -> EvalResult<Value> {
    Ok(Value::Bool(matches!(
        one_arg("procedure?", args)?,
        Value::Lambda(_) | Value::Builtin(..)
    )))
}

fn b_string_append(args: &[Value]) -> EvalResult<Value> {
    let mut out = String::new();
    for a in args {
        match a {
            Value::Str(s) => out.push_str(s),
            other => {
                return Err(EvalError::new(format!(
                    "string-append: expected a string, found {}",
                    other.type_name()
                )))
            }
        }
    }
    Ok(Value::str(out))
}

fn b_string_prefix_p(args: &[Value]) -> EvalResult<Value> {
    let [prefix, text] = args else {
        return Err(EvalError::new(format!(
            "string-prefix?: expected exactly 2 arguments, got {}",
            args.len()
        )));
    };
    let prefix = as_string("string-prefix?", prefix)?;
    let text = as_string("string-prefix?", text)?;
    Ok(Value::Bool(text.starts_with(&prefix)))
}

fn as_string(name: &str, v: &Value) -> EvalResult<String> {
    match v {
        Value::Str(s) => Ok(s.to_string()),
        other => Err(EvalError::new(format!(
            "{name}: expected a string, found {}",
            other.type_name()
        ))),
    }
}

fn as_map<'a>(name: &str, v: &'a Value) -> EvalResult<&'a BTreeMap<String, Value>> {
    match v {
        Value::Map(m) => Ok(m),
        other => Err(EvalError::new(format!(
            "{name}: expected a map, found {}",
            other.type_name()
        ))),
    }
}

fn b_make_map(args: &[Value]) -> EvalResult<Value> {
    if args.len() % 2 != 0 {
        return Err(EvalError::new(format!(
            "make-map: expected an even number of key/value arguments, got {}",
            args.len()
        )));
    }
    let mut map = BTreeMap::new();
    for pair in args.chunks(2) {
        let key = as_string("make-map", &pair[0])?;
        map.insert(key, pair[1].clone());
    }
    Ok(Value::Map(map))
}

fn b_map_ref(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 2 && args.len() != 3 {
        return Err(EvalError::new(format!(
            "map-ref: expected 2 or 3 arguments, got {}",
            args.len()
        )));
    }
    let map = as_map("map-ref", &args[0])?;
    let key = as_string("map-ref", &args[1])?;
    Ok(map
        .get(&key)
        .cloned()
        .unwrap_or_else(|| args.get(2).cloned().unwrap_or(Value::Bool(false))))
}

fn b_map_set(args: &[Value]) -> EvalResult<Value> {
    if args.len() != 3 {
        return Err(EvalError::new(format!(
            "map-set: expected exactly 3 arguments, got {}",
            args.len()
        )));
    }
    let mut map = as_map("map-set", &args[0])?.clone();
    let key = as_string("map-set", &args[1])?;
    map.insert(key, args[2].clone());
    Ok(Value::Map(map))
}

fn b_map_delete(args: &[Value]) -> EvalResult<Value> {
    let (map, key) = two_args("map-delete", args)?;
    let mut map = as_map("map-delete", map)?.clone();
    let key = as_string("map-delete", key)?;
    map.remove(&key);
    Ok(Value::Map(map))
}

fn b_map_has_key_p(args: &[Value]) -> EvalResult<Value> {
    let (map, key) = two_args("map-has-key?", args)?;
    let map = as_map("map-has-key?", map)?;
    let key = as_string("map-has-key?", key)?;
    Ok(Value::Bool(map.contains_key(&key)))
}

fn b_map_keys(args: &[Value]) -> EvalResult<Value> {
    let map = as_map("map-keys", one_arg("map-keys", args)?)?;
    Ok(Value::list(map.keys().cloned().map(Value::str).collect()))
}

fn b_map_values(args: &[Value]) -> EvalResult<Value> {
    let map = as_map("map-values", one_arg("map-values", args)?)?;
    Ok(Value::list(map.values().cloned().collect()))
}

fn b_map_to_alist(args: &[Value]) -> EvalResult<Value> {
    let map = as_map("map->alist", one_arg("map->alist", args)?)?;
    Ok(Value::list(
        map.iter()
            .map(|(key, value)| Value::cons(Value::str(key), value.clone()))
            .collect(),
    ))
}

fn alist_entry(entry: &Value) -> EvalResult<(String, Value)> {
    let Value::Pair(p) = entry else {
        return Err(EvalError::new(format!(
            "alist->map: entry must be a pair, found {}",
            entry.type_name()
        )));
    };
    let key = as_string("alist->map", &p.0)?;
    let value = match &p.1 {
        Value::Pair(rest) if rest.1.is_nil() => rest.0.clone(),
        other => other.clone(),
    };
    Ok((key, value))
}

fn b_alist_to_map(args: &[Value]) -> EvalResult<Value> {
    let entries = one_arg("alist->map", args)?.to_vec()?;
    let mut map = BTreeMap::new();
    for entry in entries {
        let (key, value) = alist_entry(&entry)?;
        map.insert(key, value);
    }
    Ok(Value::Map(map))
}

fn b_number_to_string(args: &[Value]) -> EvalResult<Value> {
    let v = one_arg("number->string", args)?;
    match v {
        Value::Int(i) => Ok(Value::str(i.to_string())),
        Value::Float(x) => Ok(Value::str(x.to_string())),
        other => Err(EvalError::new(format!(
            "number->string: expected a number, found {}",
            other.type_name()
        ))),
    }
}

fn b_string_to_number(args: &[Value]) -> EvalResult<Value> {
    let v = one_arg("string->number", args)?;
    match v {
        Value::Str(s) => {
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Int(i))
            } else if let Ok(x) = s.parse::<f64>() {
                Ok(Value::Float(x))
            } else {
                Ok(Value::Bool(false))
            }
        }
        other => Err(EvalError::new(format!(
            "string->number: expected a string, found {}",
            other.type_name()
        ))),
    }
}

fn b_equal_p(args: &[Value]) -> EvalResult<Value> {
    let (a, b) = two_args("equal?", args)?;
    Ok(Value::Bool(a == b))
}
