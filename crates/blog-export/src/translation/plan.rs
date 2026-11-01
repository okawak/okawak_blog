//! Pure update decisions; adapters only supply fingerprints and candidate identity.
use crate::{ExportError, Result};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Reuse,
    Generate,
    Protect,
}

pub(super) fn decide(
    input: &str,
    current: Option<&str>,
    provenance: Option<(&str, &str)>,
) -> Decision {
    match (current, provenance) {
        (None, _) => Decision::Generate,
        (Some(_), Some((previous_input, _))) if previous_input == input => Decision::Reuse,
        (Some(hash), Some((_, generated_hash))) if generated_hash == hash => Decision::Generate,
        _ => Decision::Protect,
    }
}

/// A validated action, with no invalid current/candidate combinations left over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UpdatePlan {
    Reuse,
    Generate,
    GenerateCandidate,
    ReuseCandidate,
}

impl UpdatePlan {
    pub(super) fn action(self) -> &'static str {
        match self {
            Self::Reuse => "reuse",
            Self::Generate => "generate",
            Self::GenerateCandidate => "generate candidate",
            Self::ReuseCandidate => "reuse candidate",
        }
    }

    pub(super) fn requires_response(self) -> bool {
        matches!(self, Self::Generate | Self::GenerateCandidate)
    }

    pub(super) fn protects_current(self) -> bool {
        matches!(self, Self::GenerateCandidate | Self::ReuseCandidate)
    }
}

pub(super) fn plan_update(
    current: Decision,
    candidate: Option<Decision>,
    identity: &str,
) -> Result<UpdatePlan> {
    match (current, candidate) {
        (Decision::Reuse, _) => Ok(UpdatePlan::Reuse),
        (_, Some(Decision::Protect)) => Err(ExportError::translation_conflict(format!(
            "manually edited candidate {identity}; move it aside before generating a replacement"
        ))),
        (Decision::Generate, Some(Decision::Reuse)) => {
            Err(ExportError::translation_conflict(format!(
                "candidate {identity} already matches this input; accept it or move it aside before generating a replacement"
            )))
        }
        (Decision::Generate, _) => Ok(UpdatePlan::Generate),
        (Decision::Protect, Some(Decision::Reuse)) => Ok(UpdatePlan::ReuseCandidate),
        (Decision::Protect, _) => Ok(UpdatePlan::GenerateCandidate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_action_describes_each_plan() {
        assert_eq!(UpdatePlan::Reuse.action(), "reuse");
        assert_eq!(UpdatePlan::Generate.action(), "generate");
        assert_eq!(UpdatePlan::GenerateCandidate.action(), "generate candidate");
        assert_eq!(UpdatePlan::ReuseCandidate.action(), "reuse candidate");
    }

    #[test]
    fn guard_preserves_manual_edits_even_when_input_is_unchanged() {
        let provenance = ("old-input", "machine");
        assert_eq!(
            decide("old-input", Some("manual"), Some(provenance)),
            Decision::Reuse
        );
        assert_eq!(
            decide("new-input", Some("manual"), Some(provenance)),
            Decision::Protect
        );
        assert_eq!(
            decide("new-input", Some("machine"), Some(provenance)),
            Decision::Generate
        );
        assert_eq!(decide("new-input", Some("manual"), None), Decision::Protect);
        assert_eq!(decide("new-input", None, None), Decision::Generate);
    }

    #[test]
    fn updates_distinguish_published_output_from_review_candidates() {
        use Decision::{Generate, Protect, Reuse};
        for (current, candidate, expected) in [
            (Reuse, None, UpdatePlan::Reuse),
            (Reuse, Some(Reuse), UpdatePlan::Reuse),
            (Reuse, Some(Generate), UpdatePlan::Reuse),
            (Reuse, Some(Protect), UpdatePlan::Reuse),
            (Generate, None, UpdatePlan::Generate),
            (Generate, Some(Generate), UpdatePlan::Generate),
            (Protect, None, UpdatePlan::GenerateCandidate),
            (Protect, Some(Generate), UpdatePlan::GenerateCandidate),
            (Protect, Some(Reuse), UpdatePlan::ReuseCandidate),
        ] {
            assert_eq!(plan_update(current, candidate, "item").unwrap(), expected);
        }
    }

    #[test]
    fn changed_reviewed_candidates_block_both_generation_destinations() {
        for current in [Decision::Generate, Decision::Protect] {
            let error = plan_update(current, Some(Decision::Protect), "item").unwrap_err();
            assert!(matches!(error, crate::ExportError::TranslationConflict(_)));
            assert!(error.to_string().contains("manually edited candidate item"));
        }
    }

    #[test]
    fn matching_candidates_require_acceptance_before_replacing_published_output() {
        let error = plan_update(Decision::Generate, Some(Decision::Reuse), "item").unwrap_err();
        assert!(matches!(error, crate::ExportError::TranslationConflict(_)));
        assert!(error.to_string().contains("candidate item already matches"));
    }
}
