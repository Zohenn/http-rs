use crate::rules::lexer::{RuleToken, RuleTokenKind};
use crate::rules::object::{AsAny, Downcast, MemberKind, Object};
use crate::rules::object2::MemberKind2;
use crate::rules::scope::RuleScope;
use crate::rules::value::Value;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

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

    let a: Rc<dyn Any> = Rc::new(RefCell::new(Box::new(42i32)));
    let b = &a;

    println!(
        "{:?} teowitjow",
        b.as_ref().downcast_ref::<RefCell<Box<i32>>>()
    );

    let var = scope.get_var(&target);

    match var {
        Some(Value::Object2(obj)) => {
            let Some(member) = obj.get_member(&member) else {
                todo!()
            };

            match member.kind {
                MemberKind2::Field => member.eval(vec![var.unwrap().clone()]),
                MemberKind2::Method => Value::CallableMethod(obj.clone(), member.callable.clone()),
            }
        }
        // Some(Value::Object(obj_any)) => {
        //     let object = obj_any.downcast_ref::<Rc<dyn Object>>().unwrap();
        //     let Some(member) = object.get_member(&member) else {
        //         todo!()
        //     };
        //
        //     match member.kind {
        //         MemberKind::Field(val) => val,
        //         MemberKind::Method(call) => Value::CallableMethod(obj_any.clone(), call.clone()),
        //     }
        // }
        // Some(Value::ObjectMut(obj_any)) => {
        //     println!("{:?}", obj_any.type_id());
        //     println!("{:?}", TypeId::of::<Rc<dyn Any>>());
        //     println!("{:?}", obj_any.as_ref().type_id());
        //     println!("{:?}", TypeId::of::<RefCell<Box<dyn Any>>>());
        //     let object = obj_any.as_ref().downcast_ref::<RefCell<Box<dyn Object>>>().unwrap();
        //     let Some(member) = object.borrow().get_member(&member) else {
        //         todo!()
        //     };
        //
        //     match member.kind {
        //         MemberKind::Field(val) => val,
        //         MemberKind::Method(call) => Value::CallableMethodMut(obj_any.clone(), call.clone()),
        //     }
        // }
        _ => todo!(),
    }
}

fn eval_call_expr<'a>(target: Value, args: Value, scope: &'a RuleScope) -> Value {
    // let Value::Ident(target) = target else {
    //     unreachable!()
    // };
    //
    // println!("{:?}", target);

    let Value::Many(mut args) = args else {
        unreachable!()
    };

    for arg in args.iter() {
        match arg {
            Value::String(s) => println!("{s}"),
            Value::Int(_) => {}
            Value::Bool(_) => {}
            Value::Ident(_) => {}
            // Value::Object(_) => {}
            Value::Object2(_) => {}
            // Value::ObjectMut(_) => {},
            Value::Callable(_) => {}
            Value::CallableMethod(_, _) => {}
            // Value::CallableMethodMut(_, _) => {}
            Value::Many(_) => {}
        }
    }

    let func = match &target {
        Value::Ident(target) => scope.get_var(target),
        Value::CallableMethod(obj, call) => {
            args.insert(0, Value::Object2(obj.clone()));
            Some(&target)
        }
        // Value::CallableMethodMut(obj, call) => {
        //     args.insert(0, Value::ObjectMut(obj.clone()));
        //     Some(&target)
        // },
        _ => None,
    };

    match func {
        Some(Value::Callable(callable)) => {
            callable(args);
        }
        Some(Value::CallableMethod(obj, callable)) => {
            callable(args);
        }

        // Some(Value::CallableMethodMut(obj, callable)) => {
        //     callable(args);
        // }
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
