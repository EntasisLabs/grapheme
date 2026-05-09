use serde::{Deserialize, Serialize};

use crate::ast::{Definition, FieldCall, OpKind, Pipeline, Program};

use super::capability::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub imports: Vec<HirImport>,
    pub executable_defs: Vec<HirExecutable>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirImport {
    pub alias: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirExecutable {
    pub kind: HirExecutableKind,
    pub name: String,
    pub pipelines: Vec<HirPipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HirExecutableKind {
    Query,
    Mutation,
    Subscription,
    Fragment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirPipeline {
    pub steps: Vec<HirStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStep {
    pub op: String,
    pub module: Option<String>,
    pub arg_count: usize,
    pub has_selection: bool,
    pub capability: Capability,
}

pub fn lower_from_ast(program: &Program) -> HirProgram {
    let imports = program
        .imports
        .iter()
        .map(|i| HirImport {
            alias: i.alias.clone(),
            path: i.path.clone(),
        })
        .collect::<Vec<_>>();

    let mut executable_defs = Vec::new();

    for def in &program.definitions {
        match def {
            Definition::Query(q) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Query,
                name: q.name.clone(),
                pipelines: lower_pipelines(&q.pipelines),
            }),
            Definition::Mutation(m) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Mutation,
                name: m.name.clone(),
                pipelines: lower_pipelines(&m.pipelines),
            }),
            Definition::Subscription(s) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Subscription,
                name: s.name.clone(),
                pipelines: lower_pipelines(&s.pipelines),
            }),
            Definition::Fragment(f) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Fragment,
                name: f.name.clone(),
                pipelines: lower_pipelines(&f.pipelines),
            }),
            Definition::Schema(_) | Definition::ModuleProposal(_) => {}
        }
    }

    let mut capabilities = executable_defs
        .iter()
        .flat_map(|d| d.pipelines.iter())
        .flat_map(|p| p.steps.iter())
        .map(|s| s.capability.clone())
        .collect::<Vec<_>>();

    capabilities.sort();
    capabilities.dedup();

    HirProgram {
        imports,
        executable_defs,
        capabilities,
    }
}

fn lower_pipelines(pipelines: &[Pipeline]) -> Vec<HirPipeline> {
    pipelines
        .iter()
        .map(|p| HirPipeline {
            steps: p.steps.iter().map(lower_step).collect(),
        })
        .collect()
}

fn lower_step(step: &FieldCall) -> HirStep {
    let capability = match &step.module {
        Some(module) => Capability::from_module_op(module, &step.name),
        None => Capability::from_bare_op(&step.name),
    };

    HirStep {
        op: step.name.clone(),
        module: step.module.clone(),
        arg_count: step.args.len(),
        has_selection: step.selection.is_some(),
        capability,
    }
}

pub fn op_kind_to_str(kind: &OpKind) -> &'static str {
    match kind {
        OpKind::Query => "query",
        OpKind::Mutation => "mutation",
    }
}
