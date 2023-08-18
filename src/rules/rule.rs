use crate::request::Request;
use crate::response::Response;
use crate::response_status_code::ResponseStatusCode;
use crate::rules::callable::wrap_callable;
use crate::rules::grammar::{Lit, Statement, StatementKind};
use crate::rules::object::IntoObject;
use crate::rules::scope::RuleScope;
use crate::rules::value::Value;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum RuleAction {
    SetHeader(String, String),
    RedirectReturn(ResponseStatusCode, String),
    CustomReturn(ResponseStatusCode, Option<String>),
}

pub enum RuleEvaluationResult {
    Continue,
    Finish,
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

    pub fn evaluate(
        &self,
        request: Rc<RefCell<Request>>,
        response: Rc<RefCell<Response>>,
    ) -> RuleEvaluationResult {
        let mut scope = RuleScope::new();
        scope.update_var("request", Value::Object(request.clone().into_object()));
        scope.update_var(
            "log",
            Value::Callable(wrap_callable(|text: String| {
                info!("{}", text);
                Value::Bool(true)
            })),
        );
        scope.update_var("response", Value::Object(response.clone().into_object()));

        Self::evaluate_statements(&self.statements, request, response, &scope)
    }

    fn evaluate_statements(
        statements: &[Statement],
        request: Rc<RefCell<Request>>,
        response: Rc<RefCell<Response>>,
        scope: &RuleScope,
    ) -> RuleEvaluationResult {
        for statement in statements {
            let response = response.clone();

            match &statement.kind {
                StatementKind::Func(func_name, args) => {
                    if func_name == "set_header" {
                        match &args[..] {
                            [Lit::String(arg1), Lit::String(arg2)] => {
                                let mut out_response = response.borrow_mut();
                                out_response.set_header(arg1, arg2);
                            }
                            _ => panic!(),
                        }
                    }
                }
                StatementKind::Redirect(response_code, location) => {
                    let mut out_response = response.borrow_mut();
                    out_response.set_status_code(*response_code);
                    out_response.set_header("Location", location);

                    return RuleEvaluationResult::Finish;
                }
                StatementKind::Return(response_code, additional_data) => {
                    let mut out_response = response.borrow_mut();
                    out_response.set_status_code(*response_code);

                    if let Some(body) = additional_data {
                        let body_bytes = body.clone().into_bytes();
                        let body_len = body_bytes.len();

                        out_response.set_body(body_bytes);
                        out_response.set_header("Content-Length", &body_len.to_string());
                    }

                    return RuleEvaluationResult::Finish;
                }
                StatementKind::If(condition_expr, statements) => match condition_expr.eval(scope) {
                    Value::Bool(val) => {
                        if val {
                            match Self::evaluate_statements(
                                statements,
                                request.clone(),
                                response,
                                scope,
                            ) {
                                RuleEvaluationResult::Continue => {}
                                RuleEvaluationResult::Finish => {
                                    return RuleEvaluationResult::Finish
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                },
                StatementKind::Expr(expr) => {
                    expr.eval(scope);
                }
            }
        }

        RuleEvaluationResult::Continue
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
