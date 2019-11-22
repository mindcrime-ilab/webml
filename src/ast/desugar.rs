use crate::ast::*;
use crate::config::Config;
use crate::id::Id;
use crate::pass::Pass;
use crate::prim::Symbol;

pub struct Desugar {
    id: Id,
}

impl Desugar {
    pub fn new(id: Id) -> Self {
        Self { id }
    }

    pub fn gensym(&mut self) -> Symbol {
        let id = self.id.next();
        Symbol("#arg".into(), id)
    }
}

impl Desugar {
    fn transform_ast(&mut self, ast: UntypedAst) -> UntypedCore {
        AST(ast
            .0
            .into_iter()
            .map(|stmt| self.transform_statement(stmt))
            .collect())
    }

    fn transform_statement(&mut self, stmt: Statement<()>) -> UntypedCoreStatement {
        use Statement::*;
        match stmt {
            Datatype { name, constructors } => self.transform_datatype(name, constructors),
            Val { rec, pattern, expr } => self.transform_val(rec, pattern, expr),
            Fun { name, params, expr } => self.transform_fun(name, params, expr),
        }
    }

    fn transform_datatype(
        &mut self,
        name: Symbol,
        constructors: Vec<(Symbol, Option<Type>)>,
    ) -> UntypedCoreStatement {
        Statement::Datatype { name, constructors }
    }

    fn transform_val(
        &mut self,
        rec: bool,
        pattern: UntypedPattern,
        expr: UntypedExpr,
    ) -> UntypedCoreStatement {
        Statement::Val {
            rec,
            pattern: self.transform_pattern(pattern),
            expr: self.transform_expr(expr),
        }
    }

    fn transform_fun(
        &mut self,
        name: Symbol,
        params: Vec<UntypedPattern>,
        expr: UntypedExpr,
    ) -> UntypedCoreStatement {
        let fun = params
            .into_iter()
            .rev()
            .fold(self.transform_expr(expr), |body, param| {
                let param_sym = self.gensym();
                Expr::Fn {
                    ty: (),
                    param: param_sym.clone(),
                    body: Expr::Case {
                        ty: (),
                        cond: Expr::Symbol {
                            name: param_sym,
                            ty: (),
                        }
                        .boxed(),
                        clauses: vec![(param, body)],
                    }
                    .boxed(),
                }
            });
        Statement::Val {
            rec: true,
            pattern: Pattern::Variable { name: name, ty: () },
            expr: fun,
        }
    }

    fn transform_expr(&mut self, expr: UntypedExpr) -> UntypedCoreExpr {
        use crate::ast::Expr::*;
        match expr {
            Binds { ty, binds, ret } => self.transform_binds(ty, binds, ret),
            BinOp { ty, op, l, r } => self.transform_binop(ty, op, l, r),
            Fn { ty, param, body } => self.transform_fn(ty, param, body),
            App { ty, fun, arg } => self.transform_app(ty, fun, arg),
            Case { ty, cond, clauses } => self.transform_case(ty, cond, clauses),
            Tuple { ty, tuple } => self.transform_tuple(ty, tuple),
            Constructor { ty, arg, name } => self.transform_constructor(ty, arg, name),
            Symbol { ty, name } => self.transform_symbol(ty, name),
            Literal { ty, value } => self.transform_literal(ty, value),
            D(DerivedExpr::If {
                ty,
                cond,
                then,
                else_,
            }) => self.transform_if(ty, cond, then, else_),
        }
    }
    fn transform_binds(
        &mut self,
        ty: (),
        binds: Vec<UntypedStatement>,
        ret: Box<UntypedExpr>,
    ) -> UntypedCoreExpr {
        Expr::Binds {
            ty,
            binds: binds
                .into_iter()
                .map(|stmt| self.transform_statement(stmt))
                .collect(),
            ret: self.transform_expr(*ret).boxed(),
        }
    }

    fn transform_binop(
        &mut self,
        ty: (),
        op: Symbol,
        l: Box<UntypedExpr>,
        r: Box<UntypedExpr>,
    ) -> UntypedCoreExpr {
        Expr::BinOp {
            ty,
            op,
            l: self.transform_expr(*l).boxed(),
            r: self.transform_expr(*r).boxed(),
        }
    }

    fn transform_fn(&mut self, ty: (), param: Symbol, body: Box<UntypedExpr>) -> UntypedCoreExpr {
        Expr::Fn {
            ty,
            param,
            body: self.transform_expr(*body).boxed(),
        }
    }

    fn transform_app(
        &mut self,
        ty: (),
        fun: Box<UntypedExpr>,
        arg: Box<UntypedExpr>,
    ) -> UntypedCoreExpr {
        Expr::App {
            ty,
            fun: self.transform_expr(*fun).boxed(),
            arg: self.transform_expr(*arg).boxed(),
        }
    }

    fn transform_if(
        &mut self,
        ty: (),
        cond: Box<UntypedExpr>,
        then: Box<UntypedExpr>,
        else_: Box<UntypedExpr>,
    ) -> UntypedCoreExpr {
        Expr::Case {
            ty,
            cond: Box::new(self.transform_expr(*cond)),
            clauses: vec![
                (
                    Pattern::Constructor {
                        ty: (),
                        arg: None,
                        name: Symbol::new("true"),
                    },
                    self.transform_expr(*then),
                ),
                (
                    Pattern::Constructor {
                        ty: (),
                        arg: None,
                        name: Symbol::new("false"),
                    },
                    self.transform_expr(*else_),
                ),
            ],
        }
    }

    fn transform_case(
        &mut self,
        ty: (),
        cond: Box<UntypedExpr>,
        clauses: Vec<(UntypedPattern, UntypedExpr)>,
    ) -> UntypedCoreExpr {
        Expr::Case {
            ty,
            cond: self.transform_expr(*cond).boxed(),
            clauses: clauses
                .into_iter()
                .map(|(p, e)| (self.transform_pattern(p), self.transform_expr(e)))
                .collect(),
        }
    }

    fn transform_tuple(&mut self, ty: (), tuple: Vec<UntypedExpr>) -> UntypedCoreExpr {
        Expr::Tuple {
            ty,
            tuple: tuple.into_iter().map(|t| self.transform_expr(t)).collect(),
        }
    }

    fn transform_constructor(
        &mut self,
        ty: (),
        arg: Option<Box<UntypedExpr>>,
        name: Symbol,
    ) -> UntypedCoreExpr {
        Expr::Constructor {
            ty,
            arg: arg.map(|e| self.transform_expr(*e).boxed()),
            name,
        }
    }
    fn transform_symbol(&mut self, ty: (), name: Symbol) -> UntypedCoreExpr {
        Expr::Symbol { ty, name }
    }

    fn transform_literal(&mut self, ty: (), value: Literal) -> UntypedCoreExpr {
        Expr::Literal { ty, value }
    }

    fn transform_pattern(&mut self, pattern: UntypedPattern) -> UntypedPattern {
        use Pattern::*;
        match pattern {
            Constant { ty, value } => self.transform_pat_constant(ty, value),
            Constructor { ty, arg, name } => self.transform_pat_constructor(ty, arg, name),
            Tuple { ty, tuple } => self.transform_pat_tuple(ty, tuple),
            Variable { ty, name } => self.transform_pat_variable(ty, name),
            Wildcard { ty } => self.transform_pat_wildcard(ty),
        }
    }

    fn transform_pat_constant(&mut self, ty: (), value: i64) -> UntypedPattern {
        Pattern::Constant { value, ty }
    }

    fn transform_pat_constructor(
        &mut self,
        ty: (),
        arg: Option<((), Symbol)>,
        name: Symbol,
    ) -> UntypedPattern {
        Pattern::Constructor { name, arg, ty }
    }

    fn transform_pat_tuple(&mut self, ty: (), tuple: Vec<((), Symbol)>) -> UntypedPattern {
        Pattern::Tuple { ty, tuple }
    }

    fn transform_pat_variable(&mut self, ty: (), name: Symbol) -> UntypedPattern {
        Pattern::Variable { name, ty }
    }

    fn transform_pat_wildcard(&mut self, ty: ()) -> UntypedPattern {
        Pattern::Wildcard { ty }
    }
}

impl<E> Pass<UntypedAst, E> for Desugar {
    type Target = UntypedCore;

    fn trans(&mut self, ast: UntypedAst, _: &Config) -> ::std::result::Result<Self::Target, E> {
        let core = self.transform_ast(ast);
        Ok(core)
    }
}
