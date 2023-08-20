use crate::rules::error::{RuleError, RuntimeErrorKind};
use crate::rules::lexer::{Position, RuleToken, RuleTokenKind};
use crate::rules::object::MemberKind;
use crate::rules::scope::RuleScope;
use crate::rules::value::{Type, Value};

type Result<T> = std::result::Result<T, RuleError>;

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
    pub fn eval(&self, scope: &RuleScope) -> Result<Value> {
        match self {
            ExprOrValue::Value(token) => eval_value(token),
            ExprOrValue::Expr(expr) => eval_expr(expr, scope),
            ExprOrValue::Many(args) => {
                let mut val_args: Vec<Value> = vec![];

                for arg in args {
                    val_args.push(arg.eval(scope)?);
                }

                let position = val_args.first().map_or(Position::zero(), |v| *v.position());

                // todo: better position
                Ok(Value::new(Type::List(val_args), position))
            }
        }
    }
}

fn eval_value(token: &RuleToken) -> Result<Value> {
    let t = match &token.kind {
        RuleTokenKind::LitStr(s) => Type::String(s.clone()),
        RuleTokenKind::LitInt(s) => Type::Int(s.parse::<u32>().unwrap()),
        RuleTokenKind::Ident(s) => Type::Ident(s.clone()),
        _ => unreachable!(),
    };

    Ok(Value::new(t, token.position))
}

fn eval_expr(expr: &Expr, scope: &RuleScope) -> Result<Value> {
    let lhs_value = expr.lhs.eval(scope)?;
    let rhs_value = expr.rhs.eval(scope)?;

    let t = match expr.operator {
        Operator::And | Operator::Or => return eval_bool_expr(&lhs_value, &expr.operator, &rhs_value),
        Operator::Eq => Type::Bool(lhs_value.eq(&rhs_value)),
        Operator::NotEq => Type::Bool(lhs_value.ne(&rhs_value)),
        Operator::Dot => return eval_path_expr(lhs_value, rhs_value, scope),
        Operator::Call => return eval_call_expr(lhs_value, rhs_value, scope),
    };

    // todo: better position
    Ok(Value::new(t, *lhs_value.position()))
}

fn eval_bool_expr(lhs_value: &Value, operator: &Operator, rhs_value: &Value) -> Result<Value> {
    let mut values = [false; 2];

    for (index, value) in [lhs_value, rhs_value].iter().enumerate() {
        let Type::Bool(v) = value.t() else {
            return Err(
                RuleError::runtime(
                    RuntimeErrorKind::IncorrectType("bool".to_owned(), value.t().type_string()),
                    *value.position(),
                )
            );
        };

        values[index] = *v;
    }

    let expr_value = match operator {
        Operator::And => values[0] && values[1],
        Operator::Or => values[0] || values[1],
        _ => {
            // guaranteed by caller
            unreachable!()
        },
    };

    // todo: better position
    Ok(Value::new(Type::Bool(expr_value), *lhs_value.position()))
}

fn eval_path_expr(target_val: Value, member_val: Value, scope: &RuleScope) -> Result<Value> {
    let (Type::Ident(target), Type::Ident(member)) = (target_val.t(), member_val.t()) else {
        // guaranteed by parser
        unreachable!()
    };

    let var = scope.get_var(target);

    let t = match var {
        Some(Type::Object(obj)) => {
            let Some(member) = obj.get_member(member) else {
                return Err(RuleError::runtime(RuntimeErrorKind::MemberNotDefined(member.to_owned(), target.to_owned()), *member_val.position()));
            };

            match member.kind {
                MemberKind::Field => member.eval(vec![Value::new(
                    var.unwrap().clone(),
                    *target_val.position(),
                )]),
                MemberKind::Method => Type::Method(obj.clone(), member.callable.clone()),
            }
        }
        Some(t) => {
            return Err(RuleError::runtime(
                RuntimeErrorKind::IncorrectType("object".to_owned(), t.type_string()),
                *target_val.position(),
            ));
        }
        None => {
            return Err(RuleError::runtime(
                RuntimeErrorKind::UnresolvedReference(target.to_owned()),
                *target_val.position(),
            ));
        }
    };

    // todo: better position
    Ok(Value::new(t, *target_val.position()))
}

fn eval_call_expr(target_val: Value, args_val: Value, scope: &RuleScope) -> Result<Value> {
    let Type::List(mut args) = args_val.take_t() else {
        // guaranteed by parser
        unreachable!()
    };

    let func = {
        if let Type::Ident(target) = target_val.t() {
            scope.get_var(target).ok_or_else(|| {
                RuleError::runtime(
                    RuntimeErrorKind::UnresolvedReference(target.to_owned()),
                    *target_val.position(),
                )
            })?
        } else {
            target_val.t()
        }
    };

    match func {
        Type::Function(callable) => {
            callable(args);
        }
        Type::Method(obj, callable) => {
            args.insert(
                0,
                Value::new(Type::Object(obj.clone()), *target_val.position()),
            );
            callable(args);
        }
        _ => {
            return Err(RuleError::runtime(
                RuntimeErrorKind::IncorrectType(
                    "callable".to_string(),
                    target_val.t().type_string(),
                ),
                *target_val.position(),
            ));
        }
    }

    // todo: better position
    Ok(Value::new(Type::Bool(true), *target_val.position()))
}

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}
