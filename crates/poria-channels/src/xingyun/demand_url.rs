use regex::Regex;
use url::Url;

/// Parsed result from a Xingyun demand URL.
#[derive(Debug, Clone)]
pub struct ParsedDemandUrl {
    pub demand_id: i64,
    pub demand_code: Option<String>,
    pub url: String,
}

/// Parse Xingyun demand URLs like:
/// `http://xingyun.jd.com/demands/view/<code>/-1?demandId=<id>`
pub fn parse_xingyun_demand_url(raw: &str) -> Result<ParsedDemandUrl, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(
            "Missing Xingyun demand URL. Expected: http://xingyun.jd.com/demands/view/<code>/-1?demandId=<id>"
                .to_string(),
        );
    }

    let parsed = Url::parse(text).map_err(|_| {
        format!(
            "Invalid Xingyun demand URL: {}. Expected: http://xingyun.jd.com/demands/view/<code>/-1?demandId=<id>",
            text
        )
    })?;

    let host = parsed.host_str().unwrap_or("").to_lowercase();
    let xingyun_re = Regex::new(r"(?:^|\.)xingyun\.jd\.com$").unwrap();
    if !xingyun_re.is_match(&host) && !host.contains("xingyun") {
        return Err(format!(
            "Not a Xingyun demand host ({}). Expected xingyun.jd.com",
            parsed.host_str().unwrap_or("")
        ));
    }

    let demand_id_raw = parsed
        .query_pairs()
        .find(|(k, _)| k == "demandId" || k == "id")
        .map(|(_, v)| v.to_string());

    let demand_id: i64 = demand_id_raw
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|&id| id > 0)
        .ok_or_else(|| {
            "Xingyun URL missing demandId query param. Example: ...?demandId=12345".to_string()
        })?;

    let demand_code = extract_demand_code(parsed.path());

    Ok(ParsedDemandUrl {
        demand_id,
        demand_code,
        url: text.to_string(),
    })
}

fn extract_demand_code(path: &str) -> Option<String> {
    let re = Regex::new(r"(?i)/demands/view/([^/]+)").unwrap();
    let re2 = Regex::new(r"(?i)/demands/(?:view/)?([^/]+)").unwrap();

    let caps = re.captures(path).or_else(|| re2.captures(path));
    if let Some(caps) = caps {
        if let Some(m) = caps.get(1) {
            let segment = m.as_str();
            // If purely numeric (or negative numeric), skip
            let numeric_re = Regex::new(r"^-?\d+$").unwrap();
            if numeric_re.is_match(segment) {
                return None;
            }
            let decoded = urlencoding_decode(segment).trim().to_string();
            if decoded.is_empty() {
                return None;
            }
            return Some(decoded);
        }
    }
    None
}

fn urlencoding_decode(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                result.push(val);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

/// Build a Xingyun demand view URL from id + optional code.
pub fn xingyun_demand_view_url(demand_id: i64, demand_code: Option<&str>) -> String {
    let segment = demand_code
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .unwrap_or("");
    let path_code = if segment.is_empty() {
        demand_id.to_string()
    } else {
        segment.to_string()
    };
    format!("http://xingyun.jd.com/demands/view/{path_code}/-1?demandId={demand_id}")
}

/// Generate feature branch name: `feature_<code>` or `feature_demand_<id>`.
/// Illegal characters are replaced by underscore.
pub fn feature_branch_name(demand_code: Option<&str>, demand_id: i64) -> String {
    let raw = match demand_code {
        Some(code) if !code.trim().is_empty() => format!("feature_{}", code.trim()),
        _ => format!("feature_demand_{}", demand_id),
    };
    let re = Regex::new(r"[^A-Za-z0-9._-]+").unwrap();
    let replaced = re.replace_all(&raw, "_");
    let multi_underscore = Regex::new(r"_+").unwrap();
    multi_underscore.replace_all(&replaced, "_").to_string()
}

/// Generate a feature slug for naming.
pub fn feature_slug(demand_code: Option<&str>, demand_id: i64) -> String {
    let code = match demand_code {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => format!("demand-{}", demand_id),
    };
    let lower = code.to_lowercase();
    let re = Regex::new(r"[^a-z0-9]+").unwrap();
    let slug = re.replace_all(&lower, "-");
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        format!("feat-{}", demand_id)
    } else {
        format!("feat-{}", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_real_xingyun_demand_url() {
        let result = parse_xingyun_demand_url(
            "http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029",
        )
        .unwrap();
        assert_eq!(result.demand_id, 4840029);
        assert_eq!(result.demand_code.as_deref(), Some("JL3R4IV4"));
        assert_eq!(
            result.url,
            "http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029"
        );
    }

    #[test]
    fn extracts_demand_id_from_id_param() {
        let result =
            parse_xingyun_demand_url("http://xingyun.jd.com/demands/view/ABC123/-1?id=12345")
                .unwrap();
        assert_eq!(result.demand_id, 12345);
        assert_eq!(result.demand_code.as_deref(), Some("ABC123"));
    }

    #[test]
    fn handles_subdomain_xingyun_urls() {
        let result = parse_xingyun_demand_url(
            "http://test.xingyun.jd.com/demands/view/CODE1/-1?demandId=999",
        )
        .unwrap();
        assert_eq!(result.demand_id, 999);
        assert_eq!(result.demand_code.as_deref(), Some("CODE1"));
    }

    #[test]
    fn throws_on_empty_input() {
        let err = parse_xingyun_demand_url("").unwrap_err();
        assert!(err.contains("Missing Xingyun demand URL"));
    }

    #[test]
    fn throws_on_invalid_url() {
        let err = parse_xingyun_demand_url("not-a-url").unwrap_err();
        assert!(err.contains("Invalid Xingyun demand URL"));
    }

    #[test]
    fn throws_on_non_xingyun_host() {
        let err = parse_xingyun_demand_url("http://example.com/demands/view/X/-1?demandId=1")
            .unwrap_err();
        assert!(err.contains("Not a Xingyun demand host"));
    }

    #[test]
    fn throws_when_demand_id_missing() {
        let err =
            parse_xingyun_demand_url("http://xingyun.jd.com/demands/view/CODE1/-1").unwrap_err();
        assert!(err.contains("missing demandId"));
    }

    #[test]
    fn throws_when_demand_id_not_a_number() {
        let err =
            parse_xingyun_demand_url("http://xingyun.jd.com/demands/view/CODE1/-1?demandId=abc")
                .unwrap_err();
        assert!(err.contains("missing demandId"));
    }

    #[test]
    fn omits_demand_code_when_numeric() {
        let result =
            parse_xingyun_demand_url("http://xingyun.jd.com/demands/view/123/-1?demandId=456")
                .unwrap();
        assert_eq!(result.demand_id, 456);
        assert!(result.demand_code.is_none());
    }

    #[test]
    fn trims_whitespace() {
        let result = parse_xingyun_demand_url(
            "  http://xingyun.jd.com/demands/view/CODE1/-1?demandId=100  ",
        )
        .unwrap();
        assert_eq!(result.demand_id, 100);
    }

    #[test]
    fn builds_view_url_from_code() {
        assert_eq!(
            xingyun_demand_view_url(4840029, Some("JL3R4IV4")),
            "http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029"
        );
    }

    #[test]
    fn builds_view_url_without_code() {
        assert_eq!(
            xingyun_demand_view_url(4840029, None),
            "http://xingyun.jd.com/demands/view/4840029/-1?demandId=4840029"
        );
        assert_eq!(
            xingyun_demand_view_url(4840029, Some("  ")),
            "http://xingyun.jd.com/demands/view/4840029/-1?demandId=4840029"
        );
    }

    #[test]
    fn feature_branch_from_code() {
        assert_eq!(
            feature_branch_name(Some("JL3R4IV4"), 4840029),
            "feature_JL3R4IV4"
        );
    }

    #[test]
    fn feature_branch_falls_back_to_id() {
        assert_eq!(feature_branch_name(None, 4840029), "feature_demand_4840029");
    }

    #[test]
    fn feature_branch_empty_code_falls_back() {
        assert_eq!(
            feature_branch_name(Some(""), 4840029),
            "feature_demand_4840029"
        );
    }

    #[test]
    fn feature_branch_replaces_illegal_chars() {
        assert_eq!(
            feature_branch_name(Some("CODE WITH SPACES"), 1),
            "feature_CODE_WITH_SPACES"
        );
    }

    #[test]
    fn slug_from_code() {
        assert_eq!(feature_slug(Some("JL3R4IV4"), 4840029), "feat-jl3r4iv4");
    }

    #[test]
    fn slug_falls_back_to_id() {
        assert_eq!(feature_slug(None, 4840029), "feat-demand-4840029");
    }
}
