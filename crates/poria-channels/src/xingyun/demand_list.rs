use super::demand_status::{
    format_demand_status, is_visible_demand_status, VISIBLE_DEMAND_STATUSES,
};
use super::types::{DemandListItem, DemandListQuery, DemandPage};

/// Build the JACP `/openapi/v3/demands/query` body.
///
/// Default (related-to-me): omit `receiver` and never send `processor`.
/// `accepted_by_me` adds `receiver` = current ERP (由我受理). Do not stamp
/// that ERP onto list rows unless this flag is on.
pub(crate) fn demand_list_query_body(query: &DemandListQuery, receiver: &str) -> serde_json::Value {
    let mut body = serde_json::json!({
        "current": query.current,
        "pageSize": query.page_size,
        "status": VISIBLE_DEMAND_STATUSES,
    });
    if query.accepted_by_me {
        body["receiver"] = serde_json::Value::String(receiver.to_string());
    }
    if let Some(keyword) = &query.keyword {
        body["keyword"] = serde_json::Value::String(keyword.clone());
    }
    body
}

pub(crate) fn parse_demand_page(
    data: serde_json::Value,
    requested_current: i64,
    requested_page_size: i64,
    fallback_receiver: Option<&str>,
) -> DemandPage {
    let obj = data.as_object();
    let raw_records = obj
        .and_then(|o| o.get("records"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let records: Vec<DemandListItem> = raw_records
        .into_iter()
        .filter_map(|value| parse_demand_list_item(value, fallback_receiver))
        .collect();
    let total = json_i64(obj.and_then(|o| o.get("total")), records.len() as i64);

    DemandPage {
        current: json_i64(obj.and_then(|o| o.get("current")), requested_current),
        page_size: json_i64(
            obj.and_then(|o| o.get("pageSize").or_else(|| o.get("page_size"))),
            requested_page_size,
        ),
        records,
        total,
    }
}

fn parse_demand_list_item(
    value: serde_json::Value,
    fallback_receiver: Option<&str>,
) -> Option<DemandListItem> {
    let obj = value.as_object()?;
    let id = json_i64(obj.get("id"), 0);
    if id <= 0 {
        return None;
    }

    let status = obj.get("status").and_then(json_optional_i32);
    if !is_visible_demand_status(status) {
        return None;
    }

    let demand_code = obj
        .get("demandCode")
        .or_else(|| obj.get("demand_code"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let name = if !name.is_empty() {
        name
    } else if !demand_code.is_empty() {
        demand_code.clone()
    } else {
        format!("#{id}")
    };

    let (receiver_erp, receiver_name) = receiver_from_record(obj, fallback_receiver);

    Some(DemandListItem {
        demand_code,
        id,
        name,
        receiver_erp,
        receiver_name,
        status,
        status_label: format_demand_status(status),
    })
}

fn receiver_from_record(
    obj: &serde_json::Map<String, serde_json::Value>,
    fallback_receiver: Option<&str>,
) -> (Option<String>, Option<String>) {
    if let Some(recv) = obj.get("receiver").and_then(|v| v.as_object()) {
        let erp = recv
            .get("erp")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let name = recv
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        if erp.is_some() || name.is_some() {
            return (erp, name);
        }
    }

    let fallback = fallback_receiver
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match fallback {
        Some(erp) => (Some(erp.to_string()), None),
        None => (None, None),
    }
}

fn json_i64(value: Option<&serde_json::Value>, fallback: i64) -> i64 {
    value.and_then(parse_i64).unwrap_or(fallback)
}

fn json_optional_i32(value: &serde_json::Value) -> Option<i32> {
    if value.is_null() {
        return None;
    }
    parse_i64(value).map(|n| n as i32)
}

fn parse_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().map(|n| n as i64))
        .or_else(|| value.as_f64().map(|n| n as i64))
        .or_else(|| value.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_demand_query_body_omits_receiver_by_default() {
        let query = DemandListQuery {
            current: 2,
            page_size: 10,
            ..Default::default()
        };
        let body = demand_list_query_body(&query, "heyongqi10");
        assert_eq!(body["current"], 2);
        assert_eq!(body["pageSize"], 10);
        assert!(body.get("keyword").is_none());
        assert_related_to_me_query_fields(&body);
        assert_eq!(body["status"], serde_json::json!(VISIBLE_DEMAND_STATUSES));
    }

    #[test]
    fn list_demand_query_body_includes_receiver_when_accepted_by_me() {
        let query = DemandListQuery {
            accepted_by_me: true,
            current: 2,
            page_size: 10,
            ..Default::default()
        };
        let body = demand_list_query_body(&query, "heyongqi10");
        assert_accepted_by_me_query_fields(&body, "heyongqi10");
        assert_eq!(body["status"], serde_json::json!(VISIBLE_DEMAND_STATUSES));
    }

    #[test]
    fn list_demand_query_body_includes_keyword() {
        let query = DemandListQuery {
            current: 1,
            keyword: Some("门店".into()),
            page_size: 20,
            ..Default::default()
        };
        let body = demand_list_query_body(&query, "erp1");
        assert_eq!(body["keyword"], "门店");
        assert_related_to_me_query_fields(&body);
    }

    #[test]
    fn list_demand_query_body_keeps_keyword_when_accepted_by_me() {
        let query = DemandListQuery {
            accepted_by_me: true,
            current: 1,
            keyword: Some("门店".into()),
            page_size: 20,
        };
        let body = demand_list_query_body(&query, "erp1");
        assert_eq!(body["keyword"], "门店");
        assert_accepted_by_me_query_fields(&body, "erp1");
    }

    #[test]
    fn list_demand_query_normalize_defaults() {
        let query = DemandListQuery {
            current: 0,
            keyword: Some("  ".into()),
            page_size: -1,
            ..Default::default()
        }
        .normalized();
        assert_eq!(query.current, 1);
        assert_eq!(query.page_size, 20);
        assert_eq!(query.keyword, None);
        assert!(!query.accepted_by_me);
    }

    #[test]
    fn parse_list_demand_page_records() {
        let data = serde_json::json!({
            "current": 1,
            "pageSize": 20,
            "total": 1,
            "records": [{
                "id": 4840029,
                "demandCode": "JL3R4IV4",
                "name": "门店结算",
                "status": 13,
                "receiver": { "erp": "zhangsan", "name": "张三" }
            }]
        });
        let page = parse_demand_page(data, 1, 20, None);
        assert_eq!(page.total, 1);
        assert_eq!(page.current, 1);
        assert_eq!(page.page_size, 20);
        assert_eq!(page.records.len(), 1);
        assert_eq!(page.records[0].id, 4840029);
        assert_eq!(page.records[0].demand_code, "JL3R4IV4");
        assert_eq!(page.records[0].name, "门店结算");
        assert_eq!(page.records[0].status, Some(13));
        assert_eq!(page.records[0].status_label, "待受理");
        assert_eq!(page.records[0].receiver_erp.as_deref(), Some("zhangsan"));
        assert_eq!(page.records[0].receiver_name.as_deref(), Some("张三"));
    }

    #[test]
    fn parse_list_demand_page_skips_invalid_and_completed_records() {
        let data = serde_json::json!({
            "records": [
                { "id": "bad" },
                { "id": 1, "name": "done", "status": 20 },
                { "id": 2, "demandCode": "ABC", "status": 3 }
            ],
            "total": 3
        });
        let page = parse_demand_page(data, 1, 20, None);
        assert_eq!(page.records.len(), 1);
        assert_eq!(page.records[0].id, 2);
        assert_eq!(page.records[0].name, "ABC");
        assert_eq!(page.total, 3);
    }

    #[test]
    fn parse_list_demand_page_omits_receiver_without_fallback() {
        let data = serde_json::json!({
            "records": [{
                "id": 9,
                "name": "无接收人",
                "status": 5
            }]
        });
        let page = parse_demand_page(data, 1, 20, None);
        assert_eq!(page.records[0].receiver_erp, None);
        assert_eq!(page.records[0].receiver_name, None);
    }

    #[test]
    fn parse_list_demand_page_falls_back_to_query_receiver() {
        let data = serde_json::json!({
            "records": [{
                "id": 9,
                "name": "无接收人",
                "status": 5
            }]
        });
        let page = parse_demand_page(data, 1, 20, Some("heyongqi10"));
        assert_eq!(page.records[0].receiver_erp.as_deref(), Some("heyongqi10"));
        assert_eq!(page.records[0].receiver_name, None);
        assert_eq!(page.records[0].status_label, "处理中（直接处理）");
    }

    #[test]
    fn parse_list_demand_page_accepts_string_id() {
        let data = serde_json::json!({
            "records": [{
                "id": "42",
                "demandCode": "CODE1",
                "name": "字符串 ID",
                "status": "13"
            }]
        });
        let page = parse_demand_page(data, 1, 20, None);
        assert_eq!(page.records.len(), 1);
        assert_eq!(page.records[0].id, 42);
        assert_eq!(page.records[0].status, Some(13));
    }

    fn assert_related_to_me_query_fields(body: &serde_json::Value) {
        assert!(body.get("receiver").is_none());
        assert!(body.get("processor").is_none());
        assert!(body.get("proposer").is_none());
    }

    fn assert_accepted_by_me_query_fields(body: &serde_json::Value, receiver: &str) {
        assert_eq!(body["receiver"], receiver);
        assert!(body.get("processor").is_none());
        assert!(body.get("proposer").is_none());
    }
}
