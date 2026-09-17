/// Xingyun demand status labels, aligned with h2o-plugin `DEMAND_STATUS_LABEL`.
pub const DEMAND_STATUS_LABELS: &[(i32, &str)] = &[
    (1, "草稿"),
    (2, "待沟通"),
    (3, "已受理"),
    (4, "已驳回"),
    (5, "处理中（直接处理）"),
    (6, "验证中"),
    (7, "验证成功"),
    (8, "验证失败"),
    (9, "待验收"),
    (10, "验收失败"),
    (11, "拆分"),
    (12, "处理中（处理到团队空间）"),
    (13, "待受理"),
    (18, "需求审批中"),
    (19, "验收审批中"),
    (20, "完成"),
    (21, "已取消"),
    (22, "删除"),
];

/// Workbench "in progress" statuses used by h2o-plugin `demands.list`.
pub const VISIBLE_DEMAND_STATUSES: &[i32] = &[2, 13, 3, 5, 6, 7, 8, 9, 10, 11, 12];

pub fn format_demand_status(status: Option<i32>) -> String {
    match status {
        Some(code) => DEMAND_STATUS_LABELS
            .iter()
            .find(|(value, _)| *value == code)
            .map(|(_, label)| (*label).to_string())
            .unwrap_or_else(|| format!("状态 {code}")),
        None => String::new(),
    }
}

pub fn is_visible_demand_status(status: Option<i32>) -> bool {
    status
        .map(|code| VISIBLE_DEMAND_STATUSES.contains(&code))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_demand_status_known_label() {
        assert_eq!(format_demand_status(Some(13)), "待受理");
        assert_eq!(format_demand_status(Some(2)), "待沟通");
    }

    #[test]
    fn list_demand_status_unknown_falls_back() {
        assert_eq!(format_demand_status(Some(99)), "状态 99");
        assert_eq!(format_demand_status(None), "");
    }

    #[test]
    fn list_demand_visible_status_matches_workbench() {
        assert!(is_visible_demand_status(Some(3)));
        assert!(!is_visible_demand_status(Some(20)));
        assert!(!is_visible_demand_status(None));
    }
}
