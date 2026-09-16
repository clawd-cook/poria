use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn priority(&self) -> u8 {
        match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        }
    }
}

pub trait Logger: Send + Sync {
    fn log(&self, level: LogLevel, message: &str, context: &HashMap<String, serde_json::Value>);

    fn debug(&self, msg: &str, ctx: Option<&HashMap<String, serde_json::Value>>) {
        self.log(LogLevel::Debug, msg, ctx.unwrap_or(&HashMap::new()));
    }
    fn info(&self, msg: &str, ctx: Option<&HashMap<String, serde_json::Value>>) {
        self.log(LogLevel::Info, msg, ctx.unwrap_or(&HashMap::new()));
    }
    fn warn(&self, msg: &str, ctx: Option<&HashMap<String, serde_json::Value>>) {
        self.log(LogLevel::Warn, msg, ctx.unwrap_or(&HashMap::new()));
    }
    fn error(&self, msg: &str, ctx: Option<&HashMap<String, serde_json::Value>>) {
        self.log(LogLevel::Error, msg, ctx.unwrap_or(&HashMap::new()));
    }
}

pub struct JsonLogger {
    min_level: LogLevel,
    default_context: HashMap<String, serde_json::Value>,
}

impl JsonLogger {
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level, default_context: HashMap::new() }
    }

    pub fn child(&self, extra_ctx: HashMap<String, serde_json::Value>) -> Self {
        let mut merged = self.default_context.clone();
        merged.extend(extra_ctx);
        Self { min_level: self.min_level, default_context: merged }
    }
}

impl Logger for JsonLogger {
    fn log(&self, level: LogLevel, message: &str, context: &HashMap<String, serde_json::Value>) {
        if level.priority() < self.min_level.priority() {
            return;
        }

        let mut entry = serde_json::Map::new();
        entry.insert("timestamp".into(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
        entry.insert("level".into(), serde_json::json!(level));
        entry.insert("message".into(), serde_json::json!(message));

        for (k, v) in &self.default_context {
            entry.insert(k.clone(), v.clone());
        }
        for (k, v) in context {
            entry.insert(k.clone(), v.clone());
        }

        let json = serde_json::Value::Object(entry);
        eprintln!("{}", json);
    }
}

pub fn create_logger(min_level: Option<LogLevel>) -> JsonLogger {
    JsonLogger::new(min_level.unwrap_or(LogLevel::Info))
}
