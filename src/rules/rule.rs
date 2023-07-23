#[derive(Debug, PartialEq)]
pub enum RuleAction {
    SetHeader(String, String),
    CustomReturn(u16, String),
}

#[derive(Debug)]
pub struct Rule {
    pub pattern: String,
    pub actions: Vec<RuleAction>,
}

impl Rule {
    pub fn builder() -> RuleBuilder {
        RuleBuilder::new()
    }
}

pub struct RuleBuilder {
    rule: Rule,
}

impl RuleBuilder {
    fn new() -> Self {
        RuleBuilder {
            rule: Rule {
                pattern: String::new(),
                actions: vec![],
            },
        }
    }

    pub fn pattern(mut self, pattern: String) -> Self {
        self.rule.pattern = pattern;

        self
    }

    pub fn add_action(mut self, action: RuleAction) -> Self {
        self.rule.actions.push(action);

        self
    }

    pub fn get(self) -> Rule {
        self.rule
    }
}
