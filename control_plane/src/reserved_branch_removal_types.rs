//! Exact, validated identity for a single reserved-branch removal.
use serde::{Deserialize, Serialize};

pub fn is_saved_copy(name: &str) -> bool {
    name.strip_prefix("sc-").is_some_and(|suffix| {
        suffix.len() == 24
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

pub fn is_reserved_branch(name: &str) -> bool {
    is_saved_copy(name)
        || name.strip_prefix("rc-").is_some_and(|suffix| {
            suffix.len() == 24
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RemovalAction {
    #[serde(rename = "remove_saved_branch")]
    RemoveSavedBranch,
}

/// A private single-target capability. It grants no ordinary customer/service authority.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemovalScope {
    pub project_id: String,
    pub branch: String,
    pub timeline_id: String,
    pub action: RemovalAction,
}

impl RemovalScope {
    pub fn valid(&self) -> bool {
        let id = |s: &str| {
            s.len() == 32
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        };
        id(&self.project_id) && id(&self.timeline_id) && is_reserved_branch(&self.branch)
    }

    pub fn matches(&self, project: &str, branch: &str, timeline: &str) -> bool {
        self.valid()
            && self.project_id == project
            && self.branch == branch
            && self.timeline_id == timeline
    }
}
