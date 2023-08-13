use crate::rules::lexer::{RuleToken, RuleTokenKind};

#[derive(Debug)]
pub enum Operator {
    And,
    Or,
    Eq,
    NotEq,
}

#[derive(Debug)]
pub enum ExprOrValue {
    Expr(Expr),
    Value(RuleToken),
}

impl ExprOrValue {
    pub fn eval(&self) -> Value {
        match self {
            ExprOrValue::Value(token) => eval_value(token),
            ExprOrValue::Expr(expr) => eval_expr(expr),
        }
    }
}

fn eval_value(token: &RuleToken) -> Value {
    match &token.kind {
        RuleTokenKind::LitStr(s) => Value::String(s.clone()),
        RuleTokenKind::LitInt(s) => Value::Int(s.parse::<u32>().unwrap()),
        RuleTokenKind::Ident(s) => todo!(),
        _ => unreachable!(),
    }
}

fn eval_expr(expr: &Expr) -> Value {
    let lhs_value = expr.lhs.eval();
    let rhs_value = expr.lhs.eval();

    match expr.operator {
        Operator::And => todo!(),
        Operator::Or => todo!(),
        Operator::Eq => Value::Bool(lhs_value.eq(&rhs_value)),
        Operator::NotEq => Value::Bool(lhs_value.ne(&rhs_value)),
    }
}

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}

pub enum Value {
    String(String),
    Int(u32),
    Bool(bool),
    Object,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(s1), Value::String(s2)) => s1.eq(s2),
            (Value::Int(i1), Value::Int(i2)) => i1.eq(i2),
            _ => todo!(),
            // Value::Int(_) => {}
            // Value::Bool(_) => {}
            // Value::Object => {}
        }
    }
}
