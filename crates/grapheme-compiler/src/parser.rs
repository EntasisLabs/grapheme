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

#[derive(Default)]
struct ParseState {
    synthetic_counter: usize,
    synthetic_iterators: Vec<IteratorDef>,
}

impl ParseState {
    fn push_inline_target_iterator(&mut self, steps: Vec<PipelineStep>) -> String {
        let name = format!("__inline_target_{}", self.synthetic_counter);
        self.synthetic_counter += 1;

        self.synthetic_iterators.push(IteratorDef {
            name: name.clone(),
            signature: ExecutableSignature {
                input: TypeRef::Scalar(ScalarKind::Any, false),
                output: Some(TypeRef::Scalar(ScalarKind::Any, false)),
            },
            directives: vec![],
            pipelines: vec![Pipeline { steps }],
        });

        name
    }
}

// ── Pest plumbing ─────────────────────────────────────────────

#[derive(Parser)]
#[grammar = "grapheme.pest"]
pub struct GraphemeParser;

// ── Entry Point ───────────────────────────────────────────────

pub fn parse(source: &str) -> Result<Program, GraphemeError> {
    let normalized_source = normalize_intent_attributes(source);
    let pairs = GraphemeParser::parse(Rule::program, &normalized_source)
        .map_err(|e| GraphemeError::ParseError(e.to_string()))?;

    let program_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| GraphemeError::ParseError("empty program".into()))?;

    let mut state = ParseState::default();
    parse_program(program_pair, &mut state)
}

fn normalize_intent_attributes(source: &str) -> String {
    let mut out = String::new();
    let mut pending_intent: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#[intent(") && trimmed.ends_with(")]") {
            let args_raw = &trimmed["#[intent(".len()..trimmed.len() - 2];
            let args = args_raw.replace('=', ":");
            pending_intent = Some(format!("@intent({args})"));
            continue;
        }

        if let Some(intent) = pending_intent.as_ref() {
            if is_executable_definition_line(trimmed) {
                if let Some(brace_idx) = line.find('{') {
                    out.push_str(&line[..brace_idx]);
                    out.push(' ');
                    out.push_str(intent);
                    out.push(' ');
                    out.push_str(&line[brace_idx..]);
                    out.push('\n');
                    pending_intent = None;
                    continue;
                }

                out.push_str(line);
                out.push(' ');
                out.push_str(intent);
                out.push('\n');
                pending_intent = None;
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    if let Some(intent) = pending_intent {
        out.push_str(&intent);
        out.push('\n');
    }

    out
}

fn is_executable_definition_line(trimmed: &str) -> bool {
    trimmed.starts_with("query ")
        || trimmed.starts_with("mutation ")
        || trimmed.starts_with("action ")
        || trimmed.starts_with("iterator ")
        || trimmed.starts_with("node ")
        || trimmed.starts_with("fragment ")
}

// ── Program ───────────────────────────────────────────────────

fn parse_program(pair: Pair<Rule>, state: &mut ParseState) -> Result<Program, GraphemeError> {
    let mut imports = vec![];
    let mut definitions = vec![];

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::import_decl => imports.push(parse_import(inner)?),
            Rule::definition  => {
                // Unwrap the definition wrapper to get the actual variant
                let def = inner.into_inner().next().unwrap();
                match def.as_rule() {
                    Rule::query_def        => definitions.push(Definition::Query(parse_query(def, state)?)),
                    Rule::mutation_def     => definitions.push(Definition::Mutation(parse_mutation(def, state)?)),
                    Rule::iterator_def     => definitions.push(Definition::Iterator(parse_iterator(def, state)?)),
                    Rule::node_def         => definitions.push(Definition::Iterator(parse_iterator(def, state)?)),
                    Rule::fragment_def     => definitions.push(Definition::Fragment(parse_fragment(def, state)?)),
                    Rule::subscription_def => definitions.push(Definition::Subscription(parse_subscription(def, state)?)),
                    Rule::struct_def       => definitions.push(Definition::Struct(parse_struct_def(def)?)),
                    Rule::enum_def         => definitions.push(Definition::Enum(parse_enum_def(def)?)),
                    Rule::state_machine_def => definitions.push(Definition::StateMachine(parse_state_machine_def(def)?)),
                    Rule::schema_def       => definitions.push(Definition::Schema(parse_schema(def)?)),
                    Rule::module_proposal  => definitions.push(Definition::ModuleProposal(parse_module_proposal(def)?)),
                    r => return Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
                }
            }
            Rule::EOI => {}
            r => return Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
        }
    }

    for iterator in state.synthetic_iterators.drain(..) {
        definitions.push(Definition::Iterator(iterator));
    }

    Ok(Program { imports, definitions })
}

// ── Imports ───────────────────────────────────────────────────

fn parse_import(pair: Pair<Rule>) -> Result<ImportDecl, GraphemeError> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();
    let (kind, alias_pair) = if first.as_rule() == Rule::import_kind {
        (ImportKind::Types, inner.next().unwrap())
    } else {
        (ImportKind::Module, first)
    };
    let alias = alias_pair.as_str().to_string();
    let path  = parse_string_lit(inner.next().unwrap());
    Ok(ImportDecl { kind, alias, path })
}

// ── Schema ────────────────────────────────────────────────────

fn parse_schema(pair: Pair<Rule>) -> Result<SchemaDef, GraphemeError> {
    let types = pair
        .into_inner()
        .map(parse_type_def)
        .collect::<Result<_, _>>()?;
    Ok(SchemaDef { types })
}

fn parse_struct_def(pair: Pair<Rule>) -> Result<StructDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut fields = Vec::new();

    for p in inner {
        if p.as_rule() != Rule::struct_field_def {
            continue;
        }

        let field_source = p.as_str();
        let optional = field_source.contains("?:");
        let mut fi = p.into_inner();
        let field_name = fi.next().unwrap().as_str().to_string();
        let type_ref = parse_type_ref(fi.next().unwrap())?;
        fields.push(StructFieldDef {
            name: field_name,
            type_ref,
            optional,
        });
    }

    Ok(StructDef { name, fields })
}

fn parse_enum_def(pair: Pair<Rule>) -> Result<EnumDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("enum missing name".to_string()))?
        .as_str()
        .to_string();
    let members = inner.map(|p| p.as_str().to_string()).collect::<Vec<_>>();

    if members.is_empty() {
        return Err(GraphemeError::ParseError(format!(
            "enum '{}' must declare at least one member",
            name
        )));
    }

    Ok(EnumDef { name, members })
}

fn parse_state_machine_def(pair: Pair<Rule>) -> Result<StateMachineDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("state_machine missing name".to_string()))?
        .as_str()
        .to_string();
    let enum_name = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("state_machine missing enum target".to_string()))?
        .as_str()
        .to_string();

    let mut terminals = Vec::new();
    let mut transitions = Vec::new();

    for entry in inner {
        match entry.as_rule() {
            Rule::state_terminal_def => {
                let terminal = entry
                    .into_inner()
                    .next()
                    .ok_or_else(|| {
                        GraphemeError::ParseError(
                            "state_machine terminal missing member".to_string(),
                        )
                    })?
                    .as_str()
                    .to_string();
                terminals.push(terminal);
            }
            Rule::state_transition_def => {
                let mut ti = entry.into_inner();
                let from = ti
                    .next()
                    .ok_or_else(|| {
                        GraphemeError::ParseError(
                            "state_machine transition missing from member".to_string(),
                        )
                    })?
                    .as_str()
                    .to_string();
                let to = ti
                    .next()
                    .ok_or_else(|| {
                        GraphemeError::ParseError(
                            "state_machine transition missing to member".to_string(),
                        )
                    })?
                    .as_str()
                    .to_string();
                transitions.push(StateTransitionDef { from, to });
            }
            _ => {}
        }
    }

    if transitions.is_empty() {
        return Err(GraphemeError::ParseError(format!(
            "state_machine '{}' must define at least one transition",
            name
        )));
    }

    Ok(StateMachineDef {
        name,
        enum_name,
        terminals,
        transitions,
    })
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
        Rule::qualified_ident => Ok(TypeRef::Named(inner_pair.as_str().to_string(), non_null)),
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

fn parse_query(pair: Pair<Rule>, state: &mut ParseState) -> Result<QueryDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let (variables, signature, directives, pipelines) = parse_operation_body(inner, state)?;
    Ok(QueryDef { name, variables, signature, directives, pipelines })
}

fn parse_mutation(pair: Pair<Rule>, state: &mut ParseState) -> Result<MutationDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let (variables, signature, directives, pipelines) = parse_operation_body(inner, state)?;
    Ok(MutationDef { name, variables, signature, directives, pipelines })
}

fn parse_iterator(pair: Pair<Rule>, state: &mut ParseState) -> Result<IteratorDef, GraphemeError> {
    let mut inner   = pair.into_inner();
    let name        = inner.next().unwrap().as_str().to_string();
    let (_, signature, directives, pipelines) = parse_operation_body(inner, state)?;
    let signature = signature.ok_or_else(|| {
        GraphemeError::ParseError(format!("iterator '{}' is missing required signature", name))
    })?;
    Ok(IteratorDef { name, signature, directives, pipelines })
}

fn parse_fragment(pair: Pair<Rule>, state: &mut ParseState) -> Result<FragmentDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let (_, signature, directives, pipelines) = parse_operation_body(inner, state)?;
    let signature = signature.ok_or_else(|| {
        GraphemeError::ParseError(format!("fragment '{}' is missing required signature", name))
    })?;

    if !directives.is_empty() {
        return Err(GraphemeError::ParseError(format!(
            "fragment '{}' does not support directives in Phase A",
            name
        )));
    }

    Ok(FragmentDef {
        name,
        signature,
        pipelines,
    })
}

fn parse_subscription(pair: Pair<Rule>, state: &mut ParseState) -> Result<SubscriptionDef, GraphemeError> {
    let mut inner = pair.into_inner();
    let name      = inner.next().unwrap().as_str().to_string();
    let (variables, signature, directives, pipelines) = parse_operation_body(inner, state)?;
    Ok(SubscriptionDef { name, variables, signature, directives, pipelines })
}

/// Shared body parser for query/mutation/subscription
fn parse_operation_body<'a>(
    inner: impl Iterator<Item = Pair<'a, Rule>>,
    state: &mut ParseState,
) -> Result<(Vec<VariableDef>, Option<ExecutableSignature>, Vec<Directive>, Vec<Pipeline>), GraphemeError> {
    let mut variables  = vec![];
    let mut signature = None;
    let mut directives = vec![];
    let mut pipelines  = vec![];

    for p in inner {
        match p.as_rule() {
            Rule::variable_defs => {
                for vd in p.into_inner() {
                    variables.push(parse_variable_def(vd)?);
                }
            }
            Rule::executable_signature => {
                signature = Some(parse_executable_signature(p)?);
            }
            Rule::directive  => directives.push(parse_directive(p)?),
            Rule::pipeline   => pipelines.push(parse_pipeline(p, state)?),
            _ => {}
        }
    }

    Ok((variables, signature, directives, pipelines))
}

fn parse_executable_signature(pair: Pair<Rule>) -> Result<ExecutableSignature, GraphemeError> {
    let mut inner = pair.into_inner();
    let input = parse_type_ref(inner.next().unwrap())?;
    let output = inner.next().map(parse_type_ref).transpose()?;
    Ok(ExecutableSignature { input, output })
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

fn parse_pipeline(pair: Pair<Rule>, state: &mut ParseState) -> Result<Pipeline, GraphemeError> {
    let mut steps = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::match_step => steps.push(PipelineStep::Field(parse_match_step_as_match_call(p, state)?)),
            Rule::if_step => steps.push(PipelineStep::Field(parse_if_step_as_branch_call(p, state)?)),
            Rule::transition_step => steps.push(PipelineStep::Field(parse_transition_step_as_set_fields_call(p)?)),
            Rule::apply_step => steps.push(PipelineStep::Field(parse_apply_step_as_apply_lane_call(p)?)),
            Rule::set_step => steps.push(PipelineStep::Field(parse_set_step_as_set_fields_call(p)?)),
            Rule::struct_init_step => steps.push(PipelineStep::StructInit(parse_struct_init_step(p)?)),
            Rule::field_call => steps.push(PipelineStep::Field(parse_field_call(p)?)),
            Rule::call_step  => steps.push(PipelineStep::Call(parse_call_step(p)?)),
            Rule::pipe_step  => {
                // pipe_step = { "|>" ~ (match_step | if_step | transition_step | apply_step | set_step | struct_init_step | call_step | field_call) }
                let inner = p.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::match_step => steps.push(PipelineStep::Field(parse_match_step_as_match_call(inner, state)?)),
                    Rule::if_step => steps.push(PipelineStep::Field(parse_if_step_as_branch_call(inner, state)?)),
                    Rule::transition_step => steps.push(PipelineStep::Field(parse_transition_step_as_set_fields_call(inner)?)),
                    Rule::apply_step => steps.push(PipelineStep::Field(parse_apply_step_as_apply_lane_call(inner)?)),
                    Rule::set_step => steps.push(PipelineStep::Field(parse_set_step_as_set_fields_call(inner)?)),
                    Rule::struct_init_step => steps.push(PipelineStep::StructInit(parse_struct_init_step(inner)?)),
                    Rule::field_call => steps.push(PipelineStep::Field(parse_field_call(inner)?)),
                    Rule::call_step => steps.push(PipelineStep::Call(parse_call_step(inner)?)),
                    r => return Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
                }
            }
            _ => {}
        }
    }

    Ok(Pipeline { steps })
}

fn parse_match_step_as_match_call(pair: Pair<Rule>, state: &mut ParseState) -> Result<FieldCall, GraphemeError> {
    let (field_name, cases, default_target) = parse_match_spec(pair, state)?;

    let case_values = cases
        .into_iter()
        .map(|(eq, target)| {
            Value::Object(vec![
                ("eq".to_string(), eq),
                ("then".to_string(), target),
            ])
        })
        .collect::<Vec<_>>();

    Ok(FieldCall {
        module: Some("flow".to_string()),
        name: "match".to_string(),
        args: vec![
            ("field".to_string(), Value::String(field_name)),
            ("cases".to_string(), Value::List(case_values)),
            ("default".to_string(), default_target),
        ],
        directives: vec![],
        selection: None,
    })
}

fn parse_match_spec(pair: Pair<Rule>, state: &mut ParseState) -> Result<(String, Vec<(Value, Value)>, Value), GraphemeError> {
    let mut inner = pair.into_inner();
    let match_var = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("match-step missing variable".to_string()))?
        .as_str()
        .to_string();
    let field_name = parse_current_field_name(&match_var, "match-step")?;

    let mut cases = Vec::new();
    let mut default_target = None;

    for entry in inner {
        match entry.as_rule() {
            Rule::match_case => {
                let case_items = entry.into_inner().collect::<Vec<_>>();
                if case_items.len() < 2 {
                    return Err(GraphemeError::ParseError(
                        "match case requires at least one value and one target".to_string(),
                    ));
                }

                let target = parse_match_target_value(
                    case_items
                        .last()
                        .cloned()
                        .ok_or_else(|| GraphemeError::ParseError("match case missing target".to_string()))?,
                    state,
                )?;

                for value_pair in case_items.into_iter().take_while(|p| p.as_rule() == Rule::value) {
                    let eq_value = parse_value(value_pair)?;
                    cases.push((eq_value, target.clone()));
                }
            }
            Rule::match_default => {
                let target_pair = entry
                    .into_inner()
                    .next()
                    .ok_or_else(|| GraphemeError::ParseError("match default missing target".to_string()))?;
                default_target = Some(parse_match_target_value(target_pair, state)?);
            }
            _ => {}
        }
    }

    if cases.is_empty() {
        return Err(GraphemeError::ParseError(
            "match-step requires at least one case".to_string(),
        ));
    }

    let default_target = default_target.ok_or_else(|| {
        GraphemeError::ParseError("match-step requires a default target".to_string())
    })?;

    Ok((field_name, cases, default_target))
}

fn parse_set_step_as_set_fields_call(pair: Pair<Rule>) -> Result<FieldCall, GraphemeError> {
    let object = pair
        .into_inner()
        .next()
        .ok_or_else(|| GraphemeError::ParseError("set-step missing object body".to_string()))?;

    let mut fields = Vec::new();
    for field in object.into_inner() {
        let mut fi = field.into_inner();
        let key = fi.next().unwrap().as_str().to_string();
        let value = parse_value(fi.next().unwrap())?;
        fields.push((key, value));
    }

    Ok(FieldCall {
        module: Some("core".to_string()),
        name: "set_fields".to_string(),
        args: vec![("fields".to_string(), Value::Object(fields))],
        directives: vec![],
        selection: None,
    })
}

fn parse_apply_step_as_apply_lane_call(pair: Pair<Rule>) -> Result<FieldCall, GraphemeError> {
    let mut inner = pair.into_inner();
    let lane = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("apply-step missing lane name".to_string()))?
        .as_str()
        .to_string();

    if lane != "state" && lane != "data" {
        return Err(GraphemeError::ParseError(format!(
            "apply-step lane must be 'state' or 'data', got '{}'",
            lane
        )));
    }

    let object = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("apply-step missing object body".to_string()))?;

    let mut fields = Vec::new();
    for field in object.into_inner() {
        let mut fi = field.into_inner();
        let key = fi.next().unwrap().as_str().to_string();
        let value = parse_value(fi.next().unwrap())?;
        fields.push((key, value));
    }

    Ok(FieldCall {
        module: Some("core".to_string()),
        name: "apply_lane".to_string(),
        args: vec![
            ("lane".to_string(), Value::String(lane)),
            ("fields".to_string(), Value::Object(fields)),
        ],
        directives: vec![],
        selection: None,
    })
}

fn parse_transition_step_as_set_fields_call(pair: Pair<Rule>) -> Result<FieldCall, GraphemeError> {
    let mut inner = pair.into_inner();
    let left_var = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("transition-step missing left variable".to_string()))?
        .as_str()
        .to_string();
    let to_value = parse_value(
        inner
            .next()
            .ok_or_else(|| GraphemeError::ParseError("transition-step missing target value".to_string()))?,
    )?;
    let to_value = match to_value {
        Value::Symbol(member) => Value::String(member),
        other => other,
    };

    let field_name = parse_current_field_name(&left_var, "transition-step")?;
    let mut fields = vec![(field_name, to_value)];

    if let Some(extra_obj) = inner.next() {
        for field in extra_obj.into_inner() {
            let mut fi = field.into_inner();
            let key = fi.next().unwrap().as_str().to_string();
            let value = parse_value(fi.next().unwrap())?;
            fields.push((key, value));
        }
    }

    Ok(FieldCall {
        module: Some("core".to_string()),
        name: "set_fields".to_string(),
        args: vec![("fields".to_string(), Value::Object(fields))],
        directives: vec![],
        selection: None,
    })
}

fn parse_match_target_value(pair: Pair<Rule>, state: &mut ParseState) -> Result<Value, GraphemeError> {
    let inner = if pair.as_rule() == Rule::match_target {
        pair.into_inner().next().ok_or_else(|| {
            GraphemeError::ParseError("match target is empty".to_string())
        })?
    } else {
        pair
    };

    match inner.as_rule() {
        Rule::symbol_lit => Ok(Value::Symbol(inner.as_str().to_string())),
        Rule::branch_target => parse_branch_target_value(inner, state),
        Rule::match_step => {
            let (field_name, cases, default_target) = parse_match_spec(inner, state)?;
            let case_values = cases
                .into_iter()
                .map(|(eq, target)| {
                    Value::Object(vec![
                        ("eq".to_string(), eq),
                        ("then".to_string(), target),
                    ])
                })
                .collect::<Vec<_>>();

            Ok(Value::Object(vec![
                (
                    "$match".to_string(),
                    Value::Object(vec![
                        ("field".to_string(), Value::String(field_name)),
                        ("cases".to_string(), Value::List(case_values)),
                        ("default".to_string(), default_target),
                    ]),
                ),
            ]))
        }
        r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
    }
}

fn parse_branch_target_value(pair: Pair<Rule>, state: &mut ParseState) -> Result<Value, GraphemeError> {
    let inner = if pair.as_rule() == Rule::branch_target {
        pair.into_inner().next().ok_or_else(|| {
            GraphemeError::ParseError("branch target is empty".to_string())
        })?
    } else {
        pair
    };

    match inner.as_rule() {
        Rule::symbol_lit => Ok(Value::Symbol(inner.as_str().to_string())),
        Rule::inline_target_pipeline => {
            if let Some(symbol) = extract_symbol_target_from_inline_pipeline(inner.clone())? {
                return Ok(Value::Symbol(symbol));
            }
            let steps = parse_inline_target_pipeline_steps(inner, state)?;
            let target = state.push_inline_target_iterator(steps);
            Ok(Value::Symbol(target))
        }
        Rule::inline_target_step
        | Rule::transition_step
        | Rule::apply_step
        | Rule::set_step
        | Rule::struct_init_step
        | Rule::field_call
        | Rule::call_step
        | Rule::if_step
        | Rule::match_step => {
            if let Some(symbol) = extract_symbol_target_from_inline_step(inner.clone())? {
                return Ok(Value::Symbol(symbol));
            }
            let step = parse_inline_target_step(inner, state)?;
            let target = state.push_inline_target_iterator(vec![step]);
            Ok(Value::Symbol(target))
        }
        r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
    }
}

fn extract_symbol_target_from_inline_step(pair: Pair<Rule>) -> Result<Option<String>, GraphemeError> {
    let step_pair = if pair.as_rule() == Rule::inline_target_step {
        pair.into_inner().next().ok_or_else(|| {
            GraphemeError::ParseError("inline target step is empty".to_string())
        })?
    } else {
        pair
    };

    match step_pair.as_rule() {
        Rule::field_call => {
            let call = parse_field_call(step_pair)?;
            if call.module.is_none()
                && call.args.is_empty()
                && call.directives.is_empty()
                && call.selection.is_none()
            {
                Ok(Some(call.name))
            } else {
                Ok(None)
            }
        }
        Rule::call_step => {
            let call = parse_call_step(step_pair)?;
            if call.args.is_empty() && call.directives.is_empty() && call.selection.is_none() {
                Ok(Some(call.target))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn extract_symbol_target_from_inline_pipeline(
    pair: Pair<Rule>,
) -> Result<Option<String>, GraphemeError> {
    let mut entries = pair.into_inner();
    let first = match entries.next() {
        Some(p) => p,
        None => return Ok(None),
    };

    if entries.next().is_some() {
        return Ok(None);
    }

    extract_symbol_target_from_inline_step(first)
}

fn parse_inline_target_pipeline_steps(
    pair: Pair<Rule>,
    state: &mut ParseState,
) -> Result<Vec<PipelineStep>, GraphemeError> {
    let mut steps = Vec::new();
    for entry in pair.into_inner() {
        match entry.as_rule() {
            Rule::inline_target_step => {
                let step = entry.into_inner().next().ok_or_else(|| {
                    GraphemeError::ParseError("inline target step is empty".to_string())
                })?;
                steps.push(parse_inline_target_step(step, state)?);
            }
            Rule::inline_target_pipe => {
                let step = entry.into_inner().next().ok_or_else(|| {
                    GraphemeError::ParseError("inline target pipe step is empty".to_string())
                })?;
                steps.push(parse_inline_target_step(step, state)?);
            }
            _ => {}
        }
    }
    Ok(steps)
}

fn parse_inline_target_step(
    pair: Pair<Rule>,
    state: &mut ParseState,
) -> Result<PipelineStep, GraphemeError> {
    let step_pair = if pair.as_rule() == Rule::inline_target_step {
        pair.into_inner().next().ok_or_else(|| {
            GraphemeError::ParseError("inline target step is empty".to_string())
        })?
    } else {
        pair
    };

    match step_pair.as_rule() {
        Rule::transition_step => Ok(PipelineStep::Field(parse_transition_step_as_set_fields_call(step_pair)?)),
        Rule::apply_step => Ok(PipelineStep::Field(parse_apply_step_as_apply_lane_call(step_pair)?)),
        Rule::set_step => Ok(PipelineStep::Field(parse_set_step_as_set_fields_call(step_pair)?)),
        Rule::struct_init_step => Ok(PipelineStep::StructInit(parse_struct_init_step(step_pair)?)),
        Rule::field_call => Ok(PipelineStep::Field(parse_field_call(step_pair)?)),
        Rule::call_step => Ok(PipelineStep::Call(parse_call_step(step_pair)?)),
        Rule::if_step => Ok(PipelineStep::Field(parse_if_step_as_branch_call(step_pair, state)?)),
        Rule::match_step => Ok(PipelineStep::Field(parse_match_step_as_match_call(step_pair, state)?)),
        r => Err(GraphemeError::UnexpectedRule(format!("{r:?}"))),
    }
}

fn parse_if_step_as_branch_call(pair: Pair<Rule>, state: &mut ParseState) -> Result<FieldCall, GraphemeError> {
    let mut inner = pair.into_inner();
    let left_var = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("if-step missing left variable".to_string()))?
        .as_str()
        .to_string();
    let cmp = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("if-step missing comparator".to_string()))?
        .as_str()
        .to_string();
    let right_value = parse_value(
        inner
            .next()
            .ok_or_else(|| GraphemeError::ParseError("if-step missing right value".to_string()))?,
    )?;
    let then_target = parse_branch_target_value(
        inner
            .next()
            .ok_or_else(|| GraphemeError::ParseError("if-step missing then target".to_string()))?,
        state,
    )?;
    let else_target = parse_branch_target_value(
        inner
            .next()
            .ok_or_else(|| GraphemeError::ParseError("if-step missing else target".to_string()))?,
        state,
    )?;

    let field_name = parse_current_field_name(&left_var, "if-step")?;

    let cmp_key = match cmp.as_str() {
        "==" => "eq",
        ">" => "gt",
        ">=" => "gte",
        "<" => "lt",
        "<=" => "lte",
        _ => {
            return Err(GraphemeError::ParseError(format!(
                "unsupported if-step comparator: {}",
                cmp
            )))
        }
    };

    let when = Value::Object(vec![
        ("field".to_string(), Value::String(field_name)),
        (cmp_key.to_string(), right_value),
    ]);

    Ok(FieldCall {
        module: Some("flow".to_string()),
        name: "branch".to_string(),
        args: vec![
            ("when".to_string(), when),
            ("then".to_string(), then_target),
            ("else".to_string(), else_target),
        ],
        directives: vec![],
        selection: None,
    })
}

fn parse_current_field_name(var_expr: &str, context: &str) -> Result<String, GraphemeError> {
    let var_name = var_expr.trim_start_matches('$');
    var_name
        .strip_prefix("current.")
        .map(|s| s.to_string())
        .ok_or_else(|| {
            GraphemeError::ParseError(format!(
                "{} expression must reference $current.<field>",
                context
            ))
        })
}

fn parse_struct_init_step(pair: Pair<Rule>) -> Result<StructInitStep, GraphemeError> {
    let mut inner = pair.into_inner();
    let type_name = inner.next().unwrap().as_str().to_string();
    let object = inner
        .next()
        .ok_or_else(|| GraphemeError::ParseError("struct initializer missing object body".to_string()))?;

    let mut fields = Vec::new();
    for field in object.into_inner() {
        let mut fi = field.into_inner();
        let key = fi.next().unwrap().as_str().to_string();
        let value = parse_value(fi.next().unwrap())?;
        fields.push((key, value));
    }

    Ok(StructInitStep { type_name, fields })
}

fn parse_call_step(pair: Pair<Rule>) -> Result<CallStep, GraphemeError> {
    let mut target = String::new();
    let mut args = vec![];
    let mut directives = vec![];
    let mut selection = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => {
                target = p.as_str().to_string();
            }
            Rule::arg_list => {
                for arg in p.into_inner() {
                    if arg.as_rule() == Rule::named_arg {
                        args.push(parse_named_arg(arg)?);
                    }
                }
            }
            Rule::directive => directives.push(parse_directive(p)?),
            Rule::selection_set => selection = Some(parse_selection_set(p)?),
            _ => {}
        }
    }

    Ok(CallStep {
        target,
        args,
        directives,
        selection,
    })
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
            // variable includes full token, e.g. "$name" or "$current.value".
            let name = inner.as_str().trim_start_matches('$').to_string();
            Ok(Value::Variable(name))
        }
        Rule::symbol_lit => Ok(Value::Symbol(inner.as_str().to_string())),
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
    let raw_name  = inner.next().unwrap().as_str().to_string();
    let name = match raw_name.as_str() {
        "r" => "retry".to_string(),
        "t" => "timeout".to_string(),
        _ => raw_name,
    };
    let mut args = Vec::new();

    for p in inner {
        match p.as_rule() {
            Rule::arg_list => {
                for arg in p.into_inner() {
                    if arg.as_rule() == Rule::named_arg {
                        args.push(parse_named_arg(arg)?);
                    }
                }
            }
            Rule::object_value => {
                for field in p.into_inner() {
                    let mut fi = field.into_inner();
                    let key = fi.next().unwrap().as_str().to_string();
                    let value = parse_value(fi.next().unwrap())?;
                    args.push((key, value));
                }
            }
            _ => {}
        }
    }

    Ok(Directive { name, args })
}

// ── Helpers ───────────────────────────────────────────────────

fn parse_string_lit(pair: Pair<Rule>) -> String {
    let raw = pair.as_str();
    // Strip surrounding quotes
    raw[1..raw.len()-1].to_string()
}
