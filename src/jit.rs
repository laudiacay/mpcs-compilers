use crate::ast::{BOp, Lit, UOp};
use crate::typecheck::{
    maybe_deref, TCAtomType, TCExp, TCExtern, TCFunc, TCProg, TCStmt, TCType, TypedExp,
};
use anyhow::{anyhow, Result};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::{Linkage, Module};
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, InstructionOpcode, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate, OptimizationLevel};
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::Path;
// may need pub fn set_triple(&self, triple: &TargetTriple)
// pub fn write_bitcode_to_path(&self, path: &Path) -> bool
//pub fn write_bitcode_to_memory(&self) -> MemoryBuffer

//pub fn verify(&self) -> Result<(), LLVMString>
//pub fn link_in_module(&self, other: Self) -> Result<(), LLVMString>

//extern fn printstmt(a: f64, b: f64) -> f64 {
//    a + b
//}

/*
#[no_mangle]
pub extern "C" fn kaleido_println() {
    println!("{:?}\n", x as u8 as char);
}
*/

static mut CMD_LINE_ARGS: Vec<String> = vec![];

#[no_mangle]
pub extern "C" fn arg(i: i32) -> i32 {
    unsafe {
        if i < CMD_LINE_ARGS.len() as i32 || i < 0 {
            return CMD_LINE_ARGS[i as usize].parse().unwrap();
        } else {
            println!("error: argument out of bounds");
            std::process::exit(1);
        }
    }
}

#[no_mangle]
pub extern "C" fn argf(i: i32) -> f64 {
    unsafe {
        if i < CMD_LINE_ARGS.len() as i32 || i < 0 as i32 {
            return CMD_LINE_ARGS[i as usize].parse().unwrap();
        } else {
            println!("error: argument out of bounds");
            std::process::exit(1);
        }
    }
}

#[no_mangle]
pub extern "C" fn __printint__(i: i32) {
    println!("{}", i);
}
#[no_mangle]
pub extern "C" fn __printbool__(b: bool) {
    println!("{}", b);
}
#[no_mangle]
pub extern "C" fn __printfloat__(f: f64) {
    println!("{}", f);
}
#[no_mangle]
pub extern "C" fn __printstr__(slit: u64) {
    let stri_bytes = unsafe { &*(slit as *const String) };
    println!("{}", stri_bytes);
}

// making sure rustc doesn't remove arg and argf
#[used]
static EXTERNAL_FNS1: [extern "C" fn(i32) -> i32; 1] = [arg];

#[used]
static EXTERNAL_FNS2: [extern "C" fn(i32) -> f64; 1] = [argf];
#[used]
static EXTERNAL_FNS3: [extern "C" fn(i32); 1] = [__printint__];
#[used]
static EXTERNAL_FNS4: [extern "C" fn(bool); 1] = [__printbool__];
#[used]
static EXTERNAL_FNS5: [extern "C" fn(f64); 1] = [__printfloat__];
#[used]
static EXTERNAL_FNS6: [extern "C" fn(u64); 1] = [__printstr__];

struct JitDoer<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    main_builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    // is this necessary to store...?
    current_fn_being_compiled: Option<FunctionValue<'ctx>>,
    current_fn_stack_variables: HashMap<String, (PointerValue<'ctx>, TCType)>,
}

impl<'ast: 'ctx, 'ctx> JitDoer<'ctx> {
    fn init(context: &'ctx Context, module_name: &str, opt_lvl: OptimizationLevel) -> Result<Self> {
        let module = context.create_module(module_name);
        let execution_engine = module
            .create_jit_execution_engine(opt_lvl)
            .expect("error! cannot create jit execution engine");
        let main_builder = context.create_builder();
        let ret = Self {
            context,
            module,
            main_builder,
            execution_engine,
            current_fn_being_compiled: None,
            current_fn_stack_variables: HashMap::new(),
        };
        ret.gen_print_externs();
        Ok(ret)
    }

    fn gen_print_externs(&self) {
        let void_type = self.context.void_type();
        let todo: Vec<(&str, BasicTypeEnum)> = vec![
            ("__printint__", self.context.i32_type().into()),
            ("__printfloat__", self.context.f64_type().into()),
            ("__printbool__", self.context.bool_type().into()),
        ];
        for (fn_name, arg_type) in todo.iter() {
            let args: Vec<BasicTypeEnum> = vec![arg_type.clone()];
            let fn_type = void_type.fn_type(args.as_slice(), false);
            self.module
                .add_function(fn_name, fn_type, Some(Linkage::ExternalWeak));
        }
        let fn_name = "__printstr__";
        let target_data = self.execution_engine.get_target_data();
        let ptr_type = self.context.ptr_sized_int_type(&target_data, None).into();
        let vec_size_type = self.context.i32_type().into();

        let args: Vec<BasicTypeEnum> = vec![ptr_type, vec_size_type];
        let fn_type = void_type.fn_type(args.as_slice(), false);
        self.module
            .add_function(fn_name, fn_type, Some(Linkage::ExternalWeak));
    }

    fn add_var_spot_to_fn_stack_frame(
        &mut self,
        argtype: BasicTypeEnum<'ctx>,
        tctype: TCType,
        varname: String,
    ) -> Result<PointerValue<'ctx>> {
        let bldr = self.context.create_builder();

        let entry = self
            .current_fn_being_compiled
            .unwrap()
            .get_first_basic_block()
            .unwrap();

        match entry.get_first_instruction() {
            Some(first_instr) => bldr.position_before(&first_instr),
            None => bldr.position_at_end(entry),
        }

        let var_spot = match argtype {
            BasicTypeEnum::FloatType(ft) => bldr.build_alloca(ft, &varname),
            BasicTypeEnum::IntType(it) => bldr.build_alloca(it, &varname),
            BasicTypeEnum::PointerType(pt) => bldr.build_alloca(pt, &varname),
            _ => Err(anyhow!("unsupported argument type to add to stack frame"))?,
        };
        self.current_fn_stack_variables
            .insert(varname, (var_spot, tctype));
        Ok(var_spot)
    }

    fn lift_extern(&mut self, extern_: TCExtern) -> Result<()> {
        let ret_type_: Option<BasicTypeEnum> = self.lift_type(extern_.type_.clone())?;
        let args: Vec<BasicTypeEnum> = extern_
            .args
            .iter()
            .map(|x| {
                self.lift_type(x.clone())
                    .and_then(|result| result.ok_or(anyhow!("func args cannot have void type")))
            })
            .collect::<Result<Vec<_>>>()?;
        let fn_type = match ret_type_ {
            None => self.context.void_type().fn_type(args.as_slice(), false),
            Some(basictype) => basictype.clone().fn_type(args.as_slice(), false),
        };
        self.module.add_function(
            &extern_.globid.clone(),
            fn_type,
            Some(Linkage::ExternalWeak),
        );
        Ok(())
    }

    fn lift_function(&mut self, func: TCFunc) -> Result<()> {
        let ret_type_: Option<BasicTypeEnum> = self.lift_type(func.type_.clone())?;
        let args: Vec<BasicTypeEnum> = func
            .args
            .iter()
            .map(|x| {
                self.lift_type(x.type_.clone())
                    .and_then(|result| result.ok_or(anyhow!("func args cannot have void type")))
            })
            .collect::<Result<Vec<_>>>()?;
        let fn_type = match ret_type_ {
            None => self.context.void_type().fn_type(args.as_slice(), false),
            Some(basictype) => basictype.fn_type(args.as_slice(), false),
        };
        let fn_ = self
            .module
            .add_function(&func.globid.clone(), fn_type, None);

        self.current_fn_being_compiled = Some(fn_);
        self.current_fn_stack_variables = HashMap::new();
        let function_block = self.context.append_basic_block(fn_, "entry");
        self.main_builder.position_at_end(function_block);

        for (i, arg) in fn_.get_param_iter().enumerate() {
            let arg_name = func.args[i].varid.clone();
            let arg_type = func.args[i].type_.clone();
            let alloca = self.add_var_spot_to_fn_stack_frame(args[i], arg_type, arg_name)?;
            self.main_builder.build_store(alloca, arg);
        }

        let _last_stmt = &func.blk.stmts.last().clone();

        if !self.lift_stmt(&TCStmt::Blk(func.blk))? {
            match ret_type_ {
                None => self.main_builder.build_return(None),
                Some(_) => self.main_builder.build_unreachable(),
            };
        };

        //if fn_.verify(true) {
        //    Ok(())
        //} else {
        //    Err(anyhow!("function did not verify"))
        //}
        Ok(())
    }

    fn lift_stmt(&mut self, stmt: &TCStmt) -> Result<bool> {
        match stmt {
            TCStmt::Blk(blk) => {
                /*
                 * int x = 4;
                 * int y = 5;
                 * ref int z = x;
                 * {
                 *   int x = 7;
                 *   y = 9;
                 *   z; // 4
                 * }
                 * x; // 4
                 * y; // 9
                 * z; // 4
                 */
                let parent_block_scope = self.current_fn_stack_variables.clone();

                let mut returns = false;
                for inner_stmt in blk.stmts.iter() {
                    if self.lift_stmt(inner_stmt)? {
                        returns = true;
                        break;
                    }
                }

                self.current_fn_stack_variables = parent_block_scope;
                Ok(returns)
            }
            TCStmt::ReturnStmt(ret) => {
                match ret {
                    Some(ret) => {
                        // unwrapping and rewrapping because of some referencing bullshit
                        // unwrap can't fail because return expression can't be void and the
                        // typechecker confirms this
                        let lifted_ret = self.lift_exp(ret)?.unwrap();
                        self.main_builder.build_return(Some(&lifted_ret));
                    }
                    None => {
                        self.main_builder.build_return(None);
                    }
                };
                Ok(true)
            }
            TCStmt::VDeclStmt { vdecl, exp } => {
                let lifted_type = self.lift_type(vdecl.type_)?.unwrap();
                let var = self.add_var_spot_to_fn_stack_frame(
                    lifted_type,
                    vdecl.type_,
                    vdecl.varid.clone(),
                )?;

                if let TCType::Ref(_, _) = vdecl.type_ {
                    if let TCExp::VarVal(tar_varid) = &exp.exp {
                        let (tar_ptr, tar_type) = self
                            .current_fn_stack_variables
                            .get(&tar_varid.clone())
                            .unwrap();
                        if let TCType::Ref(_, _) = tar_type {
                            let val = self.main_builder.build_load(*tar_ptr, "load");
                            self.main_builder.build_store(var, val);
                        } else {
                            self.main_builder.build_store(var, *tar_ptr);
                        }
                    } else {
                        Err(anyhow!(
                            "reference type assigned to non-variable (likely bug in typechecker)"
                        ))?
                    }
                } else {
                    let lifted_exp = self.lift_exp(exp)?.unwrap();
                    self.main_builder.build_store(var, lifted_exp);
                };
                Ok(false)
            }
            TCStmt::ExpStmt(exp) => {
                self.lift_exp(exp)?;
                Ok(false)
            }
            TCStmt::WhileStmt { cond, stmt: body } => {
                // unwrapping this means that a while statement can only be encountered within a
                // function body, which is right
                let parent = self.current_fn_being_compiled.unwrap();
                let lifted_cond = self.lift_exp(cond)?.unwrap().into_int_value();
                let loop_bb = self.context.append_basic_block(parent, "loop"); // loop body
                let post_bb = self.context.append_basic_block(parent, "endwhile"); // end of loop

                // check condition to see whether to enter loop at all
                self.main_builder
                    .build_conditional_branch(lifted_cond, loop_bb, post_bb);

                // execute body and check condition again
                self.main_builder.position_at_end(loop_bb);
                if !self.lift_stmt(body)? {
                    let end_cond = self.lift_exp(cond)?.unwrap().into_int_value();
                    self.main_builder
                        .build_conditional_branch(end_cond, loop_bb, post_bb);
                }

                // end of loop
                self.main_builder.position_at_end(post_bb);
                Ok(false)
            }
            TCStmt::IfStmt {
                cond,
                stmt: body,
                else_stmt,
            } => {
                let parent = self.current_fn_being_compiled.unwrap();
                let lifted_cond = self.lift_exp(cond)?.unwrap().into_int_value();
                let body_bb = self.context.append_basic_block(parent, "if");
                // if there is only an if,
                // build it, don't care whether returns for this functions retval
                // always include postbb and a jump in case
                match else_stmt {
                    None => {
                        let post_bb = self.context.append_basic_block(parent, "endif");
                        self.main_builder
                            .build_conditional_branch(lifted_cond, body_bb, post_bb);
                        self.main_builder.position_at_end(body_bb);
                        if !self.lift_stmt(body)? {
                            self.main_builder.build_unconditional_branch(post_bb);
                        }
                        self.main_builder.position_at_end(post_bb);
                        Ok(false)
                    }
                    Some(real_else_stmt_of_atlanta) => {
                        let else_bb = self.context.append_basic_block(parent, "else");
                        self.main_builder
                            .build_conditional_branch(lifted_cond, body_bb, else_bb);
                        self.main_builder.position_at_end(body_bb);
                        let if_returns = self.lift_stmt(body)?;
                        self.main_builder.position_at_end(else_bb);
                        let else_returns = self.lift_stmt(real_else_stmt_of_atlanta)?;
                        if if_returns && else_returns {
                            Ok(true)
                        } else {
                            let post_bb = self.context.append_basic_block(parent, "endif");
                            if !if_returns {
                                self.main_builder.position_at_end(body_bb);
                                self.main_builder.build_unconditional_branch(post_bb);
                            }
                            if !else_returns {
                                self.main_builder.position_at_end(else_bb);
                                self.main_builder.build_unconditional_branch(post_bb);
                            }
                            self.main_builder.position_at_end(post_bb);
                            Ok(false)
                        }
                    }
                }
            }
            TCStmt::PrintStmt(exp) => {
                let lifted_exp = self.lift_exp(exp)?.unwrap();
                let globid = match exp.type_ {
                    TCType::AtomType(TCAtomType::IntType) => "__printint__",
                    TCType::AtomType(TCAtomType::BoolType) => "__printbool__",
                    TCType::AtomType(TCAtomType::FloatType) => "__printfloat__",
                    TCType::AtomType(TCAtomType::CIntType) => "__printint__",
                    _ => unimplemented!("cant print refs yikes"),
                };

                let func = self.module.get_function(globid).unwrap();
                // lift args
                let args = vec![lifted_exp];

                // build call using lifted args
                // unwrap is safe here because typechecker guarantees function arguments are not void
                let args_arr = args
                    .iter()
                    .by_ref()
                    .map(|&val| val.into())
                    .collect::<Vec<BasicValueEnum>>();

                self.main_builder
                    .build_call(func, args_arr.as_slice(), "call");
                /*
                match call.try_as_basic_value().left() {
                    Some(val) => val,
                    None => Err(anyhow!("invalid function call"))?,
                };
                */
                Ok(false)
            }
            TCStmt::PrintStmtSlit(strlit) => {
                let strlen_arg = self
                    .context
                    .i32_type()
                    .const_int(strlit.len().try_into().unwrap(), false)
                    .into();

                let target_data = self.execution_engine.get_target_data();
                let strlit_2 = &strlit[1..strlit.len() - 1].to_string();
                let strlit_box = Box::new(strlit_2.clone());
                let strlit_box_leaked = Box::leak(strlit_box);
                let strlit_ptr_arg = self
                    .context
                    .ptr_sized_int_type(&target_data, None)
                    .const_int(strlit_box_leaked as *const String as u64, false)
                    .into();
                let func = self.module.get_function("__printstr__").unwrap();
                let _ptrval = strlit_box_leaked as *const String as u64;

                self.main_builder
                    .build_call(func, &[strlit_ptr_arg, strlen_arg], "call");
                Ok(false)
            }
        }
    }

    // check the type of an expression and get its value
    fn lift_exp(&self, exp: &TypedExp) -> Result<Option<BasicValueEnum<'ctx>>> {
        match exp.type_ {
            TCType::AtomType(_tca) => self.lift_tcexp(&exp.exp),
            TCType::VoidType => self.lift_exp_to_void(&exp.exp),
            _ => Err(anyhow!(
                "typed expression with reference type (likely a bug)"
            ))?,
        }
    }

    fn lift_exp_to_void(&self, exp: &TCExp) -> Result<Option<BasicValueEnum<'ctx>>> {
        if let TCExp::FuncCall { globid, exps } = exp {
            let func = self.module.get_function(globid.as_str()).unwrap();

            // lift args
            let mut args = Vec::with_capacity(exps.len());
            for (i, e) in exps.iter().enumerate() {
                let param = func
                    .get_nth_param(i as u32)
                    .ok_or(anyhow!("couldn't get function parameter"))?;
                if let BasicValueEnum::PointerValue(_) = param {
                    if let TCExp::VarVal(varid) = &e.exp {
                        let the_variable = self
                            .current_fn_stack_variables
                            .get(varid)
                            .ok_or(anyhow!("no such variable (bug)"))?
                            .0;
                        args.push(BasicValueEnum::PointerValue(the_variable));
                    } else {
                        Err(anyhow!("expected varval in reference type argument"))?;
                    }
                } else {
                    args.push(self.lift_exp(&e)?.unwrap());
                }
            }

            // build call using lifted args
            let args_arr = args
                .iter()
                .by_ref()
                .map(|&val| val.into())
                .collect::<Vec<BasicValueEnum>>();

            self.main_builder
                .build_call(func, args_arr.as_slice(), "call");
        /*
        let func_opt = self.module.get_function(globid.as_str());
        if let None = func_opt {
            // should be unreachable, as this would be caught during typechecking
            Err(anyhow!("unknown function"))?;
        }
        let func = func_opt.unwrap();

        // lift args
        let mut args = vec![];
        for e in exps {
            args.push(self.lift_exp(&e)?);
        }

        // build call using lifted args
        // unwrap is safe here because typechecker guarantees function arguments are not void
        let args_arr = args
            .iter()
            .by_ref()
            .map(|&val| val.unwrap().into())
            .collect::<Vec<BasicValueEnum>>();

        // don't need to save the value as this is only called for void functions
        // but do we need to check if the call itself is valid?
        self.main_builder
            .build_call(func, args_arr.as_slice(), "call"); // wtf is the 'name' supposed to be
        */
        } else {
            // should be unreachable
            Err(anyhow!(
                "found expression of void type other than function call"
            ))?;
        }
        Ok(None)
    }

    fn lift_tcexp(&self, exp: &TCExp) -> Result<Option<BasicValueEnum<'ctx>>> {
        let val = match exp {
            TCExp::Assign { varid, exp } => {
                let ass_val = self.lift_exp(&exp)?.unwrap();
                let (var, type_) = self.current_fn_stack_variables.get(varid.as_str()).unwrap(); // variable verified by typechecker already

                if let TCType::Ref(_, _) = type_ {
                    let ptr = self.main_builder.build_load(*var, "load");
                    self.main_builder
                        .build_store(ptr.into_pointer_value(), ass_val);
                } else {
                    self.main_builder.build_store(*var, ass_val);
                }

                ass_val
            }
            TCExp::Cast { type_, exp } => {
                // if casting to/from voids isn't caught by our typechecker im leaving this planet
                let lifted_exp = self.lift_exp(&exp)?.unwrap();
                let lifted_type = self.lift_type(*type_)?.unwrap();

                // I have no idea if it makes sense to use this or build_int_cast
                // from https://llvm.org/docs/LangRef.html#bitcast-to-instruction bitcast means to
                // cast w/o changing any bits
                match type_ {
                    TCType::AtomType(TCAtomType::IntType) => match exp.type_ {
                        TCType::AtomType(TCAtomType::IntType) => lifted_exp,
                        TCType::AtomType(TCAtomType::FloatType) => {
                            let cast = self.main_builder.build_cast(
                                InstructionOpcode::FPToSI,
                                lifted_exp,
                                lifted_type,
                                "cast",
                            );
                            cast
                        }
                        TCType::AtomType(TCAtomType::CIntType) => {
                            unimplemented!("cint");
                        }
                        _ => Err(anyhow!("unsupported cast"))?,
                    },
                    TCType::AtomType(TCAtomType::FloatType) => match exp.type_ {
                        TCType::AtomType(TCAtomType::IntType) => {
                            let cast = self.main_builder.build_cast(
                                InstructionOpcode::SIToFP,
                                lifted_exp,
                                lifted_type,
                                "cast",
                            );
                            cast
                        }
                        TCType::AtomType(TCAtomType::FloatType) => lifted_exp,
                        TCType::AtomType(TCAtomType::CIntType) => {
                            unimplemented!("cint");
                        }
                        _ => Err(anyhow!("unsupported cast"))?,
                    },
                    TCType::AtomType(TCAtomType::CIntType) => match exp.type_ {
                        TCType::AtomType(TCAtomType::IntType) => {
                            unimplemented!("cint");
                        }
                        TCType::AtomType(TCAtomType::FloatType) => unimplemented!("cint"),
                        TCType::AtomType(TCAtomType::CIntType) => lifted_exp,
                        _ => Err(anyhow!("unsupported cast"))?,
                    },
                    TCType::AtomType(TCAtomType::BoolType) => lifted_exp,
                    _ => Err(anyhow!("nonatomic type came out of cast, this makes no sense,"))?,
                }
            }
            TCExp::BinOp { op, lhs, rhs } => {
                let lifted_lhs = self.lift_exp(&lhs)?.unwrap();
                let lifted_rhs = self.lift_exp(&rhs)?.unwrap();
                match (lifted_lhs, lifted_rhs) {
                    (BasicValueEnum::IntValue(lhs_val), BasicValueEnum::IntValue(rhs_val)) => {
                        let checked_overflow = maybe_deref(lhs.type_)? == TCAtomType::CIntType;

                        match op {
                            BOp::Add => BasicValueEnum::IntValue(if !checked_overflow {
                                self.main_builder.build_int_add(lhs_val, rhs_val, "add")
                            } else {
                                unimplemented!("cint");
                            }),
                            BOp::Sub => BasicValueEnum::IntValue(if !checked_overflow {
                                self.main_builder.build_int_sub(lhs_val, rhs_val, "sub")
                            } else {
                                unimplemented!("cint");
                            }),
                            BOp::Mult => BasicValueEnum::IntValue(if !checked_overflow {
                                self.main_builder.build_int_mul(lhs_val, rhs_val, "mul")
                            } else {
                                unimplemented!("cint");
                            }),
                            BOp::Div => BasicValueEnum::IntValue(
                                self.main_builder
                                    .build_int_signed_div(lhs_val, rhs_val, "div"),
                            ),
                            BOp::EqTo => {
                                BasicValueEnum::IntValue(self.main_builder.build_int_compare(
                                    IntPredicate::EQ,
                                    lhs_val,
                                    rhs_val,
                                    "eq",
                                ))
                            }
                            BOp::Gt => {
                                BasicValueEnum::IntValue(self.main_builder.build_int_compare(
                                    IntPredicate::SGT,
                                    lhs_val,
                                    rhs_val,
                                    "gt",
                                ))
                            }
                            BOp::Lt => {
                                BasicValueEnum::IntValue(self.main_builder.build_int_compare(
                                    IntPredicate::SLT,
                                    lhs_val,
                                    rhs_val,
                                    "lt",
                                ))
                            }
                            BOp::And => BasicValueEnum::IntValue(
                                self.main_builder.build_and(lhs_val, rhs_val, "and"),
                            ),
                            BOp::Or => BasicValueEnum::IntValue(
                                self.main_builder.build_or(lhs_val, rhs_val, "or"),
                            ),
                        }
                    }
                    (BasicValueEnum::FloatValue(lhs_val), BasicValueEnum::FloatValue(rhs_val)) => {
                        match op {
                            BOp::Add => BasicValueEnum::FloatValue(
                                self.main_builder.build_float_add(lhs_val, rhs_val, "add"),
                            ),
                            BOp::Sub => BasicValueEnum::FloatValue(
                                self.main_builder.build_float_sub(lhs_val, rhs_val, "sub"),
                            ),
                            BOp::Mult => BasicValueEnum::FloatValue(
                                self.main_builder.build_float_mul(lhs_val, rhs_val, "mul"),
                            ),
                            BOp::Div => BasicValueEnum::FloatValue(
                                self.main_builder.build_float_div(lhs_val, rhs_val, "div"),
                            ),
                            BOp::EqTo => {
                                BasicValueEnum::IntValue(self.main_builder.build_float_compare(
                                    FloatPredicate::UEQ,
                                    lhs_val,
                                    rhs_val,
                                    "eq",
                                ))
                            }
                            BOp::Gt => {
                                BasicValueEnum::IntValue(self.main_builder.build_float_compare(
                                    FloatPredicate::UGT,
                                    lhs_val,
                                    rhs_val,
                                    "gt",
                                ))
                            }
                            BOp::Lt => {
                                BasicValueEnum::IntValue(self.main_builder.build_float_compare(
                                    FloatPredicate::ULT,
                                    lhs_val,
                                    rhs_val,
                                    "lt",
                                ))
                            }
                            _ => Err(anyhow!(
                                "illegal operation on float values (most likely 'and' or 'or')"
                            ))?,
                        }
                    }
                    // should be unreachable due to typechecker
                    _ => Err(anyhow!("mismatched binop types!!"))?,
                }
            }
            TCExp::UnaryOp { op, exp } => {
                let lifted_exp = self.lift_exp(&exp)?.unwrap();

                match lifted_exp {
                    BasicValueEnum::IntValue(val) => match op {
                        UOp::SignedNeg => {
                            BasicValueEnum::IntValue(self.main_builder.build_int_neg(val, "neg"))
                        }
                        UOp::BitwiseNeg => {
                            BasicValueEnum::IntValue(self.main_builder.build_not(val, "not"))
                        }
                    },
                    BasicValueEnum::FloatValue(val) => match op {
                        UOp::SignedNeg => BasicValueEnum::FloatValue(
                            self.main_builder.build_float_neg(val, "neg"),
                        ),
                        _ => Err(anyhow!("invalid op in float-typed unary expression"))?,
                    },
                    _ => Err(anyhow!("invalid value in unary expression"))?,
                }
            }
            TCExp::Literal(lit) => match lit {
                Lit::LitInt(i) => {
                    BasicValueEnum::IntValue(self.context.i32_type().const_int(*i as u64, true))
                }
                Lit::LitFloat(i) => {
                    BasicValueEnum::FloatValue(self.context.f64_type().const_float(*i))
                }
                Lit::LitBool(i) => {
                    BasicValueEnum::IntValue(self.context.bool_type().const_int(*i as u64, true))
                }
            },
            TCExp::VarVal(varid) => {
                // typechecker makes sure variable is in scope here
                let var = self.current_fn_stack_variables.get(varid.as_str()).unwrap();
                match var.1 {
                    TCType::Ref(_, _type_) => {
                        let loc1 = self.main_builder.build_load(var.0, varid.as_str());
                        self.main_builder
                            .build_load(loc1.into_pointer_value(), varid.as_str())
                    }
                    TCType::AtomType(_) => self.main_builder.build_load(var.0, varid.as_str()),
                    _ => Err(anyhow!("void variable spooooky ooooo!!!"))?,
                }
            }
            TCExp::FuncCall { globid, exps } => {
                // missing function caught during typechecking
                let func = self.module.get_function(globid.as_str()).unwrap();

                // lift args
                let mut args = Vec::with_capacity(exps.len());
                for (i, e) in exps.iter().enumerate() {
                    let param = func
                        .get_nth_param(i as u32)
                        .ok_or(anyhow!("couldn't get function parameter"))?;
                    if let BasicValueEnum::PointerValue(_) = param {
                        if let TCExp::VarVal(varid) = &e.exp {
                            let the_variable = self
                                .current_fn_stack_variables
                                .get(varid)
                                .ok_or(anyhow!("no such variable (bug)"))?
                                .0;
                            args.push(BasicValueEnum::PointerValue(the_variable));
                        } else {
                            Err(anyhow!("expected varval in reference type argument"))?;
                        }
                    } else {
                        args.push(self.lift_exp(&e)?.unwrap());
                    }
                }

                // build call using lifted args
                let args_arr = args
                    .iter()
                    .by_ref()
                    .map(|&val| val.into())
                    .collect::<Vec<BasicValueEnum>>();

                let call = self
                    .main_builder
                    .build_call(func, args_arr.as_slice(), "call");
                match call.try_as_basic_value().left() {
                    Some(val) => val,
                    None => Err(anyhow!("invalid function call"))?,
                }
            }
        };
        Ok(Some(val))
    }

    fn lift_type(&self, type_: TCType) -> Result<Option<BasicTypeEnum<'ctx>>> {
        Ok(match type_ {
            TCType::AtomType(type_) => Some(self.lift_atom_type(type_)?),
            TCType::VoidType => None,
            // TODO: WHAT IS HAPPENING WITH NOALIAS????
            TCType::Ref(_noalias, type_) => Some(
                self.lift_atom_type(type_)?
                    .ptr_type(AddressSpace::Generic)
                    .into(),
            ),
        })
    }

    fn lift_atom_type(&self, type_: TCAtomType) -> Result<BasicTypeEnum<'ctx>> {
        Ok(match type_ {
            TCAtomType::IntType => self.context.i32_type().into(),
            TCAtomType::CIntType => Err(anyhow!("cints not supported yet"))?,
            TCAtomType::FloatType => self.context.f64_type().into(),
            TCAtomType::BoolType => self.context.bool_type().into(),
        })
    }
}

type KaleidoRunFunc = unsafe extern "C" fn() -> i32;

fn jit_compile_kaleido_prog<'a>(
    ctxt: &'a Context,
    toplvl_filename: &'a str,
    ast: TCProg,
    opt: bool,
) -> Result<JitFunction<'a, KaleidoRunFunc>> {
    //https://thedan64.github.io/inkwell/inkwell/enum.OptimizationLevel.html
    let mut jit_doer = JitDoer::init(ctxt, toplvl_filename, OptimizationLevel::None)?;
    for e in ast.externs {
        // what do i do with this???
        let _ext = jit_doer.lift_extern(e)?;
    }
    for f in ast.funcs {
        // what do i do with this???
        let _fn = jit_doer.lift_function(f)?;
    }

    if opt {
        optimize(&jit_doer.module);
    }

    // pull out jitted run function and OFF we go!!
    let efn = unsafe { jit_doer.execution_engine.get_function("run")? };
    Ok(efn)
}

pub fn jit(input_filename: &str, ast: TCProg, args: Vec<String>, opt: bool) -> Result<i32> {
    let ctxt = Context::create();
    unsafe { CMD_LINE_ARGS = args };
    let func = jit_compile_kaleido_prog(&ctxt, input_filename, ast, opt)?;
    Ok(unsafe { func.call() })
}

pub fn emit_llvm(
    input_filename: &str,
    output_filename: &str,
    ast: TCProg,
    opt: bool,
) -> Result<()> {
    let ctxt = Context::create();
    let mut jit_doer = JitDoer::init(&ctxt, input_filename, OptimizationLevel::None)?;
    for e in ast.externs {
        // what do i do with this???
        let _ext = jit_doer.lift_extern(e)?;
    }
    for f in ast.funcs {
        // what do i do with this???
        let _fn = jit_doer.lift_function(f)?;
    }

    if opt {
        optimize(&jit_doer.module);
    }

    match jit_doer.module.print_to_file(Path::new(output_filename)) {
        Ok(_) => Ok(()),
        Err(msg) => {
            println!("{:?}", msg);
            Err(anyhow!("couldn't print module"))
        }
    }
}

// run the optimization pipeline for the given module
fn optimize(module: &Module) {
    let pass_manager_builder = PassManagerBuilder::create();
    pass_manager_builder.set_optimization_level(OptimizationLevel::Aggressive);

    let fpm = PassManager::create(module);
    pass_manager_builder.populate_function_pass_manager(&fpm);
    let mut maybe_cur_fn = module.get_first_function();
    loop {
        if let Some(cur_fn) = maybe_cur_fn {
            fpm.run_on(&cur_fn);
            maybe_cur_fn = cur_fn.get_next_function();
        } else {
            break;
        }
    }
}
