use std::collections::HashMap;

use crate::value::Value;

#[derive(Debug, Clone, Default)]
pub struct Environment {
    frames: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.frames.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.frames.last_mut().unwrap().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for frame in self.frames.iter().rev() {
            if let Some(val) = frame.get(name) {
                return Some(val);
            }
        }
        None
    }

    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        for frame in self.frames.iter_mut().rev() {
            if frame.contains_key(name) {
                frame.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }
}

