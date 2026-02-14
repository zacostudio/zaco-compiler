//! Zaco Native Code Generator using Cranelift
//!
//! This module provides native code generation capabilities for the Zaco compiler
//! using the Cranelift code generator. It translates Zaco IR to native machine code.

mod error;
mod runtime;
mod translator;

pub use error::CodegenError;

use cranelift::prelude::*;
use cranelift_module::{DataDescription, FuncId as ClifFuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

// Import from zaco_ir with explicit names to avoid conflicts
use zaco_ir::{
    FuncId, IrFunction, IrModule, IrType,
};

use crate::runtime::{RuntimeFunctions, declare_runtime_functions};
use crate::translator::FunctionTranslator;

/// Main code generator that translates Zaco IR to native code via Cranelift
pub struct CodeGenerator {
    /// Cranelift object module for producing object files
    module: ObjectModule,
    /// Cranelift context for function compilation
    ctx: codegen::Context,
    /// Function builder context (reused across functions)
    func_builder_ctx: FunctionBuilderContext,
    /// Cranelift pointer type for the target
    pointer_type: Type,
    /// Map from Zaco function IDs to Cranelift function IDs
    func_id_map: HashMap<FuncId, ClifFuncId>,
    /// Runtime function IDs
    runtime_funcs: RuntimeFunctions,
    /// String literal data IDs
    string_data_map: HashMap<usize, cranelift_module::DataId>,
}

impl CodeGenerator {
    /// Create a new code generator with native target configuration
    pub fn new() -> Result<Self, CodegenError> {
        // Get native target triple
        let _triple = target_lexicon::Triple::host();

        // Create ISA builder for the target
        let isa_builder = cranelift_native::builder()
            .map_err(|e| CodegenError::new(format!("Failed to create ISA builder: {}", e)))?;

        // Configure settings — enable PIC for macOS linker compatibility
        let mut flag_builder = settings::builder();
        flag_builder
            .set("is_pic", "true")
            .map_err(|e| CodegenError::new(format!("Failed to set is_pic: {}", e)))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError::new(format!("Failed to create ISA: {}", e)))?;

        // Determine pointer type based on target
        let pointer_type = isa.pointer_type();

        // Create object builder
        let builder = ObjectBuilder::new(
            isa,
            "zaco_module",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| CodegenError::new(format!("Failed to create object builder: {}", e)))?;

        // Create module
        let module = ObjectModule::new(builder);

        Ok(Self {
            module,
            ctx: codegen::Context::new(),
            func_builder_ctx: FunctionBuilderContext::new(),
            pointer_type,
            func_id_map: HashMap::new(),
            runtime_funcs: RuntimeFunctions::default(),
            string_data_map: HashMap::new(),
        })
    }

    /// Compile a complete IR module to object file bytes
    pub fn compile_module(mut self, ir_module: &IrModule) -> Result<Vec<u8>, CodegenError> {
        // Declare runtime functions first
        declare_runtime_functions(&mut self.module, &mut self.runtime_funcs, self.pointer_type)?;

        // Declare all functions (for forward references)
        for function in &ir_module.functions {
            self.declare_function(function)?;
        }

        // Declare string literals as data objects
        for (idx, string) in ir_module.string_literals.iter().enumerate() {
            self.declare_string_literal(idx, string)?;
        }

        // Compile each function
        for function in &ir_module.functions {
            self.compile_function(function, ir_module)?;
        }

        // Finalize the module and produce object file (consumes self.module)
        let object_product = self.module.finish();

        Ok(object_product
            .emit()
            .map_err(|e| CodegenError::new(format!("Failed to emit object file: {}", e)))?)
    }

    /// Declare a function signature in the module
    fn declare_function(&mut self, ir_func: &IrFunction) -> Result<(), CodegenError> {
        let mut signature = self.module.make_signature();

        // Add parameters
        for (_, ty) in &ir_func.params {
            let cl_type = self.ir_type_to_cranelift(ty)?;
            signature.params.push(AbiParam::new(cl_type));
        }

        // Add return type
        if ir_func.return_type != IrType::Void {
            let cl_type = self.ir_type_to_cranelift(&ir_func.return_type)?;
            signature.returns.push(AbiParam::new(cl_type));
        }

        // Determine linkage
        let linkage = if ir_func.is_public || ir_func.name == "main" {
            Linkage::Export
        } else {
            Linkage::Local
        };

        // Declare function
        let clif_func_id = self
            .module
            .declare_function(&ir_func.name, linkage, &signature)
            .map_err(|e| CodegenError::new(format!("Failed to declare function: {}", e)))?;

        self.func_id_map.insert(ir_func.id, clif_func_id);

        Ok(())
    }

    /// Declare a string literal as a data object (null-terminated for C runtime)
    fn declare_string_literal(&mut self, index: usize, string: &str) -> Result<(), CodegenError> {
        let mut data_desc = DataDescription::new();
        // Include null terminator for C string compatibility
        let mut bytes = string.as_bytes().to_vec();
        bytes.push(0);
        data_desc.define(bytes.into_boxed_slice());

        let name = format!("str_literal_{}", index);
        let data_id = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| {
                CodegenError::new(format!("Failed to declare string literal: {}", e))
            })?;

        self.module.define_data(data_id, &data_desc).map_err(|e| {
            CodegenError::new(format!("Failed to define string literal: {}", e))
        })?;

        self.string_data_map.insert(index, data_id);

        Ok(())
    }

    /// Compile a single function
    pub fn compile_function(
        &mut self,
        ir_func: &IrFunction,
        ir_module: &IrModule,
    ) -> Result<(), CodegenError> {
        // Get Cranelift function ID
        let clif_func_id = *self
            .func_id_map
            .get(&ir_func.id)
            .ok_or_else(|| CodegenError::new(format!("Function {} not declared", ir_func.name)))?;

        // Clear and build signature using the target's default calling convention
        self.ctx.func.signature.clear(self.module.isa().default_call_conv());
        for (_, ty) in &ir_func.params {
            let cl_type = self.ir_type_to_cranelift(ty)?;
            self.ctx.func.signature.params.push(AbiParam::new(cl_type));
        }
        if ir_func.return_type != IrType::Void {
            let cl_type = self.ir_type_to_cranelift(&ir_func.return_type)?;
            self.ctx
                .func
                .signature
                .returns
                .push(AbiParam::new(cl_type));
        }

        // Create FunctionBuilder using split borrows from self
        let pointer_type = self.pointer_type;
        let builder = FunctionBuilder::new(
            &mut self.ctx.func,
            &mut self.func_builder_ctx,
        );

        // Create translator with separate field borrows (no conflict with builder)
        let mut translator = FunctionTranslator::new(
            &mut self.module,
            &self.func_id_map,
            &self.runtime_funcs,
            &self.string_data_map,
            ir_func,
            ir_module,
            pointer_type,
        );

        // Translate the function (pass builder by value since finalize() consumes it)
        translator.translate(builder)?;

        // Verify the function before defining (to get detailed error messages)
        if let Err(errors) = cranelift::codegen::verify_function(&self.ctx.func, self.module.isa()) {
            return Err(CodegenError::new(format!(
                "Verifier errors in function '{}':\n{}",
                ir_func.name, errors
            )));
        }

        // Define the function in the module
        self.module
            .define_function(clif_func_id, &mut self.ctx)
            .map_err(|e| CodegenError::new(format!("Failed to define function: {}", e)))?;

        // Clear the context for the next function
        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    /// Convert IR type to Cranelift type
    fn ir_type_to_cranelift(&self, ir_type: &IrType) -> Result<Type, CodegenError> {
        let cl_type = match ir_type {
            IrType::I64 => types::I64,
            IrType::F64 => types::F64,
            IrType::Bool => types::I8,
            IrType::Ptr => self.pointer_type,
            IrType::Str => self.pointer_type,
            IrType::Array(_) => self.pointer_type,
            IrType::Struct(_) => self.pointer_type,
            IrType::FuncPtr(_) => self.pointer_type,
            IrType::Promise(_) => self.pointer_type,
            IrType::Void => {
                return Err(CodegenError::new("Cannot convert Void to Cranelift type"));
            }
        };
        Ok(cl_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaco_ir::{Constant, Instruction, Place, RValue, Terminator, Value as IrValue};

    #[test]
    fn test_codegen_creation() {
        let result = CodeGenerator::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_conversion() {
        let codegen = CodeGenerator::new().unwrap();

        assert_eq!(
            codegen.ir_type_to_cranelift(&IrType::I64).unwrap(),
            types::I64
        );
        assert_eq!(
            codegen.ir_type_to_cranelift(&IrType::F64).unwrap(),
            types::F64
        );
        assert_eq!(
            codegen.ir_type_to_cranelift(&IrType::Bool).unwrap(),
            types::I8
        );
        assert!(codegen.ir_type_to_cranelift(&IrType::Ptr).is_ok());
        assert!(codegen.ir_type_to_cranelift(&IrType::Void).is_err());
    }

    #[test]
    fn test_simple_function_compile() {
        let codegen = CodeGenerator::new().unwrap();
        let mut module = IrModule::new();

        // Create a simple function: fn main() -> i64 { return 42; }
        let mut func = IrFunction::new(FuncId(0), "main".to_string(), vec![], IrType::I64);
        func.is_public = true;

        let entry = func.new_block();
        func.entry_block = entry;

        let result_temp = func.add_temp(IrType::I64);
        func.block_mut(entry).push_instruction(Instruction::Assign {
            dest: Place::from_temp(result_temp),
            value: RValue::Use(IrValue::Const(Constant::I64(42))),
        });
        func.block_mut(entry)
            .set_terminator(Terminator::Return(Some(IrValue::Temp(result_temp))));

        module.add_function(func);

        let result = codegen.compile_module(&module);
        assert!(result.is_ok());
    }
}
