use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct MetricEntry {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub counters: HashMap<String, f64>,
    pub histograms: HashMap<String, Vec<f64>>,
    pub timestamp: String,
}

pub trait MetricsCollector: Send + Sync {
    fn increment_counter(&self, name: &str, labels: Option<&HashMap<String, String>>);
    fn record_histogram(&self, name: &str, value: f64, labels: Option<&HashMap<String, String>>);
    fn snapshot(&self) -> MetricsSnapshot;
}

pub struct InMemoryMetricsCollector {
    counters: Mutex<HashMap<String, f64>>,
    histograms: Mutex<HashMap<String, Vec<f64>>>,
}

impl InMemoryMetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
            histograms: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_key(name: &str, labels: Option<&HashMap<String, String>>) -> String {
    match labels {
        Some(m) if !m.is_empty() => {
            let mut pairs: Vec<_> = m.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            let label_str: Vec<_> = pairs
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            format!("{}{{{}}}", name, label_str.join(","))
        }
        _ => name.to_string(),
    }
}

impl MetricsCollector for InMemoryMetricsCollector {
    fn increment_counter(&self, name: &str, labels: Option<&HashMap<String, String>>) {
        let key = encode_key(name, labels);
        let mut counters = self.counters.lock().unwrap();
        *counters.entry(key).or_insert(0.0) += 1.0;
    }

    fn record_histogram(&self, name: &str, value: f64, labels: Option<&HashMap<String, String>>) {
        let key = encode_key(name, labels);
        let mut histograms = self.histograms.lock().unwrap();
        histograms.entry(key).or_default().push(value);
    }

    fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            counters: self.counters.lock().unwrap().clone(),
            histograms: self.histograms.lock().unwrap().clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let mc = InMemoryMetricsCollector::new();
        mc.increment_counter("requests", None);
        mc.increment_counter("requests", None);
        let snap = mc.snapshot();
        assert_eq!(snap.counters["requests"], 2.0);
    }

    #[test]
    fn test_counter_with_labels() {
        let mc = InMemoryMetricsCollector::new();
        let mut labels = HashMap::new();
        labels.insert("stage".into(), "dev".into());
        mc.increment_counter("stage_total", Some(&labels));
        let snap = mc.snapshot();
        assert_eq!(snap.counters["stage_total{stage=\"dev\"}"], 1.0);
    }

    #[test]
    fn test_histogram() {
        let mc = InMemoryMetricsCollector::new();
        mc.record_histogram("duration", 1.5, None);
        mc.record_histogram("duration", 2.0, None);
        let snap = mc.snapshot();
        assert_eq!(snap.histograms["duration"], vec![1.5, 2.0]);
    }
}
