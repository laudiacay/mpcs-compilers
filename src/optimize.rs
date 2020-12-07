use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::module::Module;
use inkwell::OptimizationLevel;

// flags indicating which optimization passes to perform
// names of each field correspond to snake-cased names of passes from the inkwell module
// arguments are in the order that they are performed in run_pipeline
// note: default for bools is false, so a default OFlag turns every flag off
#[derive(Debug, Default, PartialEq)]
pub struct OFlags {
    // function fuckery eeeuuuugh
    pub basic_alias_analysis: bool,
    pub argument_promotion: bool,
    pub function_inlining: bool,

    // control flow/dead code
    pub cfg_simplification: bool,
    pub aggressive_dce: bool,
    pub strip_dead_prototypes: bool,

    // loop optimizations
    pub ind_var_simplify: bool,
    pub loop_vectorize: bool,

    // constant shit idk
    pub reassociate: bool,
    pub sccp: bool,

    // weird asm
    pub instruction_combining: bool,
    pub promote_memory_to_register: bool,

    // cleanup
    pub dead_arg_elimination: bool,
}

// perform the pipeline specified by oflags on the given module
pub fn run_pipeline(module: &Module, oflags: OFlags) {
    // ??????????????
    let pm = PassManager::create(());

    if oflags.basic_alias_analysis {
        pm.add_basic_alias_analysis_pass();
    }
    if oflags.argument_promotion {
        pm.add_argument_promotion_pass();
    }
    if oflags.function_inlining {
        pm.add_function_inlining_pass();
    }
    if oflags.cfg_simplification {
        pm.add_cfg_simplification_pass();
    }
    if oflags.aggressive_dce {
        pm.add_aggressive_dce_pass();
    }
    if oflags.strip_dead_prototypes {
        pm.add_strip_dead_prototypes_pass();
    }
    if oflags.ind_var_simplify {
        pm.add_ind_var_simplify_pass();
    }
    if oflags.loop_vectorize {
        pm.add_loop_vectorize_pass();
    }
    if oflags.reassociate {
        pm.add_reassociate_pass();
    }
    if oflags.sccp {
        pm.add_sccp_pass();
    }
    if oflags.instruction_combining {
        pm.add_instruction_combining_pass();
    }
    if oflags.promote_memory_to_register {
        pm.add_promote_memory_to_register_pass();
    }
    if oflags.dead_arg_elimination {
        pm.add_dead_arg_elimination_pass();
    }

    // ??????????????????????????????????????
    pm.run_on(module);
    pm.run_on(module);
}

// run the default pipeline on each function
pub fn run_default_pipeline(module: &Module) {
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
