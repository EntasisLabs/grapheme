//! Evaluation of pure value expressions carried in MIR args.
//!
//! The compiler lowers `$a + 1`, `$x == done`, `if c then a else b` and friends
//! to `{"$expr": {"op": "+", "args": [..]}}`. Runtimes resolve each operand in
//! their own scope and call [`evaluate`].
//!
//! - `false` and `null` are falsy; every other value is truthy.
//! - `&&` and `||` short-circuit and return booleans.
//! - `Int op Int` stays `Int`; `/` truncates toward zero. Mixing `Int` and
//!   `Float` gives `Float`. `+` also joins two strings.
//! - `==`/`!=` compare numbers by value (`1 == 1.0`) and everything else
//!   structurally. `<`, `<=`, `>`, `>=` order two numbers or two strings.
//!
//! Integer overflow, division by zero and operand type mismatches evaluate to
//! an error value (`{"$expr_error": ".."}`) that propagates through enclosing
//! expressions. Runtimes check resolved step args with [`first_error`] and fail
//! the step.

use serde_json::{Map, Value as JsonValue};
use std::cmp::Ordering;

pub const EXPR_KEY: &str = "$expr";
pub const EXPR_ERROR_KEY: &str = "$expr_error";

fn error(message: String) -> JsonValue {
    let mut map = Map::new();
    map.insert(EXPR_ERROR_KEY.to_string(), JsonValue::String(message));
    JsonValue::Object(map)
}

fn error_message(value: &JsonValue) -> Option<&str> {
    let map = value.as_object()?;
    if map.len() != 1 {
        return None;
    }
    map.get(EXPR_ERROR_KEY)?.as_str()
}

/// The first expression error inside resolved args, if any.
pub fn first_error(value: &JsonValue) -> Option<&str> {
    if let Some(message) = error_message(value) {
        return Some(message);
    }
    match value {
        JsonValue::Object(map) => map.values().find_map(first_error),
        JsonValue::Array(items) => items.iter().find_map(first_error),
        _ => None,
    }
}

fn type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "Null",
        JsonValue::Bool(_) => "Bool",
        JsonValue::Number(n) if n.is_f64() => "Float",
        JsonValue::Number(_) => "Int",
        JsonValue::String(_) => "String",
        JsonValue::Array(_) => "List",
        JsonValue::Object(_) => "Object",
    }
}

fn mismatch(op: &str, left: &JsonValue, right: &JsonValue) -> JsonValue {
    error(format!(
        "cannot apply '{op}' to {} and {}",
        type_name(left),
        type_name(right)
    ))
}

/// The `(op, args)` of a `{"$expr": ..}` object, if `map` is one.
pub fn expr_parts(map: &Map<String, JsonValue>) -> Option<(&str, &[JsonValue])> {
    if map.len() != 1 {
        return None;
    }
    let expr = map.get(EXPR_KEY)?.as_object()?;
    let op = expr.get("op")?.as_str()?;
    let args = expr.get("args")?.as_array()?;
    Some((op, args.as_slice()))
}

/// Evaluates `op` over unresolved `args`, resolving operands lazily through
/// `resolve` so `&&`, `||` and `if` only evaluate the branches they need.
pub fn evaluate(
    op: &str,
    args: &[JsonValue],
    mut resolve: impl FnMut(&JsonValue) -> JsonValue,
) -> JsonValue {
    let mut arg = |index: usize| args.get(index).map(&mut resolve).unwrap_or(JsonValue::Null);
    macro_rules! operand {
        ($index:expr) => {{
            let value = arg($index);
            if error_message(&value).is_some() {
                return value;
            }
            value
        }};
    }
    match op {
        "if" => {
            if truthy(&operand!(0)) {
                arg(1)
            } else {
                arg(2)
            }
        }
        "&&" => JsonValue::Bool(truthy(&operand!(0)) && truthy(&operand!(1))),
        "||" => JsonValue::Bool(truthy(&operand!(0)) || truthy(&operand!(1))),
        "!" => JsonValue::Bool(!truthy(&operand!(0))),
        "neg" => negate(&operand!(0)),
        _ => {
            let left = operand!(0);
            let right = operand!(1);
            binary(op, &left, &right)
        }
    }
}

pub fn truthy(value: &JsonValue) -> bool {
    !matches!(value, JsonValue::Null | JsonValue::Bool(false))
}

fn binary(op: &str, left: &JsonValue, right: &JsonValue) -> JsonValue {
    let compare = |accept: fn(Ordering) -> bool| match order(left, right) {
        Some(ordering) => JsonValue::Bool(accept(ordering)),
        None => mismatch(op, left, right),
    };
    match op {
        "==" => JsonValue::Bool(values_equal(left, right)),
        "!=" => JsonValue::Bool(!values_equal(left, right)),
        "<" => compare(Ordering::is_lt),
        "<=" => compare(Ordering::is_le),
        ">" => compare(Ordering::is_gt),
        ">=" => compare(Ordering::is_ge),
        "+" => match (left, right) {
            (JsonValue::String(a), JsonValue::String(b)) => JsonValue::String(format!("{a}{b}")),
            _ => arithmetic(op, left, right, i64::checked_add, |a, b| a + b),
        },
        "-" => arithmetic(op, left, right, i64::checked_sub, |a, b| a - b),
        "*" => arithmetic(op, left, right, i64::checked_mul, |a, b| a * b),
        "/" | "%" if left.is_number() && right.as_f64() == Some(0.0) => {
            error(format!("division by zero in '{op}'"))
        }
        "/" => arithmetic(op, left, right, i64::checked_div, |a, b| a / b),
        "%" => arithmetic(op, left, right, i64::checked_rem, |a, b| a % b),
        _ => error(format!("unknown expression operator '{op}'")),
    }
}

fn arithmetic(
    op: &str,
    left: &JsonValue,
    right: &JsonValue,
    int_op: impl Fn(i64, i64) -> Option<i64>,
    float_op: impl Fn(f64, f64) -> f64,
) -> JsonValue {
    if let (Some(a), Some(b)) = (left.as_i64(), right.as_i64()) {
        return match int_op(a, b) {
            Some(result) => JsonValue::from(result),
            None => error(format!("integer overflow in '{op}'")),
        };
    }
    match (left, right) {
        (JsonValue::Number(a), JsonValue::Number(b)) => float_json(float_op(
            a.as_f64().unwrap_or(0.0),
            b.as_f64().unwrap_or(0.0),
        )),
        _ => mismatch(op, left, right),
    }
}

fn negate(value: &JsonValue) -> JsonValue {
    if let Some(int) = value.as_i64() {
        return int
            .checked_neg()
            .map(JsonValue::from)
            .unwrap_or_else(|| error("integer overflow in '-'".to_string()));
    }
    match value.as_f64() {
        Some(float) => float_json(-float),
        None => error(format!("cannot apply '-' to {}", type_name(value))),
    }
}

fn float_json(value: f64) -> JsonValue {
    serde_json::Number::from_f64(value)
        .map(JsonValue::Number)
        .unwrap_or_else(|| error(format!("non-finite result {value}")))
}

fn values_equal(left: &JsonValue, right: &JsonValue) -> bool {
    match (left, right) {
        (JsonValue::Number(_), JsonValue::Number(_)) => order(left, right) == Some(Ordering::Equal),
        _ => left == right,
    }
}

fn order(left: &JsonValue, right: &JsonValue) -> Option<Ordering> {
    match (left, right) {
        (JsonValue::Number(_), JsonValue::Number(_)) => {
            if let (Some(a), Some(b)) = (left.as_i64(), right.as_i64()) {
                return Some(a.cmp(&b));
            }
            left.as_f64()?.partial_cmp(&right.as_f64()?)
        }
        (JsonValue::String(a), JsonValue::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn eval(op: &str, args: JsonValue) -> JsonValue {
        evaluate(op, args.as_array().unwrap(), |v| v.clone())
    }

    fn error_of(value: JsonValue) -> String {
        first_error(&value).expect("expected an error").to_string()
    }

    #[test]
    fn integer_arithmetic_stays_integer() {
        assert_eq!(eval("+", json!([2, 3])), json!(5));
        assert!(eval("+", json!([2, 3])).is_i64());
        assert_eq!(eval("*", json!([4, -2])), json!(-8));
        assert_eq!(eval("/", json!([7, 2])), json!(3));
        assert_eq!(eval("/", json!([-7, 2])), json!(-3));
        assert_eq!(eval("/", json!([7.0, 2])), json!(3.5));
        assert_eq!(eval("%", json!([7, 3])), json!(1));
        assert_eq!(eval("-", json!([1.5, 1])), json!(0.5));
    }

    #[test]
    fn overflow_division_by_zero_and_mismatches_are_errors() {
        assert_eq!(
            error_of(eval("+", json!([i64::MAX, 1]))),
            "integer overflow in '+'"
        );
        assert_eq!(
            error_of(eval("/", json!([1, 0]))),
            "division by zero in '/'"
        );
        assert_eq!(
            error_of(eval("%", json!([1.5, 0.0]))),
            "division by zero in '%'"
        );
        assert_eq!(
            error_of(eval("-", json!(["a", 1]))),
            "cannot apply '-' to String and Int"
        );
        assert_eq!(
            error_of(eval("<", json!(["a", 1]))),
            "cannot apply '<' to String and Int"
        );
        assert_eq!(
            error_of(eval("neg", json!(["a"]))),
            "cannot apply '-' to String"
        );
    }

    #[test]
    fn errors_propagate_through_enclosing_expressions() {
        let inner = eval("/", json!([1, 0]));
        let outer = evaluate("+", &[inner, json!(1)], |v| v.clone());
        assert_eq!(
            error_of(json!({ "count": outer })),
            "division by zero in '/'"
        );
    }

    #[test]
    fn strings_join_and_order() {
        assert_eq!(eval("+", json!(["a", "b"])), json!("ab"));
        assert_eq!(eval("<", json!(["a", "b"])), json!(true));
    }

    #[test]
    fn equality_compares_numbers_by_value() {
        assert_eq!(eval("==", json!([1, 1.0])), json!(true));
        assert_eq!(eval("!=", json!(["done", "draft"])), json!(true));
        assert_eq!(eval("==", json!([{"a": [1]}, {"a": [1]}])), json!(true));
        assert_eq!(eval(">=", json!([3, 2.5])), json!(true));
    }

    #[test]
    fn logic_short_circuits() {
        let mut seen = Vec::new();
        let args = json!([false, "boom"]);
        let result = evaluate("&&", args.as_array().unwrap(), |v| {
            seen.push(v.clone());
            v.clone()
        });
        assert_eq!(result, json!(false));
        assert_eq!(seen, vec![json!(false)]);

        assert_eq!(eval("||", json!([null, 0])), json!(true));
        assert_eq!(eval("!", json!([null])), json!(true));
        assert_eq!(eval("if", json!([true, "a", "b"])), json!("a"));
        assert_eq!(eval("if", json!([false, "a", "b"])), json!("b"));
    }
}
