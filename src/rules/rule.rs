use crate::request::Request;
use crate::response::Response;
use crate::response_status_code::ResponseStatusCode;
use crate::rules::grammar::{Lit, Statement, StatementKind};

#[derive(Debug, PartialEq)]
pub enum RuleAction {
    SetHeader(String, String),
    RedirectReturn(ResponseStatusCode, String),
    CustomReturn(ResponseStatusCode, Option<String>),
}

pub enum RuleEvaluationResult {
    Continue(Response),
    Finish(Response),
}

#[derive(Debug)]
pub struct Rule {
    pub pattern: String,
    pub actions: Vec<RuleAction>,
    pub statements: Vec<Statement>,
}

impl Rule {
    pub fn builder() -> RuleBuilder {
        RuleBuilder::new()
    }

    pub fn matches(&self, url: &str) -> bool {
        !url.matches(&self.pattern).collect::<Vec<&str>>().is_empty()
    }

    pub fn evaluate(&self, request: &Request, response: Response) -> RuleEvaluationResult {
        let mut out_response = response;

        for statement in self.statements.iter() {
            match &statement.kind {
                StatementKind::Func(func_name, args) => {
                    if func_name == "set_header" {
                        match &args[..] {
                            [Lit::String(arg1), Lit::String(arg2)] => {
                                out_response.set_header(arg1, arg2);
                            }
                            _ => panic!(),
                        }
                    }
                }
                StatementKind::ReturnRedirect(response_code, location) => {
                    out_response.set_status_code(*response_code);
                    out_response.set_header("Location", location);

                    return RuleEvaluationResult::Finish(out_response);
                }
                StatementKind::Return(response_code, additional_data) => {
                    out_response.set_status_code(*response_code);

                    if let Some(body) = additional_data {
                        out_response.set_body(body.clone().into_bytes());
                    }

                    return RuleEvaluationResult::Finish(out_response);
                }
            }
        }

        RuleEvaluationResult::Continue(out_response)
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
                statements: vec![],
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
