//! Static typing for value expressions (RFC-0007 Part B).
//!
//! Types come from the executable's input type, its params and literals.
//! Anything else (locals, loop items, untyped input, `Any`) is `Unknown` and
//! accepted, so only definite mismatches are reported.

use std::collections::{HashMap, HashSet};
use std::fmt;

use grapheme_artifact::expr::expr_parts;
use serde_json::Value as JsonValue;

use crate::ast::{ScalarKind, TypeRef};
use crate::hir::{HirParam, HirStep};

#[derive(Debug, Clone, PartialEq)]
enum Ty {
    Int,
    Float,
    Str,
    /// String literals or bare symbols: a `String`, or members of an enum.
    Lit(Vec<String>),
    Bool,
    Null,
    Enum(String),
    Struct(String),
    List(Box<Ty>),
    Unknown,
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Int => f.write_str("Int"),
            Ty::Float => f.write_str("Float"),
            Ty::Str | Ty::Lit(_) => f.write_str("String"),
            Ty::Bool => f.write_str("Bool"),
            Ty::Null => f.write_str("Null"),
            Ty::Enum(name) | Ty::Struct(name) => f.write_str(name),
            Ty::List(inner) => write!(f, "[{inner}]"),
            Ty::Unknown => f.write_str("Any"),
        }
    }
}

impl Ty {
    fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }

    fn is_stringy(&self) -> bool {
        matches!(self, Ty::Str | Ty::Lit(_))
    }
}

pub(crate) struct ExprTypeContext<'a> {
    pub input_type: Option<&'a TypeRef>,
    pub output_type: Option<&'a TypeRef>,
    pub params: &'a [HirParam],
    pub struct_field_types: &'a HashMap<String, HashMap<String, TypeRef>>,
    pub enum_members: &'a HashMap<String, HashSet<String>>,
}

impl ExprTypeContext<'_> {
    fn ty_of(&self, type_ref: &TypeRef) -> Ty {
        match type_ref {
            TypeRef::Scalar(kind, _) => match kind {
                ScalarKind::String => Ty::Str,
                ScalarKind::Int => Ty::Int,
                ScalarKind::Float => Ty::Float,
                ScalarKind::Bool => Ty::Bool,
                ScalarKind::Any | ScalarKind::Json => Ty::Unknown,
            },
            TypeRef::Named(name, _) if self.enum_members.contains_key(name) => {
                Ty::Enum(name.clone())
            }
            TypeRef::Named(name, _) if self.struct_field_types.contains_key(name) => {
                Ty::Struct(name.clone())
            }
            TypeRef::Named(..) => Ty::Unknown,
            TypeRef::List(inner, _) => Ty::List(Box::new(self.ty_of(inner))),
        }
    }

    fn field(&self, owner: &Ty, field: &str, last: bool) -> Ty {
        match owner {
            Ty::Struct(name) => self
                .struct_field_types
                .get(name)
                .and_then(|fields| fields.get(field))
                .map(|type_ref| self.ty_of(type_ref))
                .unwrap_or(Ty::Unknown),
            Ty::List(_) | Ty::Str if field == "count" && last => Ty::Int,
            _ => Ty::Unknown,
        }
    }

    fn walk(&self, mut ty: Ty, path: &str) -> Ty {
        let segments: Vec<&str> = path.split('.').collect();
        for (index, segment) in segments.iter().enumerate() {
            ty = self.field(&ty, segment, index + 1 == segments.len());
            if ty == Ty::Unknown {
                break;
            }
        }
        ty
    }

    fn variable(&self, reference: &str) -> Ty {
        let (root, path) = match reference.split_once('.') {
            Some((root, path)) => (root, Some(path)),
            None => (reference, None),
        };
        let (root_ty, path) = match (root, path) {
            ("state" | "current", path) => (
                self.input_type
                    .map(|type_ref| self.ty_of(type_ref))
                    .unwrap_or(Ty::Unknown),
                path,
            ),
            ("args", Some(path)) => match path.split_once('.') {
                Some((name, rest)) => (self.param(name), Some(rest)),
                None => (self.param(path), None),
            },
            ("args", None) => (Ty::Unknown, None),
            (name, path) => (self.param(name), path),
        };
        match path {
            Some(path) => self.walk(root_ty, path),
            None => root_ty,
        }
    }

    fn param(&self, name: &str) -> Ty {
        self.params
            .iter()
            .find(|param| param.name == name)
            .map(|param| self.ty_of(&param.type_ref))
            .unwrap_or(Ty::Unknown)
    }

    fn check_member(&self, enum_name: &str, literals: &[String]) -> Result<(), String> {
        let Some(members) = self.enum_members.get(enum_name) else {
            return Ok(());
        };
        match literals
            .iter()
            .find(|literal| !literal.contains('{') && !members.contains(*literal))
        {
            Some(literal) => Err(format!("'{literal}' is not a member of enum '{enum_name}'")),
            None => Ok(()),
        }
    }

    fn infer(&self, value: &JsonValue) -> Result<Ty, String> {
        Ok(match value {
            JsonValue::Null => Ty::Null,
            JsonValue::Bool(_) => Ty::Bool,
            JsonValue::Number(n) if n.is_f64() => Ty::Float,
            JsonValue::Number(_) => Ty::Int,
            JsonValue::String(text) => Ty::Lit(vec![text.clone()]),
            JsonValue::Array(_) => Ty::List(Box::new(Ty::Unknown)),
            JsonValue::Object(map) => {
                if let Some((op, args)) = expr_parts(map) {
                    return self.infer_expr(op, args);
                }
                match map.get("$var").and_then(JsonValue::as_str) {
                    Some(reference) if map.len() == 1 => self.variable(reference),
                    _ => Ty::Unknown,
                }
            }
        })
    }

    fn infer_expr(&self, op: &str, args: &[JsonValue]) -> Result<Ty, String> {
        let arg = |index: usize| match args.get(index) {
            Some(value) => self.infer(value),
            None => Ok(Ty::Unknown),
        };
        let expect_bool = |ty: &Ty| match ty {
            Ty::Bool | Ty::Unknown => Ok(()),
            other => Err(format!("'{op}' expects Bool, found {other}")),
        };
        match op {
            "if" => {
                let cond = arg(0)?;
                if !matches!(cond, Ty::Bool | Ty::Unknown) {
                    return Err(format!("'if' condition must be Bool, found {cond}"));
                }
                self.unify_branches(arg(1)?, arg(2)?)
            }
            "&&" | "||" => {
                expect_bool(&arg(0)?)?;
                expect_bool(&arg(1)?)?;
                Ok(Ty::Bool)
            }
            "!" => {
                expect_bool(&arg(0)?)?;
                Ok(Ty::Bool)
            }
            "neg" => match arg(0)? {
                ty @ (Ty::Int | Ty::Float | Ty::Unknown) => Ok(ty),
                other => Err(format!("cannot apply '-' to {other}")),
            },
            "==" | "!=" => {
                self.comparable(&arg(0)?, &arg(1)?, op)?;
                Ok(Ty::Bool)
            }
            "<" | "<=" | ">" | ">=" => {
                let (left, right) = (arg(0)?, arg(1)?);
                let ordered = left == Ty::Unknown
                    || right == Ty::Unknown
                    || (left.is_numeric() && right.is_numeric())
                    || (left.is_stringy() && right.is_stringy());
                if ordered {
                    Ok(Ty::Bool)
                } else {
                    Err(format!("cannot apply '{op}' to {left} and {right}"))
                }
            }
            "+" | "-" | "*" | "/" | "%" => {
                let (left, right) = (arg(0)?, arg(1)?);
                match (&left, &right) {
                    (Ty::Unknown, _) | (_, Ty::Unknown) => Ok(Ty::Unknown),
                    (Ty::Int, Ty::Int) => Ok(Ty::Int),
                    (a, b) if a.is_numeric() && b.is_numeric() => Ok(Ty::Float),
                    (a, b) if op == "+" && a.is_stringy() && b.is_stringy() => Ok(Ty::Str),
                    _ => Err(format!("cannot apply '{op}' to {left} and {right}")),
                }
            }
            _ => Ok(Ty::Unknown),
        }
    }

    fn comparable(&self, left: &Ty, right: &Ty, op: &str) -> Result<(), String> {
        match (left, right) {
            (Ty::Unknown, _) | (_, Ty::Unknown) | (Ty::Null, _) | (_, Ty::Null) => Ok(()),
            (Ty::Enum(name), Ty::Lit(literal)) | (Ty::Lit(literal), Ty::Enum(name)) => {
                self.check_member(name, literal)
            }
            (a, b) if a.is_numeric() && b.is_numeric() => Ok(()),
            (a, b) if a.is_stringy() && b.is_stringy() => Ok(()),
            (Ty::List(_), Ty::List(_)) => Ok(()),
            (a, b) if a == b => Ok(()),
            _ => Err(format!("cannot apply '{op}' to {left} and {right}")),
        }
    }

    fn unify_branches(&self, then: Ty, otherwise: Ty) -> Result<Ty, String> {
        Ok(match (then, otherwise) {
            (Ty::Unknown, other) | (other, Ty::Unknown) => other,
            (Ty::Null, other) | (other, Ty::Null) => other,
            (Ty::Lit(mut a), Ty::Lit(b)) => {
                a.extend(b);
                Ty::Lit(a)
            }
            (Ty::Lit(_), Ty::Str) | (Ty::Str, Ty::Lit(_)) => Ty::Str,
            (Ty::Enum(name), Ty::Lit(literal)) | (Ty::Lit(literal), Ty::Enum(name)) => {
                self.check_member(&name, &literal)?;
                Ty::Enum(name)
            }
            (Ty::List(_), Ty::List(_)) => Ty::List(Box::new(Ty::Unknown)),
            (a, b) if a == b => a,
            (a, b) => {
                return Err(format!(
                    "'if' branches must have the same type, found {a} and {b}"
                ));
            }
        })
    }

    fn assignable(&self, target: &Ty, value: &Ty) -> Result<bool, String> {
        Ok(match (target, value) {
            (Ty::Unknown, _) | (_, Ty::Unknown) | (_, Ty::Null) => true,
            (Ty::Float, Ty::Int) => true,
            (Ty::Str, Ty::Lit(_)) => true,
            (Ty::Enum(name), Ty::Lit(literal)) => {
                self.check_member(name, literal)?;
                true
            }
            (Ty::List(_), Ty::List(_)) => true,
            (a, b) => a == b,
        })
    }

    fn check_assignable(&self, field: &str, target: &Ty, value: &Ty) -> Result<(), String> {
        match self.assignable(target, value) {
            Ok(true) => Ok(()),
            Ok(false) => Err(format!(
                "field '{field}' has type {target} but its expression has type {value}"
            )),
            Err(message) => Err(format!("field '{field}': {message}")),
        }
    }

    /// An expression body's value must fit the declared `-> T`.
    fn check_body_result(&self, value: &JsonValue) -> Result<(), String> {
        let ty = self.infer(value)?;
        let Some(output) = self.output_type else {
            return Ok(());
        };
        let target = self.ty_of(output);
        match self.assignable(&target, &ty) {
            Ok(true) => Ok(()),
            Ok(false) => Err(format!(
                "expression body has type {ty} but the declared output type is {target}"
            )),
            Err(message) => Err(format!("expression body: {message}")),
        }
    }

    fn patch_target(&self) -> Ty {
        let struct_ty = |type_ref: Option<&TypeRef>| match type_ref.map(|t| self.ty_of(t)) {
            Some(ty @ Ty::Struct(_)) => Some(ty),
            _ => None,
        };
        struct_ty(self.output_type)
            .or_else(|| struct_ty(self.input_type))
            .unwrap_or(Ty::Unknown)
    }

    /// Type errors in the value expressions of one step.
    pub(crate) fn check_step(&self, step: &HirStep) -> Vec<String> {
        let mut errors = Vec::new();
        if step.module.as_deref() == Some("core") && step.op == "value" {
            if let Some(Err(message)) = step.args.get("value").map(|v| self.check_body_result(v)) {
                errors.push(message);
            }
            return errors;
        }
        let patch = match (step.module.as_deref(), step.op.as_str()) {
            (Some("core"), "set_fields") => step.args.get("fields"),
            (Some("core"), "merge") => step.args.get("right"),
            _ => None,
        }
        .and_then(JsonValue::as_object);

        if let Some(patch) = patch {
            let target = self.patch_target();
            for (field, value) in patch {
                if !is_expr(value) {
                    self.collect(value, &mut errors);
                    continue;
                }
                match self.infer(value) {
                    Ok(ty) => {
                        let field_ty = self.field(&target, field, false);
                        if let Err(message) = self.check_assignable(field, &field_ty, &ty) {
                            errors.push(message);
                        }
                    }
                    Err(message) => errors.push(message),
                }
            }
            for (key, value) in step.args.as_object().into_iter().flatten() {
                if key != "fields" && key != "right" {
                    self.collect(value, &mut errors);
                }
            }
            return errors;
        }

        self.collect(&step.args, &mut errors);
        errors
    }

    fn collect(&self, value: &JsonValue, errors: &mut Vec<String>) {
        match value {
            _ if is_expr(value) => {
                if let Err(message) = self.infer(value) {
                    errors.push(message);
                }
            }
            JsonValue::Object(map) => map.values().for_each(|v| self.collect(v, errors)),
            JsonValue::Array(items) => items.iter().for_each(|v| self.collect(v, errors)),
            _ => {}
        }
    }
}

fn is_expr(value: &JsonValue) -> bool {
    value.as_object().and_then(expr_parts).is_some()
}
