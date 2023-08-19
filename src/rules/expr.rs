use crate::rules::lexer::{Position, RuleToken, RuleTokenKind};
use crate::rules::object::MemberKind;
use crate::rules::scope::RuleScope;
use crate::rules::value::{Type, Value};

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

                let position = val_args.first().map_or(Position::zero(), |v| *v.position());

                // todo: better position
                Value::new(Type::Many(val_args), position)
            }
        }
    }
}

fn eval_value(token: &RuleToken) -> Value {
    let t = match &token.kind {
        RuleTokenKind::LitStr(s) => Type::String(s.clone()),
        RuleTokenKind::LitInt(s) => Type::Int(s.parse::<u32>().unwrap()),
        RuleTokenKind::Ident(s) => Type::Ident(s.clone()),
        _ => unreachable!(),
    };

    Value::new(t, token.position)
}

fn eval_expr(expr: &Expr, scope: &RuleScope) -> Value {
    let lhs_value = expr.lhs.eval(scope);
    let rhs_value = expr.rhs.eval(scope);

    let t = match expr.operator {
        Operator::And => todo!(),
        Operator::Or => todo!(),
        Operator::Eq => Type::Bool(lhs_value.eq(&rhs_value)),
        Operator::NotEq => Type::Bool(lhs_value.ne(&rhs_value)),
        Operator::Dot => return eval_path_expr(lhs_value, rhs_value, scope),
        Operator::Call => return eval_call_expr(lhs_value, rhs_value, scope),
    };

    // todo: better position
    Value::new(t, *lhs_value.position())
}

fn eval_path_expr(target_val: Value, member_val: Value, scope: &RuleScope) -> Value {
    let (Type::Ident(target), Type::Ident(member)) = (target_val.t(), member_val.t()) else {
        // guaranteed by parser
        unreachable!()
    };

    let var = scope.get_var(target);

    let t = match var {
        Some(Type::Object(obj)) => {
            let Some(member) = obj.get_member(member) else {
                todo!()
            };

            match member.kind {
                MemberKind::Field => member.eval(vec![Value::new(
                    var.unwrap().clone(),
                    *target_val.position(),
                )]),
                MemberKind::Method => Type::Method(obj.clone(), member.callable.clone()),
            }
        }
        _ => todo!(),
    };

    // todo: better position
    Value::new(t, *target_val.position())
}

fn eval_call_expr(target: Value, args_val: Value, scope: &RuleScope) -> Value {
    let Type::Many(mut args) = args_val.take_t() else {
        // guaranteed by parser todo: verify if true
        unreachable!()
    };

    let func = match target.t() {
        Type::Ident(target) => scope.get_var(target),
        Type::Method(..) | Type::Function(..) => Some(target.t()),
        _ => todo!(),
    };

    match func {
        Some(Type::Function(callable)) => {
            callable(args);
        }
        Some(Type::Method(obj, callable)) => {
            args.insert(0, Value::new(Type::Object(obj.clone()), *target.position()));
            callable(args);
        }
        _ => todo!(),
    }

    // todo: better position
    Value::new(Type::Bool(true), *target.position())
}

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}
