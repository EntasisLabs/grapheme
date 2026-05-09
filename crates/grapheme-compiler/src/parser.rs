/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  Parser
///  Walks the pest.rs parse tree and emits typed AST nodes.
///  All error types are collected; parsing is fail-fast per rule.
/// ─────────────────────────────────────────────────────────────

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::*;
use crate::error::GraphemeError;

// ── Pest plumbing ─────────────────────────────────────────────

#[derive(Parser)]
#[grammar = "grapheme.pest"]
pub struct GraphemeParser;

// ── Entry Point ───────────────────────────────────────────────

pub fn parse(source: &str) -> Result<Program, GraphemeError> {
    let pairs = GraphemeParser::parse(Rule::program, source)
        .map_err(|e| GraphemeError::ParseError(e.to_string()))?;

    let program_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| GraphemeError::ParseError("empty program".into()))?;

    parse_program(program_pair)
}

// ── Program ───────────────────────────────────────────────────

fn parse_program(pair: Pair<Rule>) -> Result<Program, GraphemeError> {
    let mut imports = vec![];
    let mut definitions = vec![];

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::import_decl => imports.push(parse_import(inner)?),
            Rule::definition  => {
                // Unwrap the definition wrapper to get the actual variant
                let def = inner.into_inner().next().unwrap();
                match def.as_rule() {
                    Rule::query_def        => definitions.push(Definition::Query(parse_query(def)?)),
                    Rule::mutation_def     => definitions.push(Definition::Mutation(parse_mutation(def)?)),
                    Rule::fragment_def     => definitions.push(Definition::Fragment(parse_fragment(def)?)),
                    Rule::subscription_def => definitions.push(Definition::Subscription(parse_subscription(def)?)),
                    Rule::schema_def       => definitions.push(Definition::Schema(parse_schema(def)?)),
                    Rule::module_proposal  => definitions.push(Definition::ModuleProposal(parse_module_proposal(def)?)),
                    r => return Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
                }
            }
            Rule::EOI => {}
            r => return Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
        }
    }

    Ok(Program { imports, definitions })
}

// ── Imports ───────────────────────────────────────────────────

fn parse_import(pair: Pair<Rule>) -> Result<ImportDecl, GraphemeError> {
    let mut inner = pair.into_inner();
    let alias = inner.next().unwrap().as_str().to_string();
    let path  = parse_string_lit(inner.next().unwrap());
    Ok(ImportDecl { alias, path })
}

// ── Schema ────────────────────────────────────────────────────

fn parse_schema(pair: Pair<Rule>) -> Result<SchemaDef, GraphemeError> {
    let types = pair
        .into_inner()
        .map(parse_type_def)
        .collect::<Result<_, _>>()?;
    Ok(SchemaDef { types })
}

fn parse_type_def(pair: Pair<Rule>) -> Result<TypeDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut directives = vec![];
    let mut fields = vec![];

    for p in inner {
        match p.as_rule() {
            Rule::directive => directives.push(parse_directive(p)?),
            Rule::field_def => fields.push(parse_field_def(p)?),
            _ => {}
        }
    }

    Ok(TypeDef { name, fields, directives })
}

fn parse_field_def(pair: Pair<Rule>) -> Result<FieldDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name     = inner.next().unwrap().as_str().to_string();
    let type_ref = parse_type_ref(inner.next().unwrap())?;
    let directives = inner
        .filter(|p| p.as_rule() == Rule::directive)
        .map(parse_directive)
        .collect::<Result<_, _>>()?;

    Ok(FieldDef { name, type_ref, directives })
}

// ── Type References ───────────────────────────────────────────

fn parse_type_ref(pair: Pair<Rule>) -> Result<TypeRef, GraphemeError> {
    let text = pair.as_str();
    let non_null = text.ends_with('!');
    let base = if non_null { &text[..text.len()-1] } else { text };

    let inner_pair = pair.into_inner().next().unwrap();
    match inner_pair.as_rule() {
        Rule::scalar_type => {
            let kind = match inner_pair.as_str() {
                "String" => ScalarKind::String,
                "Int"    => ScalarKind::Int,
                "Float"  => ScalarKind::Float,
                "Bool"   => ScalarKind::Bool,
                "Any"    => ScalarKind::Any,
                "Json"   => ScalarKind::Json,
                s => return Err(GraphemeError::ParseError(format!("unknown scalar: {s}"))),
            };
            Ok(TypeRef::Scalar(kind, non_null))
        }
        Rule::list_type => {
            let inner_type = parse_type_ref(inner_pair.into_inner().next().unwrap())?;
            Ok(TypeRef::List(Box::new(inner_type), non_null))
        }
        Rule::ident => Ok(TypeRef::Named(base.to_string(), non_null)),
        r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
    }
}

// ── Module Proposals ──────────────────────────────────────────

fn parse_module_proposal(pair: Pair<Rule>) -> Result<ModuleProposal, GraphemeError> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let ops  = inner.map(parse_op_def).collect::<Result<_, _>>()?;
    Ok(ModuleProposal { name, ops })
}

fn parse_op_def(pair: Pair<Rule>) -> Result<OpDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let kind_str  = inner.next().unwrap().as_str();
    let kind      = if kind_str == "query" { OpKind::Query } else { OpKind::Mutation };
    let name      = inner.next().unwrap().as_str().to_string();

    let mut args    = vec![];
    let mut returns = None;

    for p in inner {
        match p.as_rule() {
            Rule::type_arg_list => {
                for arg in p.into_inner() {
                    if arg.as_rule() == Rule::type_named_arg {
                        let mut ai = arg.into_inner();
                        let aname = ai.next().unwrap().as_str().to_string();
                        let atype = parse_type_ref(ai.next().unwrap())?;
                        args.push((aname, atype));
                    }
                }
            }
            Rule::type_ref => returns = Some(parse_type_ref(p)?),
            _ => {}
        }
    }

    Ok(OpDef {
        kind,
        name,
        args,
        returns: returns.ok_or_else(|| GraphemeError::ParseError("op_def missing return type".into()))?,
    })
}

// ── Queries, Mutations, Fragments, Subscriptions ──────────────

fn parse_query(pair: Pair<Rule>) -> Result<QueryDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let (variables, directives, pipelines) = parse_operation_body(inner)?;
    Ok(QueryDef { name, variables, directives, pipelines })
}

fn parse_mutation(pair: Pair<Rule>) -> Result<MutationDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let (variables, directives, pipelines) = parse_operation_body(inner)?;
    Ok(MutationDef { name, variables, directives, pipelines })
}

fn parse_fragment(pair: Pair<Rule>) -> Result<FragmentDef, GraphemeError> {
    let mut inner   = pair.into_inner();
    let name        = inner.next().unwrap().as_str().to_string();
    let on_type     = inner.next().unwrap().as_str().to_string();
    let (_, directives, pipelines) = parse_operation_body(inner)?;
    Ok(FragmentDef { name, on_type, directives, pipelines })
}

fn parse_subscription(pair: Pair<Rule>) -> Result<SubscriptionDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let (variables, directives, pipelines) = parse_operation_body(inner)?;
    Ok(SubscriptionDef { name, variables, directives, pipelines })
}

/// Shared body parser for query/mutation/subscription
fn parse_operation_body<'a>(
    inner: impl Iterator<Item = Pair<'a, Rule>>,
) -> Result<(Vec<VariableDef>, Vec<Directive>, Vec<Pipeline>), GraphemeError> {
    let mut variables  = vec![];
    let mut directives = vec![];
    let mut pipelines  = vec![];

    for p in inner {
        match p.as_rule() {
            Rule::variable_defs => {
                for vd in p.into_inner() {
                    variables.push(parse_variable_def(vd)?);
                }
            }
            Rule::directive  => directives.push(parse_directive(p)?),
            Rule::pipeline   => pipelines.push(parse_pipeline(p)?),
            _ => {}
        }
    }

    Ok((variables, directives, pipelines))
}

fn parse_variable_def(pair: Pair<Rule>) -> Result<VariableDef, GraphemeError> {
    let mut inner = pair.into_inner();
    // grammar emits the ident directly after the $ ($ is silent in the pair)
    let name     = inner.next().unwrap().as_str().to_string();
    let type_ref = parse_type_ref(inner.next().unwrap())?;
    let default  = inner.next().map(parse_value).transpose()?;
    Ok(VariableDef { name, type_ref, default })
}

// ── Pipeline ──────────────────────────────────────────────────

fn parse_pipeline(pair: Pair<Rule>) -> Result<Pipeline, GraphemeError> {
    let mut steps = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::field_call => steps.push(parse_field_call(p)?),
            Rule::pipe_step  => {
                // pipe_step = { "|>" ~ field_call }
                let fc = p.into_inner().next().unwrap();
                steps.push(parse_field_call(fc)?);
            }
            _ => {}
        }
    }

    Ok(Pipeline { steps })
}

// ── Field Calls ───────────────────────────────────────────────

fn parse_field_call(pair: Pair<Rule>) -> Result<FieldCall, GraphemeError> {
    let mut module     = None;
    let mut name       = String::new();
    let mut args       = vec![];
    let mut directives = vec![];
    let mut selection  = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::module_prefix => {
                // module_prefix = { ident ~ "." }
                module = Some(p.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::ident => {
                name = p.as_str().to_string();
            }
            Rule::arg_list => {
                for arg in p.into_inner() {
                    if arg.as_rule() == Rule::named_arg {
                        args.push(parse_named_arg(arg)?);
                    }
                }
            }
            Rule::directive     => directives.push(parse_directive(p)?),
            Rule::selection_set => selection = Some(parse_selection_set(p)?),
            _ => {}
        }
    }

    Ok(FieldCall { module, name, args, directives, selection })
}

fn parse_named_arg(pair: Pair<Rule>) -> Result<(String, Value), GraphemeError> {
    let mut inner = pair.into_inner();
    let key       = inner.next().unwrap().as_str().to_string();
    let val       = parse_value(inner.next().unwrap())?;
    Ok((key, val))
}

// ── Selection Sets ────────────────────────────────────────────

fn parse_selection_set(pair: Pair<Rule>) -> Result<SelectionSet, GraphemeError> {
    let fields = pair
        .into_inner()
        .map(|p| {
            let selected = if p.as_rule() == Rule::selected_field {
                p.into_inner().next().unwrap()
            } else {
                p
            };
            parse_selected_field(selected)
        })
        .collect::<Result<_, _>>()?;
    Ok(SelectionSet { fields })
}

fn parse_selected_field(pair: Pair<Rule>) -> Result<SelectedField, GraphemeError> {
    match pair.as_rule() {
        Rule::spread_field => {
            let name = pair.into_inner().next().unwrap().as_str().to_string();
            Ok(SelectedField::Spread(name))
        }
        Rule::state_field => {
            let selectors = pair
                .into_inner()
                .map(|p| match p.as_str() {
                    "current"  => Ok(StateSelector::Current),
                    "diff"     => Ok(StateSelector::Diff),
                    "errors"   => Ok(StateSelector::Errors),
                    "pipeline" => Ok(StateSelector::Pipeline),
                    "proposed" => Ok(StateSelector::Proposed),
                    s => Err(GraphemeError::ParseError(format!("unknown state selector: {s}"))),
                })
                .collect::<Result<_, _>>()?;
            Ok(SelectedField::State(selectors))
        }
        Rule::aliased_field => {
            let mut inner = pair.into_inner();
            let alias     = inner.next().unwrap().as_str().to_string();
            let fc        = parse_field_call(inner.next().unwrap())?;
            Ok(SelectedField::Aliased(alias, Box::new(fc)))
        }
        Rule::plain_field => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::field_call => Ok(SelectedField::Plain(parse_field_call(inner)?)),
                Rule::ident      => Ok(SelectedField::Bare(inner.as_str().to_string())),
                r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
            }
        }
        r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
    }
}

// ── Values ────────────────────────────────────────────────────

fn parse_value(pair: Pair<Rule>) -> Result<Value, GraphemeError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::int_lit    => Ok(Value::Int(inner.as_str().parse().unwrap())),
        Rule::float_lit  => Ok(Value::Float(inner.as_str().parse().unwrap())),
        Rule::bool_lit   => Ok(Value::Bool(inner.as_str() == "true")),
        Rule::null_lit   => Ok(Value::Null),
        Rule::string_lit => Ok(Value::String(parse_string_lit(inner))),
        Rule::variable   => {
            // variable = ${ "$" ~ ident }
            let name = inner.into_inner().next().unwrap().as_str().to_string();
            Ok(Value::Variable(name))
        }
        Rule::list_value => {
            let items = inner
                .into_inner()
                .map(parse_value)
                .collect::<Result<_, _>>()?;
            Ok(Value::List(items))
        }
        Rule::object_value => {
            let fields = inner
                .into_inner()
                .map(|f| {
                    let mut fi  = f.into_inner();
                    let key     = fi.next().unwrap().as_str().to_string();
                    let val     = parse_value(fi.next().unwrap())?;
                    Ok((key, val))
                })
                .collect::<Result<_, _>>()?;
            Ok(Value::Object(fields))
        }
        r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
    }
}

// ── Directives ────────────────────────────────────────────────

fn parse_directive(pair: Pair<Rule>) -> Result<Directive, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let args      = inner
        .filter(|p| p.as_rule() == Rule::named_arg)
        .map(parse_named_arg)
        .collect::<Result<_, _>>()?;
    Ok(Directive { name, args })
}

// ── Helpers ───────────────────────────────────────────────────

fn parse_string_lit(pair: Pair<Rule>) -> String {
    let raw = pair.as_str();
    // Strip surrounding quotes
    raw[1..raw.len()-1].to_string()
}
