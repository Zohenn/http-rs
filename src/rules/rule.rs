use crate::request::Request;
use crate::response::Response;
use crate::response_status_code::ResponseStatusCode;
use crate::rules::expr::Value;
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
        Self::evaluate_statements(&self.statements, request, response)
    }

    fn evaluate_statements(
        statements: &[Statement],
        request: &Request,
        response: Response,
    ) -> RuleEvaluationResult {
        let mut out_response = response;

        for statement in statements {
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
                StatementKind::Redirect(response_code, location) => {
                    out_response.set_status_code(*response_code);
                    out_response.set_header("Location", location);

                    return RuleEvaluationResult::Finish(out_response);
                }
                StatementKind::Return(response_code, additional_data) => {
                    out_response.set_status_code(*response_code);

                    if let Some(body) = additional_data {
                        let body_bytes = body.clone().into_bytes();
                        let body_len = body_bytes.len();

                        out_response.set_body(body_bytes);
                        out_response.set_header("Content-Length", &body_len.to_string());
                    }

                    return RuleEvaluationResult::Finish(out_response);
                }
                StatementKind::If(condition_expr, statements) => match condition_expr.eval() {
                    Value::Bool(val) => {
                        if val {
                            match Self::evaluate_statements(statements, request, out_response) {
                                RuleEvaluationResult::Continue(res) => out_response = res,
                                RuleEvaluationResult::Finish(res) => {
                                    return RuleEvaluationResult::Finish(res)
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                },
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
