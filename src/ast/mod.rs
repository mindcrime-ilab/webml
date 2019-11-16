pub mod case_check;
mod pp;
pub mod rename;
pub mod typing;
mod util;

pub use self::case_check::CaseCheck;
pub use self::rename::Rename;
pub use self::typing::TyEnv as Typer;
use nom;
use std::error::Error;
use std::fmt;

use crate::ast;
use crate::prim::*;

pub type UntypedAst = AST<()>;
pub type TypedAst = AST<Type>;

#[derive(Debug, Clone, PartialEq)]
pub struct AST<Ty>(pub Vec<Val<Ty>>);

#[derive(Debug, Clone, PartialEq)]
pub struct Val<Ty> {
    pub ty: Ty,
    pub rec: bool,
    pub pattern: Pattern<Ty>,
    pub expr: Expr<Ty>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<Ty> {
    Binds {
        ty: Ty,
        binds: Vec<Val<Ty>>,
        ret: Box<Expr<Ty>>,
    },
    BinOp {
        op: Symbol,
        ty: Ty,
        l: Box<Expr<Ty>>,
        r: Box<Expr<Ty>>,
    },
    Fun {
        ty: Ty,
        param: Symbol,
        body: Box<Expr<Ty>>,
    },
    App {
        ty: Ty,
        fun: Box<Expr<Ty>>,
        arg: Box<Expr<Ty>>,
    },
    If {
        ty: Ty,
        cond: Box<Expr<Ty>>,
        then: Box<Expr<Ty>>,
        else_: Box<Expr<Ty>>,
    },
    Case {
        ty: Ty,
        cond: Box<Expr<Ty>>,
        clauses: Vec<(Pattern<Ty>, Expr<Ty>)>,
    },
    Tuple {
        ty: Ty,
        tuple: Vec<Expr<Ty>>,
    },
    Sym {
        ty: Ty,
        name: Symbol,
    },
    Lit {
        ty: Ty,
        value: Literal,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern<Ty> {
    Lit { value: Literal, ty: Ty },
    // having redundant types for now
    Tuple { tuple: Vec<(Ty, Symbol)>, ty: Ty },
    Var { name: Symbol, ty: Ty },
    Wildcard { ty: Ty },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Var(u64),
    Bool,
    Int,
    Float,
    Fun(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
}

impl<Ty> Expr<Ty> {
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

impl<Ty: Clone> Expr<Ty> {
    fn ty(&self) -> Ty {
        use self::Expr::*;
        match *self {
            Binds { ref ty, .. }
            | BinOp { ref ty, .. }
            | App { ref ty, .. }
            | If { ref ty, .. }
            | Case { ref ty, .. }
            | Tuple { ref ty, .. }
            | Sym { ref ty, .. }
            | Lit { ref ty, .. }
            | Fun { ref ty, .. } => ty.clone(),
        }
    }
}

impl<Ty> Pattern<Ty> {
    pub fn binds(&self) -> Vec<(&Symbol, &Ty)> {
        use self::Pattern::*;
        match *self {
            Lit { .. } | Wildcard { .. } => vec![],
            Var { ref name, ref ty } => vec![(name, ty)],
            Tuple { ref tuple, .. } => tuple.iter().map(|&(ref ty, ref sym)| (sym, ty)).collect(),
        }
    }
}

impl<Ty: Clone> Pattern<Ty> {
    fn ty(&self) -> Ty {
        use self::Pattern::*;
        match *self {
            Lit { ref ty, .. }
            | Var { ref ty, .. }
            | Wildcard { ref ty }
            | Tuple { ref ty, .. } => ty.clone(),
        }
    }
}

impl Type {
    pub fn fun(param: Type, ret: Type) -> Type {
        Type::Fun(Box::new(param), Box::new(ret))
    }
    pub fn unit() -> Type {
        Type::Tuple(Vec::new())
    }
}

#[derive(Debug)]
pub enum TypeError<'a> {
    MisMatch { expected: Type, actual: Type },
    CannotInfer,
    FreeVar,
    NotFunction(ast::Expr<Type>),
    ParseError(nom::Err<&'a str>),
}

impl<'a> fmt::Display for TypeError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<'a> Error for TypeError<'a> {
    fn description(&self) -> &str {
        use self::TypeError::*;
        match self {
            &MisMatch { .. } => "type mismatches against expected type",
            &CannotInfer => "cannot infer the type",
            &FreeVar => "free variable is found",
            &NotFunction(_) => "not a function",
            &ParseError(_) => "parse error",
        }
    }
}

impl<'a> From<nom::Err<&'a str>> for TypeError<'a> {
    fn from(e: nom::Err<&'a str>) -> Self {
        // fn conv<'b>(e: nom::Err<&'b [u8]>) -> nom::Err<&'b str> {
        //     use std::str::from_utf8;
        //     use nom::Err::*;
        //     match e {
        //         Code(e) => Code(e),
        //         Node(kind, box_err) => Node(kind, Box::new(conv(*box_err))),
        //         Position(kind, slice) => Position(kind, from_utf8(slice).unwrap()),
        //         NodePosition(kind, slice, box_err) => {
        //             NodePosition(kind, from_utf8(slice).unwrap(), Box::new(conv(*box_err)))
        //         }
        //     }
        // }

        TypeError::ParseError(e)
    }
}

pub type Result<'a, T> = ::std::result::Result<T, TypeError<'a>>;
