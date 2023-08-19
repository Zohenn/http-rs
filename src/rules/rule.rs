use crate::request::Request;
use crate::response::Response;
use crate::rules::callable::wrap_callable;
use crate::rules::error::RuleError;
use crate::rules::grammar::{Statement, StatementKind};
use crate::rules::object::IntoObject;
use crate::rules::scope::RuleScope;
use crate::rules::value::Type;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;

type Result<T> = std::result::Result<T, RuleError>;

pub enum RuleEvaluationResult {
    Continue,
    Finish,
}

#[derive(Debug)]
pub struct Rule {
    pub pattern: String,
    pub statements: Vec<Statement>,
}

impl Rule {
    pub fn matches(&self, url: &str) -> bool {
        !url.matches(&self.pattern).collect::<Vec<&str>>().is_empty()
    }

    pub fn evaluate(
        &self,
        request: Rc<RefCell<Request>>,
        response: Rc<RefCell<Response>>,
    ) -> Result<RuleEvaluationResult> {
        let mut scope = RuleScope::new();
        scope.update_var("request", Type::Object(request.clone().into_object()));
        scope.update_var(
            "log",
            Type::Function(wrap_callable(|text: String| {
                info!("{}", text);
                Type::Bool(true)
            })),
        );
        scope.update_var("response", Type::Object(response.clone().into_object()));

        Self::evaluate_statements(&self.statements, request, response, &scope)
    }

    fn evaluate_statements(
        statements: &[Statement],
        request: Rc<RefCell<Request>>,
        response: Rc<RefCell<Response>>,
        scope: &RuleScope,
    ) -> Result<RuleEvaluationResult> {
        for statement in statements {
            let response = response.clone();

            match &statement.kind {
                StatementKind::Redirect(response_code, location) => {
                    let mut out_response = response.borrow_mut();
                    out_response.set_status_code(*response_code);
                    out_response.set_header("Location", location);

                    return Ok(RuleEvaluationResult::Finish);
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

                    return Ok(RuleEvaluationResult::Finish);
                }
                StatementKind::If(condition_expr, statements) => {
                    let expr_value = condition_expr.eval(scope)?;
                    match expr_value.t() {
                        Type::Bool(val) => {
                            if *val {
                                match Self::evaluate_statements(
                                    statements,
                                    request.clone(),
                                    response,
                                    scope,
                                )? {
                                    RuleEvaluationResult::Continue => {}
                                    RuleEvaluationResult::Finish => {
                                        return Ok(RuleEvaluationResult::Finish)
                                    }
                                }
                            }
                        }
                        _ => todo!(),
                    }
                }
                StatementKind::Expr(expr) => {
                    expr.eval(scope)?;
                }
            }
        }

        Ok(RuleEvaluationResult::Continue)
    }
}
