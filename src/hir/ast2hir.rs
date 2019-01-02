use crate::ast;
use crate::config::Config;
use crate::hir::{Expr, HTy, Pattern, Val, HIR};
use crate::pass::Pass;
use crate::prim::*;

pub struct AST2HIR;

fn force_into(ty: ast::Ty) -> HTy {
    use crate::ast::Ty::*;
    match ty {
        Bool => HTy::Bool,
        Int => HTy::Int,
        Float => HTy::Float,
        Tuple(tys) => HTy::Tuple(tys.into_iter().map(conv_ty).collect()),
        Fun(arg, ret) => HTy::fun(conv_ty(arg), conv_ty(ret)),
    }
}

fn force_tuple(ty: ast::Ty) -> Vec<HTy> {
    use crate::ast::Ty::*;
    match ty {
        Tuple(tys) => tys.into_iter().map(conv_ty).collect(),
        _ => panic!(),
    }
}

fn conv_ty(ty: ast::TyDefer) -> HTy {
    force_into(ty.force("internal typing error"))
}

impl AST2HIR {
    fn conv_ast(&self, ast: ast::AST) -> HIR {
        HIR(ast.0.into_iter().map(|val| self.conv_val(val)).collect())
    }

    fn conv_val(&self, val: ast::Val) -> Val {
        Val {
            ty: conv_ty(val.ty),
            rec: val.rec,
            name: val.name,
            expr: self.conv_expr(val.expr),
        }
    }

    fn conv_expr(&self, expr: ast::Expr) -> Expr {
        use crate::ast::Expr as E;
        match expr {
            E::Binds { ty, binds, ret } => Expr::Binds {
                ty: conv_ty(ty),
                binds: binds.into_iter().map(|b| self.conv_val(b)).collect(),
                ret: Box::new(self.conv_expr(*ret)),
            },
            E::BinOp { op, ty, l, r } => Expr::BinOp {
                ty: conv_ty(ty),
                name: op,
                l: Box::new(self.conv_expr(*l)),
                r: Box::new(self.conv_expr(*r)),
            },
            E::Fun {
                param_ty,
                param,
                body_ty,
                body,
            } => Expr::Fun {
                param: (conv_ty(param_ty), param),
                body_ty: conv_ty(body_ty),
                body: Box::new(self.conv_expr(*body)),
                captures: Vec::new(),
            },
            E::App { ty, fun, arg } => self.conv_expr(*fun).app1(conv_ty(ty), self.conv_expr(*arg)),
            E::If {
                ty,
                cond,
                then,
                else_,
            } => Expr::Case {
                ty: conv_ty(ty),
                expr: Box::new(self.conv_expr(*cond)),
                arms: vec![
                    (
                        Pattern::Lit {
                            value: Literal::Bool(true),
                            ty: HTy::Bool,
                        },
                        self.conv_expr(*then),
                    ),
                    (
                        Pattern::Lit {
                            value: Literal::Bool(false),
                            ty: HTy::Bool,
                        },
                        self.conv_expr(*else_),
                    ),
                ],
            },
            E::Case { ty, cond, clauses } => Expr::Case {
                ty: conv_ty(ty),
                expr: Box::new(self.conv_expr(*cond)),
                arms: clauses
                    .into_iter()
                    .map(|(pat, expr)| (self.conv_pat(pat), self.conv_expr(expr)))
                    .collect(),
            },
            E::Tuple { ty, tuple } => Expr::Tuple {
                tys: force_tuple(ty.force("internal typing error")),
                tuple: tuple.into_iter().map(|e| self.conv_expr(e)).collect(),
            },
            E::Sym { ty, name } => Expr::Sym {
                ty: conv_ty(ty),
                name: name,
            },
            E::Lit { ty, value } => Expr::Lit {
                ty: conv_ty(ty),
                value: value,
            },
        }
    }
    fn conv_pat(&self, pat: ast::Pattern) -> Pattern {
        match pat {
            ast::Pattern::Lit { value, ty } => Pattern::Lit {
                value: value,
                ty: conv_ty(ty),
            },
            ast::Pattern::Tuple { tuple } => {
                let (tys, tuple) = tuple
                    .into_iter()
                    .map(|(ty, sym)| (conv_ty(ty), sym))
                    .unzip();
                Pattern::Tuple { tuple, tys }
            }
            ast::Pattern::Var { name, ty } => Pattern::Var {
                name: name,
                ty: conv_ty(ty),
            },
            ast::Pattern::Wildcard { ty } => Pattern::Var {
                name: Symbol::new("_"),
                ty: conv_ty(ty),
            },
        }
    }
}

impl<E> Pass<ast::AST, E> for AST2HIR {
    type Target = HIR;

    fn trans(&mut self, ast: ast::AST, _: &Config) -> ::std::result::Result<Self::Target, E> {
        Ok(self.conv_ast(ast))
    }
}
