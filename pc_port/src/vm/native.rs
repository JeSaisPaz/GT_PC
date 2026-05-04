use crate::vm::value::*;
use std::rc::Rc;

pub type NativeFn = Rc<dyn Fn(&[Value]) -> Value>;

/// Registry of native functions callable from Adhoc scripts.
/// Keyed by module path (e.g. "main,pdistd,MRandom,GetValue").
pub struct NativeRegistry {
    functions: std::collections::HashMap<String, NativeFn>,
    fallbacks: Vec<(String, NativeFn)>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        NativeRegistry { functions: std::collections::HashMap::new(), fallbacks: Vec::new() }
    }

    pub fn register(&mut self, path: &str, func: NativeFn) {
        self.functions.insert(path.to_string(), func);
    }

    pub fn register_fallback(&mut self, prefix: &str, func: NativeFn) {
        self.fallbacks.push((prefix.to_string(), func));
    }

    pub fn get(&self, path: &str) -> Option<NativeFn> {
        if let Some(f) = self.functions.get(path) {
            return Some(f.clone());
        }
        for (prefix, func) in &self.fallbacks {
            if path.starts_with(prefix) {
                return Some(func.clone());
            }
        }
        None
    }

    pub fn has(&self, path: &str) -> bool {
        self.functions.contains_key(path)
            || self.fallbacks.iter().any(|(p, _)| path.starts_with(p))
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }

    pub fn iter_paths(&self) -> impl Iterator<Item = &String> {
        self.functions.keys()
    }

    pub fn list(&self) {
        let mut paths: Vec<&String> = self.functions.keys().collect();
        paths.sort();
        println!("Registered native functions ({} + {} fallback patterns):", paths.len(), self.fallbacks.len());
        for p in paths {
            println!("  {}", p);
        }
    }
}
