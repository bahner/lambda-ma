//! Core builtins (ma-scheme-v1.md §8) — intentionally small; anything else
//! is either user-written ma-scheme or a convention prelude (§15), never
//! grown here without a version bump.

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
    def!("procedure?", b_procedure_p);

    // Strings
    def!("string-append", b_string_append);
    def!("number->string", b_number_to_string);
    def!("string->number", b_string_to_number);

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
