//! Typed domain models shared across Forge crates.

use std::fmt;

/// Canonical mission lifecycle status from the Forge design contract.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MissionStatus {
    Created,
    Planning,
    Building,
    Verifying,
    Reviewing,
    Completed,
    Paused,
    Blocked,
    Failed,
}

impl MissionStatus {
    /// Return the canonical status label used by the design contract.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::Planning => "PLANNING",
            Self::Building => "BUILDING",
            Self::Verifying => "VERIFYING",
            Self::Reviewing => "REVIEWING",
            Self::Completed => "COMPLETED",
            Self::Paused => "PAUSED",
            Self::Blocked => "BLOCKED",
            Self::Failed => "FAILED",
        }
    }

    /// Whether this status can be retained as a pause/block resume target.
    pub const fn can_resume(self) -> bool {
        matches!(
            self,
            Self::Created | Self::Planning | Self::Building | Self::Verifying | Self::Reviewing
        )
    }

    /// Whether this status is terminal.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

impl fmt::Display for MissionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A mission's identity and objective, independent of mutable lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mission {
    pub id: String,
    pub objective: String,
}

impl Mission {
    /// Construct a mission in its initial `CREATED` state.
    pub fn new(id: impl Into<String>, objective: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            objective: objective.into(),
        }
    }
}

/// Outcome of the verification pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationState {
    Verified,
    PartiallyVerified,
    NotVerified,
    Failed,
}

/// Informational verification state retained for compatibility.
///
/// `fresh` is caller-controlled and is not authoritative completion evidence.
/// The core completion gate intentionally does not accept this type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    pub state: VerificationState,
    pub fresh: bool,
}

impl VerificationResult {
    pub const fn new(state: VerificationState, fresh: bool) -> Self {
        Self { state, fresh }
    }
}

/// Outcome of the review pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewState {
    Approved,
    ApprovedWithWarnings,
    ChangesRequired,
    Blocked,
}

/// Severity assigned to a structured review or security finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

/// Structured finding data as specified for review output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub category: String,
    pub severity: FindingSeverity,
    pub location: Option<String>,
    pub evidence: String,
    pub explanation: String,
    pub recommendation: String,
}

/// Findings produced by the security portion of review.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SecurityResult {
    pub findings: Vec<Finding>,
}

impl SecurityResult {
    pub fn has_critical_finding(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == FindingSeverity::Critical)
    }
}

/// Review state, review findings, and the independent security-review result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewResult {
    pub state: ReviewState,
    pub findings: Vec<Finding>,
    pub security: SecurityResult,
}

impl ReviewResult {
    pub fn new(state: ReviewState) -> Self {
        Self {
            state,
            findings: Vec::new(),
            security: SecurityResult::default(),
        }
    }
}
