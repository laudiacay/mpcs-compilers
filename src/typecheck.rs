use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::ast::*;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TCProg {
    pub externs: Vec<TCExtern>,
    pub funcs: Vec<TCFunc>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TCExtern {
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TCFunc {
    pub type_: TCType,
    pub globid: String,
    pub args: Vec<TCVDecl>,
    pub blk: TCBlock,
}

fn typecheck_fn(
    fun: Func,
    defined_functions: &HashMap<String, (TCType, Vec<TCType>)>,
) -> Result<TCFunc> {
    // A function may not return a ref type.
    if let Type::Ref(_, _) = fun.type_ {
        return Err(anyhow!("functions cannot return ref types"));
    }

    let re_type: TCType = fun.type_.try_into()?;
    let mut new_args = vec![];
    let mut defined_vars: HashMap<String, TCType> = HashMap::new();
    if let Some(args) = fun.args {
        for arg in args.iter() {
            let arg_new_type: TCType = arg.type_.clone().try_into()?;
            if let Some(_) = defined_vars.insert(arg.varid.clone(), arg_new_type.clone()) {
                return Err(anyhow!("two function arguments have the same name!"));
            }
            new_args.push(TCVDecl {
                varid: arg.varid.clone(),
                type_: arg_new_type,
            });
        }
    }

    let my_block = typecheck_block(
        fun.blk,
        defined_functions,
        defined_vars,
        Some(re_type.clone()),
        HashMap::new(),
    )?;
    Ok(TCFunc {
        type_: re_type,
        globid: fun.globid,
        args: new_args,
        blk: my_block,
    })
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TCBlock {
    pub stmts: Vec<TCStmt>,
}

fn typecheck_block(
    blk: Block,
    defined_functions: &HashMap<String, (TCType, Vec<TCType>)>,
    mut defined_vars: HashMap<String, TCType>, // idk what to do with the muts and the &s tbh
    should_return: Option<TCType>,
    mut shadowed_vars: HashMap<String, TCType>,
) -> Result<TCBlock> {
    /*    /*
    All functions must be declared and/or defined before they are used.
    The initialization expression for a reference variable (including function arguments) must be a variable.
    Errors should be a line starting with 'error: ' as specified in the compiler requirements.

    Also, for every expression, determine its type (applying the explicit cast rules from the language specification, and making sure Binops are the same type on both sides or a valid cast.  When printing the AST, the type of each expression should be part of the AST nodes for each expression.
     */*/

    let mut tc_stmts = vec![];
    if let Some(stmts) = blk.stmts {
        for stmt in stmts {
            let new_stmt = typecheck_stmt(
                *stmt,
                defined_functions,
                &mut defined_vars,
                should_return.clone(),
                &mut shadowed_vars,
            )?;
            tc_stmts.push(new_stmt);
        }
    }
    Ok(TCBlock { stmts: tc_stmts })
}

fn typecheck_stmt(
    stmt: Stmt,
    defined_functions: &HashMap<String, (TCType, Vec<TCType>)>,
    defined_vars: &mut HashMap<String, TCType>, // idk what to do with the muts and the &s tbh
    should_return: Option<TCType>,
    shadowed_vars: &mut HashMap<String, TCType>,
) -> Result<TCStmt> {
    let new_stmt = match stmt {
        Stmt::Blk(b) => TCStmt::Blk(typecheck_block(
            b,
            &defined_functions,
            defined_vars.clone(),
            should_return.clone(),
            HashMap::new(), // entering a new block allows shadowing existing vars
        )?),
        Stmt::ReturnStmt(exp) => match (exp, should_return.clone()) {
            (None, None) => TCStmt::ReturnStmt(None),
            (Some(exp), Some(should_return)) => {
                let tcexp = typecheck_exp(exp, defined_functions, &defined_vars)?;
                if tcexp.type_ != should_return {
                    Err(anyhow!("function returns incorrect type"))?
                } else {
                    TCStmt::ReturnStmt(Some(tcexp))
                }
            }
            _ => Err(anyhow!("function returns incorrect type"))?,
        },
        Stmt::VDeclStmt { vdecl, exp } => {
            let vdecl: TCVDecl = vdecl.try_into()?;
            let exp = typecheck_exp(exp, defined_functions, &defined_vars)?;
            if let TCType::Ref(_, pointer_type) = vdecl.type_ {
                if let TCExp::VarVal(_) = exp.exp {
                    if exp.type_ != TCType::AtomType(pointer_type) {
                        Err(anyhow!(
                            "reference type does not match type of right-hand side of declaration"
                        ))?
                    }
                } else {
                    Err(anyhow!(
                        "reference type assigned to non-variable expression"
                    ))?
                }
            } else {
                if exp.type_ != vdecl.type_ {
                    Err(anyhow!("variable declaration assigns to wrong type"))?
                }
            }

            defined_vars.insert(vdecl.varid.clone(), vdecl.type_.clone());
            if let Some(_) = shadowed_vars.insert(vdecl.varid.clone(), vdecl.type_.clone()) {
                Err(anyhow!("duplicate variable definition"))?;
            }

            TCStmt::VDeclStmt { vdecl, exp }
        }
        Stmt::ExpStmt(exp) => {
            TCStmt::ExpStmt(typecheck_exp(exp, defined_functions, &defined_vars)?)
        }
        Stmt::WhileStmt { cond, stmt } => {
            let cond = typecheck_exp(cond, defined_functions, &defined_vars)?;
            let new_stmt = typecheck_stmt(
                *stmt,
                defined_functions,
                &mut defined_vars.clone(),
                should_return.clone(),
                &mut shadowed_vars.clone(),
            )?;

            // check that the condition is actually a bool. unsure if this is necessary.
            if let TCType::AtomType(TCAtomType::BoolType) = cond.type_ {
                TCStmt::WhileStmt {
                    cond,
                    stmt: Box::new(new_stmt),
                }
            } else {
                Err(anyhow!("non-boolean expression in while loop condition"))?
            }
        }
        Stmt::IfStmt {
            cond,
            stmt,
            else_stmt,
        } => {
            let cond = typecheck_exp(cond, defined_functions, &defined_vars)?;
            let new_stmt = typecheck_stmt(
                *stmt,
                defined_functions,
                &mut defined_vars.clone(),
                should_return.clone(),
                &mut shadowed_vars.clone(),
            )?;

            if let TCType::AtomType(TCAtomType::BoolType) = cond.type_ {
                if let Some(else_stmt) = else_stmt {
                    let new_else_stmt = typecheck_stmt(
                        *else_stmt,
                        defined_functions,
                        &mut defined_vars.clone(),
                        should_return.clone(),
                        &mut shadowed_vars.clone(),
                    )?;
                    TCStmt::IfStmt {
                        cond,
                        stmt: Box::new(new_stmt),
                        else_stmt: Some(Box::new(new_else_stmt)),
                    }
                } else {
                    TCStmt::IfStmt {
                        cond,
                        stmt: Box::new(new_stmt),
                        else_stmt: None,
                    }
                }
            } else {
                Err(anyhow!("non-boolean expression in while loop condition"))?
            }
        }
        Stmt::PrintStmt(exp) => {
            TCStmt::PrintStmt(typecheck_exp(exp, defined_functions, &defined_vars)?)
        }
        Stmt::PrintStmtSlit(stri) => TCStmt::PrintStmtSlit(stri),
    };
    Ok(new_stmt)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TypedExp {
    type_: TCType,
    exp: TCExp,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

// atom types become atom types, ref types become atoms, void types error
fn maybe_deref(type_: TCType) -> Result<TCAtomType> {
    match type_ {
        TCType::AtomType(tca) => Ok(tca),
        TCType::VoidType => Err(anyhow!("cannot deref voidtype")),
        TCType::Ref(_, tca) => Ok(tca),
    }
}

fn typecheck_exp(
    exp: Exp,
    defined_functions: &HashMap<String, (TCType, Vec<TCType>)>,
    defined_vars: &HashMap<String, TCType>,
) -> Result<TypedExp> {
    match exp {
        Exp::Assign {
            varid,
            exp: assignment_exp,
        } => {
            let assignment_exp = typecheck_exp(*assignment_exp, defined_functions, defined_vars)?;
            let vartype = defined_vars.get(&varid);
            match vartype {
                Some(type_) => {
                    // it makes sense to error on void types here- you would never have a ref to one or try to assign it
                    if maybe_deref(type_.clone())? != maybe_deref(assignment_exp.type_.clone())? {
                        Err(anyhow!(format!("mismatched types in assign statement, varid: {:?}, vartype: {:?}, expression: {:?}", varid, type_, &assignment_exp)))?
                    }
                    let new_exp = TCExp::Assign {
                        varid,
                        exp: Box::new(assignment_exp),
                    };
                    Ok(TypedExp {
                        type_: type_.clone(),
                        exp: new_exp,
                    })
                }
                None => Err(anyhow!("assign statement to undeclared variable"))?,
            }
        }
        Exp::Cast {
            type_: cast_type,
            exp: casted_exp,
        } => {
            let cast_type: TCType = cast_type.try_into()?;
            let new_exp = typecheck_exp(*casted_exp, defined_functions, defined_vars)?;

            // checking for legal casts
            // god this is so ugly
            match new_exp.type_ {
                TCType::AtomType(TCAtomType::IntType)
                | TCType::AtomType(TCAtomType::CIntType)
                | TCType::AtomType(TCAtomType::FloatType) => match cast_type {
                    TCType::AtomType(TCAtomType::IntType)
                    | TCType::AtomType(TCAtomType::CIntType)
                    | TCType::AtomType(TCAtomType::FloatType) => Ok(TypedExp {
                        type_: cast_type.clone(),
                        exp: TCExp::Cast {
                            type_: cast_type,
                            exp: Box::new(new_exp),
                        },
                    }),
                    _ => Err(anyhow!("illegal type cast (num to non-num)"))?,
                },
                TCType::AtomType(TCAtomType::BoolType) => {
                    if let TCType::AtomType(TCAtomType::BoolType) = cast_type {
                        Ok(TypedExp {
                            type_: cast_type.clone(),
                            exp: TCExp::Cast {
                                type_: cast_type,
                                exp: Box::new(new_exp),
                            },
                        })
                    } else {
                        Err(anyhow!("illegal type cast (bool to non-bool)"))?
                    }
                }
                _ => Err(anyhow!("illegal type cast (refs or something)"))?,
            }
        }
        Exp::BinOp { op, lhs, rhs } => {
            let lhs = typecheck_exp(*lhs, defined_functions, defined_vars)?;
            let rhs = typecheck_exp(*rhs, defined_functions, defined_vars)?;

            if maybe_deref(lhs.type_.clone())? != maybe_deref(rhs.type_.clone())? {
                // implicit casts NOT supported, per the spec
                Err(anyhow!("mismatched types in binary expression"))?
            }

            // if I move the new_exp definition out here the borrow checker yells at me :(
            match op {
                BOp::Mult | BOp::Div | BOp::Add | BOp::Sub => match lhs.type_ {
                    TCType::AtomType(TCAtomType::IntType)
                    | TCType::AtomType(TCAtomType::CIntType)
                    | TCType::AtomType(TCAtomType::FloatType) => {
                        let type_ = lhs.type_.clone();
                        let new_exp = TCExp::BinOp {
                            op: op.clone(),
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        };
                        Ok(TypedExp {
                            type_,
                            exp: new_exp,
                        })
                    }
                    _ => Err(anyhow!("arithmetic operation on non-num types"))?,
                },
                BOp::EqTo => {
                    let new_exp = TCExp::BinOp {
                        op: op.clone(),
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                    Ok(TypedExp {
                        type_: TCType::AtomType(TCAtomType::BoolType),
                        exp: new_exp,
                    })
                }
                BOp::Gt | BOp::Lt => match lhs.type_ {
                    TCType::AtomType(TCAtomType::IntType)
                    | TCType::AtomType(TCAtomType::CIntType)
                    | TCType::AtomType(TCAtomType::FloatType) => {
                        let new_exp = TCExp::BinOp {
                            op: op.clone(),
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        };
                        Ok(TypedExp {
                            type_: TCType::AtomType(TCAtomType::BoolType),
                            exp: new_exp,
                        })
                    }
                    _ => Err(anyhow!("comparison between non-num types"))?,
                },
                BOp::And | BOp::Or => {
                    if let TCType::AtomType(TCAtomType::BoolType) = lhs.type_ {
                        let new_exp = TCExp::BinOp {
                            op: op.clone(),
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        };
                        Ok(TypedExp {
                            type_: TCType::AtomType(TCAtomType::BoolType),
                            exp: new_exp,
                        })
                    } else {
                        Err(anyhow!("boolean operation on non-boolean types"))?
                    }
                }
            }
        }
        Exp::UnaryOp { op, exp } => {
            let exp = typecheck_exp(*exp, defined_functions, defined_vars)?;
            match (op.clone(), exp.type_.clone()) {
                (UOp::BitwiseNeg, TCType::AtomType(TCAtomType::BoolType)) => {
                    let tcexp = TCExp::UnaryOp {
                        op,
                        exp: Box::new(exp),
                    };
                    Ok(TypedExp {
                        type_: TCType::AtomType(TCAtomType::BoolType),
                        exp: tcexp,
                    })
                }
                (UOp::SignedNeg, TCType::AtomType(TCAtomType::IntType))
                | (UOp::SignedNeg, TCType::AtomType(TCAtomType::CIntType))
                | (UOp::SignedNeg, TCType::AtomType(TCAtomType::FloatType)) => {
                    let type_ = exp.type_.clone();
                    let tcexp = TCExp::UnaryOp {
                        op,
                        exp: Box::new(exp),
                    };
                    Ok(TypedExp { type_, exp: tcexp })
                }
                _ => Err(anyhow!("illegal type in unary expression"))?,
            }
        }
        Exp::Literal(lit) => {
            match lit {
                Lit::LitBool(_) => {
                    let type_ = TCType::AtomType(TCAtomType::BoolType);
                    Ok(TypedExp {
                        type_,
                        exp: TCExp::Literal(lit),
                    })
                }
                Lit::LitInt(_) => {
                    // cints not supported yet
                    let type_ = TCType::AtomType(TCAtomType::IntType);
                    Ok(TypedExp {
                        type_,
                        exp: TCExp::Literal(lit),
                    })
                }
                Lit::LitFloat(_) => {
                    let type_ = TCType::AtomType(TCAtomType::FloatType);
                    Ok(TypedExp {
                        type_,
                        exp: TCExp::Literal(lit),
                    })
                }
            }
        }
        Exp::VarVal(varid) => {
            let vartype = defined_vars.get(&varid);
            match vartype {
                None => Err(anyhow!("variable not defined"))?,
                Some(TCType::Ref(_, atype)) => {
                    // treat ref types within expressions as though they're the actual type
                    // handle dereferencing, uh, later
                    Ok(TypedExp {
                        type_: TCType::AtomType(atype.clone()),
                        exp: TCExp::VarVal(varid),
                    })
                }
                Some(atype) => Ok(TypedExp {
                    type_: atype.clone(),
                    exp: TCExp::VarVal(varid),
                }),
            }
        }
        Exp::FuncCall { globid, exps } => {
            // check that function is in defined_functions
            // if it is, grab the types of each of its arguments, typecheck the corresponding exp
            // in exps, and make sure the types match
            // but if the function signature has a ref type, the exp in exps corresponding to that
            // argument needs to be a VarVal with a matching type
            let func = defined_functions.get(&globid);
            let mut arg_exps: Vec<TypedExp> = vec![];

            if let Some((return_type, arg_types)) = func {
                if let Some(exps) = exps {
                    if exps.len() != arg_types.len() {
                        Err(anyhow!("incorrect number of function arguments"))?
                    }

                    for (arg_type, exp) in arg_types.iter().zip(exps) {
                        let exp = typecheck_exp(*exp, defined_functions, defined_vars)?;
                        let exp_type = exp.type_.clone();

                        // treat ref type arguments separately
                        if let TCType::Ref(_, atype) = arg_type {
                            match exp.exp {
                                TCExp::VarVal(_) => {
                                    if TCType::AtomType(*atype) != exp_type {
                                        Err(anyhow!("wrong type in ref type argument"))?
                                    }
                                }
                                _ => Err(anyhow!(
                                    "non-variable expression passed to ref type argument"
                                ))?,
                            }
                        } else {
                            if *arg_type != exp_type {
                                Err(anyhow!("mismatched types in function arguments"))?
                            }
                        }
                        arg_exps.push(exp);
                    }
                    let type_ = return_type.clone();
                    let new_exp = TCExp::FuncCall {
                        globid,
                        exps: arg_exps,
                    };
                    Ok(TypedExp {
                        type_,
                        exp: new_exp,
                    })
                } else {
                    if arg_types.len() > 0 {
                        Err(anyhow!(
                            "no arguments given to a function that expects arguments"
                        ))?
                    } else {
                        let type_ = return_type.clone();
                        let new_exp = TCExp::FuncCall {
                            globid,
                            exps: arg_exps,
                        };
                        Ok(TypedExp {
                            type_,
                            exp: new_exp,
                        })
                    }
                }
            } else {
                Err(anyhow!("function not defined"))?
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TCVDecl {
    pub type_: TCType,
    pub varid: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TCType {
    AtomType(TCAtomType),
    VoidType,
    Ref(bool, TCAtomType), // noalias, type
}

impl TryFrom<VDecl> for TCVDecl {
    type Error = anyhow::Error;

    fn try_from(t: VDecl) -> Result<Self, Self::Error> {
        let type_ = t.type_.try_into()?;
        if let TCType::VoidType = type_ {
            Err(anyhow!("VDecl cannot be void"))
        } else {
            Ok(TCVDecl {
                type_,
                varid: t.varid,
            })
        }
    }
}

impl TryFrom<Type> for TCType {
    type Error = anyhow::Error;

    fn try_from(t: Type) -> Result<Self, Self::Error> {
        Ok(if let Type::Ref(b, t_inner) = t {
            TCType::Ref(b, TCAtomType::try_from(*t_inner)?)
        } else if let Type::VoidType = t {
            TCType::VoidType
        } else {
            TCType::AtomType(TCAtomType::try_from(t)?)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TCAtomType {
    IntType,
    CIntType,
    FloatType,
    BoolType,
}

impl TryFrom<Type> for TCAtomType {
    type Error = anyhow::Error;

    fn try_from(t: Type) -> Result<Self, Self::Error> {
        match t {
            Type::IntType => Ok(TCAtomType::IntType),
            Type::CIntType => Ok(TCAtomType::CIntType),
            Type::FloatType => Ok(TCAtomType::FloatType),
            Type::BoolType => Ok(TCAtomType::BoolType),
            Type::VoidType => Err(anyhow!("void type can't go here :(")),
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
                    .map(|x| {
                        let tcv: Result<TCVDecl> = x.clone().try_into();
                        tcv.map(|x| x.type_)
                    })
                    .collect::<Result<Vec<TCType>>>()?,
            ),
        ) {
            return Err(anyhow!("duplicate function name: {}", f.globid.clone()));
        }
        tcprog_funcs.push(typecheck_fn(f, &mut fn_name_to_type)?);
    }
    //    All programs must define exactly one function named “run” which returns an integer (the
    // program exit status) and takes no arguments.

    if let Some(run_fun_t) = fn_name_to_type.get("run") {
        if run_fun_t.0 == TCType::AtomType(TCAtomType::IntType) && run_fun_t.1.len() == 0 {
            Ok(TCProg {
                externs: tcprog_externs,
                funcs: tcprog_funcs,
            })
        } else {
            Err(anyhow!("run function has incorrect type"))
        }
    } else {
        Err(anyhow!("no function named run"))
    }
}
