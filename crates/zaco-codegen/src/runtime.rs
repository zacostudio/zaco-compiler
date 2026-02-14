//! Runtime function declarations for memory management and utilities

use cranelift::prelude::*;
use cranelift_module::{FuncId as ClifFuncId, Linkage, Module};
use cranelift_object::ObjectModule;

use crate::error::CodegenError;

/// Runtime function declarations for memory management and utilities
#[derive(Default)]
pub(crate) struct RuntimeFunctions {
    pub(crate) zaco_alloc: Option<ClifFuncId>,
    pub(crate) zaco_free: Option<ClifFuncId>,
    pub(crate) zaco_rc_inc: Option<ClifFuncId>,
    pub(crate) zaco_rc_dec: Option<ClifFuncId>,
    pub(crate) zaco_str_concat: Option<ClifFuncId>,
    pub(crate) zaco_str_new: Option<ClifFuncId>,
    pub(crate) zaco_print_str: Option<ClifFuncId>,
    pub(crate) zaco_print_i64: Option<ClifFuncId>,
    pub(crate) zaco_print_f64: Option<ClifFuncId>,
    pub(crate) zaco_print_bool: Option<ClifFuncId>,
    pub(crate) zaco_println_str: Option<ClifFuncId>,
    pub(crate) zaco_println_i64: Option<ClifFuncId>,
    // Math functions
    pub(crate) zaco_math_floor: Option<ClifFuncId>,
    pub(crate) zaco_math_ceil: Option<ClifFuncId>,
    pub(crate) zaco_math_round: Option<ClifFuncId>,
    pub(crate) zaco_math_abs: Option<ClifFuncId>,
    pub(crate) zaco_math_sqrt: Option<ClifFuncId>,
    pub(crate) zaco_math_pow: Option<ClifFuncId>,
    pub(crate) zaco_math_sin: Option<ClifFuncId>,
    pub(crate) zaco_math_cos: Option<ClifFuncId>,
    pub(crate) zaco_math_tan: Option<ClifFuncId>,
    pub(crate) zaco_math_log: Option<ClifFuncId>,
    pub(crate) zaco_math_log2: Option<ClifFuncId>,
    pub(crate) zaco_math_log10: Option<ClifFuncId>,
    pub(crate) zaco_math_random: Option<ClifFuncId>,
    pub(crate) zaco_math_min: Option<ClifFuncId>,
    pub(crate) zaco_math_max: Option<ClifFuncId>,
    pub(crate) zaco_math_trunc: Option<ClifFuncId>,
    pub(crate) zaco_math_pi: Option<ClifFuncId>,
    pub(crate) zaco_math_e: Option<ClifFuncId>,
    // JSON functions
    pub(crate) zaco_json_parse: Option<ClifFuncId>,
    pub(crate) zaco_json_stringify: Option<ClifFuncId>,
    // Enhanced console functions
    pub(crate) zaco_console_error_str: Option<ClifFuncId>,
    pub(crate) zaco_console_error_i64: Option<ClifFuncId>,
    pub(crate) zaco_console_error_f64: Option<ClifFuncId>,
    pub(crate) zaco_console_error_bool: Option<ClifFuncId>,
    pub(crate) zaco_console_errorln: Option<ClifFuncId>,
    pub(crate) zaco_console_warn_str: Option<ClifFuncId>,
    pub(crate) zaco_console_warn_i64: Option<ClifFuncId>,
    pub(crate) zaco_console_warn_f64: Option<ClifFuncId>,
    pub(crate) zaco_console_warn_bool: Option<ClifFuncId>,
    pub(crate) zaco_console_warnln: Option<ClifFuncId>,
    // String methods
    pub(crate) zaco_str_slice: Option<ClifFuncId>,
    pub(crate) zaco_str_to_upper: Option<ClifFuncId>,
    pub(crate) zaco_str_to_lower: Option<ClifFuncId>,
    pub(crate) zaco_str_trim: Option<ClifFuncId>,
    pub(crate) zaco_str_index_of: Option<ClifFuncId>,
    pub(crate) zaco_str_includes: Option<ClifFuncId>,
    pub(crate) zaco_str_replace: Option<ClifFuncId>,
    pub(crate) zaco_str_split: Option<ClifFuncId>,
    pub(crate) zaco_str_starts_with: Option<ClifFuncId>,
    pub(crate) zaco_str_ends_with: Option<ClifFuncId>,
    pub(crate) zaco_str_eq: Option<ClifFuncId>,
    pub(crate) zaco_str_char_at: Option<ClifFuncId>,
    pub(crate) zaco_str_repeat: Option<ClifFuncId>,
    pub(crate) zaco_str_pad_start: Option<ClifFuncId>,
    pub(crate) zaco_str_pad_end: Option<ClifFuncId>,
    // Array RC
    pub(crate) zaco_array_rc_dec: Option<ClifFuncId>,
    // Array methods
    pub(crate) zaco_array_slice: Option<ClifFuncId>,
    pub(crate) zaco_array_concat: Option<ClifFuncId>,
    pub(crate) zaco_array_index_of: Option<ClifFuncId>,
    pub(crate) zaco_array_join: Option<ClifFuncId>,
    pub(crate) zaco_array_reverse: Option<ClifFuncId>,
    pub(crate) zaco_array_pop: Option<ClifFuncId>,
    // Console debug
    pub(crate) zaco_console_debug_str: Option<ClifFuncId>,
    pub(crate) zaco_console_debug_i64: Option<ClifFuncId>,
    pub(crate) zaco_console_debug_f64: Option<ClifFuncId>,
    pub(crate) zaco_console_debug_bool: Option<ClifFuncId>,
    pub(crate) zaco_console_debugln: Option<ClifFuncId>,
    // Rust runtime - fs module
    pub(crate) zaco_fs_read_file_sync: Option<ClifFuncId>,
    pub(crate) zaco_fs_write_file_sync: Option<ClifFuncId>,
    pub(crate) zaco_fs_exists_sync: Option<ClifFuncId>,
    pub(crate) zaco_fs_mkdir_sync: Option<ClifFuncId>,
    pub(crate) zaco_fs_rmdir_sync: Option<ClifFuncId>,
    pub(crate) zaco_fs_unlink_sync: Option<ClifFuncId>,
    pub(crate) zaco_fs_stat_size: Option<ClifFuncId>,
    pub(crate) zaco_fs_stat_is_file: Option<ClifFuncId>,
    pub(crate) zaco_fs_stat_is_dir: Option<ClifFuncId>,
    pub(crate) zaco_fs_readdir_sync: Option<ClifFuncId>,
    // Rust runtime - path module
    pub(crate) zaco_path_join: Option<ClifFuncId>,
    pub(crate) zaco_path_resolve: Option<ClifFuncId>,
    pub(crate) zaco_path_dirname: Option<ClifFuncId>,
    pub(crate) zaco_path_basename: Option<ClifFuncId>,
    pub(crate) zaco_path_extname: Option<ClifFuncId>,
    pub(crate) zaco_path_is_absolute: Option<ClifFuncId>,
    pub(crate) zaco_path_normalize: Option<ClifFuncId>,
    pub(crate) zaco_path_sep: Option<ClifFuncId>,
    // Rust runtime - process module
    pub(crate) zaco_process_exit: Option<ClifFuncId>,
    pub(crate) zaco_process_cwd: Option<ClifFuncId>,
    pub(crate) zaco_process_env_get: Option<ClifFuncId>,
    pub(crate) zaco_process_pid: Option<ClifFuncId>,
    pub(crate) zaco_process_platform: Option<ClifFuncId>,
    pub(crate) zaco_process_arch: Option<ClifFuncId>,
    pub(crate) zaco_process_argv: Option<ClifFuncId>,
    // Rust runtime - os module
    pub(crate) zaco_os_platform: Option<ClifFuncId>,
    pub(crate) zaco_os_arch: Option<ClifFuncId>,
    pub(crate) zaco_os_homedir: Option<ClifFuncId>,
    pub(crate) zaco_os_tmpdir: Option<ClifFuncId>,
    pub(crate) zaco_os_hostname: Option<ClifFuncId>,
    pub(crate) zaco_os_cpus: Option<ClifFuncId>,
    pub(crate) zaco_os_totalmem: Option<ClifFuncId>,
    pub(crate) zaco_os_eol: Option<ClifFuncId>,
    // Rust runtime - http module
    pub(crate) zaco_http_get: Option<ClifFuncId>,
    pub(crate) zaco_http_post: Option<ClifFuncId>,
    pub(crate) zaco_http_put: Option<ClifFuncId>,
    pub(crate) zaco_http_delete: Option<ClifFuncId>,
    // Rust runtime - init/shutdown
    pub(crate) zaco_runtime_init: Option<ClifFuncId>,
    pub(crate) zaco_runtime_shutdown: Option<ClifFuncId>,
    // Exception handling
    pub(crate) zaco_try_push: Option<ClifFuncId>,
    pub(crate) zaco_try_pop: Option<ClifFuncId>,
    pub(crate) zaco_throw: Option<ClifFuncId>,
    pub(crate) zaco_get_error: Option<ClifFuncId>,
    pub(crate) zaco_clear_error: Option<ClifFuncId>,
    // Global number functions
    pub(crate) zaco_parse_int: Option<ClifFuncId>,
    pub(crate) zaco_parse_float: Option<ClifFuncId>,
    pub(crate) zaco_is_nan: Option<ClifFuncId>,
    pub(crate) zaco_is_finite: Option<ClifFuncId>,
    // Timer functions
    pub(crate) zaco_set_timeout: Option<ClifFuncId>,
    pub(crate) zaco_set_interval: Option<ClifFuncId>,
    pub(crate) zaco_clear_timeout: Option<ClifFuncId>,
    pub(crate) zaco_clear_interval: Option<ClifFuncId>,
    // Async fs
    pub(crate) zaco_fs_read_file: Option<ClifFuncId>,
}

impl RuntimeFunctions {
    pub(crate) fn get_by_name(&self, name: &str) -> Option<ClifFuncId> {
        match name {
            "zaco_alloc" => self.zaco_alloc,
            "zaco_free" => self.zaco_free,
            "zaco_rc_inc" => self.zaco_rc_inc,
            "zaco_rc_dec" => self.zaco_rc_dec,
            "zaco_str_concat" => self.zaco_str_concat,
            "zaco_str_new" => self.zaco_str_new,
            "zaco_print_str" => self.zaco_print_str,
            "zaco_print_i64" => self.zaco_print_i64,
            "zaco_print_f64" => self.zaco_print_f64,
            "zaco_print_bool" => self.zaco_print_bool,
            "zaco_println_str" => self.zaco_println_str,
            "zaco_println_i64" => self.zaco_println_i64,
            // Math functions
            "zaco_math_floor" => self.zaco_math_floor,
            "zaco_math_ceil" => self.zaco_math_ceil,
            "zaco_math_round" => self.zaco_math_round,
            "zaco_math_abs" => self.zaco_math_abs,
            "zaco_math_sqrt" => self.zaco_math_sqrt,
            "zaco_math_pow" => self.zaco_math_pow,
            "zaco_math_sin" => self.zaco_math_sin,
            "zaco_math_cos" => self.zaco_math_cos,
            "zaco_math_tan" => self.zaco_math_tan,
            "zaco_math_log" => self.zaco_math_log,
            "zaco_math_log2" => self.zaco_math_log2,
            "zaco_math_log10" => self.zaco_math_log10,
            "zaco_math_random" => self.zaco_math_random,
            "zaco_math_min" => self.zaco_math_min,
            "zaco_math_max" => self.zaco_math_max,
            "zaco_math_trunc" => self.zaco_math_trunc,
            "zaco_math_pi" => self.zaco_math_pi,
            "zaco_math_e" => self.zaco_math_e,
            // JSON functions
            "zaco_json_parse" => self.zaco_json_parse,
            "zaco_json_stringify" => self.zaco_json_stringify,
            // Enhanced console functions
            "zaco_console_error_str" => self.zaco_console_error_str,
            "zaco_console_error_i64" => self.zaco_console_error_i64,
            "zaco_console_error_f64" => self.zaco_console_error_f64,
            "zaco_console_error_bool" => self.zaco_console_error_bool,
            "zaco_console_errorln" => self.zaco_console_errorln,
            "zaco_console_warn_str" => self.zaco_console_warn_str,
            "zaco_console_warn_i64" => self.zaco_console_warn_i64,
            "zaco_console_warnln" => self.zaco_console_warnln,
            // String methods
            "zaco_str_slice" => self.zaco_str_slice,
            "zaco_str_to_upper" => self.zaco_str_to_upper,
            "zaco_str_to_lower" => self.zaco_str_to_lower,
            "zaco_str_trim" => self.zaco_str_trim,
            "zaco_str_index_of" => self.zaco_str_index_of,
            "zaco_str_includes" => self.zaco_str_includes,
            "zaco_str_replace" => self.zaco_str_replace,
            "zaco_str_split" => self.zaco_str_split,
            "zaco_str_starts_with" => self.zaco_str_starts_with,
            "zaco_str_ends_with" => self.zaco_str_ends_with,
            "zaco_str_eq" => self.zaco_str_eq,
            "zaco_str_char_at" => self.zaco_str_char_at,
            "zaco_str_repeat" => self.zaco_str_repeat,
            "zaco_str_pad_start" => self.zaco_str_pad_start,
            "zaco_str_pad_end" => self.zaco_str_pad_end,
            // Array RC
            "zaco_array_rc_dec" => self.zaco_array_rc_dec,
            // Array methods
            "zaco_array_slice" => self.zaco_array_slice,
            "zaco_array_concat" => self.zaco_array_concat,
            "zaco_array_index_of" => self.zaco_array_index_of,
            "zaco_array_join" => self.zaco_array_join,
            "zaco_array_reverse" => self.zaco_array_reverse,
            "zaco_array_pop" => self.zaco_array_pop,
            // Console debug
            "zaco_console_debug_str" => self.zaco_console_debug_str,
            "zaco_console_debug_i64" => self.zaco_console_debug_i64,
            "zaco_console_debug_f64" => self.zaco_console_debug_f64,
            "zaco_console_debug_bool" => self.zaco_console_debug_bool,
            "zaco_console_debugln" => self.zaco_console_debugln,
            // Rust runtime - fs module
            "zaco_fs_read_file_sync" => self.zaco_fs_read_file_sync,
            "zaco_fs_write_file_sync" => self.zaco_fs_write_file_sync,
            "zaco_fs_exists_sync" => self.zaco_fs_exists_sync,
            "zaco_fs_mkdir_sync" => self.zaco_fs_mkdir_sync,
            "zaco_fs_rmdir_sync" => self.zaco_fs_rmdir_sync,
            "zaco_fs_unlink_sync" => self.zaco_fs_unlink_sync,
            "zaco_fs_stat_size" => self.zaco_fs_stat_size,
            "zaco_fs_stat_is_file" => self.zaco_fs_stat_is_file,
            "zaco_fs_stat_is_dir" => self.zaco_fs_stat_is_dir,
            "zaco_fs_readdir_sync" => self.zaco_fs_readdir_sync,
            // Rust runtime - path module
            "zaco_path_join" => self.zaco_path_join,
            "zaco_path_resolve" => self.zaco_path_resolve,
            "zaco_path_dirname" => self.zaco_path_dirname,
            "zaco_path_basename" => self.zaco_path_basename,
            "zaco_path_extname" => self.zaco_path_extname,
            "zaco_path_is_absolute" => self.zaco_path_is_absolute,
            "zaco_path_normalize" => self.zaco_path_normalize,
            "zaco_path_sep" => self.zaco_path_sep,
            // Rust runtime - process module
            "zaco_process_exit" => self.zaco_process_exit,
            "zaco_process_cwd" => self.zaco_process_cwd,
            "zaco_process_env_get" => self.zaco_process_env_get,
            "zaco_process_pid" => self.zaco_process_pid,
            "zaco_process_platform" => self.zaco_process_platform,
            "zaco_process_arch" => self.zaco_process_arch,
            "zaco_process_argv" => self.zaco_process_argv,
            // Rust runtime - os module
            "zaco_os_platform" => self.zaco_os_platform,
            "zaco_os_arch" => self.zaco_os_arch,
            "zaco_os_homedir" => self.zaco_os_homedir,
            "zaco_os_tmpdir" => self.zaco_os_tmpdir,
            "zaco_os_hostname" => self.zaco_os_hostname,
            "zaco_os_cpus" => self.zaco_os_cpus,
            "zaco_os_totalmem" => self.zaco_os_totalmem,
            "zaco_os_eol" => self.zaco_os_eol,
            // Rust runtime - http module
            "zaco_http_get" => self.zaco_http_get,
            "zaco_http_post" => self.zaco_http_post,
            "zaco_http_put" => self.zaco_http_put,
            "zaco_http_delete" => self.zaco_http_delete,
            // Rust runtime - init/shutdown
            "zaco_runtime_init" => self.zaco_runtime_init,
            "zaco_runtime_shutdown" => self.zaco_runtime_shutdown,
            // Exception handling
            "zaco_try_push" => self.zaco_try_push,
            "zaco_try_pop" => self.zaco_try_pop,
            "zaco_throw" => self.zaco_throw,
            "zaco_get_error" => self.zaco_get_error,
            "zaco_clear_error" => self.zaco_clear_error,
            // Global number functions
            "zaco_parse_int" => self.zaco_parse_int,
            "zaco_parse_float" => self.zaco_parse_float,
            "zaco_is_nan" => self.zaco_is_nan,
            "zaco_is_finite" => self.zaco_is_finite,
            // Console warn (f64, bool)
            "zaco_console_warn_f64" => self.zaco_console_warn_f64,
            "zaco_console_warn_bool" => self.zaco_console_warn_bool,
            // Timer functions
            "zaco_set_timeout" => self.zaco_set_timeout,
            "zaco_set_interval" => self.zaco_set_interval,
            "zaco_clear_timeout" => self.zaco_clear_timeout,
            "zaco_clear_interval" => self.zaco_clear_interval,
            // Async fs
            "zaco_fs_read_file" => self.zaco_fs_read_file,
            _ => None,
        }
    }
}

/// Declare all runtime support functions in the module
pub(crate) fn declare_runtime_functions(
    module: &mut ObjectModule,
    runtime_funcs: &mut RuntimeFunctions,
    pointer_type: Type,
) -> Result<(), CodegenError> {
    // zaco_alloc(size: i64) -> ptr
    let mut alloc_sig = module.make_signature();
    alloc_sig.params.push(AbiParam::new(types::I64));
    alloc_sig.returns.push(AbiParam::new(pointer_type));
    let alloc_id = module
        .declare_function("zaco_alloc", Linkage::Import, &alloc_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_alloc: {}", e)))?;
    runtime_funcs.zaco_alloc = Some(alloc_id);

    // zaco_free(ptr)
    let mut free_sig = module.make_signature();
    free_sig.params.push(AbiParam::new(pointer_type));
    let free_id = module
        .declare_function("zaco_free", Linkage::Import, &free_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_free: {}", e)))?;
    runtime_funcs.zaco_free = Some(free_id);

    // zaco_rc_inc(ptr)
    let mut rc_inc_sig = module.make_signature();
    rc_inc_sig.params.push(AbiParam::new(pointer_type));
    let rc_inc_id = module
        .declare_function("zaco_rc_inc", Linkage::Import, &rc_inc_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_rc_inc: {}", e)))?;
    runtime_funcs.zaco_rc_inc = Some(rc_inc_id);

    // zaco_rc_dec(ptr)
    let mut rc_dec_sig = module.make_signature();
    rc_dec_sig.params.push(AbiParam::new(pointer_type));
    let rc_dec_id = module
        .declare_function("zaco_rc_dec", Linkage::Import, &rc_dec_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_rc_dec: {}", e)))?;
    runtime_funcs.zaco_rc_dec = Some(rc_dec_id);

    // zaco_str_concat(a: ptr, b: ptr) -> ptr
    let mut concat_sig = module.make_signature();
    concat_sig.params.push(AbiParam::new(pointer_type));
    concat_sig.params.push(AbiParam::new(pointer_type));
    concat_sig.returns.push(AbiParam::new(pointer_type));
    let concat_id = module
        .declare_function("zaco_str_concat", Linkage::Import, &concat_sig)
        .map_err(|e| {
            CodegenError::new(format!("Failed to declare zaco_str_concat: {}", e))
        })?;
    runtime_funcs.zaco_str_concat = Some(concat_id);

    // zaco_str_new(ptr) -> ptr  (takes raw C string pointer, returns managed string)
    let mut str_new_sig = module.make_signature();
    str_new_sig.params.push(AbiParam::new(pointer_type));
    str_new_sig.returns.push(AbiParam::new(pointer_type));
    let str_new_id = module
        .declare_function("zaco_str_new", Linkage::Import, &str_new_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_new: {}", e)))?;
    runtime_funcs.zaco_str_new = Some(str_new_id);

    // zaco_print_str(ptr)
    let mut print_str_sig = module.make_signature();
    print_str_sig.params.push(AbiParam::new(pointer_type));
    let print_str_id = module
        .declare_function("zaco_print_str", Linkage::Import, &print_str_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_print_str: {}", e)))?;
    runtime_funcs.zaco_print_str = Some(print_str_id);

    // zaco_print_i64(i64)
    let mut print_i64_sig = module.make_signature();
    print_i64_sig.params.push(AbiParam::new(types::I64));
    let print_i64_id = module
        .declare_function("zaco_print_i64", Linkage::Import, &print_i64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_print_i64: {}", e)))?;
    runtime_funcs.zaco_print_i64 = Some(print_i64_id);

    // zaco_print_f64(f64)
    let mut print_f64_sig = module.make_signature();
    print_f64_sig.params.push(AbiParam::new(types::F64));
    let print_f64_id = module
        .declare_function("zaco_print_f64", Linkage::Import, &print_f64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_print_f64: {}", e)))?;
    runtime_funcs.zaco_print_f64 = Some(print_f64_id);

    // zaco_print_bool(i64)
    let mut print_bool_sig = module.make_signature();
    print_bool_sig.params.push(AbiParam::new(types::I64));
    let print_bool_id = module
        .declare_function("zaco_print_bool", Linkage::Import, &print_bool_sig)
        .map_err(|e| {
            CodegenError::new(format!("Failed to declare zaco_print_bool: {}", e))
        })?;
    runtime_funcs.zaco_print_bool = Some(print_bool_id);

    // zaco_println_str(ptr)
    let mut println_str_sig = module.make_signature();
    println_str_sig.params.push(AbiParam::new(pointer_type));
    let println_str_id = module
        .declare_function("zaco_println_str", Linkage::Import, &println_str_sig)
        .map_err(|e| {
            CodegenError::new(format!("Failed to declare zaco_println_str: {}", e))
        })?;
    runtime_funcs.zaco_println_str = Some(println_str_id);

    // zaco_println_i64(i64)
    let mut println_i64_sig = module.make_signature();
    println_i64_sig.params.push(AbiParam::new(types::I64));
    let println_i64_id = module
        .declare_function("zaco_println_i64", Linkage::Import, &println_i64_sig)
        .map_err(|e| {
            CodegenError::new(format!("Failed to declare zaco_println_i64: {}", e))
        })?;
    runtime_funcs.zaco_println_i64 = Some(println_i64_id);

    // ========== Math Functions ==========

    // zaco_math_floor(f64) -> f64
    let mut math_floor_sig = module.make_signature();
    math_floor_sig.params.push(AbiParam::new(types::F64));
    math_floor_sig.returns.push(AbiParam::new(types::F64));
    let math_floor_id = module
        .declare_function("zaco_math_floor", Linkage::Import, &math_floor_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_floor: {}", e)))?;
    runtime_funcs.zaco_math_floor = Some(math_floor_id);

    // zaco_math_ceil(f64) -> f64
    let mut math_ceil_sig = module.make_signature();
    math_ceil_sig.params.push(AbiParam::new(types::F64));
    math_ceil_sig.returns.push(AbiParam::new(types::F64));
    let math_ceil_id = module
        .declare_function("zaco_math_ceil", Linkage::Import, &math_ceil_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_ceil: {}", e)))?;
    runtime_funcs.zaco_math_ceil = Some(math_ceil_id);

    // zaco_math_round(f64) -> f64
    let mut math_round_sig = module.make_signature();
    math_round_sig.params.push(AbiParam::new(types::F64));
    math_round_sig.returns.push(AbiParam::new(types::F64));
    let math_round_id = module
        .declare_function("zaco_math_round", Linkage::Import, &math_round_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_round: {}", e)))?;
    runtime_funcs.zaco_math_round = Some(math_round_id);

    // zaco_math_abs(f64) -> f64
    let mut math_abs_sig = module.make_signature();
    math_abs_sig.params.push(AbiParam::new(types::F64));
    math_abs_sig.returns.push(AbiParam::new(types::F64));
    let math_abs_id = module
        .declare_function("zaco_math_abs", Linkage::Import, &math_abs_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_abs: {}", e)))?;
    runtime_funcs.zaco_math_abs = Some(math_abs_id);

    // zaco_math_sqrt(f64) -> f64
    let mut math_sqrt_sig = module.make_signature();
    math_sqrt_sig.params.push(AbiParam::new(types::F64));
    math_sqrt_sig.returns.push(AbiParam::new(types::F64));
    let math_sqrt_id = module
        .declare_function("zaco_math_sqrt", Linkage::Import, &math_sqrt_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_sqrt: {}", e)))?;
    runtime_funcs.zaco_math_sqrt = Some(math_sqrt_id);

    // zaco_math_pow(f64, f64) -> f64
    let mut math_pow_sig = module.make_signature();
    math_pow_sig.params.push(AbiParam::new(types::F64));
    math_pow_sig.params.push(AbiParam::new(types::F64));
    math_pow_sig.returns.push(AbiParam::new(types::F64));
    let math_pow_id = module
        .declare_function("zaco_math_pow", Linkage::Import, &math_pow_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_pow: {}", e)))?;
    runtime_funcs.zaco_math_pow = Some(math_pow_id);

    // zaco_math_sin(f64) -> f64
    let mut math_sin_sig = module.make_signature();
    math_sin_sig.params.push(AbiParam::new(types::F64));
    math_sin_sig.returns.push(AbiParam::new(types::F64));
    let math_sin_id = module
        .declare_function("zaco_math_sin", Linkage::Import, &math_sin_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_sin: {}", e)))?;
    runtime_funcs.zaco_math_sin = Some(math_sin_id);

    // zaco_math_cos(f64) -> f64
    let mut math_cos_sig = module.make_signature();
    math_cos_sig.params.push(AbiParam::new(types::F64));
    math_cos_sig.returns.push(AbiParam::new(types::F64));
    let math_cos_id = module
        .declare_function("zaco_math_cos", Linkage::Import, &math_cos_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_cos: {}", e)))?;
    runtime_funcs.zaco_math_cos = Some(math_cos_id);

    // zaco_math_tan(f64) -> f64
    let mut math_tan_sig = module.make_signature();
    math_tan_sig.params.push(AbiParam::new(types::F64));
    math_tan_sig.returns.push(AbiParam::new(types::F64));
    let math_tan_id = module
        .declare_function("zaco_math_tan", Linkage::Import, &math_tan_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_tan: {}", e)))?;
    runtime_funcs.zaco_math_tan = Some(math_tan_id);

    // zaco_math_log(f64) -> f64
    let mut math_log_sig = module.make_signature();
    math_log_sig.params.push(AbiParam::new(types::F64));
    math_log_sig.returns.push(AbiParam::new(types::F64));
    let math_log_id = module
        .declare_function("zaco_math_log", Linkage::Import, &math_log_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_log: {}", e)))?;
    runtime_funcs.zaco_math_log = Some(math_log_id);

    // zaco_math_log2(f64) -> f64
    let mut math_log2_sig = module.make_signature();
    math_log2_sig.params.push(AbiParam::new(types::F64));
    math_log2_sig.returns.push(AbiParam::new(types::F64));
    let math_log2_id = module
        .declare_function("zaco_math_log2", Linkage::Import, &math_log2_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_log2: {}", e)))?;
    runtime_funcs.zaco_math_log2 = Some(math_log2_id);

    // zaco_math_log10(f64) -> f64
    let mut math_log10_sig = module.make_signature();
    math_log10_sig.params.push(AbiParam::new(types::F64));
    math_log10_sig.returns.push(AbiParam::new(types::F64));
    let math_log10_id = module
        .declare_function("zaco_math_log10", Linkage::Import, &math_log10_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_log10: {}", e)))?;
    runtime_funcs.zaco_math_log10 = Some(math_log10_id);

    // zaco_math_random() -> f64
    let mut math_random_sig = module.make_signature();
    math_random_sig.returns.push(AbiParam::new(types::F64));
    let math_random_id = module
        .declare_function("zaco_math_random", Linkage::Import, &math_random_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_random: {}", e)))?;
    runtime_funcs.zaco_math_random = Some(math_random_id);

    // zaco_math_min(f64, f64) -> f64
    let mut math_min_sig = module.make_signature();
    math_min_sig.params.push(AbiParam::new(types::F64));
    math_min_sig.params.push(AbiParam::new(types::F64));
    math_min_sig.returns.push(AbiParam::new(types::F64));
    let math_min_id = module
        .declare_function("zaco_math_min", Linkage::Import, &math_min_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_min: {}", e)))?;
    runtime_funcs.zaco_math_min = Some(math_min_id);

    // zaco_math_max(f64, f64) -> f64
    let mut math_max_sig = module.make_signature();
    math_max_sig.params.push(AbiParam::new(types::F64));
    math_max_sig.params.push(AbiParam::new(types::F64));
    math_max_sig.returns.push(AbiParam::new(types::F64));
    let math_max_id = module
        .declare_function("zaco_math_max", Linkage::Import, &math_max_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_max: {}", e)))?;
    runtime_funcs.zaco_math_max = Some(math_max_id);

    // zaco_math_trunc(f64) -> i64
    let mut math_trunc_sig = module.make_signature();
    math_trunc_sig.params.push(AbiParam::new(types::F64));
    math_trunc_sig.returns.push(AbiParam::new(types::I64));
    let math_trunc_id = module
        .declare_function("zaco_math_trunc", Linkage::Import, &math_trunc_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_trunc: {}", e)))?;
    runtime_funcs.zaco_math_trunc = Some(math_trunc_id);

    // zaco_math_pi() -> f64
    let mut math_pi_sig = module.make_signature();
    math_pi_sig.returns.push(AbiParam::new(types::F64));
    let math_pi_id = module
        .declare_function("zaco_math_pi", Linkage::Import, &math_pi_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_pi: {}", e)))?;
    runtime_funcs.zaco_math_pi = Some(math_pi_id);

    // zaco_math_e() -> f64
    let mut math_e_sig = module.make_signature();
    math_e_sig.returns.push(AbiParam::new(types::F64));
    let math_e_id = module
        .declare_function("zaco_math_e", Linkage::Import, &math_e_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_math_e: {}", e)))?;
    runtime_funcs.zaco_math_e = Some(math_e_id);

    // ========== JSON Functions ==========

    // zaco_json_parse(ptr) -> ptr
    let mut json_parse_sig = module.make_signature();
    json_parse_sig.params.push(AbiParam::new(pointer_type));
    json_parse_sig.returns.push(AbiParam::new(pointer_type));
    let json_parse_id = module
        .declare_function("zaco_json_parse", Linkage::Import, &json_parse_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_json_parse: {}", e)))?;
    runtime_funcs.zaco_json_parse = Some(json_parse_id);

    // zaco_json_stringify(ptr) -> ptr
    let mut json_stringify_sig = module.make_signature();
    json_stringify_sig.params.push(AbiParam::new(pointer_type));
    json_stringify_sig.returns.push(AbiParam::new(pointer_type));
    let json_stringify_id = module
        .declare_function("zaco_json_stringify", Linkage::Import, &json_stringify_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_json_stringify: {}", e)))?;
    runtime_funcs.zaco_json_stringify = Some(json_stringify_id);

    // ========== Enhanced Console Functions ==========

    // zaco_console_error_str(ptr)
    let mut console_error_str_sig = module.make_signature();
    console_error_str_sig.params.push(AbiParam::new(pointer_type));
    let console_error_str_id = module
        .declare_function("zaco_console_error_str", Linkage::Import, &console_error_str_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_error_str: {}", e)))?;
    runtime_funcs.zaco_console_error_str = Some(console_error_str_id);

    // zaco_console_error_i64(i64)
    let mut console_error_i64_sig = module.make_signature();
    console_error_i64_sig.params.push(AbiParam::new(types::I64));
    let console_error_i64_id = module
        .declare_function("zaco_console_error_i64", Linkage::Import, &console_error_i64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_error_i64: {}", e)))?;
    runtime_funcs.zaco_console_error_i64 = Some(console_error_i64_id);

    // zaco_console_error_f64(f64)
    let mut console_error_f64_sig = module.make_signature();
    console_error_f64_sig.params.push(AbiParam::new(types::F64));
    let console_error_f64_id = module
        .declare_function("zaco_console_error_f64", Linkage::Import, &console_error_f64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_error_f64: {}", e)))?;
    runtime_funcs.zaco_console_error_f64 = Some(console_error_f64_id);

    // zaco_console_error_bool(i64)
    let mut console_error_bool_sig = module.make_signature();
    console_error_bool_sig.params.push(AbiParam::new(types::I64));
    let console_error_bool_id = module
        .declare_function("zaco_console_error_bool", Linkage::Import, &console_error_bool_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_error_bool: {}", e)))?;
    runtime_funcs.zaco_console_error_bool = Some(console_error_bool_id);

    // zaco_console_errorln(ptr)
    let mut console_errorln_sig = module.make_signature();
    console_errorln_sig.params.push(AbiParam::new(pointer_type));
    let console_errorln_id = module
        .declare_function("zaco_console_errorln", Linkage::Import, &console_errorln_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_errorln: {}", e)))?;
    runtime_funcs.zaco_console_errorln = Some(console_errorln_id);

    // zaco_console_warn_str(ptr)
    let mut console_warn_str_sig = module.make_signature();
    console_warn_str_sig.params.push(AbiParam::new(pointer_type));
    let console_warn_str_id = module
        .declare_function("zaco_console_warn_str", Linkage::Import, &console_warn_str_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_warn_str: {}", e)))?;
    runtime_funcs.zaco_console_warn_str = Some(console_warn_str_id);

    // zaco_console_warn_i64(i64)
    let mut console_warn_i64_sig = module.make_signature();
    console_warn_i64_sig.params.push(AbiParam::new(types::I64));
    let console_warn_i64_id = module
        .declare_function("zaco_console_warn_i64", Linkage::Import, &console_warn_i64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_warn_i64: {}", e)))?;
    runtime_funcs.zaco_console_warn_i64 = Some(console_warn_i64_id);

    // zaco_console_warnln(ptr)
    let mut console_warnln_sig = module.make_signature();
    console_warnln_sig.params.push(AbiParam::new(pointer_type));
    let console_warnln_id = module
        .declare_function("zaco_console_warnln", Linkage::Import, &console_warnln_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_warnln: {}", e)))?;
    runtime_funcs.zaco_console_warnln = Some(console_warnln_id);

    // ========== String Methods ==========

    // zaco_str_slice(ptr, i64, i64) -> ptr
    let mut str_slice_sig = module.make_signature();
    str_slice_sig.params.push(AbiParam::new(pointer_type));
    str_slice_sig.params.push(AbiParam::new(types::I64));
    str_slice_sig.params.push(AbiParam::new(types::I64));
    str_slice_sig.returns.push(AbiParam::new(pointer_type));
    let str_slice_id = module
        .declare_function("zaco_str_slice", Linkage::Import, &str_slice_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_slice: {}", e)))?;
    runtime_funcs.zaco_str_slice = Some(str_slice_id);

    // zaco_str_to_upper(ptr) -> ptr
    let mut str_to_upper_sig = module.make_signature();
    str_to_upper_sig.params.push(AbiParam::new(pointer_type));
    str_to_upper_sig.returns.push(AbiParam::new(pointer_type));
    let str_to_upper_id = module
        .declare_function("zaco_str_to_upper", Linkage::Import, &str_to_upper_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_to_upper: {}", e)))?;
    runtime_funcs.zaco_str_to_upper = Some(str_to_upper_id);

    // zaco_str_to_lower(ptr) -> ptr
    let mut str_to_lower_sig = module.make_signature();
    str_to_lower_sig.params.push(AbiParam::new(pointer_type));
    str_to_lower_sig.returns.push(AbiParam::new(pointer_type));
    let str_to_lower_id = module
        .declare_function("zaco_str_to_lower", Linkage::Import, &str_to_lower_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_to_lower: {}", e)))?;
    runtime_funcs.zaco_str_to_lower = Some(str_to_lower_id);

    // zaco_str_trim(ptr) -> ptr
    let mut str_trim_sig = module.make_signature();
    str_trim_sig.params.push(AbiParam::new(pointer_type));
    str_trim_sig.returns.push(AbiParam::new(pointer_type));
    let str_trim_id = module
        .declare_function("zaco_str_trim", Linkage::Import, &str_trim_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_trim: {}", e)))?;
    runtime_funcs.zaco_str_trim = Some(str_trim_id);

    // zaco_str_index_of(ptr, ptr) -> i64
    let mut str_index_of_sig = module.make_signature();
    str_index_of_sig.params.push(AbiParam::new(pointer_type));
    str_index_of_sig.params.push(AbiParam::new(pointer_type));
    str_index_of_sig.returns.push(AbiParam::new(types::I64));
    let str_index_of_id = module
        .declare_function("zaco_str_index_of", Linkage::Import, &str_index_of_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_index_of: {}", e)))?;
    runtime_funcs.zaco_str_index_of = Some(str_index_of_id);

    // zaco_str_includes(ptr, ptr) -> i64
    let mut str_includes_sig = module.make_signature();
    str_includes_sig.params.push(AbiParam::new(pointer_type));
    str_includes_sig.params.push(AbiParam::new(pointer_type));
    str_includes_sig.returns.push(AbiParam::new(types::I64));
    let str_includes_id = module
        .declare_function("zaco_str_includes", Linkage::Import, &str_includes_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_includes: {}", e)))?;
    runtime_funcs.zaco_str_includes = Some(str_includes_id);

    // zaco_str_replace(ptr, ptr, ptr) -> ptr
    let mut str_replace_sig = module.make_signature();
    str_replace_sig.params.push(AbiParam::new(pointer_type));
    str_replace_sig.params.push(AbiParam::new(pointer_type));
    str_replace_sig.params.push(AbiParam::new(pointer_type));
    str_replace_sig.returns.push(AbiParam::new(pointer_type));
    let str_replace_id = module
        .declare_function("zaco_str_replace", Linkage::Import, &str_replace_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_replace: {}", e)))?;
    runtime_funcs.zaco_str_replace = Some(str_replace_id);

    // zaco_str_split(ptr, ptr) -> ptr
    let mut str_split_sig = module.make_signature();
    str_split_sig.params.push(AbiParam::new(pointer_type));
    str_split_sig.params.push(AbiParam::new(pointer_type));
    str_split_sig.returns.push(AbiParam::new(pointer_type));
    let str_split_id = module
        .declare_function("zaco_str_split", Linkage::Import, &str_split_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_split: {}", e)))?;
    runtime_funcs.zaco_str_split = Some(str_split_id);

    // zaco_str_starts_with(ptr, ptr) -> i64
    let mut str_starts_with_sig = module.make_signature();
    str_starts_with_sig.params.push(AbiParam::new(pointer_type));
    str_starts_with_sig.params.push(AbiParam::new(pointer_type));
    str_starts_with_sig.returns.push(AbiParam::new(types::I64));
    let str_starts_with_id = module
        .declare_function("zaco_str_starts_with", Linkage::Import, &str_starts_with_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_starts_with: {}", e)))?;
    runtime_funcs.zaco_str_starts_with = Some(str_starts_with_id);

    // zaco_str_ends_with(ptr, ptr) -> i64
    let mut str_ends_with_sig = module.make_signature();
    str_ends_with_sig.params.push(AbiParam::new(pointer_type));
    str_ends_with_sig.params.push(AbiParam::new(pointer_type));
    str_ends_with_sig.returns.push(AbiParam::new(types::I64));
    let str_ends_with_id = module
        .declare_function("zaco_str_ends_with", Linkage::Import, &str_ends_with_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_ends_with: {}", e)))?;
    runtime_funcs.zaco_str_ends_with = Some(str_ends_with_id);

    // zaco_str_eq(ptr, ptr) -> i64
    let mut str_eq_sig = module.make_signature();
    str_eq_sig.params.push(AbiParam::new(pointer_type));
    str_eq_sig.params.push(AbiParam::new(pointer_type));
    str_eq_sig.returns.push(AbiParam::new(types::I64));
    let str_eq_id = module
        .declare_function("zaco_str_eq", Linkage::Import, &str_eq_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_eq: {}", e)))?;
    runtime_funcs.zaco_str_eq = Some(str_eq_id);

    // zaco_str_char_at(ptr, i64) -> ptr
    let mut str_char_at_sig = module.make_signature();
    str_char_at_sig.params.push(AbiParam::new(pointer_type));
    str_char_at_sig.params.push(AbiParam::new(types::I64));
    str_char_at_sig.returns.push(AbiParam::new(pointer_type));
    let str_char_at_id = module
        .declare_function("zaco_str_char_at", Linkage::Import, &str_char_at_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_char_at: {}", e)))?;
    runtime_funcs.zaco_str_char_at = Some(str_char_at_id);

    // zaco_str_repeat(ptr, i64) -> ptr
    let mut str_repeat_sig = module.make_signature();
    str_repeat_sig.params.push(AbiParam::new(pointer_type));
    str_repeat_sig.params.push(AbiParam::new(types::I64));
    str_repeat_sig.returns.push(AbiParam::new(pointer_type));
    let str_repeat_id = module
        .declare_function("zaco_str_repeat", Linkage::Import, &str_repeat_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_repeat: {}", e)))?;
    runtime_funcs.zaco_str_repeat = Some(str_repeat_id);

    // zaco_str_pad_start(ptr, i64, ptr) -> ptr
    let mut str_pad_start_sig = module.make_signature();
    str_pad_start_sig.params.push(AbiParam::new(pointer_type));
    str_pad_start_sig.params.push(AbiParam::new(types::I64));
    str_pad_start_sig.params.push(AbiParam::new(pointer_type));
    str_pad_start_sig.returns.push(AbiParam::new(pointer_type));
    let str_pad_start_id = module
        .declare_function("zaco_str_pad_start", Linkage::Import, &str_pad_start_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_pad_start: {}", e)))?;
    runtime_funcs.zaco_str_pad_start = Some(str_pad_start_id);

    // zaco_str_pad_end(ptr, i64, ptr) -> ptr
    let mut str_pad_end_sig = module.make_signature();
    str_pad_end_sig.params.push(AbiParam::new(pointer_type));
    str_pad_end_sig.params.push(AbiParam::new(types::I64));
    str_pad_end_sig.params.push(AbiParam::new(pointer_type));
    str_pad_end_sig.returns.push(AbiParam::new(pointer_type));
    let str_pad_end_id = module
        .declare_function("zaco_str_pad_end", Linkage::Import, &str_pad_end_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_str_pad_end: {}", e)))?;
    runtime_funcs.zaco_str_pad_end = Some(str_pad_end_id);

    // ========== Array RC ==========

    // zaco_array_rc_dec(ptr)
    let mut array_rc_dec_sig = module.make_signature();
    array_rc_dec_sig.params.push(AbiParam::new(pointer_type));
    let array_rc_dec_id = module
        .declare_function("zaco_array_rc_dec", Linkage::Import, &array_rc_dec_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_rc_dec: {}", e)))?;
    runtime_funcs.zaco_array_rc_dec = Some(array_rc_dec_id);

    // ========== Array Methods ==========

    // zaco_array_slice(ptr, i64, i64) -> ptr
    let mut array_slice_sig = module.make_signature();
    array_slice_sig.params.push(AbiParam::new(pointer_type));
    array_slice_sig.params.push(AbiParam::new(types::I64));
    array_slice_sig.params.push(AbiParam::new(types::I64));
    array_slice_sig.returns.push(AbiParam::new(pointer_type));
    let array_slice_id = module
        .declare_function("zaco_array_slice", Linkage::Import, &array_slice_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_slice: {}", e)))?;
    runtime_funcs.zaco_array_slice = Some(array_slice_id);

    // zaco_array_concat(ptr, ptr) -> ptr
    let mut array_concat_sig = module.make_signature();
    array_concat_sig.params.push(AbiParam::new(pointer_type));
    array_concat_sig.params.push(AbiParam::new(pointer_type));
    array_concat_sig.returns.push(AbiParam::new(pointer_type));
    let array_concat_id = module
        .declare_function("zaco_array_concat", Linkage::Import, &array_concat_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_concat: {}", e)))?;
    runtime_funcs.zaco_array_concat = Some(array_concat_id);

    // zaco_array_index_of(ptr, ptr) -> i64
    let mut array_index_of_sig = module.make_signature();
    array_index_of_sig.params.push(AbiParam::new(pointer_type));
    array_index_of_sig.params.push(AbiParam::new(pointer_type));
    array_index_of_sig.returns.push(AbiParam::new(types::I64));
    let array_index_of_id = module
        .declare_function("zaco_array_index_of", Linkage::Import, &array_index_of_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_index_of: {}", e)))?;
    runtime_funcs.zaco_array_index_of = Some(array_index_of_id);

    // zaco_array_join(ptr, ptr) -> ptr
    let mut array_join_sig = module.make_signature();
    array_join_sig.params.push(AbiParam::new(pointer_type));
    array_join_sig.params.push(AbiParam::new(pointer_type));
    array_join_sig.returns.push(AbiParam::new(pointer_type));
    let array_join_id = module
        .declare_function("zaco_array_join", Linkage::Import, &array_join_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_join: {}", e)))?;
    runtime_funcs.zaco_array_join = Some(array_join_id);

    // zaco_array_reverse(ptr)
    let mut array_reverse_sig = module.make_signature();
    array_reverse_sig.params.push(AbiParam::new(pointer_type));
    let array_reverse_id = module
        .declare_function("zaco_array_reverse", Linkage::Import, &array_reverse_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_reverse: {}", e)))?;
    runtime_funcs.zaco_array_reverse = Some(array_reverse_id);

    // zaco_array_pop(ptr) -> ptr
    let mut array_pop_sig = module.make_signature();
    array_pop_sig.params.push(AbiParam::new(pointer_type));
    array_pop_sig.returns.push(AbiParam::new(pointer_type));
    let array_pop_id = module
        .declare_function("zaco_array_pop", Linkage::Import, &array_pop_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_array_pop: {}", e)))?;
    runtime_funcs.zaco_array_pop = Some(array_pop_id);

    // ========== Console Debug Functions ==========

    // zaco_console_debug_str(ptr)
    let mut console_debug_str_sig = module.make_signature();
    console_debug_str_sig.params.push(AbiParam::new(pointer_type));
    let console_debug_str_id = module
        .declare_function("zaco_console_debug_str", Linkage::Import, &console_debug_str_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_debug_str: {}", e)))?;
    runtime_funcs.zaco_console_debug_str = Some(console_debug_str_id);

    // zaco_console_debug_i64(i64)
    let mut console_debug_i64_sig = module.make_signature();
    console_debug_i64_sig.params.push(AbiParam::new(types::I64));
    let console_debug_i64_id = module
        .declare_function("zaco_console_debug_i64", Linkage::Import, &console_debug_i64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_debug_i64: {}", e)))?;
    runtime_funcs.zaco_console_debug_i64 = Some(console_debug_i64_id);

    // zaco_console_debug_f64(f64)
    let mut console_debug_f64_sig = module.make_signature();
    console_debug_f64_sig.params.push(AbiParam::new(types::F64));
    let console_debug_f64_id = module
        .declare_function("zaco_console_debug_f64", Linkage::Import, &console_debug_f64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_debug_f64: {}", e)))?;
    runtime_funcs.zaco_console_debug_f64 = Some(console_debug_f64_id);

    // zaco_console_debug_bool(i64)
    let mut console_debug_bool_sig = module.make_signature();
    console_debug_bool_sig.params.push(AbiParam::new(types::I64));
    let console_debug_bool_id = module
        .declare_function("zaco_console_debug_bool", Linkage::Import, &console_debug_bool_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_debug_bool: {}", e)))?;
    runtime_funcs.zaco_console_debug_bool = Some(console_debug_bool_id);

    // zaco_console_debugln(ptr)
    let mut console_debugln_sig = module.make_signature();
    console_debugln_sig.params.push(AbiParam::new(pointer_type));
    let console_debugln_id = module
        .declare_function("zaco_console_debugln", Linkage::Import, &console_debugln_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_debugln: {}", e)))?;
    runtime_funcs.zaco_console_debugln = Some(console_debugln_id);

    // ========== Rust Runtime - FS Module ==========

    // zaco_fs_read_file_sync(path: *const i8, encoding: *const i8) -> *const i8
    let mut fs_read_file_sync_sig = module.make_signature();
    fs_read_file_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_read_file_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_read_file_sync_sig.returns.push(AbiParam::new(pointer_type));
    let fs_read_file_sync_id = module
        .declare_function("zaco_fs_read_file_sync", Linkage::Import, &fs_read_file_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_read_file_sync: {}", e)))?;
    runtime_funcs.zaco_fs_read_file_sync = Some(fs_read_file_sync_id);

    // zaco_fs_write_file_sync(path: *const i8, data: *const i8) -> i64
    let mut fs_write_file_sync_sig = module.make_signature();
    fs_write_file_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_write_file_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_write_file_sync_sig.returns.push(AbiParam::new(types::I64));
    let fs_write_file_sync_id = module
        .declare_function("zaco_fs_write_file_sync", Linkage::Import, &fs_write_file_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_write_file_sync: {}", e)))?;
    runtime_funcs.zaco_fs_write_file_sync = Some(fs_write_file_sync_id);

    // zaco_fs_exists_sync(path: *const i8) -> i64
    let mut fs_exists_sync_sig = module.make_signature();
    fs_exists_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_exists_sync_sig.returns.push(AbiParam::new(types::I64));
    let fs_exists_sync_id = module
        .declare_function("zaco_fs_exists_sync", Linkage::Import, &fs_exists_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_exists_sync: {}", e)))?;
    runtime_funcs.zaco_fs_exists_sync = Some(fs_exists_sync_id);

    // zaco_fs_mkdir_sync(path: *const i8, recursive: i64) -> i64
    let mut fs_mkdir_sync_sig = module.make_signature();
    fs_mkdir_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_mkdir_sync_sig.params.push(AbiParam::new(types::I64));
    fs_mkdir_sync_sig.returns.push(AbiParam::new(types::I64));
    let fs_mkdir_sync_id = module
        .declare_function("zaco_fs_mkdir_sync", Linkage::Import, &fs_mkdir_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_mkdir_sync: {}", e)))?;
    runtime_funcs.zaco_fs_mkdir_sync = Some(fs_mkdir_sync_id);

    // zaco_fs_rmdir_sync(path: *const i8) -> i64
    let mut fs_rmdir_sync_sig = module.make_signature();
    fs_rmdir_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_rmdir_sync_sig.returns.push(AbiParam::new(types::I64));
    let fs_rmdir_sync_id = module
        .declare_function("zaco_fs_rmdir_sync", Linkage::Import, &fs_rmdir_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_rmdir_sync: {}", e)))?;
    runtime_funcs.zaco_fs_rmdir_sync = Some(fs_rmdir_sync_id);

    // zaco_fs_unlink_sync(path: *const i8) -> i64
    let mut fs_unlink_sync_sig = module.make_signature();
    fs_unlink_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_unlink_sync_sig.returns.push(AbiParam::new(types::I64));
    let fs_unlink_sync_id = module
        .declare_function("zaco_fs_unlink_sync", Linkage::Import, &fs_unlink_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_unlink_sync: {}", e)))?;
    runtime_funcs.zaco_fs_unlink_sync = Some(fs_unlink_sync_id);

    // zaco_fs_stat_size(path: *const i8) -> i64
    let mut fs_stat_size_sig = module.make_signature();
    fs_stat_size_sig.params.push(AbiParam::new(pointer_type));
    fs_stat_size_sig.returns.push(AbiParam::new(types::I64));
    let fs_stat_size_id = module
        .declare_function("zaco_fs_stat_size", Linkage::Import, &fs_stat_size_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_stat_size: {}", e)))?;
    runtime_funcs.zaco_fs_stat_size = Some(fs_stat_size_id);

    // zaco_fs_stat_is_file(path: *const i8) -> i64
    let mut fs_stat_is_file_sig = module.make_signature();
    fs_stat_is_file_sig.params.push(AbiParam::new(pointer_type));
    fs_stat_is_file_sig.returns.push(AbiParam::new(types::I64));
    let fs_stat_is_file_id = module
        .declare_function("zaco_fs_stat_is_file", Linkage::Import, &fs_stat_is_file_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_stat_is_file: {}", e)))?;
    runtime_funcs.zaco_fs_stat_is_file = Some(fs_stat_is_file_id);

    // zaco_fs_stat_is_dir(path: *const i8) -> i64
    let mut fs_stat_is_dir_sig = module.make_signature();
    fs_stat_is_dir_sig.params.push(AbiParam::new(pointer_type));
    fs_stat_is_dir_sig.returns.push(AbiParam::new(types::I64));
    let fs_stat_is_dir_id = module
        .declare_function("zaco_fs_stat_is_dir", Linkage::Import, &fs_stat_is_dir_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_stat_is_dir: {}", e)))?;
    runtime_funcs.zaco_fs_stat_is_dir = Some(fs_stat_is_dir_id);

    // zaco_fs_readdir_sync(path: *const i8) -> *const i8
    let mut fs_readdir_sync_sig = module.make_signature();
    fs_readdir_sync_sig.params.push(AbiParam::new(pointer_type));
    fs_readdir_sync_sig.returns.push(AbiParam::new(pointer_type));
    let fs_readdir_sync_id = module
        .declare_function("zaco_fs_readdir_sync", Linkage::Import, &fs_readdir_sync_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_readdir_sync: {}", e)))?;
    runtime_funcs.zaco_fs_readdir_sync = Some(fs_readdir_sync_id);

    // ========== Rust Runtime - Path Module ==========

    // zaco_path_join(a: *const i8, b: *const i8) -> *const i8
    let mut path_join_sig = module.make_signature();
    path_join_sig.params.push(AbiParam::new(pointer_type));
    path_join_sig.params.push(AbiParam::new(pointer_type));
    path_join_sig.returns.push(AbiParam::new(pointer_type));
    let path_join_id = module
        .declare_function("zaco_path_join", Linkage::Import, &path_join_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_join: {}", e)))?;
    runtime_funcs.zaco_path_join = Some(path_join_id);

    // zaco_path_resolve(p: *const i8) -> *const i8
    let mut path_resolve_sig = module.make_signature();
    path_resolve_sig.params.push(AbiParam::new(pointer_type));
    path_resolve_sig.returns.push(AbiParam::new(pointer_type));
    let path_resolve_id = module
        .declare_function("zaco_path_resolve", Linkage::Import, &path_resolve_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_resolve: {}", e)))?;
    runtime_funcs.zaco_path_resolve = Some(path_resolve_id);

    // zaco_path_dirname(p: *const i8) -> *const i8
    let mut path_dirname_sig = module.make_signature();
    path_dirname_sig.params.push(AbiParam::new(pointer_type));
    path_dirname_sig.returns.push(AbiParam::new(pointer_type));
    let path_dirname_id = module
        .declare_function("zaco_path_dirname", Linkage::Import, &path_dirname_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_dirname: {}", e)))?;
    runtime_funcs.zaco_path_dirname = Some(path_dirname_id);

    // zaco_path_basename(p: *const i8) -> *const i8
    let mut path_basename_sig = module.make_signature();
    path_basename_sig.params.push(AbiParam::new(pointer_type));
    path_basename_sig.returns.push(AbiParam::new(pointer_type));
    let path_basename_id = module
        .declare_function("zaco_path_basename", Linkage::Import, &path_basename_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_basename: {}", e)))?;
    runtime_funcs.zaco_path_basename = Some(path_basename_id);

    // zaco_path_extname(p: *const i8) -> *const i8
    let mut path_extname_sig = module.make_signature();
    path_extname_sig.params.push(AbiParam::new(pointer_type));
    path_extname_sig.returns.push(AbiParam::new(pointer_type));
    let path_extname_id = module
        .declare_function("zaco_path_extname", Linkage::Import, &path_extname_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_extname: {}", e)))?;
    runtime_funcs.zaco_path_extname = Some(path_extname_id);

    // zaco_path_is_absolute(p: *const i8) -> i64
    let mut path_is_absolute_sig = module.make_signature();
    path_is_absolute_sig.params.push(AbiParam::new(pointer_type));
    path_is_absolute_sig.returns.push(AbiParam::new(types::I64));
    let path_is_absolute_id = module
        .declare_function("zaco_path_is_absolute", Linkage::Import, &path_is_absolute_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_is_absolute: {}", e)))?;
    runtime_funcs.zaco_path_is_absolute = Some(path_is_absolute_id);

    // zaco_path_normalize(p: *const i8) -> *const i8
    let mut path_normalize_sig = module.make_signature();
    path_normalize_sig.params.push(AbiParam::new(pointer_type));
    path_normalize_sig.returns.push(AbiParam::new(pointer_type));
    let path_normalize_id = module
        .declare_function("zaco_path_normalize", Linkage::Import, &path_normalize_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_normalize: {}", e)))?;
    runtime_funcs.zaco_path_normalize = Some(path_normalize_id);

    // zaco_path_sep() -> *const i8
    let mut path_sep_sig = module.make_signature();
    path_sep_sig.returns.push(AbiParam::new(pointer_type));
    let path_sep_id = module
        .declare_function("zaco_path_sep", Linkage::Import, &path_sep_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_path_sep: {}", e)))?;
    runtime_funcs.zaco_path_sep = Some(path_sep_id);

    // ========== Rust Runtime - Process Module ==========

    // zaco_process_exit(code: i64) -> void
    let mut process_exit_sig = module.make_signature();
    process_exit_sig.params.push(AbiParam::new(types::I64));
    let process_exit_id = module
        .declare_function("zaco_process_exit", Linkage::Import, &process_exit_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_exit: {}", e)))?;
    runtime_funcs.zaco_process_exit = Some(process_exit_id);

    // zaco_process_cwd() -> *const i8
    let mut process_cwd_sig = module.make_signature();
    process_cwd_sig.returns.push(AbiParam::new(pointer_type));
    let process_cwd_id = module
        .declare_function("zaco_process_cwd", Linkage::Import, &process_cwd_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_cwd: {}", e)))?;
    runtime_funcs.zaco_process_cwd = Some(process_cwd_id);

    // zaco_process_env_get(key: *const i8) -> *const i8
    let mut process_env_get_sig = module.make_signature();
    process_env_get_sig.params.push(AbiParam::new(pointer_type));
    process_env_get_sig.returns.push(AbiParam::new(pointer_type));
    let process_env_get_id = module
        .declare_function("zaco_process_env_get", Linkage::Import, &process_env_get_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_env_get: {}", e)))?;
    runtime_funcs.zaco_process_env_get = Some(process_env_get_id);

    // zaco_process_pid() -> i64
    let mut process_pid_sig = module.make_signature();
    process_pid_sig.returns.push(AbiParam::new(types::I64));
    let process_pid_id = module
        .declare_function("zaco_process_pid", Linkage::Import, &process_pid_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_pid: {}", e)))?;
    runtime_funcs.zaco_process_pid = Some(process_pid_id);

    // zaco_process_platform() -> *const i8
    let mut process_platform_sig = module.make_signature();
    process_platform_sig.returns.push(AbiParam::new(pointer_type));
    let process_platform_id = module
        .declare_function("zaco_process_platform", Linkage::Import, &process_platform_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_platform: {}", e)))?;
    runtime_funcs.zaco_process_platform = Some(process_platform_id);

    // zaco_process_arch() -> *const i8
    let mut process_arch_sig = module.make_signature();
    process_arch_sig.returns.push(AbiParam::new(pointer_type));
    let process_arch_id = module
        .declare_function("zaco_process_arch", Linkage::Import, &process_arch_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_arch: {}", e)))?;
    runtime_funcs.zaco_process_arch = Some(process_arch_id);

    // zaco_process_argv() -> *const i8
    let mut process_argv_sig = module.make_signature();
    process_argv_sig.returns.push(AbiParam::new(pointer_type));
    let process_argv_id = module
        .declare_function("zaco_process_argv", Linkage::Import, &process_argv_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_process_argv: {}", e)))?;
    runtime_funcs.zaco_process_argv = Some(process_argv_id);

    // ========== Rust Runtime - OS Module ==========

    // zaco_os_platform() -> *const i8
    let mut os_platform_sig = module.make_signature();
    os_platform_sig.returns.push(AbiParam::new(pointer_type));
    let os_platform_id = module
        .declare_function("zaco_os_platform", Linkage::Import, &os_platform_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_platform: {}", e)))?;
    runtime_funcs.zaco_os_platform = Some(os_platform_id);

    // zaco_os_arch() -> *const i8
    let mut os_arch_sig = module.make_signature();
    os_arch_sig.returns.push(AbiParam::new(pointer_type));
    let os_arch_id = module
        .declare_function("zaco_os_arch", Linkage::Import, &os_arch_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_arch: {}", e)))?;
    runtime_funcs.zaco_os_arch = Some(os_arch_id);

    // zaco_os_homedir() -> *const i8
    let mut os_homedir_sig = module.make_signature();
    os_homedir_sig.returns.push(AbiParam::new(pointer_type));
    let os_homedir_id = module
        .declare_function("zaco_os_homedir", Linkage::Import, &os_homedir_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_homedir: {}", e)))?;
    runtime_funcs.zaco_os_homedir = Some(os_homedir_id);

    // zaco_os_tmpdir() -> *const i8
    let mut os_tmpdir_sig = module.make_signature();
    os_tmpdir_sig.returns.push(AbiParam::new(pointer_type));
    let os_tmpdir_id = module
        .declare_function("zaco_os_tmpdir", Linkage::Import, &os_tmpdir_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_tmpdir: {}", e)))?;
    runtime_funcs.zaco_os_tmpdir = Some(os_tmpdir_id);

    // zaco_os_hostname() -> *const i8
    let mut os_hostname_sig = module.make_signature();
    os_hostname_sig.returns.push(AbiParam::new(pointer_type));
    let os_hostname_id = module
        .declare_function("zaco_os_hostname", Linkage::Import, &os_hostname_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_hostname: {}", e)))?;
    runtime_funcs.zaco_os_hostname = Some(os_hostname_id);

    // zaco_os_cpus() -> i64
    let mut os_cpus_sig = module.make_signature();
    os_cpus_sig.returns.push(AbiParam::new(types::I64));
    let os_cpus_id = module
        .declare_function("zaco_os_cpus", Linkage::Import, &os_cpus_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_cpus: {}", e)))?;
    runtime_funcs.zaco_os_cpus = Some(os_cpus_id);

    // zaco_os_totalmem() -> i64
    let mut os_totalmem_sig = module.make_signature();
    os_totalmem_sig.returns.push(AbiParam::new(types::I64));
    let os_totalmem_id = module
        .declare_function("zaco_os_totalmem", Linkage::Import, &os_totalmem_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_totalmem: {}", e)))?;
    runtime_funcs.zaco_os_totalmem = Some(os_totalmem_id);

    // zaco_os_eol() -> *const i8
    let mut os_eol_sig = module.make_signature();
    os_eol_sig.returns.push(AbiParam::new(pointer_type));
    let os_eol_id = module
        .declare_function("zaco_os_eol", Linkage::Import, &os_eol_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_os_eol: {}", e)))?;
    runtime_funcs.zaco_os_eol = Some(os_eol_id);

    // ========== Rust Runtime - HTTP Module ==========

    // zaco_http_get(url: ptr) -> ptr
    let mut http_get_sig = module.make_signature();
    http_get_sig.params.push(AbiParam::new(pointer_type));
    http_get_sig.returns.push(AbiParam::new(pointer_type));
    let http_get_id = module
        .declare_function("zaco_http_get", Linkage::Import, &http_get_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_http_get: {}", e)))?;
    runtime_funcs.zaco_http_get = Some(http_get_id);

    // zaco_http_post(url: ptr, body: ptr, content_type: ptr) -> ptr
    let mut http_post_sig = module.make_signature();
    http_post_sig.params.push(AbiParam::new(pointer_type));
    http_post_sig.params.push(AbiParam::new(pointer_type));
    http_post_sig.params.push(AbiParam::new(pointer_type));
    http_post_sig.returns.push(AbiParam::new(pointer_type));
    let http_post_id = module
        .declare_function("zaco_http_post", Linkage::Import, &http_post_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_http_post: {}", e)))?;
    runtime_funcs.zaco_http_post = Some(http_post_id);

    // zaco_http_put(url: ptr, body: ptr, content_type: ptr) -> ptr
    let mut http_put_sig = module.make_signature();
    http_put_sig.params.push(AbiParam::new(pointer_type));
    http_put_sig.params.push(AbiParam::new(pointer_type));
    http_put_sig.params.push(AbiParam::new(pointer_type));
    http_put_sig.returns.push(AbiParam::new(pointer_type));
    let http_put_id = module
        .declare_function("zaco_http_put", Linkage::Import, &http_put_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_http_put: {}", e)))?;
    runtime_funcs.zaco_http_put = Some(http_put_id);

    // zaco_http_delete(url: ptr) -> ptr
    let mut http_delete_sig = module.make_signature();
    http_delete_sig.params.push(AbiParam::new(pointer_type));
    http_delete_sig.returns.push(AbiParam::new(pointer_type));
    let http_delete_id = module
        .declare_function("zaco_http_delete", Linkage::Import, &http_delete_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_http_delete: {}", e)))?;
    runtime_funcs.zaco_http_delete = Some(http_delete_id);

    // ========== Rust Runtime - Init/Shutdown ==========

    // zaco_runtime_init() -> void
    let runtime_init_sig = module.make_signature();
    let runtime_init_id = module
        .declare_function("zaco_runtime_init", Linkage::Import, &runtime_init_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_runtime_init: {}", e)))?;
    runtime_funcs.zaco_runtime_init = Some(runtime_init_id);

    // zaco_runtime_shutdown() -> void
    let runtime_shutdown_sig = module.make_signature();
    let runtime_shutdown_id = module
        .declare_function("zaco_runtime_shutdown", Linkage::Import, &runtime_shutdown_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_runtime_shutdown: {}", e)))?;
    runtime_funcs.zaco_runtime_shutdown = Some(runtime_shutdown_id);

    // ========== Exception Handling ==========

    // zaco_try_push() -> i64  (returns 0 on initial call, 1 on exception)
    let mut try_push_sig = module.make_signature();
    try_push_sig.returns.push(AbiParam::new(types::I64));
    let try_push_id = module
        .declare_function("zaco_try_push", Linkage::Import, &try_push_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_try_push: {}", e)))?;
    runtime_funcs.zaco_try_push = Some(try_push_id);

    // zaco_try_pop()
    let try_pop_sig = module.make_signature();
    let try_pop_id = module
        .declare_function("zaco_try_pop", Linkage::Import, &try_pop_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_try_pop: {}", e)))?;
    runtime_funcs.zaco_try_pop = Some(try_pop_id);

    // zaco_throw(error: ptr)
    let mut throw_sig = module.make_signature();
    throw_sig.params.push(AbiParam::new(pointer_type));
    let throw_id = module
        .declare_function("zaco_throw", Linkage::Import, &throw_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_throw: {}", e)))?;
    runtime_funcs.zaco_throw = Some(throw_id);

    // zaco_get_error() -> ptr
    let mut get_error_sig = module.make_signature();
    get_error_sig.returns.push(AbiParam::new(pointer_type));
    let get_error_id = module
        .declare_function("zaco_get_error", Linkage::Import, &get_error_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_get_error: {}", e)))?;
    runtime_funcs.zaco_get_error = Some(get_error_id);

    // zaco_clear_error()
    let clear_error_sig = module.make_signature();
    let clear_error_id = module
        .declare_function("zaco_clear_error", Linkage::Import, &clear_error_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_clear_error: {}", e)))?;
    runtime_funcs.zaco_clear_error = Some(clear_error_id);

    // ========== Global Number Functions ==========

    // zaco_parse_int(ptr) -> f64
    let mut parse_int_sig = module.make_signature();
    parse_int_sig.params.push(AbiParam::new(pointer_type));
    parse_int_sig.returns.push(AbiParam::new(types::F64));
    let parse_int_id = module
        .declare_function("zaco_parse_int", Linkage::Import, &parse_int_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_parse_int: {}", e)))?;
    runtime_funcs.zaco_parse_int = Some(parse_int_id);

    // zaco_parse_float(ptr) -> f64
    let mut parse_float_sig = module.make_signature();
    parse_float_sig.params.push(AbiParam::new(pointer_type));
    parse_float_sig.returns.push(AbiParam::new(types::F64));
    let parse_float_id = module
        .declare_function("zaco_parse_float", Linkage::Import, &parse_float_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_parse_float: {}", e)))?;
    runtime_funcs.zaco_parse_float = Some(parse_float_id);

    // zaco_is_nan(f64) -> i64
    let mut is_nan_sig = module.make_signature();
    is_nan_sig.params.push(AbiParam::new(types::F64));
    is_nan_sig.returns.push(AbiParam::new(types::I64));
    let is_nan_id = module
        .declare_function("zaco_is_nan", Linkage::Import, &is_nan_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_is_nan: {}", e)))?;
    runtime_funcs.zaco_is_nan = Some(is_nan_id);

    // zaco_is_finite(f64) -> i64
    let mut is_finite_sig = module.make_signature();
    is_finite_sig.params.push(AbiParam::new(types::F64));
    is_finite_sig.returns.push(AbiParam::new(types::I64));
    let is_finite_id = module
        .declare_function("zaco_is_finite", Linkage::Import, &is_finite_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_is_finite: {}", e)))?;
    runtime_funcs.zaco_is_finite = Some(is_finite_id);

    // ========== Console Warn (missing f64/bool) ==========

    // zaco_console_warn_f64(f64)
    let mut console_warn_f64_sig = module.make_signature();
    console_warn_f64_sig.params.push(AbiParam::new(types::F64));
    let console_warn_f64_id = module
        .declare_function("zaco_console_warn_f64", Linkage::Import, &console_warn_f64_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_warn_f64: {}", e)))?;
    runtime_funcs.zaco_console_warn_f64 = Some(console_warn_f64_id);

    // zaco_console_warn_bool(i64)
    let mut console_warn_bool_sig = module.make_signature();
    console_warn_bool_sig.params.push(AbiParam::new(types::I64));
    let console_warn_bool_id = module
        .declare_function("zaco_console_warn_bool", Linkage::Import, &console_warn_bool_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_console_warn_bool: {}", e)))?;
    runtime_funcs.zaco_console_warn_bool = Some(console_warn_bool_id);

    // ========== Timer Functions ==========

    // zaco_set_timeout(callback: ptr, context: ptr, delay_ms: i64) -> i64
    let mut set_timeout_sig = module.make_signature();
    set_timeout_sig.params.push(AbiParam::new(pointer_type));
    set_timeout_sig.params.push(AbiParam::new(pointer_type));
    set_timeout_sig.params.push(AbiParam::new(types::I64));
    set_timeout_sig.returns.push(AbiParam::new(types::I64));
    let set_timeout_id = module
        .declare_function("zaco_set_timeout", Linkage::Import, &set_timeout_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_set_timeout: {}", e)))?;
    runtime_funcs.zaco_set_timeout = Some(set_timeout_id);

    // zaco_set_interval(callback: ptr, context: ptr, delay_ms: i64) -> i64
    let mut set_interval_sig = module.make_signature();
    set_interval_sig.params.push(AbiParam::new(pointer_type));
    set_interval_sig.params.push(AbiParam::new(pointer_type));
    set_interval_sig.params.push(AbiParam::new(types::I64));
    set_interval_sig.returns.push(AbiParam::new(types::I64));
    let set_interval_id = module
        .declare_function("zaco_set_interval", Linkage::Import, &set_interval_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_set_interval: {}", e)))?;
    runtime_funcs.zaco_set_interval = Some(set_interval_id);

    // zaco_clear_timeout(timer_id: i64)
    let mut clear_timeout_sig = module.make_signature();
    clear_timeout_sig.params.push(AbiParam::new(types::I64));
    let clear_timeout_id = module
        .declare_function("zaco_clear_timeout", Linkage::Import, &clear_timeout_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_clear_timeout: {}", e)))?;
    runtime_funcs.zaco_clear_timeout = Some(clear_timeout_id);

    // zaco_clear_interval(timer_id: i64)
    let mut clear_interval_sig = module.make_signature();
    clear_interval_sig.params.push(AbiParam::new(types::I64));
    let clear_interval_id = module
        .declare_function("zaco_clear_interval", Linkage::Import, &clear_interval_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_clear_interval: {}", e)))?;
    runtime_funcs.zaco_clear_interval = Some(clear_interval_id);

    // ========== Async FS ==========

    // zaco_fs_read_file(path: ptr, encoding: ptr, callback: ptr)
    let mut fs_read_file_sig = module.make_signature();
    fs_read_file_sig.params.push(AbiParam::new(pointer_type));
    fs_read_file_sig.params.push(AbiParam::new(pointer_type));
    fs_read_file_sig.params.push(AbiParam::new(pointer_type));
    let fs_read_file_id = module
        .declare_function("zaco_fs_read_file", Linkage::Import, &fs_read_file_sig)
        .map_err(|e| CodegenError::new(format!("Failed to declare zaco_fs_read_file: {}", e)))?;
    runtime_funcs.zaco_fs_read_file = Some(fs_read_file_id);

    Ok(())
}
