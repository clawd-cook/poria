use std::any::Any;
use std::collections::HashMap;

pub trait PluginLoader: Send + Sync {
    fn register(&self, id: &str, instance: Box<dyn Any + Send + Sync>);
    fn load(&self, id: &str) -> Result<&dyn Any, String>;
    fn has(&self, id: &str) -> bool;
}

pub struct InMemoryPluginLoader {
    plugins: std::sync::Mutex<HashMap<String, Box<dyn Any + Send + Sync>>>,
}

impl InMemoryPluginLoader {
    pub fn new() -> Self {
        Self { plugins: std::sync::Mutex::new(HashMap::new()) }
    }
}

impl Default for InMemoryPluginLoader {
    fn default() -> Self { Self::new() }
}

impl InMemoryPluginLoader {
    pub fn register_plugin(&self, id: &str, instance: Box<dyn Any + Send + Sync>) {
        self.plugins.lock().unwrap().insert(id.to_string(), instance);
    }

    pub fn has_plugin(&self, id: &str) -> bool {
        self.plugins.lock().unwrap().contains_key(id)
    }
}
