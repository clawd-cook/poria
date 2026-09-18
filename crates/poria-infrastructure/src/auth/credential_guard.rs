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
            .try_refresh(credentials)
            .await
            .map_err(|_| AuthExpiredDuringPipelineError)?;

        (self.probe)(&refreshed)
            .await
            .map_err(|_| AuthExpiredDuringPipelineError)?;

        Ok(refreshed)
    }

    /// Re-read `auth.json` only when the cookie actually changed (login completed
    /// in another window). SSO cookies cannot be minted silently.
    async fn try_refresh(&self, previous: &JacpCredentials) -> Result<JacpCredentials, String> {
        let creds = get_credentials(self.user_root.as_deref())
            .ok_or_else(|| "No stored credentials".to_string())?;

        if creds.cookie.trim() == previous.cookie.trim() {
            return Err("SSO cookie unchanged; silent refresh is not available".into());
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn creds(cookie: &str) -> JacpCredentials {
        JacpCredentials {
            username: "tester".into(),
            cookie: cookie.into(),
        }
    }

    #[tokio::test]
    async fn ensure_valid_returns_when_probe_ok() {
        let guard = CredentialGuard::new(Box::new(|_| Box::pin(async { Ok(()) })), None);
        let got = guard.ensure_valid(&creds("abc")).await.unwrap();
        assert_eq!(got.cookie, "abc");
    }

    #[tokio::test]
    async fn ensure_valid_fails_when_cookie_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let stored = creds("stale");
        save_credentials(&stored, Some(dir.path())).unwrap();
        let guard = CredentialGuard::new(
            Box::new(|_| Box::pin(async { Err("HTTP 401".into()) })),
            Some(dir.path().to_path_buf()),
        );
        let err = guard.ensure_valid(&stored).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Authentication expired during pipeline execution"
        );
    }

    #[tokio::test]
    async fn ensure_valid_uses_newer_disk_cookie() {
        let dir = tempfile::tempdir().unwrap();
        save_credentials(&creds("fresh"), Some(dir.path())).unwrap();
        let probes = Arc::new(AtomicUsize::new(0));
        let probes_clone = probes.clone();
        let guard = CredentialGuard::new(
            Box::new(move |c| {
                let n = probes_clone.fetch_add(1, Ordering::SeqCst);
                let cookie = c.cookie.clone();
                Box::pin(async move {
                    if n == 0 {
                        Err("HTTP 401".into())
                    } else if cookie == "fresh" {
                        Ok(())
                    } else {
                        Err("still bad".into())
                    }
                })
            }),
            Some(dir.path().to_path_buf()),
        );
        let got = guard.ensure_valid(&creds("stale")).await.unwrap();
        assert_eq!(got.cookie, "fresh");
        assert_eq!(probes.load(Ordering::SeqCst), 2);
    }
}
