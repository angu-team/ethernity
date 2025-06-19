use core::cmp::Ordering;
use ethereum_types::U256;
use crate::utils::ordered_pair;

#[derive(Debug, Clone)]
pub enum Expr {
    Const(U256),
    Arg(u32),
    SLoad(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        expr_to_string(self) == expr_to_string(other)
    }
}
impl Eq for Expr {}

impl Ord for Expr {
    fn cmp(&self, other: &Self) -> Ordering {
        expr_to_string(self).cmp(&expr_to_string(other))
    }
}

impl PartialOrd for Expr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Require(Expr),
    Write(Expr, Expr),
    Return(Expr),
    Revert,
    Expr(Expr),
}

pub fn expr_to_string(e: &Expr) -> String {
    match e {
        Expr::Const(v) => format!("CONST({:#x})", v),
        Expr::Arg(n) => format!("ARG({})", n),
        Expr::SLoad(inner) => format!("SLOAD({})", expr_to_string(inner)),
        Expr::Add(a, b) => format!("ADD({},{})", expr_to_string(a), expr_to_string(b)),
        Expr::Sub(a, b) => format!("SUB({},{})", expr_to_string(a), expr_to_string(b)),
        Expr::Mul(a, b) => format!("MUL({},{})", expr_to_string(a), expr_to_string(b)),
        Expr::Eq(a, b) => format!("EQ({},{})", expr_to_string(a), expr_to_string(b)),
    }
}

pub fn stmt_to_string(s: &Stmt) -> String {
    match s {
        Stmt::Require(c) => format!("REQUIRE({})", expr_to_string(c)),
        Stmt::Write(slot, val) => format!("WRITE({},{})", expr_to_string(slot), expr_to_string(val)),
        Stmt::Return(v) => format!("RETURN({})", expr_to_string(v)),
        Stmt::Revert => "REVERT".to_string(),
        Stmt::Expr(e) => expr_to_string(e),
    }
}

/// Helper for commutative operations.
pub fn canonicalize_commutative(a: Expr, b: Expr) -> (Expr, Expr) {
    ordered_pair(a, b)
}
