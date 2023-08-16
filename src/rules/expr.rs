use crate::rules::lexer::{RuleToken, RuleTokenKind};
use crate::rules::object::{MemberKind, Object};
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
    pub fn eval<'a>(&self, scope: &'a RuleScope) -> Value {
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

fn eval_value<'a>(token: &RuleToken) -> Value {
    match &token.kind {
        RuleTokenKind::LitStr(s) => Value::String(s.clone()),
        RuleTokenKind::LitInt(s) => Value::Int(s.parse::<u32>().unwrap()),
        RuleTokenKind::Ident(s) => Value::Ident(s.clone()),
        _ => unreachable!(),
    }
}

fn eval_expr<'a>(expr: &Expr, scope: &'a RuleScope) -> Value {
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

fn eval_path_expr<'a>(target: Value, member: Value, scope: &'a RuleScope) -> Value {
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

fn eval_call_expr<'a>(target: Value, args: Value, scope: &'a RuleScope) -> Value {
    let Value::Ident(target) = target else {
        unreachable!()
    };

    println!("{:?}", target);

    let Value::Many(args) = args else {
        unreachable!()
    };

    for arg in args.iter() {
        match arg {
            Value::String(s) => println!("{s}"),
            Value::Int(_) => {}
            Value::Bool(_) => {}
            Value::Ident(_) => {}
            Value::Object(_) => {}
            Value::Callable(_) => {}
            Value::Many(_) => {}
        }
    }

    let func = scope.get_var(&target);

    match func {
        Some(Value::Callable(callable)) => {
            callable(args);
        }
        _ => {}
    }

    Value::Bool(true)
}

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}
