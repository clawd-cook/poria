use std::collections::VecDeque;

/// In-process serial scheduler for silent pipeline execution.
///
/// Only one pipeline auto-runs at a time. Additional submissions wait in FIFO order.
#[derive(Debug, Default)]
pub struct AutoRunScheduler {
    current: Option<String>,
    queue: VecDeque<String>,
}

impl AutoRunScheduler {
    /// Queue `pipeline_id` for auto-run.
    ///
    /// Returns `Some(id)` when this id should start immediately.
    pub fn submit(&mut self, pipeline_id: String) -> Option<String> {
        if pipeline_id.trim().is_empty() {
            return None;
        }
        if self.current.as_deref() == Some(pipeline_id.as_str()) {
            return None;
        }
        if self.queue.iter().any(|id| id == &pipeline_id) {
            return None;
        }
        if self.current.is_none() {
            self.current = Some(pipeline_id.clone());
            return Some(pipeline_id);
        }
        self.queue.push_back(pipeline_id);
        None
    }

    /// Mark `pipeline_id` as finished. Returns the next id to start, if any.
    pub fn finish(&mut self, pipeline_id: &str) -> Option<String> {
        if self.current.as_deref() != Some(pipeline_id) {
            self.queue.retain(|id| id != pipeline_id);
            return None;
        }
        self.current = self.queue.pop_front();
        self.current.clone()
    }

    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Drop a waiting pipeline. Does not interrupt the in-flight run.
    pub fn drop_queued(&mut self, pipeline_id: &str) {
        self.queue.retain(|id| id != pipeline_id);
    }
}

#[cfg(test)]
mod tests {
    use super::AutoRunScheduler;

    #[test]
    fn first_submit_starts_immediately() {
        let mut scheduler = AutoRunScheduler::default();
        assert_eq!(scheduler.submit("p1".into()), Some("p1".into()));
        assert_eq!(scheduler.current(), Some("p1"));
    }

    #[test]
    fn second_submit_queues_until_finish() {
        let mut scheduler = AutoRunScheduler::default();
        scheduler.submit("p1".into());
        assert_eq!(scheduler.submit("p2".into()), None);
        assert_eq!(scheduler.submit("p2".into()), None);
        assert_eq!(scheduler.finish("p1"), Some("p2".into()));
        assert_eq!(scheduler.current(), Some("p2"));
        assert_eq!(scheduler.finish("p2"), None);
        assert_eq!(scheduler.current(), None);
    }

    #[test]
    fn submit_current_id_is_ignored() {
        let mut scheduler = AutoRunScheduler::default();
        scheduler.submit("p1".into());
        assert_eq!(scheduler.submit("p1".into()), None);
        assert_eq!(scheduler.finish("p1"), None);
    }

    #[test]
    fn finish_other_id_does_not_advance() {
        let mut scheduler = AutoRunScheduler::default();
        scheduler.submit("p1".into());
        scheduler.submit("p2".into());
        assert_eq!(scheduler.finish("p2"), None);
        assert_eq!(scheduler.current(), Some("p1"));
        assert_eq!(scheduler.finish("p1"), None);
    }

    #[test]
    fn empty_id_is_ignored() {
        let mut scheduler = AutoRunScheduler::default();
        assert_eq!(scheduler.submit("  ".into()), None);
        assert_eq!(scheduler.submit("".into()), None);
    }

    #[test]
    fn drop_queued_does_not_touch_current() {
        let mut scheduler = AutoRunScheduler::default();
        scheduler.submit("p1".into());
        scheduler.submit("p2".into());
        scheduler.drop_queued("p1");
        scheduler.drop_queued("p2");
        assert_eq!(scheduler.current(), Some("p1"));
        assert_eq!(scheduler.finish("p1"), None);
    }
}
