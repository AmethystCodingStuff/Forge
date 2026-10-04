//! Mission lifecycle rules and the centralized completion gate.

use forge_models::mission::{FindingSeverity, Mission, MissionStatus, ReviewResult, ReviewState};

/// A deterministic workspace revision minted only by a future core snapshotter.
///
/// The field is private and there is deliberately no public constructor or
/// deserialization path. The current candidate has no canonical snapshotter.
#[derive(Debug, Eq, PartialEq)]
pub struct WorkspaceRevision {
    digest: [u8; 32],
}

impl WorkspaceRevision {
    /// Read the digest of a revision that was minted inside `forge-core`.
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Opaque authority minted only after every required check passes against one
/// workspace revision. No minting path is enabled until an authorized runner,
/// bounded snapshotter, and toolchain/policy binding are implemented.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedEvidence {
    mission_id: String,
    run_id: String,
    required_check_fingerprint: String,
    revision: WorkspaceRevision,
}

impl VerifiedEvidence {
    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn required_check_fingerprint(&self) -> &str {
        &self.required_check_fingerprint
    }

    pub const fn revision(&self) -> &WorkspaceRevision {
        &self.revision
    }
}

/// Opaque review authority bound by core to a mission and revision.
///
/// A caller-set `ReviewState` is not sufficient to construct this value.
#[derive(Debug, Eq, PartialEq)]
pub struct ReviewEvidence {
    mission_id: String,
    revision: WorkspaceRevision,
    result: ReviewResult,
}

/// A mission together with its controlled lifecycle state.
///
/// The status and resume target are private so callers must use the validated
/// transition functions and cannot assign `COMPLETED` directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionLifecycle {
    mission: Mission,
    status: MissionStatus,
    resume_target: Option<MissionStatus>,
}

impl MissionLifecycle {
    pub fn new(mission: Mission) -> Self {
        Self {
            mission,
            status: MissionStatus::Created,
            resume_target: None,
        }
    }

    pub const fn mission(&self) -> &Mission {
        &self.mission
    }

    pub const fn status(&self) -> MissionStatus {
        self.status
    }

    pub const fn resume_target(&self) -> Option<MissionStatus> {
        self.resume_target
    }
}

/// Why a requested lifecycle transition is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    Illegal {
        from: MissionStatus,
        to: MissionStatus,
    },
    CompletionRequiresGate,
    ResumeRequiresStoredTarget {
        status: MissionStatus,
    },
    NotPausedOrBlocked {
        status: MissionStatus,
    },
    MissingResumeTarget {
        status: MissionStatus,
    },
    InvalidResumeTarget {
        status: MissionStatus,
        target: MissionStatus,
    },
}

/// Validate a lifecycle edge without mutating a mission.
///
/// `COMPLETED` is deliberately excluded: the only supported route to that
/// status is [`complete_mission`], which validates core-minted evidence.
pub fn validate_transition(from: MissionStatus, to: MissionStatus) -> Result<(), TransitionError> {
    if to == MissionStatus::Completed {
        return Err(TransitionError::CompletionRequiresGate);
    }

    let legal = match from {
        MissionStatus::Created => matches!(
            to,
            MissionStatus::Planning
                | MissionStatus::Paused
                | MissionStatus::Blocked
                | MissionStatus::Failed
        ),
        MissionStatus::Planning => matches!(
            to,
            MissionStatus::Building
                | MissionStatus::Paused
                | MissionStatus::Blocked
                | MissionStatus::Failed
        ),
        MissionStatus::Building => matches!(
            to,
            MissionStatus::Verifying
                | MissionStatus::Paused
                | MissionStatus::Blocked
                | MissionStatus::Failed
        ),
        MissionStatus::Verifying => matches!(
            to,
            MissionStatus::Reviewing
                | MissionStatus::Building
                | MissionStatus::Paused
                | MissionStatus::Blocked
                | MissionStatus::Failed
        ),
        MissionStatus::Reviewing => matches!(
            to,
            MissionStatus::Building
                | MissionStatus::Paused
                | MissionStatus::Blocked
                | MissionStatus::Failed
        ),
        MissionStatus::Paused | MissionStatus::Blocked => to.can_resume(),
        MissionStatus::Completed | MissionStatus::Failed => false,
    };

    if legal {
        Ok(())
    } else {
        Err(TransitionError::Illegal { from, to })
    }
}

/// Apply a normal lifecycle transition, preserving the current phase as the
/// resume target when pausing or blocking.
///
/// A paused or blocked mission must use [`resume_mission`] so it can return only
/// to its recorded target. Completion must use [`complete_mission`].
pub fn transition_mission(
    mission: &mut MissionLifecycle,
    to: MissionStatus,
) -> Result<(), TransitionError> {
    if matches!(
        mission.status,
        MissionStatus::Paused | MissionStatus::Blocked
    ) {
        return Err(TransitionError::ResumeRequiresStoredTarget {
            status: mission.status,
        });
    }

    validate_transition(mission.status, to)?;

    mission.resume_target = if matches!(to, MissionStatus::Paused | MissionStatus::Blocked) {
        Some(mission.status)
    } else {
        None
    };
    mission.status = to;
    Ok(())
}

/// Resume a paused or blocked mission to exactly its saved phase.
pub fn resume_mission(mission: &mut MissionLifecycle) -> Result<MissionStatus, TransitionError> {
    let status = mission.status;
    if !matches!(status, MissionStatus::Paused | MissionStatus::Blocked) {
        return Err(TransitionError::NotPausedOrBlocked { status });
    }

    let target = mission
        .resume_target
        .ok_or(TransitionError::MissingResumeTarget { status })?;
    if !target.can_resume() || validate_transition(status, target).is_err() {
        return Err(TransitionError::InvalidResumeTarget { status, target });
    }

    mission.status = target;
    mission.resume_target = None;
    Ok(target)
}

/// Why a requested mission completion was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionGateFailure {
    MissionNotReviewing,
    VerifiedEvidenceUnavailable,
    ReviewEvidenceUnavailable,
    WorkspaceRevisionUnavailable,
    EvidenceMissionMismatch,
    ReviewMissionMismatch,
    VerificationRevisionMismatch,
    ReviewRevisionMismatch,
    ReviewNotApproved,
    CriticalSecurityFinding,
    CompletionPipelineUnavailable,
}

/// All completion-gate conditions that failed, in deterministic contract order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionGateError {
    pub failures: Vec<CompletionGateFailure>,
}

/// Complete a mission only with opaque, same-mission, same-revision evidence.
///
/// The current revision must be a fresh canonical snapshot taken at completion.
/// This candidate has no snapshotter, authorized check runner, or evidence
/// minting path, so the gate unconditionally fails closed even if internal test
/// fixtures construct opaque values. The public `VerificationResult::fresh`
/// boolean is intentionally not accepted here.
pub fn complete_mission(
    mission: &mut MissionLifecycle,
    verification: Option<&VerifiedEvidence>,
    review: Option<&ReviewEvidence>,
    current_revision: Option<&WorkspaceRevision>,
) -> Result<(), CompletionGateError> {
    let mut failures = Vec::new();

    if mission.status != MissionStatus::Reviewing {
        failures.push(CompletionGateFailure::MissionNotReviewing);
    }

    match verification {
        None => failures.push(CompletionGateFailure::VerifiedEvidenceUnavailable),
        Some(evidence) => {
            if evidence.mission_id != mission.mission.id {
                failures.push(CompletionGateFailure::EvidenceMissionMismatch);
            }
            if let Some(current) = current_revision {
                if evidence.revision != *current {
                    failures.push(CompletionGateFailure::VerificationRevisionMismatch);
                }
            }
        }
    }

    match review {
        None => failures.push(CompletionGateFailure::ReviewEvidenceUnavailable),
        Some(evidence) => {
            if evidence.mission_id != mission.mission.id {
                failures.push(CompletionGateFailure::ReviewMissionMismatch);
            }
            if let Some(current) = current_revision {
                if evidence.revision != *current {
                    failures.push(CompletionGateFailure::ReviewRevisionMismatch);
                }
            }
            if !matches!(
                evidence.result.state,
                ReviewState::Approved | ReviewState::ApprovedWithWarnings
            ) {
                failures.push(CompletionGateFailure::ReviewNotApproved);
            }
            if evidence
                .result
                .findings
                .iter()
                .chain(evidence.result.security.findings.iter())
                .any(|finding| finding.severity == FindingSeverity::Critical)
            {
                failures.push(CompletionGateFailure::CriticalSecurityFinding);
            }
        }
    }

    if current_revision.is_none() {
        failures.push(CompletionGateFailure::WorkspaceRevisionUnavailable);
    }

    // No production code can mint the required evidence yet. Keep this explicit
    // hard stop even if future internal code accidentally fabricates the opaque
    // values before implementing the real verifier and completion resnapshot.
    failures.push(CompletionGateFailure::CompletionPipelineUnavailable);

    Err(CompletionGateError { failures })
}

#[cfg(test)]
mod tests {
    use super::{
        CompletionGateFailure, MissionLifecycle, ReviewEvidence, TransitionError, VerifiedEvidence,
        WorkspaceRevision, complete_mission, resume_mission, transition_mission,
        validate_transition,
    };
    use forge_models::mission::{
        Finding, FindingSeverity, Mission, MissionStatus, ReviewResult, ReviewState,
        VerificationResult, VerificationState,
    };

    fn mission() -> MissionLifecycle {
        MissionLifecycle::new(Mission::new(
            "mission-1",
            "implement a bounded domain slice",
        ))
    }

    fn revision(byte: u8) -> WorkspaceRevision {
        WorkspaceRevision { digest: [byte; 32] }
    }

    fn verified(mission_id: &str, revision_byte: u8) -> VerifiedEvidence {
        VerifiedEvidence {
            mission_id: mission_id.to_owned(),
            run_id: "run-test-only".to_owned(),
            required_check_fingerprint: "checks-test-only".to_owned(),
            revision: revision(revision_byte),
        }
    }

    fn review_evidence(mission_id: &str, revision_byte: u8, state: ReviewState) -> ReviewEvidence {
        ReviewEvidence {
            mission_id: mission_id.to_owned(),
            revision: revision(revision_byte),
            result: ReviewResult::new(state),
        }
    }

    fn critical_finding() -> Finding {
        Finding {
            category: "security".to_owned(),
            severity: FindingSeverity::Critical,
            location: Some("src/lib.rs:1".to_owned()),
            evidence: "critical test finding".to_owned(),
            explanation: "the finding is critical".to_owned(),
            recommendation: "remove the issue".to_owned(),
        }
    }

    fn move_to_reviewing(mission: &mut MissionLifecycle) {
        transition_mission(mission, MissionStatus::Planning).unwrap();
        transition_mission(mission, MissionStatus::Building).unwrap();
        transition_mission(mission, MissionStatus::Verifying).unwrap();
        transition_mission(mission, MissionStatus::Reviewing).unwrap();
    }

    fn completion_error(
        mission: &mut MissionLifecycle,
        verification: Option<&VerifiedEvidence>,
        review: Option<&ReviewEvidence>,
        current_revision: Option<&WorkspaceRevision>,
    ) -> Vec<CompletionGateFailure> {
        complete_mission(mission, verification, review, current_revision)
            .unwrap_err()
            .failures
    }

    #[test]
    fn allows_documented_forward_and_repair_transitions() {
        let legal = [
            (MissionStatus::Created, MissionStatus::Planning),
            (MissionStatus::Planning, MissionStatus::Building),
            (MissionStatus::Building, MissionStatus::Verifying),
            (MissionStatus::Verifying, MissionStatus::Reviewing),
            (MissionStatus::Verifying, MissionStatus::Building),
            (MissionStatus::Reviewing, MissionStatus::Building),
        ];
        for (from, to) in legal {
            assert_eq!(validate_transition(from, to), Ok(()), "{from} -> {to}");
        }
    }

    #[test]
    fn rejects_skipped_phases_and_requires_gate_for_completion() {
        assert_eq!(
            validate_transition(MissionStatus::Created, MissionStatus::Building),
            Err(TransitionError::Illegal {
                from: MissionStatus::Created,
                to: MissionStatus::Building,
            })
        );
        assert_eq!(
            validate_transition(MissionStatus::Reviewing, MissionStatus::Completed),
            Err(TransitionError::CompletionRequiresGate)
        );
        let mut value = mission();
        assert_eq!(
            transition_mission(&mut value, MissionStatus::Completed),
            Err(TransitionError::CompletionRequiresGate)
        );
        assert_eq!(value.status(), MissionStatus::Created);
    }

    #[test]
    fn pause_and_block_preserve_the_exact_resume_target() {
        let mut paused = mission();
        move_to_reviewing(&mut paused);
        transition_mission(&mut paused, MissionStatus::Paused).unwrap();
        assert_eq!(paused.status(), MissionStatus::Paused);
        assert_eq!(paused.resume_target(), Some(MissionStatus::Reviewing));
        assert_eq!(resume_mission(&mut paused), Ok(MissionStatus::Reviewing));
        assert_eq!(paused.status(), MissionStatus::Reviewing);
        assert_eq!(paused.resume_target(), None);

        let mut blocked = mission();
        transition_mission(&mut blocked, MissionStatus::Planning).unwrap();
        transition_mission(&mut blocked, MissionStatus::Building).unwrap();
        transition_mission(&mut blocked, MissionStatus::Blocked).unwrap();
        assert_eq!(blocked.resume_target(), Some(MissionStatus::Building));
        assert_eq!(resume_mission(&mut blocked), Ok(MissionStatus::Building));
    }

    #[test]
    fn paused_or_blocked_missions_cannot_skip_the_saved_resume_target() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        transition_mission(&mut value, MissionStatus::Paused).unwrap();
        assert_eq!(
            transition_mission(&mut value, MissionStatus::Building),
            Err(TransitionError::ResumeRequiresStoredTarget {
                status: MissionStatus::Paused,
            })
        );
        value.resume_target = Some(MissionStatus::Completed);
        assert_eq!(
            resume_mission(&mut value),
            Err(TransitionError::InvalidResumeTarget {
                status: MissionStatus::Paused,
                target: MissionStatus::Completed,
            })
        );
    }

    #[test]
    fn completion_requires_opaque_evidence_and_a_canonical_revision() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        let failures = completion_error(&mut value, None, None, None);
        assert_eq!(
            failures,
            vec![
                CompletionGateFailure::VerifiedEvidenceUnavailable,
                CompletionGateFailure::ReviewEvidenceUnavailable,
                CompletionGateFailure::WorkspaceRevisionUnavailable,
                CompletionGateFailure::CompletionPipelineUnavailable,
            ]
        );
        assert_eq!(value.status(), MissionStatus::Reviewing);
    }

    #[test]
    fn caller_set_fresh_boolean_and_spoofed_tool_success_cannot_complete() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        // This legacy public value can be caller-created, so it is deliberately
        // not accepted by complete_mission (nor can a JSON success field be).
        let spoof = VerificationResult::new(VerificationState::Verified, true);
        assert!(spoof.fresh);
        let revision = revision(7);
        let review = review_evidence("mission-1", 7, ReviewState::Approved);
        let failures = completion_error(&mut value, None, Some(&review), Some(&revision));
        assert_eq!(
            failures,
            vec![
                CompletionGateFailure::VerifiedEvidenceUnavailable,
                CompletionGateFailure::CompletionPipelineUnavailable,
            ]
        );
        assert_eq!(value.status(), MissionStatus::Reviewing);
    }

    #[test]
    fn stale_or_changed_revision_blocks_completion() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        let evidence = verified("mission-1", 1);
        let review = review_evidence("mission-1", 1, ReviewState::Approved);
        let current = revision(2);
        let failures = completion_error(&mut value, Some(&evidence), Some(&review), Some(&current));
        assert!(failures.contains(&CompletionGateFailure::VerificationRevisionMismatch));
        assert!(failures.contains(&CompletionGateFailure::ReviewRevisionMismatch));
        assert!(failures.contains(&CompletionGateFailure::CompletionPipelineUnavailable));
        assert_eq!(value.status(), MissionStatus::Reviewing);
    }

    #[test]
    fn mission_identity_mismatch_blocks_completion() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        let revision = revision(3);
        let evidence = verified("another-mission", 3);
        let review = review_evidence("another-mission", 3, ReviewState::Approved);
        let failures =
            completion_error(&mut value, Some(&evidence), Some(&review), Some(&revision));
        assert!(failures.contains(&CompletionGateFailure::EvidenceMissionMismatch));
        assert!(failures.contains(&CompletionGateFailure::ReviewMissionMismatch));
        assert_eq!(value.status(), MissionStatus::Reviewing);
    }

    #[test]
    fn completion_never_succeeds_until_real_pipeline_is_implemented() {
        for state in [ReviewState::Approved, ReviewState::ApprovedWithWarnings] {
            let mut value = mission();
            move_to_reviewing(&mut value);
            let revision = revision(4);
            let evidence = verified("mission-1", 4);
            let review = review_evidence("mission-1", 4, state);
            let failures =
                completion_error(&mut value, Some(&evidence), Some(&review), Some(&revision));
            assert_eq!(
                failures,
                vec![CompletionGateFailure::CompletionPipelineUnavailable]
            );
            assert_eq!(value.status(), MissionStatus::Reviewing);
        }
    }

    #[test]
    fn unapproved_review_state_is_rejected() {
        for state in [ReviewState::ChangesRequired, ReviewState::Blocked] {
            let mut value = mission();
            move_to_reviewing(&mut value);
            let revision = revision(8);
            let evidence = verified("mission-1", 8);
            let review = review_evidence("mission-1", 8, state);
            let failures =
                completion_error(&mut value, Some(&evidence), Some(&review), Some(&revision));
            assert!(failures.contains(&CompletionGateFailure::ReviewNotApproved));
            assert_eq!(value.status(), MissionStatus::Reviewing);
        }
    }

    #[test]
    fn completed_and_failed_are_terminal() {
        for terminal in [MissionStatus::Completed, MissionStatus::Failed] {
            for next in [
                MissionStatus::Planning,
                MissionStatus::Paused,
                MissionStatus::Blocked,
                MissionStatus::Failed,
            ] {
                assert!(matches!(
                    validate_transition(terminal, next),
                    Err(TransitionError::Illegal { .. })
                ));
            }
        }
    }

    #[test]
    fn completion_gate_rejects_critical_findings_from_every_collection() {
        for placement in [0, 1, 2] {
            let mut value = mission();
            move_to_reviewing(&mut value);
            let revision = revision(5);
            let evidence = verified("mission-1", 5);
            let mut review = review_evidence("mission-1", 5, ReviewState::ApprovedWithWarnings);
            match placement {
                0 => review.result.findings.push(critical_finding()),
                1 => review.result.security.findings.push(critical_finding()),
                _ => {
                    review.result.findings.push(critical_finding());
                    review.result.security.findings.push(critical_finding());
                }
            }
            let failures =
                completion_error(&mut value, Some(&evidence), Some(&review), Some(&revision));
            assert!(failures.contains(&CompletionGateFailure::CriticalSecurityFinding));
            assert!(failures.contains(&CompletionGateFailure::CompletionPipelineUnavailable));
            assert_eq!(value.status(), MissionStatus::Reviewing);
        }
    }

    #[test]
    fn high_severity_review_finding_is_not_misclassified_as_critical() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        let revision = revision(9);
        let evidence = verified("mission-1", 9);
        let mut review = review_evidence("mission-1", 9, ReviewState::ApprovedWithWarnings);
        let mut finding = critical_finding();
        finding.severity = FindingSeverity::High;
        review.result.security.findings.push(finding);

        let failures =
            completion_error(&mut value, Some(&evidence), Some(&review), Some(&revision));
        assert!(!failures.contains(&CompletionGateFailure::CriticalSecurityFinding));
        assert!(failures.contains(&CompletionGateFailure::CompletionPipelineUnavailable));
        assert_eq!(value.status(), MissionStatus::Reviewing);
    }

    #[test]
    fn missing_completion_time_revision_fails_closed() {
        let mut value = mission();
        move_to_reviewing(&mut value);
        let evidence = verified("mission-1", 6);
        let review = review_evidence("mission-1", 6, ReviewState::Approved);
        let failures = completion_error(&mut value, Some(&evidence), Some(&review), None);
        assert!(failures.contains(&CompletionGateFailure::WorkspaceRevisionUnavailable));
        assert!(failures.contains(&CompletionGateFailure::CompletionPipelineUnavailable));
        assert_eq!(value.status(), MissionStatus::Reviewing);
    }
}
