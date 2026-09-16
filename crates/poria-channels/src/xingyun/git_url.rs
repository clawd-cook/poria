use regex::Regex;

/// Normalize a git URL to HTTPS lowercase form for comparison.
pub fn normalize_git_url(git_url: &str) -> String {
    let trimmed = git_url.trim();
    // Convert SSH to HTTPS
    let ssh_re = Regex::new(r"(?i)^git@([^:]+):").unwrap();
    let result = ssh_re.replace(trimmed, |caps: &regex::Captures| {
        format!("https://{}/", &caps[1])
    });
    let result = result.trim_end_matches('/').to_string();
    // Remove trailing .git
    let git_re = Regex::new(r"(?i)\.git$").unwrap();
    let result = git_re.replace(&result, "");
    let result = result.trim_end_matches('/');
    result.to_lowercase()
}

/// Compare two git URLs after normalization.
pub fn same_git_url(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(l), Some(r)) if !l.is_empty() && !r.is_empty() => {
            normalize_git_url(l) == normalize_git_url(r)
        }
        _ => false,
    }
}

/// Extract the path portion from a git URL for repo search.
pub fn repo_search_path_from_git_url(git_url: &str) -> Option<String> {
    let cleaned = git_url.trim();
    let git_suffix_re = Regex::new(r"(?i)\.git$").unwrap();
    let cleaned = git_suffix_re.replace(cleaned, "");

    // Strip various protocol prefixes
    let ssh_re = Regex::new(r"(?i)^git@[^:]+:").unwrap();
    let ssh_scheme_re = Regex::new(r"(?i)^ssh://git@[^/]+/").unwrap();
    let https_re = Regex::new(r"(?i)^https?://[^/]+/").unwrap();

    let path = ssh_re.replace(&cleaned, "");
    let path = ssh_scheme_re.replace(&path, "");
    let path = https_re.replace(&path, "");
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ssh_to_https() {
        assert_eq!(
            normalize_git_url("git@coding.jd.com:poria/demo.git"),
            "https://coding.jd.com/poria/demo"
        );
    }

    #[test]
    fn normalize_https_url() {
        assert_eq!(
            normalize_git_url("https://Coding.JD.COM/Poria/Demo.GIT/"),
            "https://coding.jd.com/poria/demo"
        );
    }

    #[test]
    fn same_git_url_ssh_and_https() {
        assert!(same_git_url(
            Some("git@coding.jd.com:poria/demo.git"),
            Some("https://coding.jd.com/poria/demo")
        ));
    }

    #[test]
    fn same_git_url_case_insensitive() {
        assert!(same_git_url(
            Some("git@CODING.JD.COM:Poria/Demo.git"),
            Some("git@coding.jd.com:poria/demo.git")
        ));
    }

    #[test]
    fn same_git_url_returns_false_for_empty() {
        assert!(!same_git_url(Some(""), Some("git@coding.jd.com:poria/demo.git")));
        assert!(!same_git_url(None, None));
    }

    #[test]
    fn repo_search_path_ssh() {
        assert_eq!(
            repo_search_path_from_git_url("git@coding.jd.com:poria/demo.git"),
            Some("poria/demo".to_string())
        );
    }

    #[test]
    fn repo_search_path_ssh_scheme() {
        assert_eq!(
            repo_search_path_from_git_url("ssh://git@coding.jd.com/poria/demo.git"),
            Some("poria/demo".to_string())
        );
    }
}
