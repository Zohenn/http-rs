use crate::rules::lexer::{RuleToken, RuleTokenKind};
use crate::rules::object::MemberKind;
use crate::rules::scope::RuleScope;
use crate::rules::value::Value;

#[derive(Debug)]
pub enum Operator {
    And,
    Or,
    Eq,
    NotEq,
    Dot,
    Call,
}

#[derive(Debug)]
pub enum ExprOrValue {
    Expr(Expr),
    Value(RuleToken),
    Many(Vec<ExprOrValue>),
}

impl ExprOrValue {
    pub fn eval(&self, scope: &RuleScope) -> Value {
        match self {
            ExprOrValue::Value(token) => eval_value(token),
            ExprOrValue::Expr(expr) => eval_expr(expr, scope),
            ExprOrValue::Many(args) => {
                let mut val_args: Vec<Value> = vec![];

                for arg in args {
                    val_args.push(arg.eval(scope));
                }

                Value::Many(val_args)
            }
        }
    }
}

fn eval_value(token: &RuleToken) -> Value {
    match &token.kind {
        RuleTokenKind::LitStr(s) => Value::String(s.clone()),
        RuleTokenKind::LitInt(s) => Value::Int(s.parse::<u32>().unwrap()),
        RuleTokenKind::Ident(s) => Value::Ident(s.clone()),
        _ => unreachable!(),
    }
}

fn eval_expr(expr: &Expr, scope: &RuleScope) -> Value {
    let lhs_value = expr.lhs.eval(scope);
    let rhs_value = expr.rhs.eval(scope);

    match expr.operator {
        Operator::And => todo!(),
        Operator::Or => todo!(),
        Operator::Eq => Value::Bool(lhs_value.eq(&rhs_value)),
        Operator::NotEq => Value::Bool(lhs_value.ne(&rhs_value)),
        Operator::Dot => eval_path_expr(lhs_value, rhs_value, scope),
        Operator::Call => eval_call_expr(lhs_value, rhs_value, scope),
    }
}

fn eval_path_expr(target: Value, member: Value, scope: &RuleScope) -> Value {
    let (Value::Ident(target), Value::Ident(member)) = (target, member) else {
        // guaranteed by parser
        unreachable!()
    };

    let var = scope.get_var(&target);

    match var {
        Some(Value::Object(obj)) => {
            let Some(member) = obj.get_member(&member) else {
                todo!()
            };

            match member.kind {
                MemberKind::Field => member.eval(vec![var.unwrap().clone()]),
                MemberKind::Method => Value::Method(obj.clone(), member.callable.clone()),
            }
        }
        _ => todo!(),
    }
}

fn eval_call_expr(target: Value, args: Value, scope: &RuleScope) -> Value {
    let Value::Many(mut args) = args else {
        unreachable!()
    };

    let func = match &target {
        Value::Ident(target) => scope.get_var(target),
        Value::Method(..) | Value::Function(..) => Some(&target),
        _ => todo!(),
    };

    match func {
        Some(Value::Function(callable)) => {
            callable(args);
        }
        Some(Value::Method(obj, callable)) => {
            args.insert(0, Value::Object(obj.clone()));
            callable(args);
        }
        _ => todo!(),
    }

    Value::Bool(true)
}

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}
