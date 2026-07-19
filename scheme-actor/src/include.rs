//! `ma-include-ipfs` top-level-only library composition (ma-scheme-v1.md
//! §11.1).
//!
//! This is **not** a special form `eval()` handles — it is recognized and
//! expanded only when it is a direct top-level form, in a pre-pass that
//! runs once, completely before any evaluation of `set_behaviour`/
//! `do_init` text begins. This is what gives it real `include` semantics
//! (splice into the persistent environment, once, before execution)
//! rather than `load` semantics (re-run wherever reached, isolated scope)
//! — see the design discussion this module implements. Anything nested
//! inside a `define`/`lambda`/`on-message` body is *not* recognized here
//! and falls through to ordinary evaluation, where `ma-include-ipfs` is
//! not installed as a callable procedure — an honest unbound-variable
//! error at that call site, by construction, not by convention.

use crate::parser::Parser;
use crate::value::{EvalError, EvalResult, Value};

/// Mirrors the depth limit the (now-removed) runtime-level directive
/// mechanism used (`rust-ma-runtime`'s former `behaviour.rs`) — recursion
/// bound is a property of *this* algorithm regardless of which layer runs
/// it.
pub const MAX_DEPTH: usize = 16;

/// Expand every top-level `(ma-include-ipfs #/ipfs/<cid>)` /
/// `(ma-include-ipfs #/ipns/<key>)` form in `forms`, recursively, via
/// `fetch` (which resolves a literal reference string, e.g.
/// `"#/ipfs/bafy..."`, to UTF-8 source text — in production this calls
/// the `ma_ipfs_include` host function; tests inject a fake).
pub fn expand_top_level(
    forms: Vec<Value>,
    fetch: &mut dyn FnMut(&str) -> EvalResult<String>,
) -> EvalResult<Vec<Value>> {
    let mut seen = Vec::new();
    expand_inner(forms, fetch, &mut seen, 0)
}

fn expand_inner(
    forms: Vec<Value>,
    fetch: &mut dyn FnMut(&str) -> EvalResult<String>,
    seen: &mut Vec<String>,
    depth: usize,
) -> EvalResult<Vec<Value>> {
    let mut out = Vec::with_capacity(forms.len());
    for form in forms {
        match as_include_reference(&form) {
            Some(reference) => {
                if depth >= MAX_DEPTH {
                    return Err(EvalError::new(format!(
                        "ma-include-ipfs: max recursion depth ({MAX_DEPTH}) exceeded resolving {reference}"
                    )));
                }
                if seen.iter().any(|s| s == &reference) {
                    return Err(EvalError::new(format!(
                        "ma-include-ipfs: cyclic reference detected: {reference}"
                    )));
                }
                let text = fetch(&reference)?;
                let nested_forms = Parser::parse_all(&text)?;
                seen.push(reference);
                let expanded = expand_inner(nested_forms, fetch, seen, depth + 1)?;
                seen.pop();
                out.extend(expanded);
            }
            None => out.push(form),
        }
    }
    Ok(out)
}

/// If `form` is exactly `(ma-include-ipfs <IpfsRef literal>)`, return the
/// reference string. Anything else — including `ma-include-ipfs` used
/// with the wrong argument shape, or with the right shape but nested
/// inside another form — returns `None` and is left untouched (nested
/// occurrences are handled, or rather deliberately *not* handled, by
/// falling through to ordinary evaluation later, per §11.1).
fn as_include_reference(form: &Value) -> Option<String> {
    let items = form.to_vec().ok()?;
    let [head, arg] = items.as_slice() else {
        return None;
    };
    if head.as_symbol() != Some("ma-include-ipfs") {
        return None;
    }
    match arg {
        Value::IpfsRef(r) => Some(r.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fake_fetch(
        files: HashMap<&'static str, &'static str>,
    ) -> impl FnMut(&str) -> EvalResult<String> {
        move |reference: &str| {
            files
                .get(reference)
                .map(|s| s.to_string())
                .ok_or_else(|| EvalError::new(format!("no such fixture: {reference}")))
        }
    }

    #[test]
    fn non_include_forms_pass_through_unchanged() {
        let forms = Parser::parse_all("(define x 1) (define (f) x)").unwrap();
        let mut fetch = fake_fetch(HashMap::new());
        let expanded = expand_top_level(forms.clone(), &mut fetch).unwrap();
        assert_eq!(expanded, forms);
    }

    #[test]
    fn expands_a_single_top_level_include() {
        let mut files = HashMap::new();
        files.insert("#/ipfs/helper", "(define (bump!) (inc-prop! \"n\" 1))");
        let forms =
            Parser::parse_all("(ma-include-ipfs #/ipfs/helper)\n(define (on-message m) (bump!))")
                .unwrap();
        let mut fetch = fake_fetch(files);
        let expanded = expand_top_level(forms, &mut fetch).unwrap();
        assert_eq!(expanded.len(), 2); // (define (bump!) ...) + (define (on-message ...) ...)
        assert_eq!(
            expanded[0],
            Parser::parse_all("(define (bump!) (inc-prop! \"n\" 1))").unwrap()[0]
        );
    }

    #[test]
    fn expands_recursively_nested_includes() {
        let mut files = HashMap::new();
        files.insert("#/ipfs/a", "(ma-include-ipfs #/ipfs/b)\n(define x 1)");
        files.insert("#/ipfs/b", "(define y 2)");
        let forms = Parser::parse_all("(ma-include-ipfs #/ipfs/a)").unwrap();
        let mut fetch = fake_fetch(files);
        let expanded = expand_top_level(forms, &mut fetch).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0], Parser::parse_all("(define y 2)").unwrap()[0]);
        assert_eq!(expanded[1], Parser::parse_all("(define x 1)").unwrap()[0]);
    }

    #[test]
    fn nested_include_inside_a_lambda_body_is_not_expanded() {
        // Not a top-level form -> left untouched. Since ma-include-ipfs is
        // never installed as a callable, this is an honest unbound-variable
        // error later, at eval time -- not this pre-pass's concern.
        let forms =
            Parser::parse_all("(define (on-message m) (ma-include-ipfs #/ipfs/helper))").unwrap();
        let mut fetch = fake_fetch(HashMap::new());
        let expanded = expand_top_level(forms.clone(), &mut fetch).unwrap();
        assert_eq!(expanded, forms);
    }

    #[test]
    fn rejects_cycles() {
        let mut files = HashMap::new();
        files.insert("#/ipfs/a", "(ma-include-ipfs #/ipfs/b)");
        files.insert("#/ipfs/b", "(ma-include-ipfs #/ipfs/a)");
        let forms = Parser::parse_all("(ma-include-ipfs #/ipfs/a)").unwrap();
        let mut fetch = fake_fetch(files);
        assert!(expand_top_level(forms, &mut fetch).is_err());
    }

    #[test]
    fn rejects_too_deep_chains() {
        let mut files = HashMap::new();
        for i in 0..(MAX_DEPTH + 5) {
            let this_ref: &'static str = Box::leak(format!("#/ipfs/n{i}").into_boxed_str());
            let next_ref = format!("(ma-include-ipfs #/ipfs/n{})", i + 1);
            files.insert(
                this_ref,
                Box::leak(next_ref.into_boxed_str()) as &'static str,
            );
        }
        let forms = Parser::parse_all("(ma-include-ipfs #/ipfs/n0)").unwrap();
        let mut fetch = fake_fetch(files);
        assert!(expand_top_level(forms, &mut fetch).is_err());
    }

    #[test]
    fn ipfs_and_ipns_references_both_work() {
        let mut files = HashMap::new();
        files.insert("#/ipns/mykey", "(define z 3)");
        let forms = Parser::parse_all("(ma-include-ipfs #/ipns/mykey)").unwrap();
        let mut fetch = fake_fetch(files);
        let expanded = expand_top_level(forms, &mut fetch).unwrap();
        assert_eq!(
            expanded,
            vec![Parser::parse_all("(define z 3)").unwrap()[0].clone()]
        );
    }
}
