use crate::rules::value::Value;
use std::collections::HashMap;

pub struct RuleScope {
    vars: HashMap<String, Value>,
    callables: HashMap<String, Value>,
}

impl RuleScope {
    pub fn new() -> Self {
        RuleScope {
            vars: HashMap::new(),
            callables: HashMap::new(),
        }
    }

    pub fn get_var(&self, ident: &str) -> Option<&Value> {
        self.vars.get(ident)
    }

    pub fn update_var(&mut self, ident: &str, value: Value) {
        self.vars.insert(ident.to_owned(), value);
    }
}
