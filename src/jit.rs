use inkwell::context::Context;
use inkwell::module::{Module, Linkage};
use inkwell::types::{FunctionType, BasicTypeEnum};
use inkwell::execution_engine::ExecutionEngine
use inkwell::values::FunctionValue;
use self::inkwell::builder::Builder;


// may need pub fn set_triple(&self, triple: &TargetTriple)
// pub fn write_bitcode_to_path(&self, path: &Path) -> bool
//pub fn write_bitcode_to_memory(&self) -> MemoryBuffer

//pub fn verify(&self) -> Result<(), LLVMString>
//pub fn link_in_module(&self, other: Self) -> Result<(), LLVMString>

//extern fn printstmt(a: f64, b: f64) -> f64 {
//    a + b
//}
pub struct JitProgram {
    context : Context,
    module : Module,
    builder: Builder,
    execution_engine : ExecutionEngine,
}

impl JitProgram {
    fn init(module_name: &str, opt_lvl: OptimizationLevel) -> Result<Self> {
        let context = Context::create();
        let module = context.create_module(module_name);
        //https://thedan64.github.io/inkwell/inkwell/enum.OptimizationLevel.html
        let execution_engine = module.create_jit_execution_engine(opt_lvl)?;
        let builder = context.create_builder();
        Ok(Jit {
            context, module, builder, execution_engine
        })
    }

    fn lift_extern(&self, extern_: TCExtern) -> Result<FunctionValue> {
        let ret_type_ = self.lift_type(extern_.type_)?;
        let args : Vec<BasicTypeEnum> = extern_.args.map(|x| self.lift_type(x)?).into();
        let fn_type = ret_type_.fn_type(args, false);
        Ok(self.module.add_function(extern_.globid, fn_type, Some(Linkage::ExternalWeak)))
    }
    fn lift_function(&self, func: TCFunc) -> Result<FunctionValue> {
        let ret_type_ = self.lift_type(func.type_)?;
        let args : Vec<BasicTypeEnum> = func.args.map(|x| self.lift_type(x.type_)?).into();
        let fn_type = ret_type_.fn_type(args, false);
        let fn_ = self.module.add_function(fn_.globid, fn_.type, Some(Linkage::ExternalWeak));
        for (i, arg) in fn_.get_param_iter().enumerate() {
            arg.set_name(func.args[i].varid);
        }
        // TODO: parse the block and add it
        // TODO: wtf is happening with builders
        self.context.append_basic_block(fn_, fn_.globid);

    }

    fn lift_type(&self, type_: TCType) -> Result<BasicTypeEnum> {
        match type_ {
            AtomType(type_) => self.lift_atom_type(type_),
            VoidType => self.context.void_type(),
            // TODO: WHAT IS HAPPENING WITH NOALIAS????
            Ref(noalias, type_)=> self.lift_atom_type(type_).ptr_type(AddressSpace::Generic),
        }
    }
    fn lift_atom_type(&self, type_: TCAtomType) -> Result<BasicTypeEnum> {
        Ok(match type_ {
            IntType => self.context.i32_type(),
            CIntType => Err(anyhow!("cints not supported yet"))?,
            FloatType => self.context.f64_type(),
            BoolType => self.context.bool_type(),
        })
    }
}
