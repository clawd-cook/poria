use regex::Regex;

/// Normalize a git URL to HTTPS lowercase form for comparison.
pub fn normalize_git_url(git_url: &str) -> String {
    let trimmed = git_url.trim();
    let ssh_re = Regex::new(r"(?i)^git@([^:]+):").unwrap();
    let result = ssh_re.replace(trimmed, |caps: &regex::Captures| {
        format!("https://{}/", &caps[1])
    });
    let result = result.trim_end_matches('/').to_string();
    let git_re = Regex::new(r"(?i)\.git$").unwrap();
    let result = git_re.replace(&result, "");
    let result = result.trim_end_matches('/');
    result.to_lowercase()
}

/// Extract repo name (last path segment) from a git URL.
pub fn repo_name_from_git_url(git_url: &str) -> Option<String> {
    let cleaned = git_url.trim();
    let git_re = Regex::new(r"(?i)\.git$").unwrap();
    let cleaned = git_re.replace(cleaned, "");
    let ssh_re = Regex::new(r"^git@[^:]+:").unwrap();
    let https_re = Regex::new(r"(?i)^https?://[^/]+/").unwrap();

    let path = ssh_re.replace(&cleaned, "");
    let path = https_re.replace(&path, "");

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    segments.last().map(|s| s.to_string())
}

/// Split a git URL into `(scope, name)` using the search path.
///
/// `git@coding.jd.com:ls/ls-entrance.git` → `Some(("ls", "ls-entrance"))`.
/// Nested paths keep the remainder as `name`: `group/sub/repo` → `("group", "sub/repo")`.
pub fn repo_scope_and_name_from_git_url(git_url: &str) -> Option<(String, String)> {
    let path = repo_search_path_from_git_url(git_url)?;
    if path
        .split(['/', '\\'])
        .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return None;
    }
    let (scope, name) = path.split_once('/')?;
    let scope = scope.trim();
    let name = name.trim().trim_matches('/');
    if scope.is_empty() || name.is_empty() {
        None
    } else {
        Some((scope.to_string(), name.to_string()))
    }
}

/// Extract the full path portion from a git URL for repo search.
pub fn repo_search_path_from_git_url(git_url: &str) -> Option<String> {
    let cleaned = git_url.trim();
    let git_re = Regex::new(r"(?i)\.git$").unwrap();
    let cleaned = git_re.replace(cleaned, "");

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

/// Compare two git URLs after normalization.
pub fn same_git_url(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(l), Some(r)) if !l.is_empty() && !r.is_empty() => {
            normalize_git_url(l) == normalize_git_url(r)
        }
        _ => false,
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
    fn repo_name_from_ssh() {
        assert_eq!(
            repo_name_from_git_url("git@coding.jd.com:poria/demo.git"),
            Some("demo".to_string())
        );
    }

    #[test]
    fn repo_name_from_https() {
        assert_eq!(
            repo_name_from_git_url("https://coding.jd.com/group/sub/repo.git"),
            Some("repo".to_string())
        );
    }

    #[test]
    fn repo_name_empty_input() {
        assert_eq!(repo_name_from_git_url(""), None);
    }

    #[test]
    fn search_path_from_ssh() {
        assert_eq!(
            repo_search_path_from_git_url("git@coding.jd.com:poria/demo.git"),
            Some("poria/demo".to_string())
        );
    }

    #[test]
    fn scope_and_name_from_ls_entrance() {
        assert_eq!(
            repo_scope_and_name_from_git_url("git@coding.jd.com:ls/ls-entrance.git"),
            Some(("ls".to_string(), "ls-entrance".to_string()))
        );
        assert_eq!(
            repo_search_path_from_git_url("git@coding.jd.com:ls/ls-entrance.git"),
            Some("ls/ls-entrance".to_string())
        );
    }

    #[test]
    fn scope_and_name_from_https() {
        assert_eq!(
            repo_scope_and_name_from_git_url("https://coding.jd.com/ls/ls-entrance.git"),
            Some(("ls".to_string(), "ls-entrance".to_string()))
        );
    }

    #[test]
    fn scope_and_name_nested_keeps_rest_as_name() {
        assert_eq!(
            repo_scope_and_name_from_git_url("https://coding.jd.com/group/sub/repo.git"),
            Some(("group".to_string(), "sub/repo".to_string()))
        );
    }

    #[test]
    fn scope_and_name_rejects_missing_or_single_segment() {
        assert_eq!(repo_scope_and_name_from_git_url(""), None);
        assert_eq!(repo_scope_and_name_from_git_url("git@coding.jd.com:"), None);
        assert_eq!(
            repo_scope_and_name_from_git_url("git@coding.jd.com:onlyname.git"),
            None
        );
    }

    #[test]
    fn scope_and_name_rejects_parent_directory_segments() {
        assert_eq!(
            repo_scope_and_name_from_git_url("git@coding.jd.com:ls/../evil.git"),
            None
        );
        assert_eq!(
            repo_scope_and_name_from_git_url("git@coding.jd.com:../ls-entrance.git"),
            None
        );
    }

    #[test]
    fn search_path_from_ssh_scheme() {
        assert_eq!(
            repo_search_path_from_git_url("ssh://git@coding.jd.com/poria/demo.git"),
            Some("poria/demo".to_string())
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
    fn same_git_url_false_for_empty() {
        assert!(!same_git_url(
            Some(""),
            Some("git@coding.jd.com:poria/demo.git")
        ));
        assert!(!same_git_url(None, None));
    }
}
