use super::credentials::{
    get_credentials, parse_username_from_cookie, save_credentials, JacpCredentials,
};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, thiserror::Error)]
#[error("Authentication expired during pipeline execution")]
pub struct AuthExpiredDuringPipelineError;

pub type ApiProbe = Box<
    dyn Fn(&JacpCredentials) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

pub struct CredentialGuard {
    probe: ApiProbe,
    user_root: Option<std::path::PathBuf>,
}

impl CredentialGuard {
    pub fn new(probe: ApiProbe, user_root: Option<std::path::PathBuf>) -> Self {
        Self { probe, user_root }
    }

    pub async fn ensure_valid(
        &self,
        credentials: &JacpCredentials,
    ) -> Result<JacpCredentials, AuthExpiredDuringPipelineError> {
        if (self.probe)(credentials).await.is_ok() {
            return Ok(credentials.clone());
        }

        let refreshed = self
            .try_refresh()
            .await
            .map_err(|_| AuthExpiredDuringPipelineError)?;

        (self.probe)(&refreshed)
            .await
            .map_err(|_| AuthExpiredDuringPipelineError)?;

        Ok(refreshed)
    }

    async fn try_refresh(&self) -> Result<JacpCredentials, String> {
        let creds = get_credentials(self.user_root.as_deref())
            .ok_or_else(|| "No stored credentials".to_string())?;

        let username =
            parse_username_from_cookie(&creds.cookie).unwrap_or_else(|| creds.username.clone());

        let refreshed = JacpCredentials {
            username,
            cookie: creds.cookie,
        };

        save_credentials(&refreshed, self.user_root.as_deref())?;
        Ok(refreshed)
    }
}
