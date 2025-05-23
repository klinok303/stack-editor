use crossterm::event::{
    KeyCode,
    KeyEvent, KeyModifiers,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct Bindings {
    binds: HashMap<(KeyCode, KeyModifiers), String>,
}

impl Bindings {
    pub fn insert(&mut self, action: (KeyCode, KeyModifiers), result: &str) {
        self.binds.insert(action, result.to_string());
    }

    pub fn event_check(&self, event: KeyEvent) -> Result<String, String> {
        let KeyEvent {
            code, modifiers, ..
        } = event;

        if !self.binds.contains_key(&(code, modifiers)) {
            return Err(format!("bind {:?} not found!", (code, modifiers)));
        };

        Ok(self.binds.get(&(code, modifiers)).unwrap().to_string())
    }
}
