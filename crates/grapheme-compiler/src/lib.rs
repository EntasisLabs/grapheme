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
}
