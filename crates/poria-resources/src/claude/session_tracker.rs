use std::collections::HashMap;
use std::sync::Mutex;

/// Session Tracker -- maps (pipeline_id, stage_name) to Agent session IDs.
///
/// In-memory HashMap behind a Mutex. The actual persistence is handled by
/// the stages table `agent_session_id` column in infra-persistence.
/// This tracker serves as the calling layer's thin wrapper so that the
/// agent pool and pipeline executor have a consistent API.
pub struct SessionTracker {
    sessions: Mutex<HashMap<String, String>>,
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Record an Agent session ID for a pipeline stage.
    pub fn record(&self, pipeline_id: &str, stage_name: &str, session_id: String) {
        let key = Self::make_key(pipeline_id, stage_name);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(key, session_id);
    }

    /// Retrieve a previously recorded session ID, or None if not found.
    pub fn get(&self, pipeline_id: &str, stage_name: &str) -> Option<String> {
        let key = Self::make_key(pipeline_id, stage_name);
        let sessions = self.sessions.lock().unwrap();
        sessions.get(&key).cloned()
    }

    /// Remove a session record. Returns true if the entry existed.
    pub fn remove(&self, pipeline_id: &str, stage_name: &str) -> bool {
        let key = Self::make_key(pipeline_id, stage_name);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(&key).is_some()
    }

    /// Clear all tracked sessions.
    pub fn clear(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.clear();
    }

    /// Number of tracked sessions.
    pub fn size(&self) -> usize {
        let sessions = self.sessions.lock().unwrap();
        sessions.len()
    }

    fn make_key(pipeline_id: &str, stage_name: &str) -> String {
        format!("{}::{}", pipeline_id, stage_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_get() {
        let tracker = SessionTracker::new();
        tracker.record("pipeline-1", "dev", "session-abc".into());

        assert_eq!(tracker.get("pipeline-1", "dev"), Some("session-abc".into()));
    }

    #[test]
    fn test_get_missing() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.get("pipeline-1", "dev"), None);
    }

    #[test]
    fn test_remove() {
        let tracker = SessionTracker::new();
        tracker.record("p1", "cr", "s1".into());
        assert!(tracker.remove("p1", "cr"));
        assert!(!tracker.remove("p1", "cr")); // already removed
        assert_eq!(tracker.get("p1", "cr"), None);
    }

    #[test]
    fn test_clear() {
        let tracker = SessionTracker::new();
        tracker.record("p1", "dev", "s1".into());
        tracker.record("p2", "cr", "s2".into());
        assert_eq!(tracker.size(), 2);

        tracker.clear();
        assert_eq!(tracker.size(), 0);
    }

    #[test]
    fn test_size() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.size(), 0);
        tracker.record("p1", "dev", "s1".into());
        assert_eq!(tracker.size(), 1);
        tracker.record("p1", "cr", "s2".into());
        assert_eq!(tracker.size(), 2);
        // Overwrite existing
        tracker.record("p1", "dev", "s3".into());
        assert_eq!(tracker.size(), 2); // same key, no new entry
    }

    #[test]
    fn test_overwrite() {
        let tracker = SessionTracker::new();
        tracker.record("p1", "dev", "old-session".into());
        tracker.record("p1", "dev", "new-session".into());
        assert_eq!(tracker.get("p1", "dev"), Some("new-session".into()));
    }
}
