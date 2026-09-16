/// Errors originating from the skills crate.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// The skill is a stub; real logic has not been implemented yet.
    #[error("{0}: real implementation not yet available")]
    NotImplemented(String),
}
