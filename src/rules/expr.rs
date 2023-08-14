use crate::rules::lexer::{RuleToken, RuleTokenKind};
use crate::rules::object::{MemberKind, Object};
use crate::rules::scope::RuleScope;

#[derive(Debug)]
pub enum Operator {
    And,
    Or,
    Eq,
    NotEq,
    Dot,
}

#[derive(Debug)]
pub enum ExprOrValue {
    Expr(Expr),
    Value(RuleToken),
}

impl ExprOrValue {
    pub fn eval<'a>(&self, scope: &'a RuleScope) -> Value<'a> {
        match self {
            ExprOrValue::Value(token) => eval_value(token),
            ExprOrValue::Expr(expr) => eval_expr(expr, scope),
        }
    }
}

fn eval_value<'a>(token: &RuleToken) -> Value<'a> {
    match &token.kind {
        RuleTokenKind::LitStr(s) => Value::String(s.clone()),
        RuleTokenKind::LitInt(s) => Value::Int(s.parse::<u32>().unwrap()),
        RuleTokenKind::Ident(s) => Value::Ident(s.clone()),
        _ => unreachable!(),
    }
}

fn eval_expr<'a>(expr: &Expr, scope: &'a RuleScope) -> Value<'a> {
    let lhs_value = expr.lhs.eval(scope);
    let rhs_value = expr.rhs.eval(scope);

    match expr.operator {
        Operator::And => todo!(),
        Operator::Or => todo!(),
        Operator::Eq => Value::Bool(lhs_value.eq(&rhs_value)),
        Operator::NotEq => Value::Bool(lhs_value.ne(&rhs_value)),
        Operator::Dot => eval_path_expr(lhs_value, rhs_value, scope),
    }
}

fn eval_path_expr<'a>(target: Value, member: Value, scope: &'a RuleScope) -> Value<'a> {
    let (Value::Ident(target), Value::Ident(member)) = (target, member) else {
        // guaranteed by parser
        unreachable!()
    };

    let Some(Value::Object(object)) = scope.get_var(&target) else {
        todo!()
    };

    let Some(member) = object.get_field(&member) else {
        todo!()
    };

    let MemberKind::Field(val) = member.kind else {
        todo!()
    };

    val
}

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}

pub enum Value<'a> {
    String(String),
    Int(u32),
    Bool(bool),
    Ident(String),
    Object(Box<&'a dyn Object>),
}

impl<'a> PartialEq for Value<'a> {
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
