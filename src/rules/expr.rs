use crate::rules::lexer::RuleToken;

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

#[derive(Debug)]
pub struct Expr {
    pub lhs: Box<ExprOrValue>,
    pub operator: Operator,
    pub rhs: Box<ExprOrValue>,
}
