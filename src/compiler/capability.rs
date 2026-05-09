use serde::{Deserialize, Serialize};

/// Canonical capability token used across compiler phases.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Capability(pub String);

impl Capability {
    pub fn from_module_op(module: &str, op: &str) -> Self {
        Self(format!("{}.{}", module, op))
    }

    pub fn from_bare_op(op: &str) -> Self {
        // Bare operations are treated as core runtime capabilities.
        Self(format!("core.{op}"))
    }
}

/// Draft policy object for future runtime/compile-time capability checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityPolicy {
    pub allowed: Vec<Capability>,
    pub denied: Vec<Capability>,
}

impl CapabilityPolicy {
    pub fn is_allowed(&self, cap: &Capability) -> bool {
        if self.denied.contains(cap) {
            return false;
        }

        self.allowed.is_empty() || self.allowed.contains(cap)
    }
}
