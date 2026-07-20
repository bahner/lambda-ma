//! ma-scheme evaluator (ma-scheme-v1.md §7).
//!
//! Uses a trampoline (loop-and-mutate `expr`/`env` instead of recursing)
//! for every required tail position, so self-tail-calls run in O(1) host
//! stack — required by §7 ("MUST NOT impose an artificial recursion-depth
//! or step-count limit"). Non-tail positions (operator/argument
//! evaluation, non-last body expressions, the test in `if`/`cond`, etc.)
//! use ordinary recursive `eval` calls, which is fine — deep *non-tail*
//! recursion is bounded by the host's own native stack per §7/§13.

use std::collections::HashSet;
use std::rc::Rc;

use crate::env::Env;
use crate::value::{EvalError, EvalResult, Lambda, Value};

/// Evaluate `expr` in `env`, trampolining through tail positions.
pub fn eval(expr: &Value, env: &Rc<Env>) -> EvalResult<Value> {
    let mut expr = expr.clone();
    let mut env = Rc::clone(env);

    loop {
        match &expr {
            // `:`-prefixed symbols are self-evaluating "atoms" (ma-scheme-v1.md
            // §5/§6) — used unquoted everywhere on the wire and in scripts
            // (e.g. `(list :ok payload)`), unlike ordinary symbols which
            // require a variable binding.
            Value::Symbol(s) if s.starts_with(':') => return Ok(expr),
            Value::Symbol(s) => return env.lookup(s),
            Value::Int(_)
            | Value::Float(_)
            | Value::Str(_)
            | Value::Bool(_)
            | Value::Nil
            | Value::Map(_) => return Ok(expr),
            Value::Lambda(_) | Value::Builtin(..) | Value::Msg(_) | Value::IpfsRef(_) => {
                return Ok(expr)
            }
            Value::Pair(_) => {
                let items = expr.to_vec()?;
                let Some((head, rest)) = items.split_first() else {
                    return Err(EvalError::new("cannot evaluate empty application"));
                };

                if let Some(form) = head.as_symbol() {
                    match form {
                        "quote" => {
                            if rest.len() != 1 {
                                return Err(EvalError::new(format!(
                                    "quote: expected exactly 1 argument, got {}",
                                    rest.len()
                                )));
                            }
                            return Ok(rest[0].clone());
                        }
                        "if" => {
                            if !(2..=3).contains(&rest.len()) {
                                return Err(EvalError::new(format!(
                                    "if: expected 2 or 3 arguments, got {}",
                                    rest.len()
                                )));
                            }
                            let test = rest
                                .first()
                                .ok_or_else(|| EvalError::new("if: missing test"))?;
                            let test_val = eval(test, &env)?;
                            let branch = if test_val.is_truthy() {
                                rest.get(1)
                            } else {
                                rest.get(2)
                            };
                            match branch {
                                Some(b) => {
                                    expr = b.clone();
                                    continue;
                                }
                                None => return Ok(Value::Nil),
                            }
                        }
                        "cond" => match eval_cond_select(rest, &env)? {
                            Some(next) => {
                                expr = next;
                                continue;
                            }
                            None => return Ok(Value::Nil),
                        },
                        "when" => {
                            let test = rest
                                .first()
                                .ok_or_else(|| EvalError::new("when: missing test"))?;
                            if eval(test, &env)?.is_truthy() {
                                match tail_of_body(&rest[1..], &env)? {
                                    Some(next) => {
                                        expr = next;
                                        continue;
                                    }
                                    None => return Ok(Value::Nil),
                                }
                            }
                            return Ok(Value::Nil);
                        }
                        "unless" => {
                            let test = rest
                                .first()
                                .ok_or_else(|| EvalError::new("unless: missing test"))?;
                            if !eval(test, &env)?.is_truthy() {
                                match tail_of_body(&rest[1..], &env)? {
                                    Some(next) => {
                                        expr = next;
                                        continue;
                                    }
                                    None => return Ok(Value::Nil),
                                }
                            }
                            return Ok(Value::Nil);
                        }
                        "begin" => match tail_of_body(rest, &env)? {
                            Some(next) => {
                                expr = next;
                                continue;
                            }
                            None => return Ok(Value::Nil),
                        },
                        "and" => {
                            if rest.is_empty() {
                                return Ok(Value::Bool(true));
                            }
                            for e in &rest[..rest.len() - 1] {
                                let v = eval(e, &env)?;
                                if !v.is_truthy() {
                                    return Ok(v);
                                }
                            }
                            expr = rest[rest.len() - 1].clone();
                            continue;
                        }
                        "or" => {
                            if rest.is_empty() {
                                return Ok(Value::Bool(false));
                            }
                            for e in &rest[..rest.len() - 1] {
                                let v = eval(e, &env)?;
                                if v.is_truthy() {
                                    return Ok(v);
                                }
                            }
                            expr = rest[rest.len() - 1].clone();
                            continue;
                        }
                        "define" => return eval_define(rest, &env),
                        "lambda" => return eval_lambda(None, rest, &env),
                        "set!" => return eval_set(rest, &env),
                        "let" => {
                            let (new_env, body) = eval_let(rest, &env)?;
                            match tail_of_body(&body, &new_env)? {
                                Some(next) => {
                                    expr = next;
                                    env = new_env;
                                    continue;
                                }
                                None => return Ok(Value::Nil),
                            }
                        }
                        "let*" => {
                            let (new_env, body) = eval_let_star(rest, &env)?;
                            match tail_of_body(&body, &new_env)? {
                                Some(next) => {
                                    expr = next;
                                    env = new_env;
                                    continue;
                                }
                                None => return Ok(Value::Nil),
                            }
                        }
                        "letrec" => {
                            let (new_env, body) = eval_letrec(rest, &env)?;
                            match tail_of_body(&body, &new_env)? {
                                Some(next) => {
                                    expr = next;
                                    env = new_env;
                                    continue;
                                }
                                None => return Ok(Value::Nil),
                            }
                        }
                        _ => {} // fall through to ordinary application below
                    }
                }

                // Ordinary application: evaluate operator and operands,
                // then either trampoline (lambda) or call directly (builtin).
                let f = eval(head, &env)?;
                let args: Vec<Value> = rest
                    .iter()
                    .map(|a| eval(a, &env))
                    .collect::<EvalResult<_>>()?;
                match f {
                    Value::Lambda(lam) => {
                        let new_env = bind_params(&lam, args)?;
                        match tail_of_body(&lam.body, &new_env)? {
                            Some(next) => {
                                expr = next;
                                env = new_env;
                                continue;
                            }
                            None => return Ok(Value::Nil),
                        }
                    }
                    Value::Builtin(_, f) => return f(&args),
                    other => {
                        return Err(EvalError::new(format!(
                            "cannot apply non-procedure: {other}"
                        )))
                    }
                }
            }
        }
    }
}

/// Evaluate every expression in `body` but the last (for effect), and
/// return the last one unevaluated so the caller can trampoline into it
/// as a tail position. `None` if `body` is empty.
fn tail_of_body(body: &[Value], env: &Rc<Env>) -> EvalResult<Option<Value>> {
    if body.is_empty() {
        return Ok(None);
    }
    for e in &body[..body.len() - 1] {
        eval(e, env)?;
    }
    Ok(Some(body[body.len() - 1].clone()))
}

/// Select the tail expression of the first matching `cond` clause. Clauses are
/// `(test expr...)`; a final `(else expr...)` clause is the default branch.
fn eval_cond_select(clauses: &[Value], env: &Rc<Env>) -> EvalResult<Option<Value>> {
    for (index, clause) in clauses.iter().enumerate() {
        let parts = clause.to_vec()?;
        let (test, rest) = parts
            .split_first()
            .ok_or_else(|| EvalError::new("cond: empty clause"))?;
        if matches!(test.as_symbol(), Some("else")) {
            if index + 1 != clauses.len() {
                return Err(EvalError::new("cond: else clause must be last"));
            }
            return tail_of_body(rest, env);
        }
        let test_val = eval(test, env)?;
        if test_val.is_truthy() {
            return tail_of_body(rest, env);
        }
    }
    Ok(None)
}

fn eval_define(rest: &[Value], env: &Rc<Env>) -> EvalResult<Value> {
    let target = rest
        .first()
        .ok_or_else(|| EvalError::new("define: missing target"))?;
    match target {
        // (define name value)
        Value::Symbol(name) => {
            if rest.len() != 2 {
                return Err(EvalError::new(format!(
                    "define: expected exactly 2 arguments, got {}",
                    rest.len()
                )));
            }
            let value_expr = rest
                .get(1)
                .ok_or_else(|| EvalError::new("define: missing value"))?;
            let value = eval(value_expr, env)?;
            let value = name_if_lambda(value, name.clone());
            env.define(name.clone(), value);
            Ok(Value::Nil)
        }
        // (define (name args...) body...)
        Value::Pair(_) => {
            let sig = target.to_vec()?;
            let (name_val, params) = sig
                .split_first()
                .ok_or_else(|| EvalError::new("define: empty function signature"))?;
            let name = name_val
                .as_symbol()
                .ok_or_else(|| EvalError::new("define: function name must be a symbol"))?;
            let params: Vec<Rc<str>> = params
                .iter()
                .map(|p| {
                    p.as_symbol()
                        .map(Rc::from)
                        .ok_or_else(|| EvalError::new("define: parameter must be a symbol"))
                })
                .collect::<EvalResult<_>>()?;
            ensure_unique_names("define", &params)?;
            let body = rest[1..].to_vec();
            if body.is_empty() {
                return Err(EvalError::new("define: function body must not be empty"));
            }
            let lambda = Value::Lambda(Rc::new(Lambda {
                name: Some(Rc::from(name)),
                params,
                body,
                env: Rc::clone(env),
            }));
            env.define(Rc::from(name), lambda);
            Ok(Value::Nil)
        }
        _ => Err(EvalError::new("define: invalid target")),
    }
}

fn name_if_lambda(value: Value, name: Rc<str>) -> Value {
    match value {
        Value::Lambda(lam) if lam.name.is_none() => Value::Lambda(Rc::new(Lambda {
            name: Some(name),
            params: lam.params.clone(),
            body: lam.body.clone(),
            env: Rc::clone(&lam.env),
        })),
        other => other,
    }
}

fn quote_wrap(v: Value) -> Value {
    Value::list(vec![Value::symbol("quote"), v])
}

fn eval_lambda(name: Option<Rc<str>>, rest: &[Value], env: &Rc<Env>) -> EvalResult<Value> {
    let params_expr = rest
        .first()
        .ok_or_else(|| EvalError::new("lambda: missing parameter list"))?;
    let params: Vec<Rc<str>> = params_expr
        .to_vec()?
        .iter()
        .map(|p| {
            p.as_symbol()
                .map(Rc::from)
                .ok_or_else(|| EvalError::new("lambda: parameter must be a symbol"))
        })
        .collect::<EvalResult<_>>()?;
    ensure_unique_names("lambda", &params)?;
    let body = rest[1..].to_vec();
    if body.is_empty() {
        return Err(EvalError::new("lambda: body must not be empty"));
    }
    Ok(Value::Lambda(Rc::new(Lambda {
        name,
        params,
        body,
        env: Rc::clone(env),
    })))
}

fn eval_set(rest: &[Value], env: &Rc<Env>) -> EvalResult<Value> {
    if rest.len() != 2 {
        return Err(EvalError::new(format!(
            "set!: expected exactly 2 arguments, got {}",
            rest.len()
        )));
    }
    let name = rest
        .first()
        .and_then(Value::as_symbol)
        .ok_or_else(|| EvalError::new("set!: expected a symbol as the first argument"))?;
    let value_expr = rest
        .get(1)
        .ok_or_else(|| EvalError::new("set!: missing value"))?;
    let value = eval(value_expr, env)?;
    env.set(name, value)?;
    Ok(Value::Nil)
}

/// Bind a lambda's parameters to evaluated argument values in a fresh
/// child environment (the call frame).
fn bind_params(lam: &Rc<Lambda>, args: Vec<Value>) -> EvalResult<Rc<Env>> {
    if args.len() != lam.params.len() {
        let name = lam.name.as_deref().unwrap_or("<anonymous>");
        return Err(EvalError::new(format!(
            "{name}: expected {} argument(s), got {}",
            lam.params.len(),
            args.len()
        )));
    }
    let call_env = Env::new_child(&lam.env);
    for (param, arg) in lam.params.iter().zip(args) {
        call_env.define(Rc::clone(param), arg);
    }
    Ok(call_env)
}

/// `(let ((name val)...) body...)` — all bindings evaluated in the
/// *outer* environment (no forward reference between bindings).
///
/// Also supports named let: `(let name ((arg val)...) body...)`, equivalent to
/// a local recursive procedure named `name` called with the initial values.
fn eval_let(rest: &[Value], env: &Rc<Env>) -> EvalResult<(Rc<Env>, Vec<Value>)> {
    let bindings_expr = rest
        .first()
        .ok_or_else(|| EvalError::new("let: missing bindings"))?;

    if let Some(name) = bindings_expr.as_symbol() {
        let bindings_expr = rest
            .get(1)
            .ok_or_else(|| EvalError::new("let: missing named-let bindings"))?;
        let body = rest[2..].to_vec();
        if body.is_empty() {
            return Err(EvalError::new("let: named-let body must not be empty"));
        }

        let bindings = bindings_expr.to_vec()?;
        let pairs: Vec<(Rc<str>, Value)> = bindings
            .iter()
            .map(binding_pair)
            .collect::<EvalResult<_>>()?;
        let params: Vec<Rc<str>> = pairs.iter().map(|(param, _)| Rc::clone(param)).collect();
        ensure_unique_names("let", &params)?;
        let args: Vec<Value> = pairs
            .iter()
            .map(|(_, value_expr)| eval(value_expr, env))
            .collect::<EvalResult<_>>()?;

        let new_env = Env::new_child(env);
        let lambda = Value::Lambda(Rc::new(Lambda {
            name: Some(Rc::from(name)),
            params,
            body,
            env: Rc::clone(&new_env),
        }));
        new_env.define(Rc::from(name), lambda);

        let mut call = Vec::with_capacity(args.len() + 1);
        call.push(Value::symbol(name));
        call.extend(args.into_iter().map(quote_wrap));
        return Ok((new_env, vec![Value::list(call)]));
    }

    let body = rest[1..].to_vec();
    if body.is_empty() {
        return Err(EvalError::new("let: body must not be empty"));
    }
    let new_env = Env::new_child(env);
    let pairs: Vec<(Rc<str>, Value)> = bindings_expr
        .to_vec()?
        .iter()
        .map(binding_pair)
        .collect::<EvalResult<_>>()?;
    let names: Vec<Rc<str>> = pairs.iter().map(|(name, _)| Rc::clone(name)).collect();
    ensure_unique_names("let", &names)?;
    for (name, value_expr) in pairs {
        let value = eval(&value_expr, env)?;
        new_env.define(name, value);
    }
    Ok((new_env, body))
}

/// `(let* ((name val)...) body...)` — each binding's value expression is
/// evaluated with all *previous* bindings already visible.
fn eval_let_star(rest: &[Value], env: &Rc<Env>) -> EvalResult<(Rc<Env>, Vec<Value>)> {
    let bindings_expr = rest
        .first()
        .ok_or_else(|| EvalError::new("let*: missing bindings"))?;
    let body = rest[1..].to_vec();
    if body.is_empty() {
        return Err(EvalError::new("let*: body must not be empty"));
    }
    let new_env = Env::new_child(env);
    for binding in bindings_expr.to_vec()? {
        let (name, value_expr) = binding_pair(&binding)?;
        let value = eval(&value_expr, &new_env)?;
        new_env.define(name, value);
    }
    Ok((new_env, body))
}

/// `(letrec ((name val)...) body...)` — all names are pre-bound (to a
/// placeholder) before any value expression is evaluated, so mutually
/// recursive `lambda` definitions can refer to each other and to
/// themselves.
fn eval_letrec(rest: &[Value], env: &Rc<Env>) -> EvalResult<(Rc<Env>, Vec<Value>)> {
    let bindings_expr = rest
        .first()
        .ok_or_else(|| EvalError::new("letrec: missing bindings"))?;
    let body = rest[1..].to_vec();
    if body.is_empty() {
        return Err(EvalError::new("letrec: body must not be empty"));
    }
    let bindings = bindings_expr.to_vec()?;
    let new_env = Env::new_child(env);
    let pairs: Vec<(Rc<str>, Value)> = bindings
        .iter()
        .map(binding_pair)
        .collect::<EvalResult<_>>()?;
    let names: Vec<Rc<str>> = pairs.iter().map(|(name, _)| Rc::clone(name)).collect();
    ensure_unique_names("letrec", &names)?;
    for (name, _) in &pairs {
        new_env.define(Rc::clone(name), Value::Nil);
    }
    for (name, value_expr) in pairs {
        let value = eval(&value_expr, &new_env)?;
        let value = name_if_lambda(value, Rc::clone(&name));
        new_env.define(name, value);
    }
    Ok((new_env, body))
}

/// Parse a `(name value-expr)` binding pair used by `let`/`let*`/`letrec`.
fn binding_pair(binding: &Value) -> EvalResult<(Rc<str>, Value)> {
    let parts = binding.to_vec()?;
    if parts.len() != 2 {
        return Err(EvalError::new("binding must be exactly (name value-expr)"));
    }
    let name = parts[0]
        .as_symbol()
        .map(Rc::from)
        .ok_or_else(|| EvalError::new("binding name must be a symbol"))?;
    Ok((name, parts[1].clone()))
}

fn ensure_unique_names(context: &str, names: &[Rc<str>]) -> EvalResult<()> {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name.as_ref()) {
            return Err(EvalError::new(format!(
                "{context}: duplicate binding name: {name}"
            )));
        }
    }
    Ok(())
}
