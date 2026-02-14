//! Function translation logic for converting Zaco IR to Cranelift IR

use cranelift::prelude::*;
use cranelift_module::{FuncId as ClifFuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use std::collections::HashMap;

use zaco_ir::{
    BinOp, Block as IrBlock, BlockId, Constant, FuncId, IrFunction, IrModule, IrType, Instruction,
    LocalId, Place, Projection, RValue, StructId, TempId, Terminator, UnOp, Value as IrValue,
};

use crate::error::CodegenError;
use crate::runtime::RuntimeFunctions;

// Alias Cranelift types to avoid conflicts
use cranelift::prelude::Value as ClifValue;
use cranelift::prelude::Block as ClifBlock;

/// Context for translating a single function
pub(crate) struct FunctionTranslator<'a> {
    /// Module reference for declaring function references
    module: &'a mut ObjectModule,
    /// Map from Zaco function IDs to Cranelift function IDs
    func_id_map: &'a HashMap<FuncId, ClifFuncId>,
    /// Runtime function IDs
    runtime_funcs: &'a RuntimeFunctions,
    /// Map from string literal indices to data IDs
    #[allow(dead_code)]
    string_data_map: &'a HashMap<usize, cranelift_module::DataId>,
    /// Map from Zaco locals/temps to Cranelift values
    value_map: HashMap<ValueKey, ClifValue>,
    /// Map from Zaco block IDs to Cranelift blocks
    block_map: HashMap<BlockId, ClifBlock>,
    /// Current IR function being translated
    ir_func: &'a IrFunction,
    /// IR module for looking up types and functions
    ir_module: &'a IrModule,
    /// Cached pointer type
    pointer_type: Type,
}

/// Key for value mapping (Local or Temp)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ValueKey {
    Local(LocalId),
    Temp(TempId),
}

impl<'a> FunctionTranslator<'a> {
    /// Create a new function translator
    pub(crate) fn new(
        module: &'a mut ObjectModule,
        func_id_map: &'a HashMap<FuncId, ClifFuncId>,
        runtime_funcs: &'a RuntimeFunctions,
        string_data_map: &'a HashMap<usize, cranelift_module::DataId>,
        ir_func: &'a IrFunction,
        ir_module: &'a IrModule,
        pointer_type: Type,
    ) -> Self {
        Self {
            module,
            func_id_map,
            runtime_funcs,
            string_data_map,
            value_map: HashMap::new(),
            block_map: HashMap::new(),
            ir_func,
            ir_module,
            pointer_type,
        }
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

    /// Translate the entire function
    pub(crate) fn translate(&mut self, mut builder: FunctionBuilder) -> Result<(), CodegenError> {
        // Create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // If this is the main function, call runtime init
        if self.ir_func.name == "main" {
            if let Some(runtime_init_id) = self.runtime_funcs.zaco_runtime_init {
                let func_ref = self.module.declare_func_in_func(runtime_init_id, builder.func);
                builder.ins().call(func_ref, &[]);
            }
        }

        // Map function parameters
        for (i, (local_id, _)) in self.ir_func.params.iter().enumerate() {
            let value = builder.block_params(entry_block)[i];
            self.value_map.insert(ValueKey::Local(*local_id), value);
        }

        // Create slots for all locals (excluding parameters)
        for (local_id, ty) in &self.ir_func.locals {
            if !self.value_map.contains_key(&ValueKey::Local(*local_id)) {
                // Create stack slot for local variable
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    ty.size_bytes() as u32,
                    0,
                ));
                // Store the slot address as the "value" for this local
                let addr = builder.ins().stack_addr(self.pointer_type, slot, 0);
                self.value_map.insert(ValueKey::Local(*local_id), addr);
            }
        }

        // Create Cranelift blocks for all IR blocks
        for block in &self.ir_func.blocks {
            let clif_block = builder.create_block();
            self.block_map.insert(block.id, clif_block);
        }

        // Always jump from the Cranelift entry block to the IR entry block
        let target = *self.block_map.get(&self.ir_func.entry_block).ok_or_else(|| {
            CodegenError::new(format!("Entry block {:?} not found", self.ir_func.entry_block))
        })?;
        builder.ins().jump(target, &[]);

        // Translate each basic block
        let blocks_to_translate: Vec<_> = self.ir_func.blocks.clone();
        for ir_block in &blocks_to_translate {
            self.translate_block(&mut builder, ir_block)?;
        }

        // Seal all blocks at once (handles arbitrary CFG shapes like switch, loops)
        builder.seal_all_blocks();

        // Finalize
        builder.finalize();

        Ok(())
    }

    /// Translate a basic block
    fn translate_block(
        &mut self,
        builder: &mut FunctionBuilder,
        ir_block: &IrBlock,
    ) -> Result<(), CodegenError> {
        let clif_block = *self.block_map.get(&ir_block.id).ok_or_else(|| {
            CodegenError::new(format!("Block {:?} not found in block_map", ir_block.id))
        })?;

        builder.switch_to_block(clif_block);

        // Translate instructions
        for instr in &ir_block.instructions {
            self.translate_instruction(builder, instr)?;
        }

        // Translate terminator
        self.translate_terminator(builder, &ir_block.terminator)?;

        Ok(())
    }

    /// Translate a single instruction
    fn translate_instruction(
        &mut self,
        builder: &mut FunctionBuilder,
        instr: &Instruction,
    ) -> Result<(), CodegenError> {
        match instr {
            Instruction::Assign { dest, value } => {
                let result_val = self.translate_rvalue(builder, value)?;
                self.store_to_place(builder, dest, result_val)?;
            }

            Instruction::Call { dest, func, args } => {
                let result = self.translate_call(builder, func, args)?;
                if let (Some(dest), Some(val)) = (dest, result) {
                    self.store_to_place(builder, dest, val)?;
                }
            }

            Instruction::Return(_) => {
                // Handled by terminator
            }

            Instruction::Branch { .. } | Instruction::Jump(_) => {
                // Handled by terminator
            }

            Instruction::Alloc { dest, ty } => {
                let size = ty.size_bytes() as i64;
                let size_val = builder.ins().iconst(types::I64, size);

                let alloc_fn = self
                    .runtime_funcs
                    .zaco_alloc
                    .ok_or_else(|| CodegenError::new("zaco_alloc not declared"))?;
                let func_ref = self.module.declare_func_in_func(alloc_fn, builder.func);
                let call = builder.ins().call(func_ref, &[size_val]);
                let ptr = builder.inst_results(call)[0];

                self.store_to_place(builder, dest, ptr)?;
            }

            Instruction::Free { value } => {
                let ptr = self.translate_value(builder, value)?;
                let free_fn = self
                    .runtime_funcs
                    .zaco_free
                    .ok_or_else(|| CodegenError::new("zaco_free not declared"))?;
                let func_ref = self.module.declare_func_in_func(free_fn, builder.func);
                builder.ins().call(func_ref, &[ptr]);
            }

            Instruction::RefCount { value, delta } => {
                let ptr = self.translate_value(builder, value)?;
                if *delta > 0 {
                    // Increment
                    for _ in 0..*delta {
                        let rc_inc_fn = self
                            .runtime_funcs
                            .zaco_rc_inc
                            .ok_or_else(|| CodegenError::new("zaco_rc_inc not declared"))?;
                        let func_ref =
                            self.module.declare_func_in_func(rc_inc_fn, builder.func);
                        builder.ins().call(func_ref, &[ptr]);
                    }
                } else if *delta < 0 {
                    // Decrement — use array-specific RC dec for array types
                    let is_array = matches!(self.infer_value_ir_type(value), Some(IrType::Array(_)));
                    for _ in 0..delta.abs() {
                        let rc_dec_fn = if is_array {
                            self.runtime_funcs
                                .zaco_array_rc_dec
                                .ok_or_else(|| CodegenError::new("zaco_array_rc_dec not declared"))?
                        } else {
                            self.runtime_funcs
                                .zaco_rc_dec
                                .ok_or_else(|| CodegenError::new("zaco_rc_dec not declared"))?
                        };
                        let func_ref =
                            self.module.declare_func_in_func(rc_dec_fn, builder.func);
                        builder.ins().call(func_ref, &[ptr]);
                    }
                }
            }

            Instruction::Clone { dest, source } => {
                // Copy the pointer and increment reference count
                let val = self.translate_value(builder, source)?;
                if let Some(rc_inc_fn) = self.runtime_funcs.zaco_rc_inc {
                    let func_ref = self.module.declare_func_in_func(rc_inc_fn, builder.func);
                    builder.ins().call(func_ref, &[val]);
                }
                self.store_to_place(builder, dest, val)?;
            }

            Instruction::Store { ptr, value } => {
                let ptr_val = self.translate_value(builder, ptr)?;
                let val = self.translate_value(builder, value)?;
                builder.ins().store(MemFlags::new(), val, ptr_val, 0);
            }

            Instruction::Load { dest, ptr } => {
                let ptr_val = self.translate_value(builder, ptr)?;
                // Infer type from destination
                let ty = self.infer_place_type(dest)?;
                let cl_type = self.ir_type_to_cranelift(&ty)?;
                let val = builder.ins().load(cl_type, MemFlags::new(), ptr_val, 0);
                self.store_to_place(builder, dest, val)?;
            }
        }

        Ok(())
    }

    /// Translate a terminator instruction
    fn translate_terminator(
        &mut self,
        builder: &mut FunctionBuilder,
        terminator: &Terminator,
    ) -> Result<(), CodegenError> {
        match terminator {
            Terminator::Return(val_opt) => {
                // If this is the main function, call runtime shutdown before returning
                if self.ir_func.name == "main" {
                    if let Some(runtime_shutdown_id) = self.runtime_funcs.zaco_runtime_shutdown {
                        let func_ref = self.module.declare_func_in_func(runtime_shutdown_id, builder.func);
                        builder.ins().call(func_ref, &[]);
                    }
                }

                if let Some(val) = val_opt {
                    let return_val = self.translate_value(builder, val)?;
                    builder.ins().return_(&[return_val]);
                } else {
                    builder.ins().return_(&[]);
                }
            }

            Terminator::Branch {
                cond,
                then_block,
                else_block,
            } => {
                let cond_val = self.translate_value(builder, cond)?;
                // brif requires an integer condition — convert F64 to bool if needed
                let cond_int = {
                    let val_type = builder.func.dfg.value_type(cond_val);
                    if val_type == types::F64 {
                        let zero = builder.ins().f64const(0.0);
                        builder.ins().fcmp(FloatCC::NotEqual, cond_val, zero)
                    } else if val_type == types::F32 {
                        let zero = builder.ins().f32const(0.0);
                        builder.ins().fcmp(FloatCC::NotEqual, cond_val, zero)
                    } else {
                        cond_val
                    }
                };
                let then_bl = *self.block_map.get(then_block).ok_or_else(|| {
                    CodegenError::new(format!("Block {:?} not found", then_block))
                })?;
                let else_bl = *self.block_map.get(else_block).ok_or_else(|| {
                    CodegenError::new(format!("Block {:?} not found", else_block))
                })?;
                builder.ins().brif(cond_int, then_bl, &[], else_bl, &[]);
            }

            Terminator::Jump(target) => {
                let target_block = *self.block_map.get(target).ok_or_else(|| {
                    CodegenError::new(format!("Block {:?} not found", target))
                })?;
                builder.ins().jump(target_block, &[]);
            }

            Terminator::Unreachable => {
                // Use a user trap code (0 = unreachable)
                if let Some(trap_code) = TrapCode::user(0) {
                    builder.ins().trap(trap_code);
                } else {
                    // Fallback: use HeapOutOfBounds as a generic trap
                    builder.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
                }
            }
        }

        Ok(())
    }

    /// Translate an RValue (right-hand side of assignment)
    fn translate_rvalue(
        &mut self,
        builder: &mut FunctionBuilder,
        rvalue: &RValue,
    ) -> Result<ClifValue, CodegenError> {
        match rvalue {
            RValue::Use(value) => self.translate_value(builder, value),

            RValue::BinaryOp { op, left, right } => {
                let lhs = self.translate_value(builder, left)?;
                let rhs = self.translate_value(builder, right)?;
                self.translate_binop(builder, *op, lhs, rhs)
            }

            RValue::UnaryOp { op, operand } => {
                let operand_val = self.translate_value(builder, operand)?;
                self.translate_unop(builder, *op, operand_val)
            }

            RValue::Cast { value, ty } => {
                let val = self.translate_value(builder, value)?;
                let src_ty = builder.func.dfg.value_type(val);
                let dst_ty = self.ir_type_to_cranelift(ty)?;

                if src_ty == dst_ty {
                    Ok(val)
                } else if src_ty == types::I64 && dst_ty == types::F64 {
                    // I64 → F64: fcvt_from_sint
                    Ok(builder.ins().fcvt_from_sint(types::F64, val))
                } else if src_ty == types::F64 && dst_ty == types::I64 {
                    // F64 → I64: saturating conversion (safe for NaN/Inf)
                    Ok(builder.ins().fcvt_to_sint_sat(types::I64, val))
                } else if src_ty == types::I64 && dst_ty == types::I8 {
                    // I64 → I8 (bool): ireduce
                    Ok(builder.ins().ireduce(types::I8, val))
                } else if src_ty == types::I8 && dst_ty == types::I64 {
                    // I8 → I64: uextend
                    Ok(builder.ins().uextend(types::I64, val))
                } else if src_ty == types::F64 && dst_ty == types::I8 {
                    // F64 → I8 (bool): fcvt_to_sint then ireduce
                    let i64_val = builder.ins().fcvt_to_sint_sat(types::I64, val);
                    Ok(builder.ins().ireduce(types::I8, i64_val))
                } else if src_ty == types::I8 && dst_ty == types::F64 {
                    // I8 → F64: uextend to I64 then fcvt_from_sint
                    let i64_val = builder.ins().uextend(types::I64, val);
                    Ok(builder.ins().fcvt_from_sint(types::F64, i64_val))
                } else if src_ty.is_int() && dst_ty.is_int() {
                    if src_ty.bits() < dst_ty.bits() {
                        Ok(builder.ins().uextend(dst_ty, val))
                    } else {
                        Ok(builder.ins().ireduce(dst_ty, val))
                    }
                } else {
                    // Unknown cast - pass through
                    Ok(val)
                }
            }

            RValue::StructInit { struct_id, fields } => {
                // Allocate struct on heap
                let struct_def = self
                    .ir_module
                    .struct_def(*struct_id)
                    .ok_or_else(|| CodegenError::new(format!("Struct {:?} not found", struct_id)))?;

                let size = struct_def.size_bytes() as i64;
                let size_val = builder.ins().iconst(types::I64, size);

                let alloc_fn = self
                    .runtime_funcs
                    .zaco_alloc
                    .ok_or_else(|| CodegenError::new("zaco_alloc not declared"))?;
                let func_ref = self.module.declare_func_in_func(alloc_fn, builder.func);
                let call = builder.ins().call(func_ref, &[size_val]);
                let ptr = builder.inst_results(call)[0];

                // Store each field
                let mut offset = 0i32;
                for (i, field_val) in fields.iter().enumerate() {
                    let val = self.translate_value(builder, field_val)?;
                    builder.ins().store(MemFlags::new(), val, ptr, offset);
                    if let Some(field_ty) = struct_def.field_type(i) {
                        offset += field_ty.size_bytes() as i32;
                    }
                }

                Ok(ptr)
            }

            RValue::ArrayInit(elements) => {
                // Allocate array on heap
                // Array layout: [length: i64][elements...]
                if elements.is_empty() {
                    // Empty array - return null or allocate header only
                    let size_val = builder.ins().iconst(types::I64, 8); // Just length field
                    let alloc_fn = self
                        .runtime_funcs
                        .zaco_alloc
                        .ok_or_else(|| CodegenError::new("zaco_alloc not declared"))?;
                    let func_ref =
                        self.module.declare_func_in_func(alloc_fn, builder.func);
                    let call = builder.ins().call(func_ref, &[size_val]);
                    let ptr = builder.inst_results(call)[0];
                    let len = builder.ins().iconst(types::I64, 0);
                    builder.ins().store(MemFlags::new(), len, ptr, 0);
                    Ok(ptr)
                } else {
                    // Translate all elements first to determine actual element size
                    let mut translated_elems = Vec::new();
                    for elem in elements {
                        let val = self.translate_value(builder, elem)?;
                        translated_elems.push(val);
                    }
                    let elem_size = if let Some(&first) = translated_elems.first() {
                        builder.func.dfg.value_type(first).bytes() as usize
                    } else {
                        8
                    };
                    let total_size = 8 + (translated_elems.len() * elem_size);
                    let size_val = builder.ins().iconst(types::I64, total_size as i64);

                    let alloc_fn = self
                        .runtime_funcs
                        .zaco_alloc
                        .ok_or_else(|| CodegenError::new("zaco_alloc not declared"))?;
                    let func_ref =
                        self.module.declare_func_in_func(alloc_fn, builder.func);
                    let call = builder.ins().call(func_ref, &[size_val]);
                    let ptr = builder.inst_results(call)[0];

                    // Store length
                    let len = builder.ins().iconst(types::I64, translated_elems.len() as i64);
                    builder.ins().store(MemFlags::new(), len, ptr, 0);

                    // Store elements with actual element size
                    for (i, val) in translated_elems.iter().enumerate() {
                        let offset = 8 + (i * elem_size);
                        builder
                            .ins()
                            .store(MemFlags::new(), *val, ptr, offset as i32);
                    }

                    Ok(ptr)
                }
            }

            RValue::StrConcat(values) => {
                if values.is_empty() {
                    // Allocate an empty string via zaco_str_new
                    if let Some(idx) = self.ir_module.string_literals.iter().position(|s| s.is_empty()) {
                        if let Some(&data_id) = self.string_data_map.get(&idx) {
                            let gv = self.module.declare_data_in_func(data_id, builder.func);
                            let raw_ptr = builder.ins().global_value(self.pointer_type, gv);
                            let str_new_fn = self.runtime_funcs.zaco_str_new
                                .ok_or_else(|| CodegenError::new("zaco_str_new not declared"))?;
                            let func_ref = self.module.declare_func_in_func(str_new_fn, builder.func);
                            let call = builder.ins().call(func_ref, &[raw_ptr]);
                            return Ok(builder.inst_results(call)[0]);
                        }
                    }
                    // Fallback: return null pointer if empty string not interned
                    return Ok(builder.ins().iconst(self.pointer_type, 0));
                }

                // Concatenate strings by calling runtime function repeatedly
                let mut result = self.translate_value(builder, &values[0])?;
                for val in &values[1..] {
                    let next = self.translate_value(builder, val)?;
                    let concat_fn = self
                        .runtime_funcs
                        .zaco_str_concat
                        .ok_or_else(|| CodegenError::new("zaco_str_concat not declared"))?;
                    let func_ref =
                        self.module.declare_func_in_func(concat_fn, builder.func);
                    let call = builder.ins().call(func_ref, &[result, next]);
                    result = builder.inst_results(call)[0];
                }

                Ok(result)
            }
        }
    }

    /// Translate a value
    fn translate_value(
        &mut self,
        builder: &mut FunctionBuilder,
        value: &IrValue,
    ) -> Result<ClifValue, CodegenError> {
        match value {
            IrValue::Const(constant) => self.translate_constant(builder, constant),

            IrValue::Local(local_id) => {
                // For locals stored on stack, load the value
                let addr = self
                    .value_map
                    .get(&ValueKey::Local(*local_id))
                    .copied()
                    .ok_or_else(|| {
                        CodegenError::new(format!("Local {:?} not found", local_id))
                    })?;

                // Determine type of local
                let ty = self
                    .ir_func
                    .locals
                    .iter()
                    .find(|(id, _)| id == local_id)
                    .map(|(_, ty)| ty)
                    .ok_or_else(|| CodegenError::new(format!("Local {:?} type not found", local_id)))?;

                // If it's a parameter (already a value), return directly
                if self.ir_func.params.iter().any(|(id, _)| id == local_id) {
                    return Ok(addr);
                }

                // Otherwise load from stack slot
                let cl_type = self.ir_type_to_cranelift(ty)?;
                Ok(builder.ins().load(cl_type, MemFlags::new(), addr, 0))
            }

            IrValue::Temp(temp_id) => {
                self.value_map
                    .get(&ValueKey::Temp(*temp_id))
                    .copied()
                    .ok_or_else(|| CodegenError::new(format!("Temp {:?} not found", temp_id)))
            }
        }
    }

    /// Translate a constant
    fn translate_constant(
        &mut self,
        builder: &mut FunctionBuilder,
        constant: &Constant,
    ) -> Result<ClifValue, CodegenError> {
        let val = match constant {
            Constant::I64(n) => builder.ins().iconst(types::I64, *n),
            Constant::F64(f) => builder.ins().f64const(*f),
            Constant::Bool(b) => builder.ins().iconst(types::I8, if *b { 1 } else { 0 }),
            Constant::Null => builder.ins().iconst(self.pointer_type, 0),
            Constant::Str(s) => {
                // Look up interned string in string_data_map
                if let Some(idx) = self.ir_module.string_literals.iter().position(|lit| lit == s) {
                    if let Some(&data_id) = self.string_data_map.get(&idx) {
                        // Get a pointer to the raw string data
                        let gv = self
                            .module
                            .declare_data_in_func(data_id, builder.func);
                        let raw_ptr = builder.ins().global_value(self.pointer_type, gv);

                        // Call zaco_str_new to create a managed string from raw data
                        let str_new_fn = self
                            .runtime_funcs
                            .zaco_str_new
                            .ok_or_else(|| CodegenError::new("zaco_str_new not declared"))?;
                        let func_ref =
                            self.module.declare_func_in_func(str_new_fn, builder.func);
                        let call = builder.ins().call(func_ref, &[raw_ptr]);
                        builder.inst_results(call)[0]
                    } else {
                        // String not in data map - shouldn't happen if lowering is correct
                        return Err(CodegenError::new(format!(
                            "String literal '{}' not found in data map",
                            s
                        )));
                    }
                } else {
                    return Err(CodegenError::new(format!(
                        "String literal '{}' not interned in module",
                        s
                    )));
                }
            }
        };
        Ok(val)
    }

    /// Translate a binary operation
    fn translate_binop(
        &self,
        builder: &mut FunctionBuilder,
        op: BinOp,
        lhs: ClifValue,
        rhs: ClifValue,
    ) -> Result<ClifValue, CodegenError> {
        let lhs_ty = builder.func.dfg.value_type(lhs);
        let is_float = lhs_ty == types::F64;

        let val = match op {
            BinOp::Add => {
                if is_float { builder.ins().fadd(lhs, rhs) }
                else { builder.ins().iadd(lhs, rhs) }
            }
            BinOp::Sub => {
                if is_float { builder.ins().fsub(lhs, rhs) }
                else { builder.ins().isub(lhs, rhs) }
            }
            BinOp::Mul => {
                if is_float { builder.ins().fmul(lhs, rhs) }
                else { builder.ins().imul(lhs, rhs) }
            }
            BinOp::Div => {
                if is_float {
                    // F64 div by zero naturally yields Inf/NaN per IEEE 754
                    builder.ins().fdiv(lhs, rhs)
                } else {
                    // Guard: replace 0 divisor with 1 to avoid trap, then select 0
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_zero = builder.ins().icmp(IntCC::Equal, rhs, zero);
                    let one = builder.ins().iconst(types::I64, 1);
                    let safe_rhs = builder.ins().select(is_zero, one, rhs);
                    let div_result = builder.ins().sdiv(lhs, safe_rhs);
                    builder.ins().select(is_zero, zero, div_result)
                }
            }
            BinOp::Mod => {
                if is_float {
                    // fmod: a - floor(a/b) * b
                    let div = builder.ins().fdiv(lhs, rhs);
                    let floored = builder.ins().floor(div);
                    let product = builder.ins().fmul(floored, rhs);
                    builder.ins().fsub(lhs, product)
                } else {
                    // Guard: replace 0 divisor with 1 to avoid trap
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_zero = builder.ins().icmp(IntCC::Equal, rhs, zero);
                    let one = builder.ins().iconst(types::I64, 1);
                    let safe_rhs = builder.ins().select(is_zero, one, rhs);
                    let rem_result = builder.ins().srem(lhs, safe_rhs);
                    builder.ins().select(is_zero, zero, rem_result)
                }
            }
            BinOp::Eq => {
                if is_float { builder.ins().fcmp(FloatCC::Equal, lhs, rhs) }
                else { builder.ins().icmp(IntCC::Equal, lhs, rhs) }
            }
            BinOp::Ne => {
                if is_float { builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs) }
                else { builder.ins().icmp(IntCC::NotEqual, lhs, rhs) }
            }
            BinOp::Lt => {
                if is_float { builder.ins().fcmp(FloatCC::LessThan, lhs, rhs) }
                else { builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs) }
            }
            BinOp::Le => {
                if is_float { builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs) }
                else { builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs) }
            }
            BinOp::Gt => {
                if is_float { builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs) }
                else { builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs) }
            }
            BinOp::Ge => {
                if is_float { builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs) }
                else { builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs) }
            }
            BinOp::And => builder.ins().band(lhs, rhs),
            BinOp::Or => builder.ins().bor(lhs, rhs),
            BinOp::BitAnd => builder.ins().band(lhs, rhs),
            BinOp::BitOr => builder.ins().bor(lhs, rhs),
            BinOp::BitXor => builder.ins().bxor(lhs, rhs),
            BinOp::Shl => builder.ins().ishl(lhs, rhs),
            BinOp::Shr => builder.ins().sshr(lhs, rhs),
        };
        Ok(val)
    }

    /// Translate a unary operation
    fn translate_unop(
        &self,
        builder: &mut FunctionBuilder,
        op: UnOp,
        operand: ClifValue,
    ) -> Result<ClifValue, CodegenError> {
        let op_ty = builder.func.dfg.value_type(operand);
        let is_float = op_ty == types::F64;

        let val = match op {
            UnOp::Neg => {
                if is_float { builder.ins().fneg(operand) }
                else { builder.ins().ineg(operand) }
            }
            UnOp::Not => {
                if is_float {
                    // !x for float: true if x == 0.0 or NaN
                    let zero = builder.ins().f64const(0.0);
                    builder.ins().fcmp(FloatCC::UnorderedOrEqual, operand, zero)
                } else {
                    let zero = builder.ins().iconst(types::I8, 0);
                    builder.ins().icmp(IntCC::Equal, operand, zero)
                }
            }
            UnOp::BitNot => builder.ins().bnot(operand),
        };
        Ok(val)
    }

    /// Coerce arguments to match a function's expected signature.
    /// Handles I8→I64 extension (bool→int), etc.
    fn coerce_call_args(
        &self,
        builder: &mut FunctionBuilder,
        func_ref: codegen::ir::FuncRef,
        arg_vals: Vec<ClifValue>,
    ) -> Vec<ClifValue> {
        let sig = builder.func.dfg.ext_funcs[func_ref].signature;
        let expected_params: Vec<_> = builder.func.dfg.signatures[sig]
            .params
            .iter()
            .map(|p| p.value_type)
            .collect();

        arg_vals
            .into_iter()
            .enumerate()
            .map(|(i, val)| {
                if i < expected_params.len() {
                    let expected_ty = expected_params[i];
                    let actual_ty = builder.func.dfg.value_type(val);
                    if actual_ty != expected_ty && actual_ty.is_int() && expected_ty.is_int() {
                        if actual_ty.bits() < expected_ty.bits() {
                            // Zero-extend smaller int to larger
                            builder.ins().uextend(expected_ty, val)
                        } else {
                            // Truncate larger int to smaller
                            builder.ins().ireduce(expected_ty, val)
                        }
                    } else {
                        val
                    }
                } else {
                    val
                }
            })
            .collect()
    }

    /// Call a cranelift function with argument coercion
    fn call_with_coercion(
        &self,
        builder: &mut FunctionBuilder,
        func_ref: codegen::ir::FuncRef,
        arg_vals: Vec<ClifValue>,
    ) -> Result<Option<ClifValue>, CodegenError> {
        let coerced = self.coerce_call_args(builder, func_ref, arg_vals);
        let call = builder.ins().call(func_ref, &coerced);
        let results = builder.inst_results(call);
        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results[0]))
        }
    }

    /// Translate a function call
    fn translate_call(
        &mut self,
        builder: &mut FunctionBuilder,
        func: &IrValue,
        args: &[IrValue],
    ) -> Result<Option<ClifValue>, CodegenError> {
        // Translate arguments
        let arg_vals: Vec<ClifValue> = args
            .iter()
            .map(|arg| self.translate_value(builder, arg))
            .collect::<Result<_, _>>()?;

        match func {
            IrValue::Const(Constant::Str(name)) => {
                // 1. Try user-defined functions in the IR module
                if let Some(ir_func) = self.ir_module.find_function(name) {
                    if let Some(&clif_func_id) = self.func_id_map.get(&ir_func.id) {
                        let func_ref =
                            self.module
                                .declare_func_in_func(clif_func_id, builder.func);
                        return self.call_with_coercion(builder, func_ref, arg_vals);
                    }
                }

                // 2. Try runtime functions
                if let Some(clif_func_id) = self.runtime_funcs.get_by_name(name) {
                    let func_ref =
                        self.module
                            .declare_func_in_func(clif_func_id, builder.func);
                    return self.call_with_coercion(builder, func_ref, arg_vals);
                }

                // 3. Try extern functions declared in the IR module
                for ext in &self.ir_module.extern_functions {
                    if ext.name == *name {
                        let mut sig = self.module.make_signature();
                        for param_ty in &ext.params {
                            let cl_type = self.ir_type_to_cranelift(param_ty)?;
                            sig.params.push(AbiParam::new(cl_type));
                        }
                        if ext.return_type != IrType::Void {
                            let cl_type = self.ir_type_to_cranelift(&ext.return_type)?;
                            sig.returns.push(AbiParam::new(cl_type));
                        }
                        let clif_func_id = self
                            .module
                            .declare_function(name, Linkage::Import, &sig)
                            .map_err(|e| {
                                CodegenError::new(format!(
                                    "Failed to declare extern function {}: {}",
                                    name, e
                                ))
                            })?;
                        let func_ref =
                            self.module
                                .declare_func_in_func(clif_func_id, builder.func);
                        return self.call_with_coercion(builder, func_ref, arg_vals);
                    }
                }

                Err(CodegenError::new(format!(
                    "Function '{}' not found in module, runtime, or externs",
                    name
                )))
            }
            _ => Err(CodegenError::new(
                "Function pointers not yet implemented".to_string(),
            )),
        }
    }

    /// Store a value to a place (handle projections)
    fn store_to_place(
        &mut self,
        builder: &mut FunctionBuilder,
        place: &Place,
        value: ClifValue,
    ) -> Result<(), CodegenError> {
        // Handle simple case: no projections
        if place.projections.is_empty() {
            match &place.base {
                IrValue::Local(local_id) => {
                    // Store to local
                    if self.ir_func.params.iter().any(|(id, _)| id == local_id) {
                        // Can't store to parameter directly - this shouldn't happen
                        return Err(CodegenError::new(format!(
                            "Cannot store to parameter {:?}",
                            local_id
                        )));
                    }

                    let addr = self
                        .value_map
                        .get(&ValueKey::Local(*local_id))
                        .copied()
                        .ok_or_else(|| {
                            CodegenError::new(format!("Local {:?} not found", local_id))
                        })?;

                    builder.ins().store(MemFlags::new(), value, addr, 0);
                    Ok(())
                }
                IrValue::Temp(temp_id) => {
                    // Store to temp
                    self.value_map.insert(ValueKey::Temp(*temp_id), value);
                    Ok(())
                }
                _ => Err(CodegenError::new(
                    "Cannot store to constant".to_string(),
                )),
            }
        } else {
            // Handle projections (field access, indexing, deref)
            let base_ptr = self.compute_place_address(builder, place)?;
            builder.ins().store(MemFlags::new(), value, base_ptr, 0);
            Ok(())
        }
    }

    /// Compute the address of a place (with projections)
    fn compute_place_address(
        &mut self,
        builder: &mut FunctionBuilder,
        place: &Place,
    ) -> Result<ClifValue, CodegenError> {
        let mut ptr = self.translate_value(builder, &place.base)?;
        let base_ir_type = self.infer_value_ir_type(&place.base);

        for projection in &place.projections {
            match projection {
                Projection::Deref => {
                    // Load the pointer value from memory (dereference)
                    let mem_flags = MemFlags::new();
                    let loaded = builder.ins().load(self.pointer_type, mem_flags, ptr, 0);
                    ptr = loaded;
                }
                Projection::Field(index) => {
                    // Compute actual field offset using struct definition if available
                    let offset = match &base_ir_type {
                        Some(IrType::Struct(struct_id)) => {
                            self.compute_struct_field_offset(*struct_id, *index)
                        }
                        _ => (*index * 8) as i64, // fallback for unknown struct types
                    };
                    let offset_val = builder.ins().iconst(types::I64, offset);
                    ptr = builder.ins().iadd(ptr, offset_val);
                }
                Projection::Index(index) => {
                    // Compute array element address using actual element size
                    let index_val = self.translate_value(builder, index)?;
                    let elem_size = match &base_ir_type {
                        Some(IrType::Array(elem_ty)) => elem_ty.size_bytes() as i64,
                        _ => 8, // fallback
                    };
                    let elem_size_val = builder.ins().iconst(types::I64, elem_size);
                    let offset = builder.ins().imul(index_val, elem_size_val);
                    ptr = builder.ins().iadd(ptr, offset);
                }
            }
        }

        Ok(ptr)
    }

    /// Attempt to infer the IR type of a value
    fn infer_value_ir_type(&self, value: &IrValue) -> Option<IrType> {
        match value {
            IrValue::Local(local_id) => {
                self.ir_func.locals.iter()
                    .find(|(id, _)| id == local_id)
                    .map(|(_, ty)| ty.clone())
            }
            IrValue::Temp(temp_id) => {
                self.ir_func.temps.iter()
                    .find(|(id, _)| id == temp_id)
                    .map(|(_, ty)| ty.clone())
            }
            _ => None,
        }
    }

    /// Compute actual byte offset for a struct field by summing preceding field sizes
    fn compute_struct_field_offset(&self, struct_id: StructId, field_index: usize) -> i64 {
        if let Some(struct_def) = self.ir_module.struct_def(struct_id) {
            let mut offset = 0i64;
            for i in 0..field_index {
                if let Some(ty) = struct_def.field_type(i) {
                    offset += ty.size_bytes() as i64;
                }
            }
            offset
        } else {
            (field_index * 8) as i64 // fallback
        }
    }

    /// Infer the type of a place
    fn infer_place_type(&self, place: &Place) -> Result<IrType, CodegenError> {
        match &place.base {
            zaco_ir::Value::Local(local_id) => {
                let ty = self
                    .ir_func
                    .locals
                    .iter()
                    .find(|(id, _)| id == local_id)
                    .map(|(_, ty)| ty.clone())
                    .ok_or_else(|| {
                        CodegenError::new(format!("Local {:?} type not found", local_id))
                    })?;
                // TODO: Apply projections to refine type
                Ok(ty)
            }
            IrValue::Temp(temp_id) => {
                let ty = self
                    .ir_func
                    .temps
                    .iter()
                    .find(|(id, _)| id == temp_id)
                    .map(|(_, ty)| ty.clone())
                    .ok_or_else(|| {
                        CodegenError::new(format!("Temp {:?} type not found", temp_id))
                    })?;
                Ok(ty)
            }
            _ => Ok(IrType::Ptr), // Default to pointer for constants
        }
    }

    /// Get a default/zero value for a type
    #[allow(dead_code)]
    fn default_value_for_type(
        &self,
        builder: &mut FunctionBuilder,
        ty: &IrType,
    ) -> Result<ClifValue, CodegenError> {
        match ty {
            IrType::I64 => Ok(builder.ins().iconst(types::I64, 0)),
            IrType::F64 => Ok(builder.ins().f64const(0.0)),
            IrType::Bool => Ok(builder.ins().iconst(types::I8, 0)),
            IrType::Ptr | IrType::Str | IrType::Array(_) | IrType::Struct(_) | IrType::FuncPtr(_) | IrType::Promise(_) => {
                Ok(builder.ins().iconst(self.pointer_type, 0))
            }
            IrType::Void => Err(CodegenError::new("Cannot create default value for Void")),
        }
    }
}
