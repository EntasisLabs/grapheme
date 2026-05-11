pub mod ast;
pub mod compiler_api;
pub mod error;
pub mod hir;
pub mod mir_lower;
pub mod parser;
pub mod pipeline;
pub mod verifier;

use grapheme_artifact::{build_artifact_from_mir, ArtifactEnvelope};

pub use compiler_api::{CompiledScript, Compiler, CompilerOptions};
pub use error::CompilerError;
pub use parser::parse;
pub use pipeline::{compile_program, CompilationArtifact, CompileOptions};

pub fn compile(source: &str) -> Result<CompilationArtifact, CompilerError> {
	let ast = parse(source)?;
	compile_program(ast, CompileOptions::default())
}

pub fn compile_to_artifact(source: &str, entrypoint: Option<&str>) -> Result<ArtifactEnvelope, CompilerError> {
	let compilation = compile(source)?;
	build_artifact_from_mir(&compilation.mir, entrypoint)
		.map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))
}

#[cfg(test)]
mod tests {
		use super::*;
		use grapheme_artifact::mir::MirCompareOp;

		#[test]
		fn rejects_invalid_loop_merge_value() {
				let source = r#"
query InvalidLoopMerge {
	call Ticker
}

iterator Ticker on Any @loop(max: 3, merge: "invalid") {
	core.echo(message: "tick")
}
"#;

				let err = compile(source).expect_err("compile should fail for invalid merge mode");
				let msg = err.to_string();
				assert!(msg.contains("@loop merge must be one of replace|append|reduce|none"));
		}

		#[test]
		fn supports_bare_iterator_invocation_step() {
				let source = r#"
query UseIteratorSugar {
	PollJob
}

iterator PollJob on Any {
	core.echo(message: "tick")
}
"#;

				let compilation = compile(source).expect("compile should accept bare iterator invocation");
				let query = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "UseIteratorSugar")
						.expect("query function present");

				let first_inst = query
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("query has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, .. } => {
								assert_eq!(module.as_deref(), Some("call"));
								assert_eq!(op, "PollJob");
						}
						_ => panic!("expected call instruction"),
				}
		}

		#[test]
		fn supports_fragment_definition_in_phase_a_without_emitting_mir_function() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any {
	core.echo(message: "tick")
}

fragment SharedPrep on Any {
	core.echo(message: "prep")
}
"#;

				let compilation = compile(source).expect("compile should accept fragment syntax in phase A");
				assert!(compilation.mir.functions.iter().any(|f| f.name == "Run"));
				assert!(compilation.mir.functions.iter().any(|f| f.name == "Worker"));
				assert!(
						!compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "SharedPrep")
				);
		}

		#[test]
		fn rejects_fragment_directives_in_phase_a() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any {
	core.echo(message: "tick")
}

fragment SharedPrep on Any @loop(max: 2) {
	core.echo(message: "prep")
}
"#;

				let err = compile(source).expect_err("compile should reject fragment directives in phase A");
				let msg = err.to_string();
				assert!(msg.contains("does not support directives in Phase A"));
		}

		#[test]
		fn expands_fragment_invocation_into_caller_pipeline_in_phase_b() {
				let source = r#"
query Run {
	SharedPrep
	|> Worker
}

iterator Worker on Any {
	core.echo(message: "tick")
}

fragment SharedPrep on Any {
	core.set_fields(fields: { status: "queued" })
}
"#;

				let compilation = compile(source).expect("compile should inline fragment steps");
				let run_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Run")
						.expect("Run function present");

				let first_inst = run_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Run has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, .. } => {
								assert_eq!(module.as_deref(), Some("core"));
								assert_eq!(op, "set_fields");
						}
						_ => panic!("expected inlined fragment call instruction"),
				}

				assert!(
						!compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "SharedPrep")
				);
		}

		#[test]
		fn rejects_fragment_expansion_cycle_in_phase_b() {
				let source = r#"
query Run {
	A
}

fragment A on Any {
	B
}

fragment B on Any {
	A
}
"#;

				let err = compile(source).expect_err("compile should reject fragment expansion cycle");
				let msg = err.to_string();
				assert!(msg.contains("fragment expansion cycle detected"));
		}

		#[test]
		fn recursive_directive_injects_self_call_max_depth() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any @recursive(max_depth: 3) {
	Step
}
"#;

				let compilation = compile(source).expect("compile should accept recursive directive");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, args, .. } => {
								assert_eq!(module.as_deref(), Some("call"));
								assert_eq!(op, "Step");
								assert_eq!(args.get("max_depth"), Some(&serde_json::json!(3)));
						}
						_ => panic!("expected call instruction"),
				}
		}

		#[test]
		fn recursive_directive_allows_omitted_max_depth() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any @recursive {
	core.echo(message: "tick")
}
"#;

				let compilation = compile(source).expect("compile should allow recursive directive without max_depth");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				assert!(!step_fn.blocks.is_empty());
		}

		#[test]
		fn flow_branch_lowers_to_branch_call_instruction() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	flow.branch(
		when: { field: "status", eq: "done" },
		then: "$return",
		else: "Step"
	)
}
"#;

				let compilation = compile(source).expect("compile should lower flow.branch");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::BranchCall {
								field,
						cmp,
						value,
								then_target,
								else_target,
								..
						} => {
								assert_eq!(field, "status");
						assert_eq!(cmp, &MirCompareOp::Eq);
						assert_eq!(value, &serde_json::json!("done"));
								assert_eq!(then_target, "$return");
								assert_eq!(else_target.as_deref(), Some("Step"));
						}
						_ => panic!("expected branch_call instruction"),
				}
		}

		#[test]
		fn flow_branch_symbol_targets_lower_to_branch_call_instruction() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	flow.branch(
		when: { field: "status", eq: "done" },
		then: return,
		else: Step
	)
}
"#;

				let compilation = compile(source).expect("compile should lower symbol flow.branch targets");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::BranchCall {
								then_target,
								else_target,
								..
						} => {
								assert_eq!(then_target, "$return");
								assert_eq!(else_target.as_deref(), Some("Step"));
						}
						_ => panic!("expected branch_call instruction"),
				}
		}

		#[test]
		fn flow_branch_gte_lowers_to_branch_call_instruction() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	flow.branch(
		when: { field: "score", gte: 90.0 },
		then: return,
		else: Step
	)
}
"#;

				let compilation = compile(source).expect("compile should lower flow.branch gte");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::BranchCall {
								field,
								cmp,
								value,
								then_target,
								else_target,
								..
						} => {
								assert_eq!(field, "score");
								assert_eq!(cmp, &MirCompareOp::Gte);
								assert_eq!(value, &serde_json::json!(90.0));
								assert_eq!(then_target, "$return");
								assert_eq!(else_target.as_deref(), Some("Step"));
						}
						_ => panic!("expected branch_call instruction"),
				}
		}

		#[test]
		fn if_else_sugar_lowers_to_branch_call_instruction() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	if $current.score >= 90.0 then return else Step
}
"#;

				let compilation = compile(source).expect("compile should lower if/else sugar");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::BranchCall {
								field,
								cmp,
								value,
								then_target,
								else_target,
								..
						} => {
								assert_eq!(field, "score");
								assert_eq!(cmp, &MirCompareOp::Gte);
								assert_eq!(value, &serde_json::json!(90.0));
								assert_eq!(then_target, "$return");
								assert_eq!(else_target.as_deref(), Some("Step"));
						}
						_ => panic!("expected branch_call instruction"),
				}
		}

		#[test]
		fn match_sugar_lowers_to_match_call_instruction() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	match $current.status {
		case done => return
		default => Step
	}
}
"#;

				let compilation = compile(source).expect("compile should lower match sugar");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::MatchCall {
								field,
								cases,
								default_target,
								..
						} => {
								assert_eq!(field, "status");
								assert_eq!(cases.len(), 1);
								assert_eq!(cases[0].eq, serde_json::json!("done"));
								match &cases[0].then_target {
										grapheme_artifact::MirMatchTarget::Target(target) => {
												assert_eq!(target, "$return");
										}
										_ => panic!("expected plain target for first match case"),
								}
								match default_target {
										grapheme_artifact::MirMatchTarget::Target(target) => {
												assert_eq!(target, "Step");
										}
										_ => panic!("expected plain target for match default"),
								}
						}
						_ => panic!("expected match_call instruction"),
				}
		}

		#[test]
		fn nested_match_sugar_lowers_to_nested_targets() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	match $current.status {
		case queued => match $current.priority {
			case high => Escalate
			default => Continue
		}
		default => return
	}
}

iterator Escalate on Any {
	core.echo(message: "escalate")
}

iterator Continue on Any {
	core.echo(message: "continue")
}
"#;

				let compilation = compile(source).expect("compile should lower nested match sugar");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::MatchCall { cases, .. } => {
								assert_eq!(cases.len(), 1);
								match &cases[0].then_target {
										grapheme_artifact::MirMatchTarget::Nested { field, cases, .. } => {
												assert_eq!(field, "priority");
												assert_eq!(cases.len(), 1);
										}
										_ => panic!("expected nested match target"),
								}
						}
						_ => panic!("expected match_call instruction"),
				}
		}

		#[test]
		fn match_multi_case_sugar_lowers_all_cases() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	match $current.status {
		case queued, running => Loop
		default => return
	}
}

iterator Loop on Any {
	core.echo(message: "loop")
}
"#;

				let compilation = compile(source).expect("compile should lower multi-case match");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::MatchCall { cases, .. } => {
								assert_eq!(cases.len(), 2);
								assert_eq!(cases[0].eq, serde_json::json!("queued"));
								assert_eq!(cases[1].eq, serde_json::json!("running"));
						}
						_ => panic!("expected match_call instruction"),
				}
		}

		#[test]
		fn if_else_inline_targets_lower_to_synthetic_iterators() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	if $current.score >= 90.0 then transition $current.status -> passing else transition $current.status -> retry |> set { attempts: 1 }
}
"#;

				let compilation = compile(source).expect("compile should lower inline if targets");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::BranchCall {
								then_target,
								else_target,
								..
						} => {
								assert!(then_target.starts_with("__inline_target_"));
								let else_target = else_target.as_ref().expect("else target present");
								assert!(else_target.starts_with("__inline_target_"));

								let then_fn = compilation
										.mir
										.functions
										.iter()
										.find(|f| f.name == *then_target)
										.expect("synthetic then function present");
								let then_inst = then_fn
										.blocks
										.first()
										.and_then(|b| b.instructions.first())
										.expect("synthetic then function has instruction");
								match then_inst {
										grapheme_artifact::MirInst::Call { module, op, .. } => {
												assert_eq!(module.as_deref(), Some("core"));
												assert_eq!(op, "set_fields");
										}
										_ => panic!("expected call instruction in synthetic then function"),
								}

								let else_fn = compilation
										.mir
										.functions
										.iter()
										.find(|f| f.name == *else_target)
										.expect("synthetic else function present");
								let else_insts = else_fn
										.blocks
										.first()
										.map(|b| &b.instructions)
										.expect("synthetic else function has block");
								assert_eq!(else_insts.len(), 2);
						}
						_ => panic!("expected branch_call instruction"),
				}
		}

		#[test]
		fn match_inline_targets_lower_to_synthetic_iterators() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	match $current.status {
		case planned => transition $current.status -> validating
		default => transition $current.status -> failed |> set { reason: "fallback" }
	}
}
"#;

				let compilation = compile(source).expect("compile should lower inline match targets");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::MatchCall {
								cases,
								default_target,
								..
						} => {
								assert_eq!(cases.len(), 1);
								let case_target = match &cases[0].then_target {
										grapheme_artifact::MirMatchTarget::Target(target) => target,
										_ => panic!("expected plain target for case target"),
								};
								assert!(case_target.starts_with("__inline_target_"));

								let default_target = match default_target {
										grapheme_artifact::MirMatchTarget::Target(target) => target,
										_ => panic!("expected plain target for default target"),
								};
								assert!(default_target.starts_with("__inline_target_"));
								assert_ne!(case_target, default_target);

								let default_fn = compilation
										.mir
										.functions
										.iter()
										.find(|f| f.name == *default_target)
										.expect("synthetic default function present");
								let default_insts = default_fn
										.blocks
										.first()
										.map(|b| &b.instructions)
										.expect("synthetic default function has block");
								assert_eq!(default_insts.len(), 2);
						}
						_ => panic!("expected match_call instruction"),
				}
		}

		#[test]
		fn mixed_branch_targets_preserve_plain_symbols() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	if $current.score >= 90.0 then transition $current.status -> passing else return
}
"#;

				let compilation = compile(source).expect("compile should lower mixed branch targets");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");

				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::BranchCall {
								then_target,
								else_target,
								..
						} => {
								assert!(then_target.starts_with("__inline_target_"));
								assert_eq!(else_target.as_deref(), Some("$return"));
						}
						_ => panic!("expected branch_call instruction"),
				}
		}

		#[test]
		fn set_step_sugar_lowers_to_core_set_fields() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	set { status: "running", timeline: "work" }
}
"#;

				let compilation = compile(source).expect("compile should lower set-step sugar");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");
				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, .. } => {
								assert_eq!(module.as_deref(), Some("core"));
								assert_eq!(op, "set_fields");
						}
						_ => panic!("expected call instruction"),
				}
		}

		#[test]
		fn transition_step_sugar_lowers_to_core_set_fields() {
				let source = r#"
query Run {
	Step
}

iterator Step on Any {
	transition $current.status -> running { timeline: "work" }
}
"#;

				let compilation = compile(source).expect("compile should lower transition-step sugar");
				let step_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Step")
						.expect("Step function present");
				let first_inst = step_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Step has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, args, .. } => {
								assert_eq!(module.as_deref(), Some("core"));
								assert_eq!(op, "set_fields");
								assert_eq!(args.get("fields").and_then(|f| f.get("status")), Some(&serde_json::json!("running")));
								assert_eq!(args.get("fields").and_then(|f| f.get("timeline")), Some(&serde_json::json!("work")));
						}
						_ => panic!("expected call instruction"),
				}
		}

		#[test]
		fn supports_retry_timeout_short_alias_directives() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any @r(max: 3, backoff_ms: 100, on_fail: Fallback) @t(ms: 5000, on_timeout: Fallback) {
	core.echo(message: "tick")
}

iterator Fallback on Any {
	core.echo(message: "fallback")
}
"#;

				let compilation = compile(source).expect("compile should accept r/t directive aliases");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Worker")
				);
		}

		#[test]
		fn core_default_directive_assigns_core_module_for_bare_ops() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any @core_default {
	echo(message: "tick")
}
"#;

				let compilation = compile(source).expect("compile should support core_default mode");
				let worker = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Worker")
						.expect("Worker function present");

				let first_inst = worker
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Worker has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, .. } => {
								assert_eq!(module.as_deref(), Some("core"));
								assert_eq!(op, "echo");
						}
						_ => panic!("expected call instruction"),
				}
		}

		#[test]
		fn supports_struct_and_typed_executable_signatures() {
				let source = r#"
struct FibState {
	 a: Float
	 b: Float
	 i: Float
	 threshold?: Float
}

query Run on FibState -> FibState {
	Step
}

iterator Step on FibState -> FibState {
	core.echo(message: "tick")
}
"#;

				let compilation = compile(source).expect("compile should support typed signatures and struct defs");
				let query = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Run")
						.expect("query function present");

				assert!(!query.blocks.is_empty());
		}

		#[test]
		fn rejects_unknown_current_field_in_typed_scope() {
				let source = r#"
struct FibState {
	 a: Float
	 b: Float
}

query Run on FibState -> FibState {
	Step
}

iterator Step on FibState -> FibState {
	core.echo(message: $current.missing)
}
"#;

				let err = compile(source).expect_err("compile should fail for unknown typed field");
				let msg = err.to_string();
				assert!(msg.contains("unknown field '$current.missing'"));
		}

		#[test]
		fn rejects_unknown_named_type_in_signature() {
				let source = r#"
query Run on MissingType -> MissingType {
	core.echo(message: "hi")
}
"#;

				let err = compile(source).expect_err("compile should fail for unknown named type");
				let msg = err.to_string();
				assert!(msg.contains("references unknown type 'MissingType'"));
		}

		#[test]
		fn rejects_missing_required_output_fields_in_typed_scope() {
				let source = r#"
struct FibState {
	 a: Float
	 b: Float
}

query Run on FibState -> FibState {
	core.set_fields(fields: { a: 1.0 })
}
"#;

				let err = compile(source).expect_err("compile should fail when required output fields are missing");
				let msg = err.to_string();
				assert!(msg.contains("missing required fields"));
		}

		#[test]
		fn rejects_unknown_output_fields_in_typed_scope() {
				let source = r#"
struct FibState {
	 a: Float
	 b: Float
}

query Run on FibState -> FibState {
	core.set_fields(fields: { a: 1.0, b: 2.0, rogue: 9.0 })
}
"#;

				let err = compile(source).expect_err("compile should fail when setting undeclared output fields");
				let msg = err.to_string();
				assert!(msg.contains("is not declared on output type 'FibState'"));
		}

		#[test]
		fn supports_struct_initializer_pipeline_step() {
				let source = r#"
struct FibState {
	 a: Float
	 b: Float
	 i: Float
	 threshold: Float
}

query Run on FibState -> FibState {
	FibState { a: 0.0, b: 1.0, i: 0.0, threshold: 144.0 }
	|> Step
}

iterator Step on FibState -> FibState {
	core.echo(message: "tick")
}
"#;

				let compilation = compile(source).expect("compile should support struct initializer step");
				let run_fn = compilation
						.mir
						.functions
						.iter()
						.find(|f| f.name == "Run")
						.expect("Run function present");

				let first_inst = run_fn
						.blocks
						.first()
						.and_then(|b| b.instructions.first())
						.expect("Run has first instruction");

				match first_inst {
						grapheme_artifact::MirInst::Call { module, op, args, .. } => {
								assert_eq!(module.as_deref(), Some("core"));
								assert_eq!(op, "set_fields");
								assert_eq!(args.get("fields").and_then(|f| f.get("a")), Some(&serde_json::json!(0.0)));
						}
						_ => panic!("expected call instruction"),
				}
		}

		#[test]
		fn supports_namespaced_types_with_import_types() {
				let source = r#"
import types Domain from "./domain.gr"

query Run on Domain::FibState -> Domain::FibState {
	core.echo(message: "ok")
}
"#;

				let compilation = compile(source).expect("compile should accept namespaced imported types");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Run")
				);
		}

		#[test]
		fn rejects_namespaced_type_without_types_import() {
				let source = r#"
query Run on Domain::FibState -> Domain::FibState {
	core.echo(message: "ok")
}
"#;

				let err = compile(source).expect_err("compile should fail without type namespace import");
				let msg = err.to_string();
				assert!(msg.contains("unknown type namespace 'Domain'"));
		}

		#[test]
		fn supports_enum_type_and_member_in_typed_branch() {
				let source = r#"
enum JobStatus { queued, running, done, timeout }

struct JobState {
	 status: JobStatus
	 attempt: Float
}

query Run on JobState -> JobState {
	Step
}

iterator Step on JobState -> JobState {
	if $current.status == done then return else Step
}
"#;

				let compilation = compile(source).expect("compile should support enum branch members");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Run")
				);
		}

		#[test]
		fn rejects_unknown_enum_member_in_typed_branch() {
				let source = r#"
enum JobStatus { queued, running, done, timeout }

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	Step
}

iterator Step on JobState -> JobState {
	if $current.status == archived then return else Step
}
"#;

				let err = compile(source).expect_err("compile should reject unknown enum member");
				let msg = err.to_string();
				assert!(msg.contains("unknown enum member 'archived'"));
		}

		#[test]
		fn supports_state_machine_over_enum() {
				let source = r#"
enum JobStatus { queued, running, blocked, done, timeout }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> blocked
	transition blocked -> running
	transition running -> done
	transition running -> timeout
	terminal done
	terminal timeout
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	core.echo(message: "ok")
}
"#;

				let compilation = compile(source).expect("compile should support state_machine definitions");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Run")
				);
		}

		#[test]
		fn rejects_state_machine_unknown_member() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> blocked
	transition running -> done
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	core.echo(message: "ok")
}
"#;

				let err = compile(source).expect_err("compile should reject unknown state-machine members");
				let msg = err.to_string();
				assert!(msg.contains("transition to 'blocked' is not a member"));
		}

		#[test]
		fn rejects_state_machine_transition_from_terminal() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> done
	transition done -> queued
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	core.echo(message: "ok")
}
"#;

				let err = compile(source).expect_err("compile should reject outgoing transitions from terminal states");
				let msg = err.to_string();
				assert!(msg.contains("terminals cannot have outgoing transitions"));
		}

		#[test]
		fn rejects_invalid_state_machine_transition_in_pipeline_literals() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> done
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	core.set_fields(fields: { status: "queued" })
	|> core.set_fields(fields: { status: "done" })
}
"#;

				let err = compile(source).expect_err("compile should reject invalid literal transition");
				let msg = err.to_string();
				assert!(msg.contains("invalid transition"));
		}

		#[test]
		fn accepts_valid_state_machine_transition_in_pipeline_literals() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> done
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	core.set_fields(fields: { status: "queued" })
	|> core.set_fields(fields: { status: "running" })
	|> core.set_fields(fields: { status: "done" })
}
"#;

				let compilation = compile(source).expect("compile should accept valid literal transitions");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Run")
				);
		}

		#[test]
		fn rejects_invalid_branch_target_transition_for_state_machine() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> done
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	Route
}

iterator Route on JobState -> JobState {
	if $current.status == queued then JumpToDone else return
}

iterator JumpToDone on JobState -> JobState {
	core.set_fields(fields: { status: "done" })
}
"#;

				let err = compile(source).expect_err("compile should reject invalid branch transition");
				let msg = err.to_string();
				assert!(msg.contains("branch then target 'JumpToDone' makes invalid transition"));
		}

		#[test]
		fn accepts_valid_branch_target_transition_for_state_machine() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> done
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	Route
}

iterator Route on JobState -> JobState {
	if $current.status == queued then StartRunning else return
}

iterator StartRunning on JobState -> JobState {
	core.set_fields(fields: { status: "running" })
}
"#;

				let compilation = compile(source).expect("compile should accept valid branch transition");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Run")
				);
		}

		#[test]
		fn rejects_invalid_match_target_transition_for_state_machine() {
				let source = r#"
enum JobStatus { queued, running, done }

state_machine JobLifecycle on JobStatus {
	transition queued -> running
	transition running -> done
	terminal done
}

struct JobState {
	 status: JobStatus
}

query Run on JobState -> JobState {
	Route
}

iterator Route on JobState -> JobState {
	match $current.status {
		case queued => JumpToDone
		default => return
	}
}

iterator JumpToDone on JobState -> JobState {
	core.set_fields(fields: { status: "done" })
}
"#;

				let err = compile(source).expect_err("compile should reject invalid match transition");
				let msg = err.to_string();
				assert!(msg.contains("invalid transition"));
		}

		#[test]
		fn supports_retry_and_timeout_directives_on_iterator() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any @retry(max: 3, backoff_ms: 100, on_fail: Fallback) @timeout(ms: 5000, on_timeout: Fallback) {
	core.echo(message: "tick")
}

iterator Fallback on Any {
	core.echo(message: "fallback")
}
"#;

				let compilation = compile(source).expect("compile should accept retry/timeout directives");
				assert!(
						compilation
								.mir
								.functions
								.iter()
								.any(|f| f.name == "Worker")
				);
		}

		#[test]
		fn rejects_retry_directive_with_unknown_target() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any @retry(max: 2, on_fail: MissingStep) {
	core.echo(message: "tick")
}
"#;

				let err = compile(source).expect_err("compile should reject unknown @retry target");
				let msg = err.to_string();
				assert!(msg.contains("@retry on_fail target 'MissingStep' not found"));
		}

		#[test]
		fn rejects_timeout_directive_with_invalid_ms() {
				let source = r#"
query Run {
	Worker
}

iterator Worker on Any @timeout(ms: 0, on_timeout: return) {
	core.echo(message: "tick")
}
"#;

				let err = compile(source).expect_err("compile should reject invalid @timeout ms");
				let msg = err.to_string();
				assert!(msg.contains("@timeout ms must be >= 1"));
		}
}
