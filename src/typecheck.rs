use anyhow::{anyhow, Result};

use crate::ast::{BOp, Extern, Func, Lit, Prog, Type, UOp, VDecl};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, PartialEq)]
pub struct TCProg {
    externs: Vec<TCExtern>,
    funcs: Vec<TCFunc>,
}

#[derive(Debug, PartialEq)]
struct TCExtern {
    pub type_: TCType,
    pub globid: String,
    pub args: Vec<TCType>,
}

impl TryFrom<Extern> for TCExtern {
    type Error = anyhow::Error;

    fn try_from(e: Extern) -> Result<Self, Self::Error> {
        Ok(TCExtern {
            type_: e.type_.try_into()?,
            globid: e.globid,
            args: e
                .args
                .unwrap_or(vec![])
                .iter()
                .map(|x| x.clone().try_into())
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq)]
struct TCFunc {
    pub type_: TCType,
    pub globid: String,
    pub args: Vec<TCVDecl>,
    pub blk: TCBlock,
}

fn typecheck_fn(
    _fun: Func,
    _defined_functions: &mut HashMap<String, (TCType, Vec<TCType>)>,
) -> Result<TCFunc> {
    /*
    In ​ <vdecl>​ , the type may not be void.
    In ​ ref ​ <type>​ , the type may not be void or itself a reference type.
    All functions must be declared and/or defined before they are used.
    A function may not return a ref type.
    The initialization expression for a reference variable (including function arguments) must be a variable.
    All programs must define exactly one function named “run” which returns an integer (the
    program exit status) and takes no arguments.
    Errors should be a line starting with 'error: ' as specified in the compiler requirements.

    Also, for every expression, determine its type (applying the explicit cast rules from the language specification, and making sure Binops are the same type on both sides or a valid cast.  When printing the AST, the type of each expression should be part of the AST nodes for each expression.
     */
    unimplemented!("typechecking fun not implemented");
}

#[derive(Debug, PartialEq)]
pub struct TCBlock {
    pub stmts: Vec<TCStmt>,
}

#[derive(Debug, PartialEq)]
pub enum TCStmt {
    Blk(TCBlock),
    ReturnStmt(Option<TypedExp>),
    VDeclStmt {
        vdecl: TCVDecl,
        exp: TypedExp,
    },
    ExpStmt(TypedExp),
    WhileStmt {
        cond: TypedExp,
        stmt: Box<TCStmt>,
    },
    IfStmt {
        cond: TypedExp,
        stmt: Box<TCStmt>,
        else_stmt: Option<Box<TCStmt>>,
    },
    PrintStmt(TypedExp),
    PrintStmtSlit(String),
}

#[derive(Debug, PartialEq)]
pub struct TypedExp {
    // ifs and whiles have no type- representing unit as none
    type_: Option<TCType>,
    exp: TCExp,
}

#[derive(Debug, PartialEq)]
pub enum TCExp {
    Assign {
        varid: String,
        exp: Box<TypedExp>,
    },
    Cast {
        type_: TCType,
        exp: Box<TypedExp>,
    },
    BinOp {
        op: BOp,
        lhs: Box<TypedExp>,
        rhs: Box<TypedExp>,
    },
    UnaryOp {
        op: UOp,
        exp: Box<TypedExp>,
    },
    Literal(Lit),
    VarVal(String),
    FuncCall {
        globid: String,
        exps: Vec<TypedExp>,
    },
}

#[derive(Debug, PartialEq)]
pub struct TCVDecl {
    pub type_: Box<TCType>,
    pub varid: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TCType {
    AtomType(TCAtomType),
    Ref(bool, TCAtomType), // noalias, type
}

impl TryFrom<VDecl> for TCType {
    type Error = anyhow::Error;

    fn try_from(t: VDecl) -> Result<Self, Self::Error> {
        t.type_.try_into()
    }
}

impl TryFrom<Type> for TCType {
    type Error = anyhow::Error;

    fn try_from(t: Type) -> Result<Self, Self::Error> {
        Ok(if let Type::Ref(b, t_inner) = t {
            TCType::Ref(b, TCAtomType::try_from(*t_inner)?)
        } else {
            TCType::AtomType(TCAtomType::try_from(t)?)
        })
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TCAtomType {
    IntType,
    CIntType,
    FloatType,
    BoolType,
    VoidType,
}

impl TryFrom<Type> for TCAtomType {
    type Error = anyhow::Error;

    fn try_from(t: Type) -> Result<Self, Self::Error> {
        match t {
            Type::IntType => Ok(TCAtomType::IntType),
            Type::CIntType => Ok(TCAtomType::CIntType),
            Type::FloatType => Ok(TCAtomType::FloatType),
            Type::BoolType => Ok(TCAtomType::BoolType),
            Type::VoidType => Ok(TCAtomType::VoidType),
            Type::Ref(_, _) => Err(anyhow!("tried to convert ref type to atom type")),
        }
    }
}

pub fn typecheck(prog: Prog) -> Result<TCProg> {
    let mut fn_name_to_type: HashMap<String, (TCType, Vec<TCType>)> = HashMap::new();
    let mut tcprog_externs = vec![];
    for e in prog.externs {
        let e_tc: TCExtern = e.try_into()?;
        if let Some(_) =
            fn_name_to_type.insert(e_tc.globid.clone(), (e_tc.type_.clone(), e_tc.args.clone()))
        {
            return Err(anyhow!("duplicate extern name: {}", e_tc.globid.clone()));
        }
        tcprog_externs.push(e_tc);
    }

    let mut tcprog_funcs = vec![];
    for f in prog.funcs {
        if let Some(_) = fn_name_to_type.insert(
            f.globid.clone(),
            (
                f.type_.clone().try_into()?,
                (&f.args)
                    .as_ref()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|x| x.clone().try_into())
                    .collect::<Result<Vec<TCType>>>()?,
            ),
        ) {
            return Err(anyhow!("duplicate function name: {}", f.globid.clone()));
        }
        tcprog_funcs.push(typecheck_fn(f, &mut fn_name_to_type)?);
    }
    unimplemented!("need to do... everything else?");
    Ok(TCProg {
        externs: tcprog_externs,
        funcs: tcprog_funcs,
    })
}
