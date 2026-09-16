use poria_core::types::IssueClass;

/// Classifies an error message into a known issue category.
pub fn classify(error_message: &str) -> IssueClass {
    let msg = error_message;

    if msg.contains("compilation") || msg.contains("build failed") || msg.contains("tsc") {
        return IssueClass::CompilationError;
    }
    if msg.contains("test fail") || msg.contains("vitest") || msg.contains("jest") {
        return IssueClass::TestFailure;
    }
    if msg.contains("timeout") || msg.contains("AGENT_TIMEOUT") {
        return IssueClass::AgentTimeout;
    }
    if msg.contains("rate limit") || msg.contains("429") || msg.contains("overloaded") {
        return IssueClass::LlmRateLimit;
    }
    if msg.contains("ambiguous") || msg.contains("P0 unanswered") {
        return IssueClass::RequirementAmbiguous;
    }
    if msg.contains("PRD")
        && (msg.contains("empty") || msg.contains("invalid") || msg.contains("export failed"))
    {
        return IssueClass::PrdInvalid;
    }
    if msg.contains("merge conflict") || msg.contains("CONFLICT") {
        return IssueClass::MergeConflict;
    }
    if msg.contains("cr score") || msg.contains("LOW_CR_SCORE") {
        return IssueClass::LowCrScore;
    }
    if msg.contains("diff too large") || msg.contains("DIFF_TOO_LARGE") {
        return IssueClass::DiffTooLarge;
    }
    if msg.contains("permission denied") || msg.contains("403") || msg.contains("forbidden") {
        return IssueClass::PermissionDenied;
    }
    if msg.contains("ECONNREFUSED") || msg.contains("ENOTFOUND") || msg.contains("infra") {
        return IssueClass::InfraFailure;
    }
    if msg.contains("security") || msg.contains("malicious") || msg.contains("blocked_dependency")
    {
        return IssueClass::SecurityViolation;
    }
    if msg.contains("out_of_scope") || msg.contains("OutputGuardError") {
        return IssueClass::OutOfScopeChange;
    }
    if msg.contains("auth expired")
        || msg.contains("cookie expired")
        || msg.contains("AuthExpired")
    {
        return IssueClass::AuthExpired;
    }

    IssueClass::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_compilation() {
        assert_eq!(classify("compilation error on line 5"), IssueClass::CompilationError);
        assert_eq!(classify("build failed"), IssueClass::CompilationError);
        assert_eq!(classify("tsc exited with code 1"), IssueClass::CompilationError);
    }

    #[test]
    fn test_classify_test_failure() {
        assert_eq!(classify("test fail in suite A"), IssueClass::TestFailure);
        assert_eq!(classify("vitest exited 1"), IssueClass::TestFailure);
    }

    #[test]
    fn test_classify_timeout() {
        assert_eq!(classify("AGENT_TIMEOUT reached"), IssueClass::AgentTimeout);
        assert_eq!(classify("timeout waiting for response"), IssueClass::AgentTimeout);
    }

    #[test]
    fn test_classify_rate_limit() {
        assert_eq!(classify("rate limit exceeded"), IssueClass::LlmRateLimit);
        assert_eq!(classify("HTTP 429"), IssueClass::LlmRateLimit);
        assert_eq!(classify("model overloaded"), IssueClass::LlmRateLimit);
    }

    #[test]
    fn test_classify_requirement() {
        assert_eq!(classify("requirement is ambiguous"), IssueClass::RequirementAmbiguous);
        assert_eq!(classify("P0 unanswered"), IssueClass::RequirementAmbiguous);
    }

    #[test]
    fn test_classify_prd_invalid() {
        assert_eq!(classify("PRD is empty"), IssueClass::PrdInvalid);
        assert_eq!(classify("PRD export failed"), IssueClass::PrdInvalid);
    }

    #[test]
    fn test_classify_merge_conflict() {
        assert_eq!(classify("merge conflict in file.ts"), IssueClass::MergeConflict);
        assert_eq!(classify("CONFLICT detected"), IssueClass::MergeConflict);
    }

    #[test]
    fn test_classify_cr_score() {
        assert_eq!(classify("cr score below threshold"), IssueClass::LowCrScore);
        assert_eq!(classify("LOW_CR_SCORE"), IssueClass::LowCrScore);
    }

    #[test]
    fn test_classify_diff_too_large() {
        assert_eq!(classify("diff too large"), IssueClass::DiffTooLarge);
        assert_eq!(classify("DIFF_TOO_LARGE"), IssueClass::DiffTooLarge);
    }

    #[test]
    fn test_classify_permission() {
        assert_eq!(classify("permission denied"), IssueClass::PermissionDenied);
        assert_eq!(classify("403 forbidden"), IssueClass::PermissionDenied);
    }

    #[test]
    fn test_classify_infra() {
        assert_eq!(classify("ECONNREFUSED"), IssueClass::InfraFailure);
        assert_eq!(classify("infra down"), IssueClass::InfraFailure);
    }

    #[test]
    fn test_classify_security() {
        assert_eq!(classify("security violation"), IssueClass::SecurityViolation);
        assert_eq!(classify("blocked_dependency found"), IssueClass::SecurityViolation);
    }

    #[test]
    fn test_classify_out_of_scope() {
        assert_eq!(classify("out_of_scope edit"), IssueClass::OutOfScopeChange);
        assert_eq!(classify("OutputGuardError"), IssueClass::OutOfScopeChange);
    }

    #[test]
    fn test_classify_auth_expired() {
        assert_eq!(classify("auth expired"), IssueClass::AuthExpired);
        assert_eq!(classify("cookie expired"), IssueClass::AuthExpired);
        assert_eq!(classify("AuthExpired"), IssueClass::AuthExpired);
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify("some random error"), IssueClass::Unknown);
    }
}
