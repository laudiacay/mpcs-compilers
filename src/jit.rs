use crate::typecheck::{TCAtomType, TCExtern, TCFunc, TCProg, TCType};
use anyhow::{anyhow, Result};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicType, BasicTypeEnum, AnyTypeEnum};
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;
use std::collections::HashMap;
// may need pub fn set_triple(&self, triple: &TargetTriple)
// pub fn write_bitcode_to_path(&self, path: &Path) -> bool
//pub fn write_bitcode_to_memory(&self) -> MemoryBuffer

//pub fn verify(&self) -> Result<(), LLVMString>
//pub fn link_in_module(&self, other: Self) -> Result<(), LLVMString>

//extern fn printstmt(a: f64, b: f64) -> f64 {
//    a + b
//}

/*#[no_mangle]
pub extern "C" fn kaleido_println() {
    println!("{:?}\n", x as u8 as char);
}
#[no_mangle]
pub extern "C" fn arg() {
    println!("{:?}\n", x as u8 as char);
}
#[no_mangle]
pub extern "C" fn argf() -> {
    println!("{:?}\n", x as u8 as char);
}
// making sure rustc doesn't remove kaleido_println
#[used]
static EXTERNAL_FNS: [extern fn(f64) -> f64; 1] = [kaleido_println];
*/

struct JitDoer<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    main_builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    // is this necessary to store...?
    current_fn_being_compiled: Option<FunctionValue<'ctx>>,
    // unsure if this remains correct after like injecting instructions... hm
    current_fn_vdecl_builder: Option<Builder<'ctx>>,
    current_fn_stack_variables: Option<HashMap<String, PointerValue<'ctx>>>,
}

impl<'ctx> JitDoer<'ctx> {
    fn init(context: &'ctx Context, module_name: &str, opt_lvl: OptimizationLevel) -> &'ctx Self {
        let module = context.create_module(module_name);
        let execution_engine = module
            .create_jit_execution_engine(opt_lvl)
            .expect("error! cannot create jit execution engine");
        let main_builder = context.create_builder();
        &Self {
            context,
            module,
            main_builder,
            execution_engine,
            current_fn_being_compiled: None,
            current_fn_vdecl_builder: None,
            current_fn_stack_variables: None,
        }
    }

    fn add_var_spot_to_fn_stack_frame(
        &self,
        argtype: BasicTypeEnum<'ctx>,
        varname: String,
    ) -> Result<PointerValue<'ctx>> {
        let bldr = self.current_fn_vdecl_builder
            .ok_or(anyhow!("vdecl builder unset!"))?;
        let var_spot = match argtype {
            BasicTypeEnum::FloatType(ft) => bldr.build_alloca(ft, &varname),
            BasicTypeEnum::IntType(it) => bldr.build_alloca(it, &varname),
            BasicTypeEnum::PointerType(pt) => bldr.build_alloca(pt, &varname),
        };
        self.current_fn_stack_variables
            .ok_or(anyhow!("fn stack vars unset"))?
            .insert(varname, var_spot);

        Ok(var_spot)
    }

    fn lift_extern<'a>(&self, extern_: &'a TCExtern) -> Result<FunctionValue> {
        let ret_type_ : Option<BasicTypeEnum> = self.lift_type(extern_.type_)?;
        let args: Vec<BasicTypeEnum> = extern_
            .args
            .iter()
            .map(|x| self.lift_type(*x).and_then(|result| result.ok_or(anyhow!("func args cannot have void type"))))
            .collect::<Result<Vec<_>>>()?;
        let fn_type = ret_type_.fn_type(args.as_slice(), false);
        Ok(self.module.add_function(
            &extern_.globid.clone(),
            fn_type,
            Some(Linkage::ExternalWeak),
        ))
    }

    fn lift_function<'a>(&self, func: &'a TCFunc) -> Result<FunctionValue> {
        let ret_type_ : Option<BasicTypeEnum> = self.lift_type(func.type_)?;
        let args: Vec<BasicTypeEnum> = func
            .args
            .iter()
            .map(|x| self.lift_type(x.type_).and_then(|result| result.ok_or(anyhow!("func args cannot have void type"))))
            .collect::<Result<Vec<_>>>()?;
        let fn_type = match ret_type_ {
            None => self.context.void_type().fn_type(args.as_slice(), false),
            Some(basictype) => basictype.fn_type(args.as_slice(), false),
        };
        let fn_ =
            self.module
                .add_function(&func.globid.clone(), fn_type, Some(Linkage::ExternalWeak));

        self.current_fn_being_compiled = Some(fn_);
        self.current_fn_stack_variables = Some(HashMap::new());
        let function_block = self.context.append_basic_block(fn_, "fnblock");
        self.main_builder.position_at_end(function_block);
        self.current_fn_vdecl_builder = Some(self.context.create_builder());
        self.current_fn_vdecl_builder
            .unwrap()
            .position_at_end(function_block);
        for (i, arg) in args.iter().enumerate() {
            // add space for each argument to the builder
            let varname = func.args[i].varid;
            self.add_var_spot_to_fn_stack_frame(*arg, varname);
        }
        //TODO: i parse the BasicBlock and add it

        Ok(fn_)
    }

    fn lift_type(&self, type_: TCType) -> Result<Option<BasicTypeEnum>> {
        Ok(match type_ {
            TCType::AtomType(type_) => Some(self.lift_atom_type(type_)?),
            TCType::VoidType =>None ,
            // TODO: WHAT IS HAPPENING WITH NOALIAS????
            TCType::Ref(noalias, type_) => {
                Some(self.lift_atom_type(type_)?.ptr_type(AddressSpace::Generic).into())
            }
        })
    }
    fn lift_atom_type(&self, type_: TCAtomType) -> Result<BasicTypeEnum> {
        Ok(match type_ {
            TCAtomType::IntType => self.context.i32_type().into(),
            TCAtomType::CIntType => Err(anyhow!("cints not supported yet"))?,
            TCAtomType::FloatType => self.context.f64_type().into(),
            TCAtomType::BoolType => self.context.bool_type().into(),
        })
    }
}

type KaleidoRunFunc = unsafe extern "C" fn(Vec<i32>, Vec<f64>) -> i32;

fn jit_compile_kaleido_prog(
    toplvl_filename: &str,
    ast: TCProg,
) -> Result<JitFunction<KaleidoRunFunc>> {
    let context = Context::create();
    //https://thedan64.github.io/inkwell/inkwell/enum.OptimizationLevel.html
    let jit_doer = JitDoer::init(&context, toplvl_filename, OptimizationLevel::None);
    for e in ast.externs {
        // what do i do with this???
        let _ext = jit_doer.lift_extern(&e);
    }
    for f in ast.funcs {
        // what do i do with this???
        let _fn = jit_doer.lift_function(&f);
    }
    // pull out jitted run function and OFF we go!!
    unimplemented!("not done");
}

pub fn jit(input_filename: &str, ast: TCProg) -> Result<i32> {
    let func = jit_compile_kaleido_prog(input_filename, ast)?;
    unimplemented!("need to capture arguments and cast?");
    Ok(unsafe { func.call(vec![], vec![]) })
}
