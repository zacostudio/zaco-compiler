//! AST → IR Lowering
//!
//! Translates a Zaco AST `Program` into an `IrModule` suitable for codegen.

use std::collections::{HashMap, HashSet};
use std::fmt;

use zaco_ast::*;

use crate::{
    BinOp, BlockId, Constant, FuncId, IrFunction, IrModule, IrStruct, IrType, Instruction, LocalId,
    Place, RValue, StructId, TempId, Terminator, UnOp, Value,
};

/// Errors produced during lowering.
#[derive(Debug, Clone)]
pub struct LowerError {
    pub message: String,
    pub span: Span,
}

impl LowerError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lower error: {}", self.message)
    }
}

impl std::error::Error for LowerError {}

/// Variable info tracked during lowering.
#[derive(Debug, Clone)]
struct VarInfo {
    local_id: LocalId,
    ir_type: IrType,
    /// If true, local_id holds a box pointer; access via zaco_box_get/set.
    is_boxed: bool,
}

/// Class metadata tracked during lowering.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ClassInfo {
    struct_id: StructId,
    /// Field names and types in order (matching IrStruct field order)
    /// For inherited classes, parent fields come first.
    fields: Vec<(String, IrType)>,
    /// Method names (without class prefix)
    methods: Vec<String>,
    /// Parent class name (if extends) — reserved for runtime class metadata
    parent: Option<String>,
    /// Number of fields inherited from parent — reserved for runtime type queries
    parent_field_count: usize,
    /// Getter property names (without class prefix)
    getters: Vec<String>,
    /// Setter property names (without class prefix)
    setters: Vec<String>,
    /// Static method names (without class prefix)
    static_methods: Vec<String>,
    /// Static property names and types
    static_properties: Vec<(String, IrType)>,
}

/// Closure binding info tracked during lowering.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ClosureInfo {
    /// Generated function name
    func_name: String,
    /// Names of captured variables (in order) — reserved for debugging/reflection
    captured_vars: Vec<String>,
    /// Environment struct ID — reserved for env lifetime tracking
    env_struct_id: Option<StructId>,
    /// Local ID holding the environment pointer (in the calling scope)
    env_local: Option<LocalId>,
}

/// Scope for tracking variable bindings.
struct Scope {
    vars: HashMap<String, VarInfo>,
}

impl Scope {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

/// Main lowering context.
pub struct Lowerer {
    module: IrModule,
    errors: Vec<LowerError>,
    next_func_id: usize,
    /// Scope stack (innermost last)
    scopes: Vec<Scope>,
    /// Maps imported names to their source module
    /// e.g., "readFileSync" → "fs", "join" → "path"
    imported_bindings: HashMap<String, String>,
    /// Loop context stack: (header_block, exit_block) for continue targets
    loop_stack: Vec<(BlockId, BlockId)>,
    /// Break target stack: exit blocks for loops and switch statements
    break_stack: Vec<BlockId>,
    /// Set of already-declared extern functions (O(1) lookup)
    extern_set: HashSet<String>,
    /// Class metadata: class_name → ClassInfo
    class_info: HashMap<String, ClassInfo>,
    /// Next struct ID counter
    next_struct_id: usize,
    /// Current `this` variable info (set when lowering class methods/constructors)
    this_var: Option<VarInfo>,
    /// Current class name (set when lowering class methods/constructors)
    current_class: Option<String>,
    /// Closure bindings: variable_name → ClosureInfo
    closure_bindings: HashMap<String, ClosureInfo>,
    /// Next closure ID counter
    next_closure_id: usize,
    /// Parent class name for the current constructor (for super() resolution)
    current_class_parent: Option<String>,
    /// Current function being lowered (name, return_type) for recursive call detection
    current_function: Option<(String, IrType)>,
    /// Whether the user program defines a function named "main"
    has_user_main: bool,
    /// Optional module name for non-entry modules.
    /// When set, the top-level wrapper is named `__module_init_<name>` instead of "main".
    module_name: Option<String>,
    /// Source file path for __dirname/__filename resolution.
    file_path: Option<String>,
}

/// Context for lowering a single function body.
struct FuncCtx<'a> {
    func: &'a mut IrFunction,
    current_block: BlockId,
}

impl<'a> FuncCtx<'a> {
    fn emit(&mut self, instr: Instruction) {
        self.func.block_mut(self.current_block).push_instruction(instr);
    }

    fn set_terminator(&mut self, term: Terminator) {
        self.func.block_mut(self.current_block).set_terminator(term);
    }

    fn new_block(&mut self) -> BlockId {
        self.func.new_block()
    }

    fn switch_to(&mut self, block: BlockId) {
        self.current_block = block;
    }

    fn add_local(&mut self, ty: IrType) -> LocalId {
        self.func.add_local(ty)
    }

    fn add_temp(&mut self, ty: IrType) -> TempId {
        self.func.add_temp(ty)
    }
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            module: IrModule::new(),
            errors: Vec::new(),
            next_func_id: 0,
            scopes: Vec::new(),
            imported_bindings: HashMap::new(),
            loop_stack: Vec::new(),
            break_stack: Vec::new(),
            extern_set: HashSet::new(),
            class_info: HashMap::new(),
            next_struct_id: 0,
            this_var: None,
            current_class: None,
            closure_bindings: HashMap::new(),
            next_closure_id: 0,
            current_class_parent: None,
            current_function: None,
            has_user_main: false,
            module_name: None,
            file_path: None,
        }
    }

    /// Set the module name for non-entry modules.
    /// The top-level wrapper will be named `__module_init_<name>` instead of "main".
    pub fn with_module_name(mut self, name: String) -> Self {
        self.module_name = Some(name);
        self
    }

    /// Set the source file path for __dirname/__filename resolution.
    pub fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Set the starting FuncId offset so that IDs don't collide across modules.
    pub fn with_func_id_offset(mut self, offset: usize) -> Self {
        self.next_func_id = offset;
        self
    }

    /// Set the starting StructId offset so that IDs don't collide across modules.
    pub fn with_struct_id_offset(mut self, offset: usize) -> Self {
        self.next_struct_id = offset;
        self
    }

    fn alloc_func_id(&mut self) -> FuncId {
        let id = FuncId(self.next_func_id);
        self.next_func_id += 1;
        id
    }

    fn alloc_struct_id(&mut self) -> StructId {
        let id = StructId(self.next_struct_id);
        self.next_struct_id += 1;
        id
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_var(&mut self, name: &str, info: VarInfo) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name.to_string(), info);
        }
    }

    fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.vars.get(name) {
                return Some(info);
            }
        }
        None
    }

    /// Ensure an extern function is declared in the module.
    fn ensure_extern(&mut self, name: &str, params: Vec<IrType>, ret: IrType) {
        if self.extern_set.insert(name.to_string()) {
            self.module.add_extern_function(name.to_string(), params, ret);
        }
    }

    /// Lower an entire program into an IR module.
    pub fn lower_program(mut self, program: &Program) -> Result<IrModule, Vec<LowerError>> {
        // Detect if user defines a function named "main" — if so, we'll rename it
        // to avoid conflicting with the compiler-generated entry point wrapper.
        for item in &program.items {
            if let ModuleItem::Decl(decl_node) = &item.value {
                if let Decl::Function(func_decl) = &decl_node.value {
                    if func_decl.name.value.name == "main" {
                        self.has_user_main = true;
                        break;
                    }
                }
            }
        }

        // Determine wrapper function name and return type based on module context.
        // Entry module gets "main" (returns I64 exit code).
        // Non-entry modules get "__module_init_<name>" (returns void).
        let is_entry = self.module_name.is_none();
        let (wrapper_name, wrapper_ret) = if let Some(ref mod_name) = self.module_name {
            (format!("__module_init_{}", mod_name), IrType::Void)
        } else {
            ("main".to_string(), IrType::I64)
        };

        let wrapper_id = self.alloc_func_id();
        let mut wrapper_func = IrFunction::new(wrapper_id, wrapper_name, vec![], wrapper_ret);
        wrapper_func.is_public = true;

        let entry = wrapper_func.new_block();
        wrapper_func.entry_block = entry;

        let mut ctx = FuncCtx {
            func: &mut wrapper_func,
            current_block: entry,
        };

        self.push_scope();

        // Lower each top-level item
        for item in &program.items {
            self.lower_module_item(&mut ctx, &item.value);
        }

        self.pop_scope();

        if is_entry {
            // Return 0 from main
            let zero_temp = ctx.add_temp(IrType::I64);
            ctx.emit(Instruction::Assign {
                dest: Place::from_temp(zero_temp),
                value: RValue::Use(Value::Const(Constant::I64(0))),
            });
            ctx.set_terminator(Terminator::Return(Some(Value::Temp(zero_temp))));
        } else {
            // Non-entry modules just return void
            ctx.set_terminator(Terminator::Return(None));
        }

        self.module.add_function(wrapper_func);

        // Record how many IDs were allocated so the driver can compute offsets
        // for subsequent modules during multi-module compilation.
        self.module.next_func_id = self.next_func_id;
        self.module.next_struct_id = self.next_struct_id;

        if self.errors.is_empty() {
            Ok(self.module)
        } else {
            Err(self.errors)
        }
    }

    fn lower_module_item(&mut self, ctx: &mut FuncCtx, item: &ModuleItem) {
        match item {
            ModuleItem::Stmt(stmt_node) => {
                self.lower_stmt(ctx, &stmt_node.value, &stmt_node.span);
            }
            ModuleItem::Decl(decl_node) => {
                self.lower_decl(ctx, &decl_node.value, &decl_node.span);
            }
            ModuleItem::Import(import_decl) => {
                self.lower_import(import_decl);
            }
            ModuleItem::Export(export_decl) => {
                self.lower_export(ctx, export_decl);
            }
        }
    }

    fn lower_import(&mut self, import_decl: &ImportDecl) {
        let source = &import_decl.source;
        for spec in &import_decl.specifiers {
            match spec {
                ImportSpecifier::Named { imported, local, .. } => {
                    let local_name = local.as_ref().unwrap_or(imported).value.name.clone();
                    self.imported_bindings.insert(local_name, source.clone());
                }
                ImportSpecifier::Default(ident) => {
                    self.imported_bindings.insert(ident.value.name.clone(), source.clone());
                }
                ImportSpecifier::Namespace(ident) => {
                    self.imported_bindings.insert(ident.value.name.clone(), source.clone());
                }
            }
        }
    }

    fn lower_export(&mut self, ctx: &mut FuncCtx, export_decl: &ExportDecl) {
        match export_decl {
            ExportDecl::Decl(decl) => {
                // Lower the declaration normally
                self.lower_decl(ctx, &decl.value, &decl.span);
                // Mark the last added function as public
                if let Decl::Function(func_decl) = &decl.value {
                    let name = &func_decl.name.value.name;
                    if let Some(func) = self.module.functions.iter_mut().rev().find(|f| f.name == *name) {
                        func.is_public = true;
                    }
                }
            }
            ExportDecl::Default(expr) => {
                // Lower as expression
                let _ = self.lower_expr(ctx, &expr.value, &expr.span);
            }
            ExportDecl::DefaultDecl(decl) => {
                self.lower_decl(ctx, &decl.value, &decl.span);
                // Mark as public
                if let Decl::Function(func_decl) = &decl.value {
                    let name = &func_decl.name.value.name;
                    if let Some(func) = self.module.functions.iter_mut().rev().find(|f| f.name == *name) {
                        func.is_public = true;
                    }
                }
            }
            _ => {
                // Named exports, re-exports — handle later
            }
        }
    }

    fn lower_decl(&mut self, ctx: &mut FuncCtx, decl: &Decl, span: &Span) {
        match decl {
            Decl::Var(var_decl) => {
                self.lower_var_decl(ctx, var_decl, span);
            }
            Decl::Function(func_decl) => {
                self.lower_function_decl(ctx, func_decl, span);
            }
            Decl::Class(class_decl) => {
                self.lower_class_decl(ctx, class_decl, span);
            }
            Decl::Interface(_)
            | Decl::TypeAlias(_)
            | Decl::Enum(_)
            | Decl::Module(_) => {
                // Type-level declarations — skip for codegen
            }
        }
    }

    fn lower_stmt(&mut self, ctx: &mut FuncCtx, stmt: &Stmt, span: &Span) {
        match stmt {
            Stmt::Expr(expr_node) => {
                // Lower expression for side effects
                let _ = self.lower_expr(ctx, &expr_node.value, &expr_node.span);
            }
            Stmt::VarDecl(var_decl) => {
                self.lower_var_decl(ctx, var_decl, span);
            }
            Stmt::Return(opt_expr) => {
                if let Some(expr_node) = opt_expr {
                    if let Some(val) = self.lower_expr(ctx, &expr_node.value, &expr_node.span) {
                        ctx.set_terminator(Terminator::Return(Some(val)));
                    }
                } else {
                    ctx.set_terminator(Terminator::Return(None));
                }
            }
            Stmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                self.lower_if(ctx, condition, then_stmt, else_stmt.as_deref(), span);
            }
            Stmt::While { condition, body } => {
                self.lower_while(ctx, condition, body, span);
            }
            Stmt::For {
                init,
                condition,
                update,
                body,
            } => {
                self.lower_for(ctx, init.as_ref(), condition.as_ref(), update.as_ref(), body, span);
            }
            Stmt::Block(block) => {
                self.push_scope();
                for s in &block.stmts {
                    self.lower_stmt(ctx, &s.value, &s.span);
                }
                self.pop_scope();
            }
            Stmt::Break(_) => {
                if let Some(&exit_block) = self.break_stack.last() {
                    ctx.set_terminator(Terminator::Jump(exit_block));
                    // Create unreachable block for any code after break
                    let dead_block = ctx.new_block();
                    ctx.switch_to(dead_block);
                }
            }
            Stmt::Continue(_) => {
                if let Some(&(header_block, _)) = self.loop_stack.last() {
                    ctx.set_terminator(Terminator::Jump(header_block));
                    // Create unreachable block for any code after continue
                    let dead_block = ctx.new_block();
                    ctx.switch_to(dead_block);
                }
            }
            Stmt::Throw(expr_node) => {
                self.lower_throw(ctx, expr_node, span);
            }
            Stmt::Try {
                block,
                catch,
                finally,
            } => {
                self.lower_try(ctx, block, catch.as_ref(), finally.as_ref(), span);
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                self.lower_switch(ctx, discriminant, cases, span);
            }
            Stmt::ForIn { left, right, body } => {
                self.lower_for_in(ctx, left, right, body, span);
            }
            Stmt::ForOf {
                left,
                right,
                body,
                ..
            } => {
                self.lower_for_of(ctx, left, right, body, span);
            }
            Stmt::Empty | Stmt::Debugger => {}
            _ => {
                // Other statements not yet implemented
            }
        }
    }

    fn lower_var_decl(&mut self, ctx: &mut FuncCtx, var_decl: &VarDecl, _span: &Span) {
        for declarator in &var_decl.declarations {
            match &declarator.pattern.value {
                Pattern::Ident { name, .. } => {
                    let name = name.value.name.clone();
                    let ir_type = if let Some(ref init) = declarator.init {
                        self.infer_expr_type(&init.value)
                    } else {
                        IrType::F64
                    };
                    let local_id = ctx.add_local(ir_type.clone());
                    self.define_var(&name, VarInfo { local_id, ir_type, is_boxed: false });
                    if let Some(ref init) = declarator.init {
                        if let Some(val) = self.lower_expr(ctx, &init.value, &init.span) {
                            if let Value::Const(Constant::Str(ref func_name)) = val {
                                if let Some(closure_info) = self.closure_bindings.get(func_name).cloned() {
                                    self.closure_bindings.insert(name.clone(), closure_info);
                                }
                            }
                            ctx.emit(Instruction::Assign {
                                dest: Place::from_local(local_id),
                                value: RValue::Use(val),
                            });
                        }
                    }
                }
                Pattern::Object { properties, .. } => {
                    let init_val = declarator.init.as_ref().and_then(|init| {
                        self.lower_expr(ctx, &init.value, &init.span)
                    });
                    let obj_val = match init_val {
                        Some(v) => v,
                        None => continue,
                    };
                    let obj_local = ctx.add_local(IrType::Ptr);
                    ctx.emit(Instruction::Assign {
                        dest: Place::from_local(obj_local),
                        value: RValue::Use(obj_val),
                    });
                    for prop in properties {
                        let key_str = match &prop.key {
                            PropertyName::Ident(ident) => ident.value.name.clone(),
                            PropertyName::String(s) => s.clone(),
                            PropertyName::Number(n) => format!("{}", n),
                            PropertyName::Computed(_) => continue,
                        };
                        let var_name = match &prop.value.value {
                            Pattern::Ident { name, .. } => name.value.name.clone(),
                            _ => continue,
                        };
                        let ir_type = IrType::F64;
                        self.ensure_extern("zaco_object_get_f64", vec![IrType::Ptr, IrType::Ptr], ir_type.clone());
                        self.module.intern_string(key_str.clone());
                        let key_val = Value::Const(Constant::Str(key_str));
                        let result_temp = ctx.add_temp(ir_type.clone());
                        ctx.emit(Instruction::Call {
                            dest: Some(Place::from_temp(result_temp)),
                            func: Value::Const(Constant::Str("zaco_object_get_f64".to_string())),
                            args: vec![Value::Local(obj_local), key_val],
                        });
                        let local_id = ctx.add_local(ir_type.clone());
                        self.define_var(&var_name, VarInfo { local_id, ir_type, is_boxed: false });
                        ctx.emit(Instruction::Assign {
                            dest: Place::from_local(local_id),
                            value: RValue::Use(Value::Temp(result_temp)),
                        });
                    }
                }
                Pattern::Array { elements, .. } => {
                    let init_val = declarator.init.as_ref().and_then(|init| {
                        self.lower_expr(ctx, &init.value, &init.span)
                    });
                    let arr_val = match init_val {
                        Some(v) => v,
                        None => continue,
                    };
                    let arr_local = ctx.add_local(IrType::Ptr);
                    ctx.emit(Instruction::Assign {
                        dest: Place::from_local(arr_local),
                        value: RValue::Use(arr_val),
                    });
                    self.ensure_extern("zaco_array_get_f64", vec![IrType::Ptr, IrType::I64], IrType::F64);
                    for (i, elem) in elements.iter().enumerate() {
                        let pat = match elem {
                            Some(pat_node) => pat_node,
                            None => continue,
                        };
                        let var_name = match &pat.value {
                            Pattern::Ident { name, .. } => name.value.name.clone(),
                            _ => continue,
                        };
                        let ir_type = IrType::F64;
                        let idx_val = Value::Const(Constant::I64(i as i64));
                        let result_temp = ctx.add_temp(ir_type.clone());
                        ctx.emit(Instruction::Call {
                            dest: Some(Place::from_temp(result_temp)),
                            func: Value::Const(Constant::Str("zaco_array_get_f64".to_string())),
                            args: vec![Value::Local(arr_local), idx_val],
                        });
                        let local_id = ctx.add_local(ir_type.clone());
                        self.define_var(&var_name, VarInfo { local_id, ir_type, is_boxed: false });
                        ctx.emit(Instruction::Assign {
                            dest: Place::from_local(local_id),
                            value: RValue::Use(Value::Temp(result_temp)),
                        });
                    }
                }
                _ => continue,
            }
        }
    }

    /// Lower an expression, returning the IR value it produces.
    fn lower_expr(&mut self, ctx: &mut FuncCtx, expr: &Expr, span: &Span) -> Option<Value> {
        match expr {
            Expr::Literal(lit) => self.lower_literal(ctx, lit, span),

            Expr::Ident(ident) => {
                // Handle __dirname and __filename globals
                match ident.name.as_str() {
                    "__dirname" => {
                        let dir = self.file_path.as_ref()
                            .and_then(|p| {
                                std::path::Path::new(p).parent()
                                    .map(|d| d.to_string_lossy().into_owned())
                            })
                            .unwrap_or_else(|| ".".to_string());
                        return Some(Value::Const(Constant::Str(dir)));
                    }
                    "__filename" => {
                        let path = self.file_path.clone()
                            .unwrap_or_else(|| "<unknown>".to_string());
                        return Some(Value::Const(Constant::Str(path)));
                    }
                    _ => {}
                }

                if let Some(info) = self.lookup_var(&ident.name).cloned() {
                    if info.is_boxed {
                        // Boxed variable: read through box pointer
                        self.ensure_extern("zaco_box_get", vec![IrType::Ptr], IrType::Ptr);
                        let temp = ctx.add_temp(info.ir_type.clone());
                        ctx.emit(Instruction::Call {
                            dest: Some(Place::from_temp(temp)),
                            func: Value::Const(Constant::Str("zaco_box_get".to_string())),
                            args: vec![Value::Local(info.local_id)],
                        });
                        Some(Value::Temp(temp))
                    } else {
                        Some(Value::Local(info.local_id))
                    }
                } else {
                    // Unknown identifier — might be a global like `console`
                    None
                }
            }

            Expr::Binary { left, op, right } => {
                self.lower_binary(ctx, left, *op, right, span)
            }

            Expr::Unary { op, expr: operand } => {
                self.lower_unary(ctx, *op, operand, span)
            }

            Expr::Assignment {
                target,
                op,
                value,
            } => self.lower_assignment(ctx, target, *op, value, span),

            Expr::Call { callee, args, .. } => self.lower_call(ctx, callee, args, span),

            Expr::Member { object, property, .. } => {
                self.lower_member_expr(ctx, object, property, span)
            }

            Expr::Paren(inner) => self.lower_expr(ctx, &inner.value, &inner.span),

            Expr::Template { parts, exprs } => self.lower_template(ctx, parts, exprs, span),

            Expr::Array(elements) => self.lower_array_literal(ctx, elements, span),

            Expr::Object(props) => self.lower_object_literal(ctx, props, span),

            Expr::Await(expr) => self.lower_await(ctx, expr, span),

            Expr::New { callee, args, .. } => self.lower_new_expr(ctx, callee, args, span),

            Expr::This => self.lower_this_expr(),

            Expr::Arrow { params, body, return_type, .. } => {
                self.lower_arrow_expr(ctx, params, return_type.as_deref(), body, span)
            }

            Expr::Function { name, params, return_type, body, .. } => {
                self.lower_function_expr(ctx, name.as_ref(), params, return_type.as_deref(), body, span)
            }

            Expr::Ternary { condition, then_expr, else_expr } => {
                self.lower_ternary(ctx, condition, then_expr, else_expr, span)
            }

            Expr::TaggedTemplate { tag, parts, exprs } => {
                self.lower_tagged_template(ctx, tag, parts, exprs, span)
            }

            Expr::Yield { argument, delegate } => {
                self.lower_yield_expr(ctx, argument.as_deref(), *delegate, span)
            }

            Expr::OptionalMember { object, property } => {
                self.lower_optional_member(ctx, object, property, span)
            }

            Expr::OptionalCall { callee, args, .. } => {
                self.lower_optional_call(ctx, callee, args, span)
            }

            Expr::OptionalIndex { object, index } => {
                self.lower_optional_index(ctx, object, index, span)
            }

            _ => {
                // Unsupported expression
                None
            }
        }
    }

    fn lower_literal(
        &mut self,
        _ctx: &mut FuncCtx,
        lit: &Literal,
        _span: &Span,
    ) -> Option<Value> {
        match lit {
            Literal::Number(n) => {
                // TypeScript number is always IEEE 754 double (f64)
                Some(Value::Const(Constant::F64(*n)))
            }
            Literal::String(s) => {
                // Intern the string
                self.module.intern_string(s.clone());
                Some(Value::Const(Constant::Str(s.clone())))
            }
            Literal::Boolean(b) => Some(Value::Const(Constant::Bool(*b))),
            Literal::Null => Some(Value::Const(Constant::Null)),
            Literal::Undefined => Some(Value::Const(Constant::Null)),
            Literal::RegExp { .. } => None,
        }
    }

    fn lower_binary(
        &mut self,
        ctx: &mut FuncCtx,
        left: &Node<Expr>,
        op: BinaryOp,
        right: &Node<Expr>,
        _span: &Span,
    ) -> Option<Value> {
        // Handle short-circuit logical operators before evaluating right side
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.lower_short_circuit(ctx, left, op, right);
        }

        // Handle nullish coalescing (??) — short-circuit on null/0
        if matches!(op, BinaryOp::NullishCoalesce) {
            return self.lower_nullish_coalesce(ctx, left, right);
        }

        let lhs = self.lower_expr(ctx, &left.value, &left.span)?;
        let rhs = self.lower_expr(ctx, &right.value, &right.span)?;

        // Check if this is string concatenation
        if matches!(op, BinaryOp::Add) {
            let left_ty = self.infer_expr_type(&left.value);
            let right_ty = self.infer_expr_type(&right.value);
            if left_ty == IrType::Str || right_ty == IrType::Str {
                // Convert non-string operands to strings
                let lhs_str = if left_ty != IrType::Str {
                    self.ensure_extern("zaco_f64_to_str", vec![IrType::F64], IrType::Str);
                    let conv_temp = ctx.add_temp(IrType::Str);
                    ctx.emit(Instruction::Call {
                        dest: Some(Place::from_temp(conv_temp)),
                        func: Value::Const(Constant::Str("zaco_f64_to_str".to_string())),
                        args: vec![lhs],
                    });
                    Value::Temp(conv_temp)
                } else {
                    lhs
                };
                let rhs_str = if right_ty != IrType::Str {
                    self.ensure_extern("zaco_f64_to_str", vec![IrType::F64], IrType::Str);
                    let conv_temp = ctx.add_temp(IrType::Str);
                    ctx.emit(Instruction::Call {
                        dest: Some(Place::from_temp(conv_temp)),
                        func: Value::Const(Constant::Str("zaco_f64_to_str".to_string())),
                        args: vec![rhs],
                    });
                    Value::Temp(conv_temp)
                } else {
                    rhs
                };
                let temp = ctx.add_temp(IrType::Str);
                ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(temp),
                    value: RValue::StrConcat(vec![lhs_str, rhs_str]),
                });
                return Some(Value::Temp(temp));
            }
        }

        // Handle string equality/inequality via runtime call
        if matches!(op, BinaryOp::Eq | BinaryOp::StrictEq | BinaryOp::NotEq | BinaryOp::StrictNotEq) {
            let left_ty = self.infer_expr_type(&left.value);
            let right_ty = self.infer_expr_type(&right.value);
            if left_ty == IrType::Str && right_ty == IrType::Str {
                self.ensure_extern("zaco_str_eq", vec![IrType::Str, IrType::Str], IrType::I64);
                let eq_temp = ctx.add_temp(IrType::I64);
                ctx.emit(Instruction::Call {
                    dest: Some(Place::from_temp(eq_temp)),
                    func: Value::Const(Constant::Str("zaco_str_eq".to_string())),
                    args: vec![lhs, rhs],
                });
                // For NotEq/StrictNotEq: result = (eq == 0), i.e. compare with 0
                if matches!(op, BinaryOp::NotEq | BinaryOp::StrictNotEq) {
                    let result = ctx.add_temp(IrType::Bool);
                    ctx.emit(Instruction::Assign {
                        dest: Place::from_temp(result),
                        value: RValue::BinaryOp {
                            op: BinOp::Eq,
                            left: Value::Temp(eq_temp),
                            right: Value::Const(Constant::I64(0)),
                        },
                    });
                    return Some(Value::Temp(result));
                }
                // For Eq/StrictEq: result = (eq != 0)
                let result = ctx.add_temp(IrType::Bool);
                ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(result),
                    value: RValue::BinaryOp {
                        op: BinOp::Ne,
                        left: Value::Temp(eq_temp),
                        right: Value::Const(Constant::I64(0)),
                    },
                });
                return Some(Value::Temp(result));
            }
        }

        // `in` operator: check if property exists in object
        // Placeholder: both operands evaluated for side effects, returns false
        if matches!(op, BinaryOp::In) {
            self.ensure_extern("zaco_obj_has_prop", vec![IrType::Ptr, IrType::Str], IrType::Bool);
            let temp = ctx.add_temp(IrType::Bool);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(temp)),
                func: Value::Const(Constant::Str("zaco_obj_has_prop".to_string())),
                args: vec![rhs, lhs],
            });
            return Some(Value::Temp(temp));
        }

        // `instanceof` operator: check if value is instance of a class
        // Placeholder: both operands evaluated for side effects, returns false
        if matches!(op, BinaryOp::InstanceOf) {
            self.ensure_extern("zaco_instanceof", vec![IrType::Ptr, IrType::Ptr], IrType::Bool);
            let temp = ctx.add_temp(IrType::Bool);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(temp)),
                func: Value::Const(Constant::Str("zaco_instanceof".to_string())),
                args: vec![lhs, rhs],
            });
            return Some(Value::Temp(temp));
        }

        let ir_op = match op {
            BinaryOp::Add => BinOp::Add,
            BinaryOp::Sub => BinOp::Sub,
            BinaryOp::Mul => BinOp::Mul,
            BinaryOp::Div => BinOp::Div,
            BinaryOp::Mod => BinOp::Mod,
            BinaryOp::Eq | BinaryOp::StrictEq => BinOp::Eq,
            BinaryOp::NotEq | BinaryOp::StrictNotEq => BinOp::Ne,
            BinaryOp::Lt => BinOp::Lt,
            BinaryOp::LtEq => BinOp::Le,
            BinaryOp::Gt => BinOp::Gt,
            BinaryOp::GtEq => BinOp::Ge,
            BinaryOp::BitAnd => BinOp::BitAnd,
            BinaryOp::BitOr => BinOp::BitOr,
            BinaryOp::BitXor => BinOp::BitXor,
            BinaryOp::LeftShift => BinOp::Shl,
            BinaryOp::RightShift => BinOp::Shr,
            _ => return None, // Pow, >>> not yet handled
        };

        let result_type = if matches!(
            ir_op,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        ) {
            IrType::Bool
        } else {
            self.infer_expr_type(&left.value)
        };

        let temp = ctx.add_temp(result_type);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(temp),
            value: RValue::BinaryOp {
                op: ir_op,
                left: lhs,
                right: rhs,
            },
        });
        Some(Value::Temp(temp))
    }

    /// Lower short-circuit logical operators (&&, ||).
    ///
    /// `a && b`: If `a` is falsy, return `a`. Otherwise return `b`.
    /// `a || b`: If `a` is truthy, return `a`. Otherwise return `b`.
    fn lower_short_circuit(
        &mut self,
        ctx: &mut FuncCtx,
        left: &Node<Expr>,
        op: BinaryOp,
        right: &Node<Expr>,
    ) -> Option<Value> {
        let lhs = self.lower_expr(ctx, &left.value, &left.span)?;

        let result_type = self.infer_expr_type(&left.value);

        // Create a local to hold the result
        let result_local = ctx.add_local(result_type);

        // Store left value as initial result
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(result_local),
            value: RValue::Use(lhs.clone()),
        });

        let eval_right_block = ctx.new_block();
        let merge_block = ctx.new_block();

        // Branch based on the operator:
        // &&: if lhs is truthy, evaluate right; otherwise, keep left (skip right)
        // ||: if lhs is truthy, keep left (skip right); otherwise, evaluate right
        match op {
            BinaryOp::And => {
                ctx.set_terminator(Terminator::Branch {
                    cond: lhs,
                    then_block: eval_right_block,
                    else_block: merge_block,
                });
            }
            BinaryOp::Or => {
                ctx.set_terminator(Terminator::Branch {
                    cond: lhs,
                    then_block: merge_block,
                    else_block: eval_right_block,
                });
            }
            _ => unreachable!(),
        }

        // Evaluate right operand in its own block
        ctx.switch_to(eval_right_block);
        if let Some(rhs) = self.lower_expr(ctx, &right.value, &right.span) {
            ctx.emit(Instruction::Assign {
                dest: Place::from_local(result_local),
                value: RValue::Use(rhs),
            });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));

        // Continue in merge block
        ctx.switch_to(merge_block);

        Some(Value::Local(result_local))
    }

    /// Lower nullish coalescing (`??`): `a ?? b`
    /// If `a` is null/0 (for pointer types), use `b`; otherwise use `a`.
    fn lower_nullish_coalesce(
        &mut self,
        ctx: &mut FuncCtx,
        left: &Node<Expr>,
        right: &Node<Expr>,
    ) -> Option<Value> {
        let lhs = self.lower_expr(ctx, &left.value, &left.span)?;

        let result_type = self.infer_expr_type(&left.value);
        let result_local = ctx.add_local(result_type.clone());

        // Store LHS as initial result
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(result_local),
            value: RValue::Use(lhs.clone()),
        });

        let eval_right_block = ctx.new_block();
        let merge_block = ctx.new_block();

        // Null check: compare LHS with null (0 for pointer types)
        let is_null = self.emit_null_check(ctx, lhs, &result_type);

        // If null → evaluate RHS; otherwise → keep LHS (jump to merge)
        ctx.set_terminator(Terminator::Branch {
            cond: is_null,
            then_block: eval_right_block,
            else_block: merge_block,
        });

        // Evaluate RHS and store as result
        ctx.switch_to(eval_right_block);
        if let Some(rhs) = self.lower_expr(ctx, &right.value, &right.span) {
            ctx.emit(Instruction::Assign {
                dest: Place::from_local(result_local),
                value: RValue::Use(rhs),
            });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));

        ctx.switch_to(merge_block);
        Some(Value::Local(result_local))
    }

    /// Emit a null check for a value, returning a boolean Value that is true if the value is null.
    /// For pointer types (Ptr, Str, Struct, Array, FuncPtr, Promise): compare with 0/null.
    /// For other types: compare with 0 (as i64).
    fn emit_null_check(&self, ctx: &mut FuncCtx, val: Value, ty: &IrType) -> Value {
        let null_val = if ty.is_pointer() {
            Value::Const(Constant::Null)
        } else {
            Value::Const(Constant::I64(0))
        };
        let cmp_temp = ctx.add_temp(IrType::Bool);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(cmp_temp),
            value: RValue::BinaryOp {
                op: BinOp::Eq,
                left: val,
                right: null_val,
            },
        });
        Value::Temp(cmp_temp)
    }

    fn lower_unary(
        &mut self,
        ctx: &mut FuncCtx,
        op: UnaryOp,
        operand: &Node<Expr>,
        _span: &Span,
    ) -> Option<Value> {
        let val = self.lower_expr(ctx, &operand.value, &operand.span)?;

        // void: evaluate operand for side effects, return undefined (null)
        if op == UnaryOp::Void {
            return Some(Value::Const(Constant::Null));
        }

        // delete: evaluate operand for side effects, return true
        if op == UnaryOp::Delete {
            return Some(Value::Const(Constant::Bool(true)));
        }

        let ir_op = match op {
            UnaryOp::Minus => UnOp::Neg,
            UnaryOp::Not => UnOp::Not,
            UnaryOp::BitNot => UnOp::BitNot,
            _ => return None, // typeof, ++, -- not yet handled
        };

        let result_type = match ir_op {
            UnOp::Not => IrType::Bool,
            _ => self.infer_expr_type(&operand.value),
        };

        let temp = ctx.add_temp(result_type);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(temp),
            value: RValue::UnaryOp { op: ir_op, operand: val },
        });
        Some(Value::Temp(temp))
    }

    fn lower_assignment(
        &mut self,
        ctx: &mut FuncCtx,
        target: &Node<Expr>,
        op: AssignmentOp,
        value: &Node<Expr>,
        _span: &Span,
    ) -> Option<Value> {
        // Handle nullish assignment (??=) before evaluating RHS
        if op == AssignmentOp::NullishAssign {
            return self.lower_nullish_assign(ctx, target, value);
        }

        let rhs = self.lower_expr(ctx, &value.value, &value.span)?;

        // Handle member assignment: this.field = value or obj.field = value
        if let Expr::Member { object, property, .. } = &target.value {
            return self.lower_member_assignment(ctx, object, property, op, rhs);
        }

        // Get the target local
        let target_name = match &target.value {
            Expr::Ident(ident) => ident.name.clone(),
            _ => return None, // Complex assignment targets not yet supported
        };

        let info = self.lookup_var(&target_name)?.clone();

        let final_val = if op == AssignmentOp::Assign {
            rhs
        } else {
            // Compound assignment: target op= value → target = target op value
            let lhs = if info.is_boxed {
                // Read current value through box pointer
                self.ensure_extern("zaco_box_get", vec![IrType::Ptr], IrType::Ptr);
                let read_temp = ctx.add_temp(info.ir_type.clone());
                ctx.emit(Instruction::Call {
                    dest: Some(Place::from_temp(read_temp)),
                    func: Value::Const(Constant::Str("zaco_box_get".to_string())),
                    args: vec![Value::Local(info.local_id)],
                });
                Value::Temp(read_temp)
            } else {
                Value::Local(info.local_id)
            };
            let ir_op = match op {
                AssignmentOp::AddAssign => BinOp::Add,
                AssignmentOp::SubAssign => BinOp::Sub,
                AssignmentOp::MulAssign => BinOp::Mul,
                AssignmentOp::DivAssign => BinOp::Div,
                AssignmentOp::ModAssign => BinOp::Mod,
                AssignmentOp::BitAndAssign => BinOp::BitAnd,
                AssignmentOp::BitOrAssign => BinOp::BitOr,
                AssignmentOp::BitXorAssign => BinOp::BitXor,
                AssignmentOp::LeftShiftAssign => BinOp::Shl,
                AssignmentOp::RightShiftAssign => BinOp::Shr,
                _ => return None,
            };
            let temp = ctx.add_temp(info.ir_type.clone());
            ctx.emit(Instruction::Assign {
                dest: Place::from_temp(temp),
                value: RValue::BinaryOp {
                    op: ir_op,
                    left: lhs,
                    right: rhs,
                },
            });
            Value::Temp(temp)
        };

        if info.is_boxed {
            // Boxed variable: write through box pointer
            self.ensure_extern("zaco_box_set", vec![IrType::Ptr, IrType::Ptr], IrType::Void);
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str("zaco_box_set".to_string())),
                args: vec![Value::Local(info.local_id), final_val.clone()],
            });
        } else {
            ctx.emit(Instruction::Assign {
                dest: Place::from_local(info.local_id),
                value: RValue::Use(final_val.clone()),
            });
        }

        Some(final_val)
    }

    /// Lower nullish assignment (`??=`): `a ??= b`
    fn lower_nullish_assign(&mut self, ctx: &mut FuncCtx, target: &Node<Expr>, value: &Node<Expr>) -> Option<Value> {
        let target_name = match &target.value { Expr::Ident(ident) => ident.name.clone(), _ => return None };
        let info = self.lookup_var(&target_name)?.clone();
        let current_val = Value::Local(info.local_id);
        let val_type = info.ir_type.clone();
        let assign_block = ctx.new_block();
        let merge_block = ctx.new_block();
        let is_null = self.emit_null_check(ctx, current_val, &val_type);
        ctx.set_terminator(Terminator::Branch { cond: is_null, then_block: assign_block, else_block: merge_block });
        ctx.switch_to(assign_block);
        if let Some(rhs) = self.lower_expr(ctx, &value.value, &value.span) {
            ctx.emit(Instruction::Assign { dest: Place::from_local(info.local_id), value: RValue::Use(rhs) });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));
        ctx.switch_to(merge_block);
        Some(Value::Local(info.local_id))
    }

    /// Lower optional member access (`obj?.prop`).
    fn lower_optional_member(&mut self, ctx: &mut FuncCtx, object: &Node<Expr>, property: &Node<Ident>, span: &Span) -> Option<Value> {
        let base = self.lower_expr(ctx, &object.value, &object.span)?;
        let base_type = self.infer_expr_type(&object.value);
        let result_type = self.infer_expr_type(&Expr::Member { object: Box::new(object.clone()), property: property.clone(), computed: false });
        let result_local = ctx.add_local(result_type.clone());
        let null_val = if result_type.is_pointer() { Value::Const(Constant::Null) } else { Value::Const(Constant::I64(0)) };
        ctx.emit(Instruction::Assign { dest: Place::from_local(result_local), value: RValue::Use(null_val) });
        let then_block = ctx.new_block();
        let merge_block = ctx.new_block();
        let is_null = self.emit_null_check(ctx, base, &base_type);
        ctx.set_terminator(Terminator::Branch { cond: is_null, then_block: merge_block, else_block: then_block });
        ctx.switch_to(then_block);
        if let Some(member_val) = self.lower_member_expr(ctx, object, property, span) {
            ctx.emit(Instruction::Assign { dest: Place::from_local(result_local), value: RValue::Use(member_val) });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));
        ctx.switch_to(merge_block);
        Some(Value::Local(result_local))
    }

    /// Lower optional call (`obj?.(args)`).
    fn lower_optional_call(&mut self, ctx: &mut FuncCtx, callee: &Node<Expr>, args: &[Node<Expr>], span: &Span) -> Option<Value> {
        let base = self.lower_expr(ctx, &callee.value, &callee.span)?;
        let base_type = self.infer_expr_type(&callee.value);
        let result_type = self.infer_expr_type(&Expr::Call { callee: Box::new(callee.clone()), type_args: None, args: args.to_vec() });
        let result_local = ctx.add_local(result_type.clone());
        let null_val = if result_type.is_pointer() { Value::Const(Constant::Null) } else { Value::Const(Constant::I64(0)) };
        ctx.emit(Instruction::Assign { dest: Place::from_local(result_local), value: RValue::Use(null_val) });
        let then_block = ctx.new_block();
        let merge_block = ctx.new_block();
        let is_null = self.emit_null_check(ctx, base, &base_type);
        ctx.set_terminator(Terminator::Branch { cond: is_null, then_block: merge_block, else_block: then_block });
        ctx.switch_to(then_block);
        if let Some(call_val) = self.lower_call(ctx, callee, args, span) {
            ctx.emit(Instruction::Assign { dest: Place::from_local(result_local), value: RValue::Use(call_val) });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));
        ctx.switch_to(merge_block);
        Some(Value::Local(result_local))
    }

    /// Lower optional index access (`obj?.[index]`).
    fn lower_optional_index(&mut self, ctx: &mut FuncCtx, object: &Node<Expr>, index: &Node<Expr>, _span: &Span) -> Option<Value> {
        let base = self.lower_expr(ctx, &object.value, &object.span)?;
        let base_type = self.infer_expr_type(&object.value);
        let result_local = ctx.add_local(IrType::F64);
        ctx.emit(Instruction::Assign { dest: Place::from_local(result_local), value: RValue::Use(Value::Const(Constant::I64(0))) });
        let then_block = ctx.new_block();
        let merge_block = ctx.new_block();
        let is_null = self.emit_null_check(ctx, base.clone(), &base_type);
        ctx.set_terminator(Terminator::Branch { cond: is_null, then_block: merge_block, else_block: then_block });
        ctx.switch_to(then_block);
        if let Some(idx_val) = self.lower_expr(ctx, &index.value, &index.span) {
            self.ensure_extern("zaco_array_get", vec![IrType::Ptr, IrType::I64], IrType::F64);
            let elem_temp = ctx.add_temp(IrType::F64);
            ctx.emit(Instruction::Call { dest: Some(Place::from_temp(elem_temp)), func: Value::Const(Constant::Str("zaco_array_get".to_string())), args: vec![base, idx_val] });
            ctx.emit(Instruction::Assign { dest: Place::from_local(result_local), value: RValue::Use(Value::Temp(elem_temp)) });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));
        ctx.switch_to(merge_block);
        Some(Value::Local(result_local))
    }

    /// Lower a function call. Handles built-in modules specially.
    fn lower_call(
        &mut self,
        ctx: &mut FuncCtx,
        callee: &Node<Expr>,
        args: &[Node<Expr>],
        span: &Span,
    ) -> Option<Value> {
        // Handle super(args) call in constructor — calls parent constructor
        if matches!(&callee.value, Expr::Super) {
            return self.lower_super_call(ctx, args, span);
        }

        // Check for member calls (console.log, Math.floor, obj.method(), etc.)
        if let Expr::Member {
            object, property, ..
        } = &callee.value
        {
            if let Expr::Ident(obj_ident) = &object.value {
                let obj_name = &obj_ident.name;
                let method = &property.value.name;

                // Handle console methods
                if obj_name == "console" {
                    match method.as_str() {
                        "log" => {
                            return self.lower_console_log(ctx, args, span);
                        }
                        "error" | "warn" | "info" | "debug" => {
                            return self.lower_console_method(ctx, args, method, span);
                        }
                        _ => {}
                    }
                }

                // Handle Math methods
                if obj_name == "Math" {
                    return self.lower_math_method(ctx, method, args, span);
                }

                // Handle JSON methods
                if obj_name == "JSON" {
                    return self.lower_json_method(ctx, method, args, span);
                }

                // Handle process methods
                if obj_name == "process" {
                    return self.lower_process_method(ctx, method, args, span);
                }

                // Handle ClassName.staticMethod(args) — static method calls
                if let Some(ci) = self.class_info.get(obj_name).cloned() {
                    if ci.static_methods.contains(&method.to_string()) {
                        let func_name = format!("{}_{}", obj_name, method);
                        let mut arg_vals = Vec::new();
                        for arg in args {
                            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                                arg_vals.push(val);
                            } else {
                                return None;
                            }
                        }
                        let return_type = self.module.find_function(&func_name)
                            .map(|f| f.return_type.clone())
                            .unwrap_or(IrType::Void);
                        if return_type == IrType::Void {
                            ctx.emit(Instruction::Call {
                                dest: None,
                                func: Value::Const(Constant::Str(func_name)),
                                args: arg_vals,
                            });
                            return None;
                        } else {
                            let result = ctx.add_temp(return_type);
                            ctx.emit(Instruction::Call {
                                dest: Some(Place::from_temp(result)),
                                func: Value::Const(Constant::Str(func_name)),
                                args: arg_vals,
                            });
                            return Some(Value::Temp(result));
                        }
                    }
                }

                // Handle class instance method calls: obj.method(args)
                if let Some(info) = self.lookup_var(obj_name).cloned() {
                    if let IrType::Struct(struct_id) = &info.ir_type {
                        // Find class name from struct_id
                        if let Some((class_name, _)) = self.class_info.iter()
                            .find(|(_, ci)| ci.struct_id == *struct_id)
                            .map(|(k, v)| (k.clone(), v.clone()))
                        {
                            return self.lower_method_call(ctx, &class_name, method, &info, args, span);
                        }
                    }
                }
            }

            // Handle this.method(args) — method call on `this`
            if matches!(&object.value, Expr::This) {
                if let (Some(this_info), Some(class_name)) = (self.this_var.clone(), self.current_class.clone()) {
                    let method = &property.value.name;
                    return self.lower_method_call(ctx, &class_name, method, &this_info, args, span);
                }
            }

            // Handle super.method(args) — calls parent class method with `this`
            if matches!(&object.value, Expr::Super) {
                if let (Some(this_info), Some(parent_name)) = (self.this_var.clone(), self.current_class_parent.clone()) {
                    let method = &property.value.name;
                    return self.lower_method_call(ctx, &parent_name, method, &this_info, args, span);
                }
            }

            // Handle Promise.then/catch/finally chaining
            if let Expr::Ident(obj_ident) = &object.value {
                let method = &property.value.name;
                if matches!(method.as_str(), "then" | "catch" | "finally") {
                    if let Some(info) = self.lookup_var(&obj_ident.name).cloned() {
                        if matches!(info.ir_type, IrType::Promise(_)) {
                            return self.lower_promise_chain_method(ctx, &info, method, args, span);
                        }
                    }
                }
            }

            // Handle array.map/filter/forEach callbacks
            if let Expr::Ident(obj_ident) = &object.value {
                let method = &property.value.name;
                if matches!(method.as_str(), "map" | "filter" | "forEach" | "find" | "some" | "every" | "reduce") {
                    if let Some(info) = self.lookup_var(&obj_ident.name).cloned() {
                        if matches!(info.ir_type, IrType::Ptr) {
                            return self.lower_array_callback_method(ctx, &obj_ident.name, method, &info, args, span);
                        }
                    }
                }
            }
        }

        // Check for direct function calls (imported functions)
        let func_name = match &callee.value {
            Expr::Ident(ident) => ident.name.clone(),
            _ => return None, // Complex callees not yet supported
        };

        // Handle global built-in functions (parseInt, parseFloat, isNaN, isFinite, timers)
        if let Some((runtime_fn, param_types, ret_type)) = match func_name.as_str() {
            "parseInt" => Some(("zaco_parse_int", vec![IrType::Str], IrType::F64)),
            "parseFloat" => Some(("zaco_parse_float", vec![IrType::Str], IrType::F64)),
            "isNaN" => Some(("zaco_is_nan", vec![IrType::F64], IrType::Bool)),
            "isFinite" => Some(("zaco_is_finite", vec![IrType::F64], IrType::Bool)),
            "setTimeout" => Some(("zaco_set_timeout", vec![IrType::Ptr, IrType::Ptr, IrType::I64], IrType::I64)),
            "setInterval" => Some(("zaco_set_interval", vec![IrType::Ptr, IrType::Ptr, IrType::I64], IrType::I64)),
            "clearTimeout" => Some(("zaco_clear_timeout", vec![IrType::I64], IrType::Void)),
            "clearInterval" => Some(("zaco_clear_interval", vec![IrType::I64], IrType::Void)),
            _ => None,
        } {
            let mut arg_vals = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                if i < param_types.len() {
                    if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                        arg_vals.push(val);
                    }
                }
            }

            // For setTimeout/setInterval: inject null context between callback and delay.
            // TS signature: setTimeout(callback, delay) → 2 args
            // Runtime signature: zaco_set_timeout(callback, context, delay) → 3 args
            if func_name == "setTimeout" || func_name == "setInterval" {
                let null_ctx = ctx.add_temp(IrType::Ptr);
                ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(null_ctx),
                    value: RValue::Use(Value::Const(Constant::Null)),
                });
                if arg_vals.len() >= 2 {
                    arg_vals.insert(1, Value::Temp(null_ctx));
                }
            }

            if ret_type == IrType::Void {
                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str(runtime_fn.to_string())),
                    args: arg_vals,
                });
                return None;
            }
            let dest_temp = ctx.add_temp(ret_type);
            let dest = Place::from_temp(dest_temp);
            ctx.emit(Instruction::Call {
                dest: Some(dest.clone()),
                func: Value::Const(Constant::Str(runtime_fn.to_string())),
                args: arg_vals,
            });
            return Some(dest.base);
        }

        // Check if this is an imported function
        if let Some(module) = self.imported_bindings.get(&func_name).cloned() {
            return self.lower_imported_function_call(ctx, &module, &func_name, args, span);
        }

        // Check if this is a closure call
        if let Some(closure_info) = self.closure_bindings.get(&func_name).cloned() {
            return self.lower_closure_call(ctx, &closure_info, args, span);
        }

        // Regular function call — rename "main" to "_user_main" if needed
        let func_name = if func_name == "main" && self.has_user_main {
            "_user_main".to_string()
        } else {
            func_name
        };

        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        // Determine return type by looking up the called function's signature
        let return_type = self.module.find_function(&func_name)
            .map(|f| f.return_type.clone())
            .or_else(|| {
                // Check if this is a recursive call to the current function
                if let Some((ref cur_name, ref cur_ret)) = self.current_function {
                    if *cur_name == func_name {
                        return Some(cur_ret.clone());
                    }
                }
                None
            })
            .unwrap_or(IrType::Void);
        let dest = if return_type != IrType::Void {
            let temp = ctx.add_temp(return_type);
            Some(Place::from_temp(temp))
        } else {
            None
        };

        ctx.emit(Instruction::Call {
            dest: dest.clone(),
            func: Value::Const(Constant::Str(func_name)),
            args: arg_vals,
        });

        dest.map(|p| p.base)
    }

    /// Lower `console.log(args...)` to appropriate runtime calls.
    fn lower_console_log(
        &mut self,
        ctx: &mut FuncCtx,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        for (i, arg) in args.iter().enumerate() {
            // Print space separator between arguments (except first)
            if i > 0 {
                let space_str = " ".to_string();
                self.module.intern_string(space_str.clone());
                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str("zaco_print_str".to_string())),
                    args: vec![Value::Const(Constant::Str(space_str))],
                });
            }

            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                let arg_type = self.infer_expr_type(&arg.value);
                let runtime_fn = match arg_type {
                    IrType::Str => "zaco_print_str",
                    IrType::I64 => "zaco_print_i64",
                    IrType::F64 => "zaco_print_f64",
                    IrType::Bool => "zaco_print_bool",
                    _ => "zaco_print_str", // fallback
                };

                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str(runtime_fn.to_string())),
                    args: vec![val],
                });
            }
        }

        // Print newline at end — emit empty string println to get the newline
        let empty = "".to_string();
        self.module.intern_string(empty.clone());
        ctx.emit(Instruction::Call {
            dest: None,
            func: Value::Const(Constant::Str("zaco_println_str".to_string())),
            args: vec![Value::Const(Constant::Str(empty))],
        });

        None // console.log returns undefined
    }

    /// Lower `console.log/error/warn(args...)` to appropriate runtime calls.
    fn lower_console_method(
        &mut self,
        ctx: &mut FuncCtx,
        args: &[Node<Expr>],
        method: &str,
        _span: &Span,
    ) -> Option<Value> {
        let prefix = match method {
            "error" => "zaco_console_error",
            "warn" => "zaco_console_warn",
            "debug" => "zaco_console_debug",
            "info" => "zaco_print", // info uses same as log
            _ => "zaco_print",
        };

        for (i, arg) in args.iter().enumerate() {
            // Print space separator between arguments (except first)
            if i > 0 {
                let space_str = " ".to_string();
                self.module.intern_string(space_str.clone());
                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str(format!("{}_str", prefix))),
                    args: vec![Value::Const(Constant::Str(space_str))],
                });
            }

            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                let arg_type = self.infer_expr_type(&arg.value);
                let runtime_fn = match arg_type {
                    IrType::Str => format!("{}_str", prefix),
                    IrType::I64 => format!("{}_i64", prefix),
                    IrType::F64 => format!("{}_f64", prefix),
                    IrType::Bool => format!("{}_bool", prefix),
                    _ => format!("{}_str", prefix), // fallback
                };

                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str(runtime_fn)),
                    args: vec![val],
                });
            }
        }

        // Print newline using method-specific function
        let println_fn = match method {
            "error" => "zaco_console_errorln",
            "warn" => "zaco_console_warnln",
            "debug" => "zaco_console_debugln",
            _ => "zaco_println_str",
        };
        let empty = "".to_string();
        self.module.intern_string(empty.clone());
        ctx.emit(Instruction::Call {
            dest: None,
            func: Value::Const(Constant::Str(println_fn.to_string())),
            args: vec![Value::Const(Constant::Str(empty))],
        });

        None // console methods return undefined
    }

    /// Lower Math method calls to runtime functions.
    fn lower_math_method(
        &mut self,
        ctx: &mut FuncCtx,
        method: &str,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let (runtime_fn, param_types, return_type) = match method {
            "floor" => ("zaco_math_floor", vec![IrType::F64], IrType::F64),
            "ceil" => ("zaco_math_ceil", vec![IrType::F64], IrType::F64),
            "round" => ("zaco_math_round", vec![IrType::F64], IrType::F64),
            "abs" => ("zaco_math_abs", vec![IrType::F64], IrType::F64),
            "sqrt" => ("zaco_math_sqrt", vec![IrType::F64], IrType::F64),
            "pow" => ("zaco_math_pow", vec![IrType::F64, IrType::F64], IrType::F64),
            "sin" => ("zaco_math_sin", vec![IrType::F64], IrType::F64),
            "cos" => ("zaco_math_cos", vec![IrType::F64], IrType::F64),
            "tan" => ("zaco_math_tan", vec![IrType::F64], IrType::F64),
            "log" => ("zaco_math_log", vec![IrType::F64], IrType::F64),
            "log2" => ("zaco_math_log2", vec![IrType::F64], IrType::F64),
            "log10" => ("zaco_math_log10", vec![IrType::F64], IrType::F64),
            "random" => ("zaco_math_random", vec![], IrType::F64),
            "min" => ("zaco_math_min", vec![IrType::F64, IrType::F64], IrType::F64),
            "max" => ("zaco_math_max", vec![IrType::F64, IrType::F64], IrType::F64),
            "trunc" => ("zaco_math_trunc", vec![IrType::F64], IrType::F64),
            _ => return None, // Unknown Math method
        };

        self.ensure_extern(runtime_fn, param_types, return_type.clone());

        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        let temp = ctx.add_temp(return_type);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(temp)),
            func: Value::Const(Constant::Str(runtime_fn.to_string())),
            args: arg_vals,
        });

        Some(Value::Temp(temp))
    }

    /// Lower JSON method calls to runtime functions.
    fn lower_json_method(
        &mut self,
        ctx: &mut FuncCtx,
        method: &str,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let (runtime_fn, param_types, return_type) = match method {
            "parse" => ("zaco_json_parse", vec![IrType::Str], IrType::Str),
            "stringify" => ("zaco_json_stringify", vec![IrType::Ptr], IrType::Str),
            _ => return None,
        };

        self.ensure_extern(runtime_fn, param_types, return_type.clone());

        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        let temp = ctx.add_temp(return_type);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(temp)),
            func: Value::Const(Constant::Str(runtime_fn.to_string())),
            args: arg_vals,
        });

        Some(Value::Temp(temp))
    }

    /// Lower process method calls to runtime functions.
    fn lower_process_method(
        &mut self,
        ctx: &mut FuncCtx,
        method: &str,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let (runtime_fn, param_types, return_type) = match method {
            "exit" => ("zaco_process_exit", vec![IrType::I64], IrType::Void),
            "cwd" => ("zaco_process_cwd", vec![], IrType::Str),
            "pid" => ("zaco_process_pid", vec![], IrType::I64),
            "platform" => ("zaco_process_platform", vec![], IrType::Str),
            "arch" => ("zaco_process_arch", vec![], IrType::Str),
            _ => return None,
        };

        self.ensure_extern(runtime_fn, param_types, return_type.clone());

        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        if return_type == IrType::Void {
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str(runtime_fn.to_string())),
                args: arg_vals,
            });
            None
        } else {
            let temp = ctx.add_temp(return_type);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(temp)),
                func: Value::Const(Constant::Str(runtime_fn.to_string())),
                args: arg_vals,
            });
            Some(Value::Temp(temp))
        }
    }

    /// Look up the runtime function name, parameter types, and return type for an
    /// imported (module, func_name) pair. Returns `None` for unknown imports.
    fn imported_func_signature(module: &str, func_name: &str) -> Option<(&'static str, Vec<IrType>, IrType)> {
        let sig = match (module, func_name) {
            // fs module
            ("fs", "readFileSync") => ("zaco_fs_read_file_sync", vec![IrType::Str, IrType::Str], IrType::Str),
            ("fs", "writeFileSync") => ("zaco_fs_write_file_sync", vec![IrType::Str, IrType::Str], IrType::Void),
            ("fs", "existsSync") => ("zaco_fs_exists_sync", vec![IrType::Str], IrType::Bool),
            ("fs", "mkdirSync") => ("zaco_fs_mkdir_sync", vec![IrType::Str, IrType::I64], IrType::Void),
            // TODO: fs.readFile async callback API not yet safely supported.
            // Closures are lowered as struct pointers, but the runtime expects
            // extern "C" fn(*const c_char, *const c_char). Needs a trampoline mechanism.
            // ("fs", "readFile") => ("zaco_fs_read_file", vec![IrType::Str, IrType::Str, IrType::Ptr], IrType::Void),

            // path module
            ("path", "join") => ("zaco_path_join", vec![IrType::Str, IrType::Str], IrType::Str),
            ("path", "resolve") => ("zaco_path_resolve", vec![IrType::Str], IrType::Str),
            ("path", "dirname") => ("zaco_path_dirname", vec![IrType::Str], IrType::Str),
            ("path", "basename") => ("zaco_path_basename", vec![IrType::Str], IrType::Str),
            ("path", "extname") => ("zaco_path_extname", vec![IrType::Str], IrType::Str),

            // os module
            ("os", "platform") => ("zaco_os_platform", vec![], IrType::Str),
            ("os", "arch") => ("zaco_os_arch", vec![], IrType::Str),
            ("os", "homedir") => ("zaco_os_homedir", vec![], IrType::Str),
            ("os", "tmpdir") => ("zaco_os_tmpdir", vec![], IrType::Str),
            ("os", "hostname") => ("zaco_os_hostname", vec![], IrType::Str),
            ("os", "cpus") => ("zaco_os_cpus", vec![], IrType::Ptr),

            // process module
            ("process", "exit") => ("zaco_process_exit", vec![IrType::I64], IrType::Void),
            ("process", "cwd") => ("zaco_process_cwd", vec![], IrType::Str),

            // http module
            ("http", "get") => ("zaco_http_get", vec![IrType::Str], IrType::Str),
            ("http", "post") => ("zaco_http_post", vec![IrType::Str, IrType::Str, IrType::Str], IrType::Str),
            ("http", "put") => ("zaco_http_put", vec![IrType::Str, IrType::Str, IrType::Str], IrType::Str),
            ("http", "delete") => ("zaco_http_delete", vec![IrType::Str], IrType::Str),

            _ => return None,
        };
        Some(sig)
    }

    /// Lower imported function calls (fs, path, os, process, http modules).
    fn lower_imported_function_call(
        &mut self,
        ctx: &mut FuncCtx,
        module: &str,
        func_name: &str,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let (runtime_fn, param_types, return_type) = match Self::imported_func_signature(module, func_name) {
            Some(sig) => sig,
            None => return None, // Unknown import
        };

        self.ensure_extern(runtime_fn, param_types, return_type.clone());

        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        if return_type == IrType::Void {
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str(runtime_fn.to_string())),
                args: arg_vals,
            });
            None
        } else {
            let temp = ctx.add_temp(return_type);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(temp)),
                func: Value::Const(Constant::Str(runtime_fn.to_string())),
                args: arg_vals,
            });
            Some(Value::Temp(temp))
        }
    }

    /// Lower ternary/conditional expression: `cond ? then : else`
    fn lower_ternary(
        &mut self,
        ctx: &mut FuncCtx,
        condition: &Node<Expr>,
        then_expr: &Node<Expr>,
        else_expr: &Node<Expr>,
        _span: &Span,
    ) -> Option<Value> {
        let cond_val = self.lower_expr(ctx, &condition.value, &condition.span)?;

        let result_type = self.infer_expr_type(&then_expr.value);
        let result_local = ctx.add_local(result_type.clone());

        let then_block = ctx.new_block();
        let else_block = ctx.new_block();
        let merge_block = ctx.new_block();

        ctx.set_terminator(Terminator::Branch {
            cond: cond_val,
            then_block,
            else_block,
        });

        // Then branch
        ctx.switch_to(then_block);
        if let Some(then_val) = self.lower_expr(ctx, &then_expr.value, &then_expr.span) {
            ctx.emit(Instruction::Assign {
                dest: Place::from_local(result_local),
                value: RValue::Use(then_val),
            });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));

        // Else branch
        ctx.switch_to(else_block);
        if let Some(else_val) = self.lower_expr(ctx, &else_expr.value, &else_expr.span) {
            ctx.emit(Instruction::Assign {
                dest: Place::from_local(result_local),
                value: RValue::Use(else_val),
            });
        }
        ctx.set_terminator(Terminator::Jump(merge_block));

        // Continue from merge block
        ctx.switch_to(merge_block);

        Some(Value::Local(result_local))
    }

    fn lower_template(
        &mut self,
        ctx: &mut FuncCtx,
        parts: &[String],
        exprs: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        // Build a concatenation of all parts and expressions
        let mut values = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                self.module.intern_string(part.clone());
                values.push(Value::Const(Constant::Str(part.clone())));
            }
            if i < exprs.len() {
                if let Some(val) = self.lower_expr(ctx, &exprs[i].value, &exprs[i].span) {
                    // TODO: coerce non-string values to string
                    values.push(val);
                }
            }
        }

        if values.is_empty() {
            let empty = "".to_string();
            self.module.intern_string(empty.clone());
            return Some(Value::Const(Constant::Str(empty)));
        }
        if values.len() == 1 {
            return Some(values.into_iter().next().unwrap());
        }

        let temp = ctx.add_temp(IrType::Str);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(temp),
            value: RValue::StrConcat(values),
        });
        Some(Value::Temp(temp))
    }

    fn lower_array_literal(
        &mut self,
        ctx: &mut FuncCtx,
        elements: &[Option<Node<Expr>>],
        _span: &Span,
    ) -> Option<Value> {
        let mut vals = Vec::new();
        for elem in elements {
            if let Some(ref expr_node) = elem {
                if let Some(val) = self.lower_expr(ctx, &expr_node.value, &expr_node.span) {
                    vals.push(val);
                }
            }
        }
        let temp = ctx.add_temp(IrType::Array(Box::new(IrType::F64)));
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(temp),
            value: RValue::ArrayInit(vals),
        });
        Some(Value::Temp(temp))
    }

    fn lower_object_literal(
        &mut self,
        ctx: &mut FuncCtx,
        props: &[ObjectProperty],
        _span: &Span,
    ) -> Option<Value> {
        self.ensure_extern("zaco_object_new", vec![], IrType::Ptr);
        let obj_temp = ctx.add_temp(IrType::Ptr);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(obj_temp)),
            func: Value::Const(Constant::Str("zaco_object_new".to_string())),
            args: vec![],
        });

        for prop in props {
            match prop {
                ObjectProperty::Property { key, value, .. } => {
                    let key_str = match key {
                        PropertyName::Ident(ident) => ident.value.name.clone(),
                        PropertyName::String(s) => s.clone(),
                        PropertyName::Number(n) => format!("{}", n),
                        PropertyName::Computed(_) => continue,
                    };

                    self.module.intern_string(key_str.clone());
                    let key_val = Value::Const(Constant::Str(key_str));

                    if let Some(val) = self.lower_expr(ctx, &value.value, &value.span) {
                        let val_type = self.infer_expr_type(&value.value);
                        let setter_name = match &val_type {
                            IrType::Str => "zaco_object_set_str",
                            IrType::F64 => "zaco_object_set_f64",
                            IrType::I64 | IrType::Bool => "zaco_object_set_i64",
                            _ => "zaco_object_set_ptr",
                        };
                        let setter_val_type = match &val_type {
                            IrType::Str => IrType::Ptr,
                            IrType::Bool => IrType::I64,
                            other => other.clone(),
                        };
                        self.ensure_extern(
                            setter_name,
                            vec![IrType::Ptr, IrType::Ptr, setter_val_type],
                            IrType::Void,
                        );
                        ctx.emit(Instruction::Call {
                            dest: None,
                            func: Value::Const(Constant::Str(setter_name.to_string())),
                            args: vec![Value::Temp(obj_temp), key_val, val],
                        });
                    }
                }
                ObjectProperty::Spread(_) | ObjectProperty::Method { .. } => continue,
            }
        }

        Some(Value::Temp(obj_temp))
    }

    fn lower_if(
        &mut self,
        ctx: &mut FuncCtx,
        condition: &Node<Expr>,
        then_stmt: &Node<Stmt>,
        else_stmt: Option<&Node<Stmt>>,
        _span: &Span,
    ) {
        let cond_val = match self.lower_expr(ctx, &condition.value, &condition.span) {
            Some(v) => v,
            None => return,
        };

        let then_block = ctx.new_block();
        let else_block = ctx.new_block();
        let merge_block = ctx.new_block();

        ctx.set_terminator(Terminator::Branch {
            cond: cond_val,
            then_block,
            else_block,
        });

        // Then branch
        ctx.switch_to(then_block);
        self.push_scope();
        self.lower_stmt(ctx, &then_stmt.value, &then_stmt.span);
        self.pop_scope();
        // Only add jump if the block doesn't already have a return terminator
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(merge_block));
        }

        // Else branch
        ctx.switch_to(else_block);
        if let Some(else_s) = else_stmt {
            self.push_scope();
            self.lower_stmt(ctx, &else_s.value, &else_s.span);
            self.pop_scope();
        }
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(merge_block));
        }

        ctx.switch_to(merge_block);
    }

    fn lower_while(
        &mut self,
        ctx: &mut FuncCtx,
        condition: &Node<Expr>,
        body: &Node<Stmt>,
        _span: &Span,
    ) {
        let cond_block = ctx.new_block();
        let body_block = ctx.new_block();
        let exit_block = ctx.new_block();

        ctx.set_terminator(Terminator::Jump(cond_block));

        // Condition
        ctx.switch_to(cond_block);
        let cond_val = match self.lower_expr(ctx, &condition.value, &condition.span) {
            Some(v) => v,
            None => return,
        };
        ctx.set_terminator(Terminator::Branch {
            cond: cond_val,
            then_block: body_block,
            else_block: exit_block,
        });

        // Body
        ctx.switch_to(body_block);
        self.push_scope();
        self.loop_stack.push((cond_block, exit_block));
        self.break_stack.push(exit_block);
        self.lower_stmt(ctx, &body.value, &body.span);
        self.break_stack.pop();
        self.loop_stack.pop();
        self.pop_scope();
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(cond_block));
        }

        ctx.switch_to(exit_block);
    }

    fn lower_for(
        &mut self,
        ctx: &mut FuncCtx,
        init: Option<&ForInit>,
        condition: Option<&Node<Expr>>,
        update: Option<&Node<Expr>>,
        body: &Node<Stmt>,
        _span: &Span,
    ) {
        self.push_scope();

        // Init
        if let Some(for_init) = init {
            match for_init {
                ForInit::VarDecl(vd) => self.lower_var_decl(ctx, vd, &Span::new(0, 0, 0)),
                ForInit::Expr(e) => {
                    let _ = self.lower_expr(ctx, &e.value, &e.span);
                }
            }
        }

        let cond_block = ctx.new_block();
        let body_block = ctx.new_block();
        let update_block = ctx.new_block();
        let exit_block = ctx.new_block();

        ctx.set_terminator(Terminator::Jump(cond_block));

        // Condition
        ctx.switch_to(cond_block);
        if let Some(cond_expr) = condition {
            let cond_val = match self.lower_expr(ctx, &cond_expr.value, &cond_expr.span) {
                Some(v) => v,
                None => {
                    self.pop_scope();
                    return;
                }
            };
            ctx.set_terminator(Terminator::Branch {
                cond: cond_val,
                then_block: body_block,
                else_block: exit_block,
            });
        } else {
            // No condition = always true
            ctx.set_terminator(Terminator::Jump(body_block));
        }

        // Body
        ctx.switch_to(body_block);
        self.loop_stack.push((update_block, exit_block));
        self.break_stack.push(exit_block);
        self.lower_stmt(ctx, &body.value, &body.span);
        self.break_stack.pop();
        self.loop_stack.pop();
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(update_block));
        }

        // Update
        ctx.switch_to(update_block);
        if let Some(update_expr) = update {
            let _ = self.lower_expr(ctx, &update_expr.value, &update_expr.span);
        }
        ctx.set_terminator(Terminator::Jump(cond_block));

        ctx.switch_to(exit_block);
        self.pop_scope();
    }

    fn lower_throw(
        &mut self,
        ctx: &mut FuncCtx,
        expr_node: &Node<Expr>,
        _span: &Span,
    ) {
        // Ensure runtime functions are declared
        self.ensure_extern("zaco_throw", vec![IrType::Ptr], IrType::Void);

        // Lower the throw expression
        let val = if let Some(v) = self.lower_expr(ctx, &expr_node.value, &expr_node.span) {
            v
        } else {
            // If expression couldn't be lowered, throw null
            Value::Const(Constant::Null)
        };

        // Call zaco_throw(value)
        ctx.emit(Instruction::Call {
            dest: None,
            func: Value::Const(Constant::Str("zaco_throw".to_string())),
            args: vec![val],
        });

        // Code after throw is unreachable
        let dead_block = ctx.new_block();
        ctx.switch_to(dead_block);
    }

    fn lower_try(
        &mut self,
        ctx: &mut FuncCtx,
        block: &Node<BlockStmt>,
        catch: Option<&CatchClause>,
        finally: Option<&Node<BlockStmt>>,
        _span: &Span,
    ) {
        // Ensure runtime functions are declared
        self.ensure_extern("zaco_try_push", vec![], IrType::I64);
        self.ensure_extern("zaco_try_pop", vec![], IrType::Void);
        self.ensure_extern("zaco_get_error", vec![], IrType::Ptr);
        self.ensure_extern("zaco_clear_error", vec![], IrType::Void);

        let try_block = ctx.new_block();
        let catch_block = ctx.new_block();
        let finally_block = ctx.new_block();
        let continue_block = ctx.new_block();

        // Call zaco_try_push() → returns 0 normally, 1 on exception
        let result_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(result_temp)),
            func: Value::Const(Constant::Str("zaco_try_push".to_string())),
            args: vec![],
        });

        // Compare result == 0 (normal path)
        let zero_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(zero_temp),
            value: RValue::Use(Value::Const(Constant::I64(0))),
        });
        let cond_temp = ctx.add_temp(IrType::Bool);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(cond_temp),
            value: RValue::BinaryOp {
                op: BinOp::Eq,
                left: Value::Temp(result_temp),
                right: Value::Temp(zero_temp),
            },
        });

        // Branch: if result == 0, go to try_block; else go to catch_block
        ctx.set_terminator(Terminator::Branch {
            cond: Value::Temp(cond_temp),
            then_block: try_block,
            else_block: catch_block,
        });

        // === Try block ===
        ctx.switch_to(try_block);
        self.push_scope();
        for s in &block.value.stmts {
            self.lower_stmt(ctx, &s.value, &s.span);
        }
        self.pop_scope();

        // Pop try context on normal exit
        ctx.emit(Instruction::Call {
            dest: None,
            func: Value::Const(Constant::Str("zaco_try_pop".to_string())),
            args: vec![],
        });

        // Jump to finally block
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(finally_block));
        }

        // === Catch block ===
        ctx.switch_to(catch_block);
        if let Some(catch_clause) = catch {
            self.push_scope();

            // Get the error value
            let error_temp = ctx.add_temp(IrType::Ptr);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(error_temp)),
                func: Value::Const(Constant::Str("zaco_get_error".to_string())),
                args: vec![],
            });

            // Clear the error
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str("zaco_clear_error".to_string())),
                args: vec![],
            });

            // Bind error to catch parameter if present
            if let Some(ref param) = catch_clause.param {
                if let Pattern::Ident { name, .. } = &param.value {
                    let local_id = ctx.add_local(IrType::Ptr);
                    self.define_var(
                        &name.value.name,
                        VarInfo {
                            local_id,
                            ir_type: IrType::Ptr,
                            is_boxed: false,
                        },
                    );
                    ctx.emit(Instruction::Assign {
                        dest: Place::from_local(local_id),
                        value: RValue::Use(Value::Temp(error_temp)),
                    });
                }
            }

            // Lower catch body
            for s in &catch_clause.body.value.stmts {
                self.lower_stmt(ctx, &s.value, &s.span);
            }
            self.pop_scope();
        } else {
            // No catch clause — just get and clear error, then continue to finally
            let error_temp = ctx.add_temp(IrType::Ptr);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(error_temp)),
                func: Value::Const(Constant::Str("zaco_get_error".to_string())),
                args: vec![],
            });
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str("zaco_clear_error".to_string())),
                args: vec![],
            });
        }

        // Jump to finally
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(finally_block));
        }

        // === Finally block ===
        ctx.switch_to(finally_block);
        if let Some(finally_block_ast) = finally {
            self.push_scope();
            for s in &finally_block_ast.value.stmts {
                self.lower_stmt(ctx, &s.value, &s.span);
            }
            self.pop_scope();
        }

        // Jump to continue
        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(continue_block));
        }

        ctx.switch_to(continue_block);
    }

    fn lower_function_decl(
        &mut self,
        _ctx: &mut FuncCtx,
        func_decl: &FunctionDecl,
        _span: &Span,
    ) {
        let is_async = func_decl.is_async;
        let is_generator = func_decl.is_generator;

        if is_generator {
            self.lower_generator_function_decl(func_decl);
        } else if is_async {
            self.lower_async_function_decl(func_decl);
        } else {
            self.lower_sync_function_decl(func_decl);
        }
    }

    fn lower_sync_function_decl(&mut self, func_decl: &FunctionDecl) {
        let mut func_name = func_decl.name.value.name.clone();
        // Rename user-defined "main" to avoid conflict with compiler wrapper
        if func_name == "main" && self.has_user_main {
            func_name = "_user_main".to_string();
        }
        let func_id = self.alloc_func_id();

        // Build parameter list
        let mut ir_params = Vec::new();
        for (i, param) in func_decl.params.iter().enumerate() {
            let ir_type = self.infer_param_type(param);
            let local_id = LocalId(i);
            ir_params.push((local_id, ir_type));
        }

        // Infer return type
        let return_type = if let Some(ref ret_ty) = func_decl.return_type {
            self.ast_type_to_ir(&ret_ty.value)
        } else {
            IrType::Void
        };

        // Track current function for recursive call detection
        let prev_function = self.current_function.take();
        self.current_function = Some((func_name.clone(), return_type.clone()));

        let mut ir_func = IrFunction::new(func_id, func_name.clone(), ir_params.clone(), return_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;

        let mut func_ctx = FuncCtx {
            func: &mut ir_func,
            current_block: entry,
        };

        self.push_scope();

        // Register params in scope
        for (i, param) in func_decl.params.iter().enumerate() {
            let param_name = match &param.pattern.value {
                Pattern::Ident { name, .. } => name.value.name.clone(),
                _ => format!("_param{}", i),
            };
            let (local_id, ir_type) = &ir_params[i];
            self.define_var(
                &param_name,
                VarInfo {
                    local_id: *local_id,
                    ir_type: ir_type.clone(),
                    is_boxed: false,
                },
            );
        }

        // Lower body
        if let Some(ref body) = func_decl.body {
            for s in &body.value.stmts {
                self.lower_stmt(&mut func_ctx, &s.value, &s.span);
            }
        }

        // If no terminator set, add implicit return
        if matches!(
            func_ctx.func.block(func_ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            if return_type == IrType::Void {
                func_ctx.set_terminator(Terminator::Return(None));
            } else {
                // Return default value matching the return type
                let default_val = match &return_type {
                    IrType::F64 => Value::Const(Constant::F64(0.0)),
                    IrType::Bool => Value::Const(Constant::Bool(false)),
                    _ => Value::Const(Constant::I64(0)),
                };
                let temp = func_ctx.add_temp(return_type);
                func_ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(temp),
                    value: RValue::Use(default_val),
                });
                func_ctx.set_terminator(Terminator::Return(Some(Value::Temp(temp))));
            }
        }

        self.pop_scope();
        self.current_function = prev_function;

        self.module.add_function(ir_func);
    }

    fn lower_async_function_decl(&mut self, func_decl: &FunctionDecl) {
        let func_name = func_decl.name.value.name.clone();
        let func_id = self.alloc_func_id();

        // Build parameter list
        let mut ir_params = Vec::new();
        for (i, param) in func_decl.params.iter().enumerate() {
            let ir_type = self.infer_param_type(param);
            let local_id = LocalId(i);
            ir_params.push((local_id, ir_type));
        }

        // Infer inner return type (what the async function actually returns)
        // If the user specified Promise<T>, extract T. Otherwise, wrap in Promise.
        let return_type = if let Some(ref ret_ty) = func_decl.return_type {
            let ir_type = self.ast_type_to_ir(&ret_ty.value);
            // If it's already a Promise type, use it as-is
            if matches!(ir_type, IrType::Promise(_)) {
                ir_type
            } else {
                // Otherwise, wrap it in Promise
                IrType::Promise(Box::new(ir_type))
            }
        } else {
            // No return type specified → Promise<void>
            IrType::Promise(Box::new(IrType::Void))
        };

        // Ensure all promise-related extern functions are declared up front
        self.ensure_extern("zaco_promise_new", vec![], IrType::Ptr);
        self.ensure_extern("zaco_promise_resolve", vec![IrType::Ptr, IrType::Ptr], IrType::Void);

        let mut ir_func = IrFunction::new(func_id, func_name.clone(), ir_params.clone(), return_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;

        let mut func_ctx = FuncCtx {
            func: &mut ir_func,
            current_block: entry,
        };

        self.push_scope();

        // Register params in scope
        for (i, param) in func_decl.params.iter().enumerate() {
            let param_name = match &param.pattern.value {
                Pattern::Ident { name, .. } => name.value.name.clone(),
                _ => format!("_param{}", i),
            };
            let (local_id, ir_type) = &ir_params[i];
            self.define_var(
                &param_name,
                VarInfo {
                    local_id: *local_id,
                    ir_type: ir_type.clone(),
                    is_boxed: false,
                },
            );
        }

        // Create a new Promise
        let promise_temp = func_ctx.add_temp(IrType::Ptr);
        func_ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(promise_temp)),
            func: Value::Const(Constant::Str("zaco_promise_new".to_string())),
            args: vec![],
        });

        // Lower body - for now we execute it synchronously and resolve the promise
        // TODO: true async with task spawning
        if let Some(ref body) = func_decl.body {
            for s in &body.value.stmts {
                self.lower_stmt(&mut func_ctx, &s.value, &s.span);
            }
        }

        // Get the return value and resolve the promise
        // If the last statement was a return, we need to resolve the promise with that value
        // For now, we'll just return the promise (simplified version)
        if matches!(
            func_ctx.func.block(func_ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            // No explicit return, resolve with undefined/void
            func_ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str("zaco_promise_resolve".to_string())),
                args: vec![Value::Temp(promise_temp), Value::Const(Constant::Null)],
            });

            func_ctx.set_terminator(Terminator::Return(Some(Value::Temp(promise_temp))));
        }

        self.pop_scope();

        self.module.add_function(ir_func);
    }


    // =========================================================================
    // Generator function lowering (function* / yield)
    // =========================================================================

    /// Lower a generator function (function*) using a state-machine transformation.
    fn lower_generator_function_decl(&mut self, func_decl: &FunctionDecl) {
        let func_name = func_decl.name.value.name.clone();

        // Ensure generator runtime externs
        self.ensure_extern("zaco_generator_new", vec![IrType::Ptr, IrType::Ptr], IrType::Ptr);
        self.ensure_extern("zaco_generator_set_value", vec![IrType::Ptr, IrType::Ptr], IrType::Void);
        self.ensure_extern("zaco_generator_set_done", vec![IrType::Ptr], IrType::Void);

        // Collect yield points from the function body
        let yield_values = self.collect_yield_values(func_decl);
        let num_states = yield_values.len();

        // 1) Create the state struct: { state_index: I64 }
        let state_struct_id = self.alloc_struct_id();
        let state_struct = IrStruct::new(
            state_struct_id,
            format!("{func_name}__state"),
            vec![("state_index".to_string(), IrType::I64)],
        );
        self.module.add_struct(state_struct);

        // 2) Create the "next" function: <name>__next(state_ptr: Ptr) -> Ptr
        let next_func_id = self.alloc_func_id();
        let next_func_name = format!("{func_name}__next");
        let state_param = LocalId(0);
        let mut next_func = IrFunction::new(
            next_func_id,
            next_func_name.clone(),
            vec![(state_param, IrType::Ptr)],
            IrType::Ptr,
        );
        let next_entry = next_func.new_block();
        next_func.entry_block = next_entry;

        {
            let mut nctx = FuncCtx {
                func: &mut next_func,
                current_block: next_entry,
            };

            // Load state_index from state struct
            let idx_temp = nctx.add_temp(IrType::I64);
            nctx.emit(Instruction::Load {
                dest: Place::from_temp(idx_temp),
                ptr: Value::Local(state_param),
            });

            // Create blocks for each state + a "done" block
            let mut state_blocks = Vec::new();
            for _ in 0..num_states {
                state_blocks.push(nctx.new_block());
            }
            let done_block = nctx.new_block();

            // Chain of comparisons to dispatch to the right state
            let mut current_check_block = nctx.current_block;
            for (i, &state_block) in state_blocks.iter().enumerate() {
                nctx.switch_to(current_check_block);
                let cmp_temp = nctx.add_temp(IrType::Bool);
                nctx.emit(Instruction::Assign {
                    dest: Place::from_temp(cmp_temp),
                    value: RValue::BinaryOp {
                        op: BinOp::Eq,
                        left: Value::Temp(idx_temp),
                        right: Value::Const(Constant::I64(i as i64)),
                    },
                });
                let next_check = if i + 1 < num_states {
                    nctx.new_block()
                } else {
                    done_block
                };
                nctx.set_terminator(Terminator::Branch {
                    cond: Value::Temp(cmp_temp),
                    then_block: state_block,
                    else_block: next_check,
                });
                current_check_block = next_check;
            }

            // Emit each state block
            for (i, &state_block) in state_blocks.iter().enumerate() {
                nctx.switch_to(state_block);

                let yield_val = match &yield_values[i] {
                    Some(expr) => self.lower_yield_value_simple(&mut nctx, expr),
                    None => Value::Const(Constant::Null),
                };

                // Store yielded value
                nctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str("zaco_generator_set_value".to_string())),
                    args: vec![Value::Local(state_param), yield_val],
                });

                // Advance state index
                let next_state = nctx.add_temp(IrType::I64);
                nctx.emit(Instruction::Assign {
                    dest: Place::from_temp(next_state),
                    value: RValue::Use(Value::Const(Constant::I64((i + 1) as i64))),
                });
                nctx.emit(Instruction::Store {
                    ptr: Value::Local(state_param),
                    value: Value::Temp(next_state),
                });

                nctx.set_terminator(Terminator::Return(Some(Value::Local(state_param))));
            }

            // Done block
            nctx.switch_to(done_block);
            nctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str("zaco_generator_set_done".to_string())),
                args: vec![Value::Local(state_param)],
            });
            nctx.set_terminator(Terminator::Return(Some(Value::Local(state_param))));
        }

        self.module.add_function(next_func);

        // 3) Create the wrapper function: <name>(params...) -> Ptr
        let wrapper_func_id = self.alloc_func_id();
        let mut ir_params = Vec::new();
        for (i, param) in func_decl.params.iter().enumerate() {
            let ir_type = self.infer_param_type(param);
            ir_params.push((LocalId(i), ir_type));
        }

        let mut wrapper_func = IrFunction::new(
            wrapper_func_id,
            func_name.clone(),
            ir_params,
            IrType::Ptr,
        );
        let wrapper_entry = wrapper_func.new_block();
        wrapper_func.entry_block = wrapper_entry;

        {
            let mut wctx = FuncCtx {
                func: &mut wrapper_func,
                current_block: wrapper_entry,
            };

            // Allocate state struct
            let state_local = wctx.add_local(IrType::Ptr);
            wctx.emit(Instruction::Alloc {
                dest: Place::from_local(state_local),
                ty: IrType::Struct(state_struct_id),
            });

            // Initialize state_index to 0
            wctx.emit(Instruction::Store {
                ptr: Value::Local(state_local),
                value: Value::Const(Constant::I64(0)),
            });

            // Create generator object
            self.module.intern_string(next_func_name.clone());
            let gen_temp = wctx.add_temp(IrType::Ptr);
            wctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(gen_temp)),
                func: Value::Const(Constant::Str("zaco_generator_new".to_string())),
                args: vec![
                    Value::Const(Constant::Str(next_func_name)),
                    Value::Local(state_local),
                ],
            });

            wctx.set_terminator(Terminator::Return(Some(Value::Temp(gen_temp))));
        }

        self.module.add_function(wrapper_func);
    }

    /// Collect yield values from a generator function body (simple sequential case).
    fn collect_yield_values(&self, func_decl: &FunctionDecl) -> Vec<Option<Expr>> {
        let mut yields = Vec::new();
        if let Some(ref body) = func_decl.body {
            for stmt in &body.value.stmts {
                self.collect_yields_from_stmt(&stmt.value, &mut yields);
            }
        }
        yields
    }

    fn collect_yields_from_stmt(&self, stmt: &Stmt, yields: &mut Vec<Option<Expr>>) {
        match stmt {
            Stmt::Expr(expr_node) => {
                self.collect_yields_from_expr(&expr_node.value, yields);
            }
            Stmt::Return(Some(expr_node)) => {
                self.collect_yields_from_expr(&expr_node.value, yields);
            }
            Stmt::Block(block) => {
                for s in &block.stmts {
                    self.collect_yields_from_stmt(&s.value, yields);
                }
            }
            _ => {}
        }
    }

    fn collect_yields_from_expr(&self, expr: &Expr, yields: &mut Vec<Option<Expr>>) {
        match expr {
            Expr::Yield { argument, .. } => {
                yields.push(argument.as_ref().map(|a| a.value.clone()));
            }
            Expr::Binary { left, right, .. } => {
                self.collect_yields_from_expr(&left.value, yields);
                self.collect_yields_from_expr(&right.value, yields);
            }
            Expr::Call { callee, args, .. } => {
                self.collect_yields_from_expr(&callee.value, yields);
                for arg in args {
                    self.collect_yields_from_expr(&arg.value, yields);
                }
            }
            Expr::Paren(inner) => {
                self.collect_yields_from_expr(&inner.value, yields);
            }
            _ => {}
        }
    }

    /// Lower a simple yield value expression.
    fn lower_yield_value_simple(&mut self, ctx: &mut FuncCtx, expr: &Expr) -> Value {
        match expr {
            Expr::Literal(Literal::Number(n)) => Value::Const(Constant::F64(*n)),
            Expr::Literal(Literal::String(s)) => {
                self.module.intern_string(s.clone());
                Value::Const(Constant::Str(s.clone()))
            }
            Expr::Literal(Literal::Boolean(b)) => Value::Const(Constant::Bool(*b)),
            Expr::Literal(Literal::Null | Literal::Undefined) => Value::Const(Constant::Null),
            _ => {
                let span = Span { start: 0, end: 0, file_id: 0 };
                self.lower_expr(ctx, expr, &span).unwrap_or(Value::Const(Constant::Null))
            }
        }
    }

    /// Lower a yield expression encountered during normal body lowering.
    fn lower_yield_expr(
        &mut self,
        ctx: &mut FuncCtx,
        argument: Option<&Node<Expr>>,
        delegate: bool,
        span: &Span,
    ) -> Option<Value> {
        if delegate {
            // TODO: implement yield* delegation
            self.errors.push(LowerError::new(
                "yield* delegation is not yet implemented",
                span.clone(),
            ));
            return None;
        }

        let yield_val = if let Some(arg) = argument {
            self.lower_expr(ctx, &arg.value, &arg.span)
                .unwrap_or(Value::Const(Constant::Null))
        } else {
            Value::Const(Constant::Null)
        };

        self.ensure_extern("zaco_generator_yield", vec![IrType::Ptr], IrType::Ptr);

        let result = ctx.add_temp(IrType::Ptr);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(result)),
            func: Value::Const(Constant::Str("zaco_generator_yield".to_string())),
            args: vec![yield_val],
        });
        Some(Value::Temp(result))
    }

    // =========================================================================
    // Tagged template literal lowering
    // =========================================================================

    /// Lower a tagged template literal: tag`hello ${name} world`
    /// → tag(["hello ", " world"], name)
    fn lower_tagged_template(
        &mut self,
        ctx: &mut FuncCtx,
        tag: &Node<Expr>,
        parts: &[String],
        exprs: &[Node<Expr>],
        span: &Span,
    ) -> Option<Value> {
        // 1. Create an array of string parts (quasis)
        let mut string_vals = Vec::new();
        for part in parts {
            self.module.intern_string(part.clone());
            string_vals.push(Value::Const(Constant::Str(part.clone())));
        }

        let strings_array = ctx.add_temp(IrType::Array(Box::new(IrType::Str)));
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(strings_array),
            value: RValue::ArrayInit(string_vals),
        });

        // 2. Lower each interpolated expression
        let mut expr_vals = Vec::new();
        for expr_node in exprs {
            if let Some(val) = self.lower_expr(ctx, &expr_node.value, &expr_node.span) {
                expr_vals.push(val);
            }
        }

        // 3. Build args: [strings_array, ...expression_values]
        let mut call_args = vec![Value::Temp(strings_array)];
        call_args.extend(expr_vals);

        // 4. Call the tag function
        if let Some(tag_val) = self.lower_expr(ctx, &tag.value, &tag.span) {
            let result = ctx.add_temp(IrType::Ptr);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(result)),
                func: tag_val,
                args: call_args,
            });
            Some(Value::Temp(result))
        } else if let Expr::Ident(ident) = &tag.value {
            let result = ctx.add_temp(IrType::Ptr);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(result)),
                func: Value::Const(Constant::Str(ident.name.clone())),
                args: call_args,
            });
            Some(Value::Temp(result))
        } else {
            self.errors.push(LowerError::new(
                "unsupported tagged template tag expression",
                span.clone(),
            ));
            None
        }
    }

    fn lower_await(&mut self, ctx: &mut FuncCtx, expr: &Node<Expr>, _span: &Span) -> Option<Value> {
        // Lower the expression that should produce a Promise
        let promise_val = self.lower_expr(ctx, &expr.value, &expr.span)?;

        // Call zaco_async_block_on to wait for the promise to resolve
        self.ensure_extern("zaco_async_block_on", vec![IrType::Ptr], IrType::Ptr);

        let result_temp = ctx.add_temp(IrType::Ptr);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(result_temp)),
            func: Value::Const(Constant::Str("zaco_async_block_on".to_string())),
            args: vec![promise_val],
        });

        Some(Value::Temp(result_temp))
    }

    // =========================================================================
    // Switch statement
    // =========================================================================

    fn lower_switch(
        &mut self,
        ctx: &mut FuncCtx,
        discriminant: &Node<Expr>,
        cases: &[SwitchCase],
        _span: &Span,
    ) {
        // Evaluate discriminant once and store in a temp
        let disc_val = match self.lower_expr(ctx, &discriminant.value, &discriminant.span) {
            Some(v) => v,
            None => return,
        };

        let disc_type = self.infer_expr_type(&discriminant.value);
        let disc_temp = ctx.add_temp(disc_type.clone());
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(disc_temp),
            value: RValue::Use(disc_val),
        });

        let exit_block = ctx.new_block();

        // Create blocks for each case body
        let mut case_body_blocks: Vec<BlockId> = Vec::new();
        for _ in cases {
            case_body_blocks.push(ctx.new_block());
        }

        // Find default case index
        let default_idx = cases.iter().position(|c| c.test.is_none());

        // Generate if-else chain for case matching
        for (i, case) in cases.iter().enumerate() {
            if let Some(ref test) = case.test {
                // Compare discriminant == test value
                let test_val = match self.lower_expr(ctx, &test.value, &test.span) {
                    Some(v) => v,
                    None => continue,
                };

                let cmp_temp = ctx.add_temp(IrType::Bool);
                ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(cmp_temp),
                    value: RValue::BinaryOp {
                        op: BinOp::Eq,
                        left: Value::Temp(disc_temp),
                        right: test_val,
                    },
                });

                let next_check = ctx.new_block();
                ctx.set_terminator(Terminator::Branch {
                    cond: Value::Temp(cmp_temp),
                    then_block: case_body_blocks[i],
                    else_block: next_check,
                });
                ctx.switch_to(next_check);
            }
        }

        // After all checks: jump to default case or exit
        if let Some(def_idx) = default_idx {
            ctx.set_terminator(Terminator::Jump(case_body_blocks[def_idx]));
        } else {
            ctx.set_terminator(Terminator::Jump(exit_block));
        }

        // Generate case bodies with fall-through
        self.break_stack.push(exit_block);

        for (i, case) in cases.iter().enumerate() {
            ctx.switch_to(case_body_blocks[i]);
            self.push_scope();

            for stmt in &case.consequent {
                self.lower_stmt(ctx, &stmt.value, &stmt.span);
            }

            self.pop_scope();

            // Fall-through: check the CURRENT block (not the original case body block)
            // because nested control flow (if-else, loops) may have created new blocks,
            // leaving ctx.current_block pointing to a merge/continuation block.
            let current = ctx.current_block;
            if matches!(
                ctx.func.block(current).terminator,
                Terminator::Unreachable
            ) {
                let fall_target = if i + 1 < cases.len() {
                    case_body_blocks[i + 1]
                } else {
                    exit_block
                };
                ctx.func.block_mut(current).terminator = Terminator::Jump(fall_target);
            }

            // Dead blocks after break/return are left with Unreachable terminator
            // (they have no predecessors and will be eliminated by Cranelift)
        }

        self.break_stack.pop();
        ctx.switch_to(exit_block);
    }

    // =========================================================================
    // For-in / For-of loops
    // =========================================================================

    /// Extract variable name from a ForInLeft
    fn extract_for_in_var_name(&self, left: &ForInLeft) -> Option<String> {
        match left {
            ForInLeft::VarDecl(vd) => {
                if let Some(declarator) = vd.declarations.first() {
                    if let Pattern::Ident { name, .. } = &declarator.pattern.value {
                        return Some(name.value.name.clone());
                    }
                }
                None
            }
            ForInLeft::Pattern(pat) => {
                if let Pattern::Ident { name, .. } = &pat.value {
                    Some(name.value.name.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Lower for-in loop (iterates over array indices).
    /// Simplified: works for arrays, yields numeric indices.
    fn lower_for_in(
        &mut self,
        ctx: &mut FuncCtx,
        left: &ForInLeft,
        right: &Node<Expr>,
        body: &Node<Stmt>,
        _span: &Span,
    ) {
        self.push_scope();

        // Evaluate the right expression (array)
        let arr_val = match self.lower_expr(ctx, &right.value, &right.span) {
            Some(v) => v,
            None => {
                self.pop_scope();
                return;
            }
        };

        // Store array in a temp for stability across blocks
        let arr_temp = ctx.add_temp(IrType::Ptr);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(arr_temp),
            value: RValue::Use(arr_val),
        });

        // Get array length via runtime call
        self.ensure_extern("zaco_array_length", vec![IrType::Ptr], IrType::I64);
        let len_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(len_temp)),
            func: Value::Const(Constant::Str("zaco_array_length".to_string())),
            args: vec![Value::Temp(arr_temp)],
        });

        // Create counter variable (i64 for array indexing)
        let counter_local = ctx.add_local(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(counter_local),
            value: RValue::Use(Value::Const(Constant::I64(0))),
        });

        // Create user-facing iteration variable (f64, TypeScript number)
        let var_name = self.extract_for_in_var_name(left);
        if let Some(ref name) = var_name {
            let var_local = ctx.add_local(IrType::F64);
            self.define_var(
                name,
                VarInfo {
                    local_id: var_local,
                    ir_type: IrType::F64,
                    is_boxed: false,
                },
            );
        }

        // Create loop blocks
        let cond_block = ctx.new_block();
        let body_block = ctx.new_block();
        let update_block = ctx.new_block();
        let exit_block = ctx.new_block();

        ctx.set_terminator(Terminator::Jump(cond_block));

        // Condition: counter < length
        ctx.switch_to(cond_block);
        let cond_temp = ctx.add_temp(IrType::Bool);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(cond_temp),
            value: RValue::BinaryOp {
                op: BinOp::Lt,
                left: Value::Local(counter_local),
                right: Value::Temp(len_temp),
            },
        });
        ctx.set_terminator(Terminator::Branch {
            cond: Value::Temp(cond_temp),
            then_block: body_block,
            else_block: exit_block,
        });

        // Body
        ctx.switch_to(body_block);

        // Bind index (counter as f64) to user variable
        if let Some(ref name) = var_name {
            if let Some(info) = self.lookup_var(name).cloned() {
                let idx_as_f64 = ctx.add_temp(IrType::F64);
                ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(idx_as_f64),
                    value: RValue::Cast {
                        value: Value::Local(counter_local),
                        ty: IrType::F64,
                    },
                });
                ctx.emit(Instruction::Assign {
                    dest: Place::from_local(info.local_id),
                    value: RValue::Use(Value::Temp(idx_as_f64)),
                });
            }
        }

        self.loop_stack.push((update_block, exit_block));
        self.break_stack.push(exit_block);
        self.lower_stmt(ctx, &body.value, &body.span);
        self.break_stack.pop();
        self.loop_stack.pop();

        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(update_block));
        }

        // Update: counter++
        ctx.switch_to(update_block);
        let inc_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(inc_temp),
            value: RValue::BinaryOp {
                op: BinOp::Add,
                left: Value::Local(counter_local),
                right: Value::Const(Constant::I64(1)),
            },
        });
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(counter_local),
            value: RValue::Use(Value::Temp(inc_temp)),
        });
        ctx.set_terminator(Terminator::Jump(cond_block));

        ctx.switch_to(exit_block);
        self.pop_scope();
    }

    /// Lower for-of loop (iterates over array values).
    /// Simplified: works for arrays, yields element values.
    fn lower_for_of(
        &mut self,
        ctx: &mut FuncCtx,
        left: &ForInLeft,
        right: &Node<Expr>,
        body: &Node<Stmt>,
        _span: &Span,
    ) {
        self.push_scope();

        // Evaluate the right expression (array)
        let arr_val = match self.lower_expr(ctx, &right.value, &right.span) {
            Some(v) => v,
            None => {
                self.pop_scope();
                return;
            }
        };

        // Store array in a temp for stability across blocks
        let arr_temp = ctx.add_temp(IrType::Ptr);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(arr_temp),
            value: RValue::Use(arr_val),
        });

        // Get array length
        self.ensure_extern("zaco_array_length", vec![IrType::Ptr], IrType::I64);
        let len_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(len_temp)),
            func: Value::Const(Constant::Str("zaco_array_length".to_string())),
            args: vec![Value::Temp(arr_temp)],
        });

        // Determine element type from the array type
        let arr_type = self.infer_expr_type(&right.value);
        let elem_type = match &arr_type {
            IrType::Array(inner) => (**inner).clone(),
            _ => IrType::F64,
        };

        // Choose runtime getter based on element type
        let (getter_name, getter_ret_type) = match &elem_type {
            IrType::Str | IrType::Ptr | IrType::Array(_) | IrType::Struct(_) => {
                ("zaco_array_get_ptr", IrType::Ptr)
            }
            _ => ("zaco_array_get_f64", IrType::F64),
        };
        self.ensure_extern(
            getter_name,
            vec![IrType::Ptr, IrType::I64],
            getter_ret_type.clone(),
        );

        // Create counter (i64)
        let counter_local = ctx.add_local(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(counter_local),
            value: RValue::Use(Value::Const(Constant::I64(0))),
        });

        // Create user-facing iteration variable
        let var_name = self.extract_for_in_var_name(left);
        let user_elem_type = if getter_ret_type == IrType::Ptr && elem_type == IrType::Str {
            IrType::Str
        } else {
            getter_ret_type.clone()
        };
        if let Some(ref name) = var_name {
            let var_local = ctx.add_local(user_elem_type.clone());
            self.define_var(
                name,
                VarInfo {
                    local_id: var_local,
                    ir_type: user_elem_type.clone(),
                    is_boxed: false,
                },
            );
        }

        // Create loop blocks
        let cond_block = ctx.new_block();
        let body_block = ctx.new_block();
        let update_block = ctx.new_block();
        let exit_block = ctx.new_block();

        ctx.set_terminator(Terminator::Jump(cond_block));

        // Condition: counter < length
        ctx.switch_to(cond_block);
        let cond_temp = ctx.add_temp(IrType::Bool);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(cond_temp),
            value: RValue::BinaryOp {
                op: BinOp::Lt,
                left: Value::Local(counter_local),
                right: Value::Temp(len_temp),
            },
        });
        ctx.set_terminator(Terminator::Branch {
            cond: Value::Temp(cond_temp),
            then_block: body_block,
            else_block: exit_block,
        });

        // Body
        ctx.switch_to(body_block);

        // Get element at counter index and bind to user variable
        if let Some(ref name) = var_name {
            if let Some(info) = self.lookup_var(name).cloned() {
                let elem_temp = ctx.add_temp(getter_ret_type.clone());
                ctx.emit(Instruction::Call {
                    dest: Some(Place::from_temp(elem_temp)),
                    func: Value::Const(Constant::Str(getter_name.to_string())),
                    args: vec![Value::Temp(arr_temp), Value::Local(counter_local)],
                });
                ctx.emit(Instruction::Assign {
                    dest: Place::from_local(info.local_id),
                    value: RValue::Use(Value::Temp(elem_temp)),
                });
            }
        }

        self.loop_stack.push((update_block, exit_block));
        self.break_stack.push(exit_block);
        self.lower_stmt(ctx, &body.value, &body.span);
        self.break_stack.pop();
        self.loop_stack.pop();

        if matches!(
            ctx.func.block(ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            ctx.set_terminator(Terminator::Jump(update_block));
        }

        // Update: counter++
        ctx.switch_to(update_block);
        let inc_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(inc_temp),
            value: RValue::BinaryOp {
                op: BinOp::Add,
                left: Value::Local(counter_local),
                right: Value::Const(Constant::I64(1)),
            },
        });
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(counter_local),
            value: RValue::Use(Value::Temp(inc_temp)),
        });
        ctx.set_terminator(Terminator::Jump(cond_block));

        ctx.switch_to(exit_block);
        self.pop_scope();
    }

    // =========================================================================
    // Class support
    // =========================================================================

    /// Lower a class declaration into struct + constructor + method functions.
    fn lower_class_decl(&mut self, _ctx: &mut FuncCtx, class_decl: &ClassDecl, span: &Span) {
        let class_name = class_decl.name.value.name.clone();

        // Step 0: Resolve parent class (if extends)
        let parent_name = class_decl.extends.as_ref().and_then(|ext| {
            if let Expr::Ident(ident) = &ext.base.value {
                Some(ident.name.clone())
            } else {
                None
            }
        });
        let parent_info = parent_name.as_ref().and_then(|name| self.class_info.get(name).cloned());
        let parent_field_count = parent_info.as_ref().map(|pi| pi.fields.len()).unwrap_or(0);

        // Step 1: Collect fields — parent fields first, then own fields
        let mut fields: Vec<(String, IrType)> = Vec::new();
        if let Some(ref pi) = parent_info {
            fields.extend(pi.fields.clone());
        }
        for member in &class_decl.members {
            if let ClassMember::Property {
                name, type_annotation, is_static, ..
            } = member
            {
                if *is_static {
                    continue;
                }
                let field_name = self.property_name_to_string(name);
                let field_type = type_annotation
                    .as_ref()
                    .map(|t| self.ast_type_to_ir(&t.value))
                    .unwrap_or(IrType::F64);
                fields.push((field_name, field_type));
            }
        }

        // Step 2: Create IrStruct
        let struct_id = self.alloc_struct_id();
        let struct_def = IrStruct::new(struct_id, class_name.clone(), fields.clone());
        self.module.add_struct(struct_def);

        // Collect method names (own + inherited)
        let mut method_names: Vec<String> = Vec::new();
        // Inherit parent methods first
        if let Some(ref pi) = parent_info {
            method_names.extend(pi.methods.clone());
        }
        // Add/override with own methods
        for member in &class_decl.members {
            if let ClassMember::Method { name, is_static, .. } = member {
                if *is_static {
                    continue;
                }
                let mname = self.property_name_to_string(name);
                // Remove parent's version if overriding
                method_names.retain(|m| m != &mname);
                method_names.push(mname);
            }
        }

        // Collect getter/setter names
        let mut getter_names: Vec<String> = Vec::new();
        let mut setter_names: Vec<String> = Vec::new();
        for member in &class_decl.members {
            match member {
                ClassMember::Getter { name, is_static, .. } if !*is_static => {
                    getter_names.push(self.property_name_to_string(name));
                }
                ClassMember::Setter { name, is_static, .. } if !*is_static => {
                    setter_names.push(self.property_name_to_string(name));
                }
                _ => {}
            }
        }

        // Collect static method names
        let mut static_method_names: Vec<String> = Vec::new();
        for member in &class_decl.members {
            if let ClassMember::Method { name, is_static, .. } = member {
                if *is_static {
                    static_method_names.push(self.property_name_to_string(name));
                }
            }
        }

        // Collect static property names and types
        let mut static_prop_info: Vec<(String, IrType)> = Vec::new();
        for member in &class_decl.members {
            if let ClassMember::Property { name, type_annotation, is_static, .. } = member {
                if *is_static {
                    let prop_name = self.property_name_to_string(name);
                    let prop_type = type_annotation
                        .as_ref()
                        .map(|t| self.ast_type_to_ir(&t.value))
                        .unwrap_or(IrType::F64);
                    static_prop_info.push((prop_name, prop_type));
                }
            }
        }

        // Register class info
        self.class_info.insert(class_name.clone(), ClassInfo {
            struct_id,
            fields: fields.clone(),
            methods: method_names.clone(),
            parent: parent_name.clone(),
            parent_field_count,
            getters: getter_names,
            setters: setter_names,
            static_methods: static_method_names,
            static_properties: static_prop_info.clone(),
        });

        // Step 3: Lower constructor
        self.lower_class_constructor(class_decl, &class_name, struct_id, &fields, parent_name.as_deref(), span);

        // Step 4: Lower own methods
        for member in &class_decl.members {
            if let ClassMember::Method {
                name, params, return_type, body, is_static, ..
            } = member
            {
                if *is_static {
                    continue;
                }
                if let Some(body) = body {
                    let method_name = self.property_name_to_string(name);
                    self.lower_class_method(
                        &class_name,
                        &method_name,
                        struct_id,
                        params,
                        return_type.as_deref(),
                        body,
                        &fields,
                        span,
                    );
                }
            }
        }

        // Step 5: Lower static methods (no self parameter)
        for member in &class_decl.members {
            if let ClassMember::Method {
                name, params, return_type, body, is_static, ..
            } = member
            {
                if !*is_static {
                    continue;
                }
                if let Some(body) = body {
                    let method_name = self.property_name_to_string(name);
                    self.lower_static_method(
                        &class_name,
                        &method_name,
                        params,
                        return_type.as_deref(),
                        body,
                        span,
                    );
                }
            }
        }

        // Step 6: Lower static properties as module-level globals
        for member in &class_decl.members {
            if let ClassMember::Property { name, type_annotation, is_static, init, .. } = member {
                if *is_static {
                    let prop_name = self.property_name_to_string(name);
                    let prop_type = type_annotation
                        .as_ref()
                        .map(|t| self.ast_type_to_ir(&t.value))
                        .unwrap_or(IrType::F64);
                    let global_name = format!("{}_{}", class_name, prop_name);
                    let init_const = init.as_ref().and_then(|e| self.expr_to_constant(&e.value));
                    self.module.add_global(global_name, prop_type, init_const);
                }
            }
        }

        // Step 7: Lower getters
        for member in &class_decl.members {
            if let ClassMember::Getter { name, return_type, body, is_static, .. } = member {
                if *is_static { continue; }
                if let Some(body) = body {
                    let getter_name = self.property_name_to_string(name);
                    let ret_type = return_type
                        .as_ref()
                        .map(|t| self.ast_type_to_ir(&t.value))
                        .unwrap_or(IrType::F64);
                    self.lower_getter_function(&class_name, &getter_name, struct_id, &ret_type, body, span);
                }
            }
        }

        // Step 8: Lower setters
        for member in &class_decl.members {
            if let ClassMember::Setter { name, param, body, is_static, .. } = member {
                if *is_static { continue; }
                if let Some(body) = body {
                    let setter_name = self.property_name_to_string(name);
                    self.lower_setter_function(&class_name, &setter_name, struct_id, param, body, span);
                }
            }
        }

        // Step 9: For inherited methods not overridden, create forwarding stubs
        if let Some(ref pi) = parent_info {
            let own_method_names: HashSet<String> = class_decl.members.iter().filter_map(|m| {
                if let ClassMember::Method { name, is_static, .. } = m {
                    if !*is_static { Some(self.property_name_to_string(name)) } else { None }
                } else {
                    None
                }
            }).collect();

            for parent_method in &pi.methods {
                if !own_method_names.contains(parent_method) {
                    // Create a thin forwarding function: Child_method → calls Parent_method
                    self.create_method_forward(&class_name, parent_method, struct_id, parent_name.as_ref().unwrap(), span);
                }
            }
        }
    }

    /// Create a forwarding stub: ChildClass_method(self, args...) → ParentClass_method(self, args...)
    fn create_method_forward(
        &mut self,
        child_class: &str,
        method_name: &str,
        child_struct_id: StructId,
        parent_class: &str,
        _span: &Span,
    ) {
        let parent_func_name = format!("{}_{}", parent_class, method_name);
        let child_func_name = format!("{}_{}", child_class, method_name);

        // Look up the parent method signature from the module
        let (param_types, ret_type) = if let Some(parent_func) = self.module.find_function(&parent_func_name) {
            let params: Vec<(LocalId, IrType)> = parent_func.params.clone();
            (params, parent_func.return_type.clone())
        } else {
            return; // Parent method not found, skip
        };

        let func_id = self.alloc_func_id();

        // Build params: same as parent but with child struct type for self
        let mut ir_params: Vec<(LocalId, IrType)> = Vec::new();
        for (i, (_, ty)) in param_types.iter().enumerate() {
            if i == 0 {
                ir_params.push((LocalId(0), IrType::Struct(child_struct_id)));
            } else {
                ir_params.push((LocalId(i), ty.clone()));
            }
        }

        let mut ir_func = IrFunction::new(func_id, child_func_name, ir_params.clone(), ret_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;

        let mut func_ctx = FuncCtx {
            func: &mut ir_func,
            current_block: entry,
        };

        // Forward call: ParentClass_method(self, args...)
        let arg_vals: Vec<Value> = ir_params.iter().map(|(lid, _)| Value::Local(*lid)).collect();

        if ret_type == IrType::Void {
            func_ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str(parent_func_name)),
                args: arg_vals,
            });
            func_ctx.set_terminator(Terminator::Return(None));
        } else {
            let result = func_ctx.add_temp(ret_type);
            func_ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(result)),
                func: Value::Const(Constant::Str(parent_func_name)),
                args: arg_vals,
            });
            func_ctx.set_terminator(Terminator::Return(Some(Value::Temp(result))));
        }

        self.module.add_function(ir_func);
    }

    /// Lower a static method into a standalone function (no self parameter)
    fn lower_static_method(&mut self, class_name: &str, method_name: &str, params: &[Param], return_type: Option<&Node<Type>>, body: &Node<BlockStmt>, _span: &Span) {
        let func_name = format!("{}_{}", class_name, method_name);
        let func_id = self.alloc_func_id();
        let mut ir_params: Vec<(LocalId, IrType)> = Vec::new();
        for (i, param) in params.iter().enumerate() {
            ir_params.push((LocalId(i), self.infer_param_type(param)));
        }
        let ret_type = return_type.map(|t| self.ast_type_to_ir(&t.value)).unwrap_or(IrType::Void);
        let mut ir_func = IrFunction::new(func_id, func_name, ir_params.clone(), ret_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;
        let mut func_ctx = FuncCtx { func: &mut ir_func, current_block: entry };
        self.push_scope();
        for (i, param) in params.iter().enumerate() {
            let pn = match &param.pattern.value { Pattern::Ident { name, .. } => name.value.name.clone(), _ => format!("_param{}", i) };
            let (lid, ty) = &ir_params[i];
            self.define_var(&pn, VarInfo { local_id: *lid, ir_type: ty.clone(), is_boxed: false });
        }
        let prev_class = self.current_class.take();
        self.current_class = Some(class_name.to_string());
        for s in &body.value.stmts { self.lower_stmt(&mut func_ctx, &s.value, &s.span); }
        if matches!(func_ctx.func.block(func_ctx.current_block).terminator, Terminator::Unreachable) {
            if ret_type == IrType::Void { func_ctx.set_terminator(Terminator::Return(None)); }
            else { let t = func_ctx.add_temp(ret_type); func_ctx.emit(Instruction::Assign { dest: Place::from_temp(t), value: RValue::Use(Value::Const(Constant::I64(0))) }); func_ctx.set_terminator(Terminator::Return(Some(Value::Temp(t)))); }
        }
        self.current_class = prev_class;
        self.pop_scope();
        self.module.add_function(ir_func);
    }

    /// Convert a constant expression to a Constant value
    fn expr_to_constant(&self, expr: &Expr) -> Option<Constant> {
        match expr {
            Expr::Literal(Literal::Number(n)) => { if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 { Some(Constant::I64(*n as i64)) } else { Some(Constant::F64(*n)) } }
            Expr::Literal(Literal::String(s)) => Some(Constant::Str(s.clone())),
            Expr::Literal(Literal::Boolean(b)) => Some(Constant::Bool(*b)),
            _ => None,
        }
    }

    /// Lower a getter: ClassName_get_propName(self) -> return_type
    fn lower_getter_function(&mut self, class_name: &str, prop_name: &str, struct_id: StructId, ret_type: &IrType, body: &Node<BlockStmt>, _span: &Span) {
        let func_name = format!("{}_get_{}", class_name, prop_name);
        let func_id = self.alloc_func_id();
        let ir_params = vec![(LocalId(0), IrType::Struct(struct_id))];
        let mut ir_func = IrFunction::new(func_id, func_name, ir_params, ret_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;
        let mut func_ctx = FuncCtx { func: &mut ir_func, current_block: entry };
        self.push_scope();
        let prev_this = self.this_var.take();
        let prev_class = self.current_class.take();
        self.this_var = Some(VarInfo { local_id: LocalId(0), ir_type: IrType::Struct(struct_id), is_boxed: false });
        self.current_class = Some(class_name.to_string());
        for s in &body.value.stmts { self.lower_stmt(&mut func_ctx, &s.value, &s.span); }
        if matches!(func_ctx.func.block(func_ctx.current_block).terminator, Terminator::Unreachable) {
            let t = func_ctx.add_temp(ret_type.clone());
            func_ctx.emit(Instruction::Assign { dest: Place::from_temp(t), value: RValue::Use(Value::Const(Constant::I64(0))) });
            func_ctx.set_terminator(Terminator::Return(Some(Value::Temp(t))));
        }
        self.this_var = prev_this;
        self.current_class = prev_class;
        self.pop_scope();
        self.module.add_function(ir_func);
    }

    /// Lower a setter: ClassName_set_propName(self, value) -> void
    fn lower_setter_function(&mut self, class_name: &str, prop_name: &str, struct_id: StructId, param: &Param, body: &Node<BlockStmt>, _span: &Span) {
        let func_name = format!("{}_set_{}", class_name, prop_name);
        let func_id = self.alloc_func_id();
        let param_type = self.infer_param_type(param);
        let ir_params = vec![(LocalId(0), IrType::Struct(struct_id)), (LocalId(1), param_type.clone())];
        let mut ir_func = IrFunction::new(func_id, func_name, ir_params, IrType::Void);
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;
        let mut func_ctx = FuncCtx { func: &mut ir_func, current_block: entry };
        self.push_scope();
        let prev_this = self.this_var.take();
        let prev_class = self.current_class.take();
        self.this_var = Some(VarInfo { local_id: LocalId(0), ir_type: IrType::Struct(struct_id), is_boxed: false });
        self.current_class = Some(class_name.to_string());
        let pn = match &param.pattern.value { Pattern::Ident { name, .. } => name.value.name.clone(), _ => "_value".to_string() };
        self.define_var(&pn, VarInfo { local_id: LocalId(1), ir_type: param_type, is_boxed: false });
        for s in &body.value.stmts { self.lower_stmt(&mut func_ctx, &s.value, &s.span); }
        if matches!(func_ctx.func.block(func_ctx.current_block).terminator, Terminator::Unreachable) {
            func_ctx.set_terminator(Terminator::Return(None));
        }
        self.this_var = prev_this;
        self.current_class = prev_class;
        self.pop_scope();
        self.module.add_function(ir_func);
    }

    /// Lower a class constructor into a function: ClassName_constructor(params) -> Ptr
    fn lower_class_constructor(
        &mut self,
        class_decl: &ClassDecl,
        class_name: &str,
        struct_id: StructId,
        fields: &[(String, IrType)],
        parent_name: Option<&str>,
        _span: &Span,
    ) {
        let constructor_name = format!("{}_constructor", class_name);
        let func_id = self.alloc_func_id();

        // Find constructor member
        let (ctor_params, ctor_body) = class_decl.members.iter().find_map(|m| {
            if let ClassMember::Constructor { params, body, .. } = m {
                Some((params.clone(), body.clone()))
            } else {
                None
            }
        }).unwrap_or_else(|| (Vec::new(), None));

        // Build parameter list for the constructor function
        let mut ir_params: Vec<(LocalId, IrType)> = Vec::new();
        for (i, param) in ctor_params.iter().enumerate() {
            let ir_type = self.infer_param_type(param);
            let local_id = LocalId(i);
            ir_params.push((local_id, ir_type));
        }

        // Return type is always Ptr (pointer to struct)
        let mut ir_func = IrFunction::new(
            func_id,
            constructor_name.clone(),
            ir_params.clone(),
            IrType::Struct(struct_id),
        );
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;

        let mut func_ctx = FuncCtx {
            func: &mut ir_func,
            current_block: entry,
        };

        self.push_scope();

        // Register constructor params in scope
        for (i, param) in ctor_params.iter().enumerate() {
            let param_name = match &param.pattern.value {
                Pattern::Ident { name, .. } => name.value.name.clone(),
                _ => format!("_param{}", i),
            };
            let (local_id, ir_type) = &ir_params[i];
            self.define_var(&param_name, VarInfo {
                local_id: *local_id,
                ir_type: ir_type.clone(),
                is_boxed: false,
            });
        }

        // Allocate struct (includes parent fields)
        let struct_size: usize = fields.iter().map(|(_, ty)| ty.size_bytes()).sum();
        let size_temp = func_ctx.add_temp(IrType::I64);
        func_ctx.emit(Instruction::Assign {
            dest: Place::from_temp(size_temp),
            value: RValue::Use(Value::Const(Constant::I64(struct_size as i64))),
        });

        let self_local = func_ctx.add_local(IrType::Struct(struct_id));
        func_ctx.emit(Instruction::Alloc {
            dest: Place::from_local(self_local),
            ty: IrType::Struct(struct_id),
        });

        // Set up `this` for the constructor body
        let prev_this = self.this_var.take();
        let prev_class = self.current_class.take();
        self.this_var = Some(VarInfo {
            local_id: self_local,
            ir_type: IrType::Struct(struct_id),
            is_boxed: false,
        });
        self.current_class = Some(class_name.to_string());

        // Initialize all fields with defaults
        for (_i, (_, field_type)) in fields.iter().enumerate() {
            let default_val = match field_type {
                IrType::F64 => Value::Const(Constant::F64(0.0)),
                IrType::I64 => Value::Const(Constant::I64(0)),
                IrType::Bool => Value::Const(Constant::Bool(false)),
                _ => Value::Const(Constant::Null),
            };
            let field_temp = func_ctx.add_temp(field_type.clone());
            func_ctx.emit(Instruction::Assign {
                dest: Place::from_temp(field_temp),
                value: RValue::Use(default_val),
            });
            func_ctx.emit(Instruction::Store {
                ptr: Value::Local(self_local),
                value: Value::Temp(field_temp),
            });
        }

        // Store parent class name so super() calls can be resolved
        let parent_for_super = parent_name.map(|s| s.to_string());

        // Lower constructor body — super() calls are handled in lower_stmt/lower_call
        // by detecting Call { callee: Expr::Super, args } pattern
        if let Some(ref body) = ctor_body {
            // Before lowering body, save parent info for super() resolution
            let prev_parent = std::mem::replace(
                &mut self.current_class_parent,
                parent_for_super,
            );

            for s in &body.value.stmts {
                self.lower_stmt(&mut func_ctx, &s.value, &s.span);
            }

            self.current_class_parent = prev_parent;
        }

        // Return self
        if matches!(
            func_ctx.func.block(func_ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            func_ctx.set_terminator(Terminator::Return(Some(Value::Local(self_local))));
        }

        // Restore this/class context
        self.this_var = prev_this;
        self.current_class = prev_class;
        self.pop_scope();

        self.module.add_function(ir_func);
    }

    /// Lower a class method into a function: ClassName_methodName(self: Ptr, params...) -> ReturnType
    fn lower_class_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        struct_id: StructId,
        params: &[Param],
        return_type: Option<&Node<Type>>,
        body: &Node<BlockStmt>,
        _fields: &[(String, IrType)],
        _span: &Span,
    ) {
        let func_name = format!("{}_{}", class_name, method_name);
        let func_id = self.alloc_func_id();

        // First param is always `self` (pointer to struct)
        let mut ir_params: Vec<(LocalId, IrType)> = Vec::new();
        ir_params.push((LocalId(0), IrType::Struct(struct_id))); // self

        for (i, param) in params.iter().enumerate() {
            let ir_type = self.infer_param_type(param);
            let local_id = LocalId(i + 1); // +1 because self is 0
            ir_params.push((local_id, ir_type));
        }

        let ret_type = return_type
            .map(|t| self.ast_type_to_ir(&t.value))
            .unwrap_or(IrType::Void);

        let mut ir_func = IrFunction::new(func_id, func_name.clone(), ir_params.clone(), ret_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;

        let mut func_ctx = FuncCtx {
            func: &mut ir_func,
            current_block: entry,
        };

        self.push_scope();

        // Set up `this` as first param
        let prev_this = self.this_var.take();
        let prev_class = self.current_class.take();
        self.this_var = Some(VarInfo {
            local_id: LocalId(0),
            ir_type: IrType::Struct(struct_id),
            is_boxed: false,
        });
        self.current_class = Some(class_name.to_string());

        // Register non-self params in scope
        for (i, param) in params.iter().enumerate() {
            let param_name = match &param.pattern.value {
                Pattern::Ident { name, .. } => name.value.name.clone(),
                _ => format!("_param{}", i),
            };
            let (local_id, ir_type) = &ir_params[i + 1]; // +1 to skip self
            self.define_var(&param_name, VarInfo {
                local_id: *local_id,
                ir_type: ir_type.clone(),
                is_boxed: false,
            });
        }

        // Lower body
        for s in &body.value.stmts {
            self.lower_stmt(&mut func_ctx, &s.value, &s.span);
        }

        // Add implicit return if needed
        if matches!(
            func_ctx.func.block(func_ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            if ret_type == IrType::Void {
                func_ctx.set_terminator(Terminator::Return(None));
            } else {
                let temp = func_ctx.add_temp(ret_type);
                func_ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(temp),
                    value: RValue::Use(Value::Const(Constant::I64(0))),
                });
                func_ctx.set_terminator(Terminator::Return(Some(Value::Temp(temp))));
            }
        }

        // Restore this/class context
        self.this_var = prev_this;
        self.current_class = prev_class;
        self.pop_scope();

        self.module.add_function(ir_func);
    }

    /// Lower `new ClassName(args)` expression
    fn lower_new_expr(
        &mut self,
        ctx: &mut FuncCtx,
        callee: &Node<Expr>,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let class_name = match &callee.value {
            Expr::Ident(ident) => ident.name.clone(),
            _ => return None,
        };

        // Verify it's a known class
        let class_info = self.class_info.get(&class_name)?.clone();

        // Lower arguments
        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        // Call ClassName_constructor(args) -> Ptr
        let constructor_name = format!("{}_constructor", class_name);
        let result = ctx.add_temp(IrType::Struct(class_info.struct_id));
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(result)),
            func: Value::Const(Constant::Str(constructor_name)),
            args: arg_vals,
        });

        Some(Value::Temp(result))
    }

    /// Lower `this` expression
    fn lower_this_expr(&self) -> Option<Value> {
        self.this_var.as_ref().map(|info| Value::Local(info.local_id))
    }

    /// Lower member expression: object.property (for reads)
    fn lower_member_expr(
        &mut self,
        ctx: &mut FuncCtx,
        object: &Node<Expr>,
        property: &Node<Ident>,
        _span: &Span,
    ) -> Option<Value> {
        // Check for Math.PI, Math.E, etc.
        if let Expr::Ident(obj_ident) = &object.value {
            if obj_ident.name == "Math" {
                match property.value.name.as_str() {
                    "PI" => return Some(Value::Const(Constant::F64(std::f64::consts::PI))),
                    "E" => return Some(Value::Const(Constant::F64(std::f64::consts::E))),
                    _ => {}
                }
            }
        }

        // Handle ClassName.staticProp — static property access
        if let Expr::Ident(obj_ident) = &object.value {
            if let Some(ci) = self.class_info.get(&obj_ident.name).cloned() {
                let prop = &property.value.name;
                if let Some((_, prop_type)) = ci.static_properties.iter().find(|(n, _)| n == prop) {
                    let global_name = format!("{}_{}", obj_ident.name, prop);
                    let result = ctx.add_temp(prop_type.clone());
                    ctx.emit(Instruction::Load {
                        dest: Place::from_temp(result),
                        ptr: Value::Const(Constant::Str(global_name)),
                    });
                    return Some(Value::Temp(result));
                }
            }
        }

        // Handle this.field — check getter first
        if matches!(&object.value, Expr::This) {
            if let (Some(this_info), Some(class_name)) = (&self.this_var, &self.current_class) {
                let field_name = &property.value.name;
                // Check if this property has a getter
                let has_getter = self.class_info.get(class_name)
                    .map(|ci| ci.getters.contains(&field_name.to_string()))
                    .unwrap_or(false);
                if has_getter {
                    let getter_func = format!("{}_get_{}", class_name, field_name);
                    let ret_type = self.module.find_function(&getter_func)
                        .map(|f| f.return_type.clone())
                        .unwrap_or(IrType::F64);
                    let result = ctx.add_temp(ret_type);
                    ctx.emit(Instruction::Call {
                        dest: Some(Place::from_temp(result)),
                        func: Value::Const(Constant::Str(getter_func)),
                        args: vec![Value::Local(this_info.local_id)],
                    });
                    return Some(Value::Temp(result));
                }
                let class_info = self.class_info.get(class_name)?;
                let field_idx = class_info.fields.iter().position(|(n, _)| n == field_name)?;
                let field_type = class_info.fields[field_idx].1.clone();

                let result = ctx.add_temp(field_type);
                ctx.emit(Instruction::Load {
                    dest: Place::from_temp(result),
                    ptr: Value::Local(this_info.local_id),
                });
                let field_place = Place::from_local(this_info.local_id).field(field_idx);
                ctx.emit(Instruction::Load {
                    dest: Place::from_temp(result),
                    ptr: field_place.base,
                });
                return self.load_struct_field(ctx, Value::Local(this_info.local_id), class_name, field_name);
            }
        }

        // Handle obj.field where obj is a class instance — check getter first
        if let Expr::Ident(obj_ident) = &object.value {
            if let Some(info) = self.lookup_var(&obj_ident.name).cloned() {
                if let IrType::Struct(struct_id) = &info.ir_type {
                    if let Some((class_name, _)) = self.class_info.iter()
                        .find(|(_, ci)| ci.struct_id == *struct_id)
                        .map(|(k, v)| (k.clone(), v.clone()))
                    {
                        let field_name = &property.value.name;
                        // Check if this property has a getter
                        let has_getter = self.class_info.get(&class_name)
                            .map(|ci| ci.getters.contains(&field_name.to_string()))
                            .unwrap_or(false);
                        if has_getter {
                            let getter_func = format!("{}_get_{}", class_name, field_name);
                            let ret_type = self.module.find_function(&getter_func)
                                .map(|f| f.return_type.clone())
                                .unwrap_or(IrType::F64);
                            let result = ctx.add_temp(ret_type);
                            ctx.emit(Instruction::Call {
                                dest: Some(Place::from_temp(result)),
                                func: Value::Const(Constant::Str(getter_func)),
                                args: vec![Value::Local(info.local_id)],
                            });
                            return Some(Value::Temp(result));
                        }
                        return self.load_struct_field(ctx, Value::Local(info.local_id), &class_name, field_name);
                    }
                }
            }
        }

        // For other member expressions, fall through
        None
    }

    /// Load a field from a struct pointer by computing offset
    fn load_struct_field(
        &self,
        ctx: &mut FuncCtx,
        obj_ptr: Value,
        class_name: &str,
        field_name: &str,
    ) -> Option<Value> {
        let class_info = self.class_info.get(class_name)?;
        let field_idx = class_info.fields.iter().position(|(n, _)| n == field_name)?;
        let field_type = class_info.fields[field_idx].1.clone();

        // Compute byte offset
        let mut offset: i64 = 0;
        for i in 0..field_idx {
            offset += class_info.fields[i].1.size_bytes() as i64;
        }

        if offset == 0 {
            // Field is at the start of struct, load directly from ptr
            let result = ctx.add_temp(field_type);
            ctx.emit(Instruction::Load {
                dest: Place::from_temp(result),
                ptr: obj_ptr,
            });
            Some(Value::Temp(result))
        } else {
            // Compute ptr + offset, then load
            // We need to add offset to the pointer
            let offset_temp = ctx.add_temp(IrType::I64);
            ctx.emit(Instruction::Assign {
                dest: Place::from_temp(offset_temp),
                value: RValue::Use(Value::Const(Constant::I64(offset))),
            });
            let addr_temp = ctx.add_temp(IrType::Ptr);
            ctx.emit(Instruction::Assign {
                dest: Place::from_temp(addr_temp),
                value: RValue::BinaryOp {
                    op: BinOp::Add,
                    left: obj_ptr,
                    right: Value::Temp(offset_temp),
                },
            });
            let result = ctx.add_temp(field_type);
            ctx.emit(Instruction::Load {
                dest: Place::from_temp(result),
                ptr: Value::Temp(addr_temp),
            });
            Some(Value::Temp(result))
        }
    }

    /// Store a value to a struct field by computing offset
    fn store_struct_field(
        &self,
        ctx: &mut FuncCtx,
        obj_ptr: Value,
        class_name: &str,
        field_name: &str,
        value: Value,
    ) -> bool {
        let class_info = match self.class_info.get(class_name) {
            Some(ci) => ci,
            None => return false,
        };
        let field_idx = match class_info.fields.iter().position(|(n, _)| n == field_name) {
            Some(idx) => idx,
            None => return false,
        };

        // Compute byte offset
        let mut offset: i64 = 0;
        for i in 0..field_idx {
            offset += class_info.fields[i].1.size_bytes() as i64;
        }

        if offset == 0 {
            ctx.emit(Instruction::Store {
                ptr: obj_ptr,
                value,
            });
        } else {
            let offset_temp = ctx.add_temp(IrType::I64);
            ctx.emit(Instruction::Assign {
                dest: Place::from_temp(offset_temp),
                value: RValue::Use(Value::Const(Constant::I64(offset))),
            });
            let addr_temp = ctx.add_temp(IrType::Ptr);
            ctx.emit(Instruction::Assign {
                dest: Place::from_temp(addr_temp),
                value: RValue::BinaryOp {
                    op: BinOp::Add,
                    left: obj_ptr,
                    right: Value::Temp(offset_temp),
                },
            });
            ctx.emit(Instruction::Store {
                ptr: Value::Temp(addr_temp),
                value,
            });
        }
        true
    }

    /// Lower member assignment: this.field = value or obj.field = value
    fn lower_member_assignment(
        &mut self,
        ctx: &mut FuncCtx,
        object: &Node<Expr>,
        property: &Node<Ident>,
        _op: AssignmentOp,
        rhs: Value,
    ) -> Option<Value> {
        let field_name = &property.value.name;

        // Handle ClassName.staticProp = value — static property write
        if let Expr::Ident(obj_ident) = &object.value {
            if let Some(ci) = self.class_info.get(&obj_ident.name).cloned() {
                if ci.static_properties.iter().any(|(n, _)| n == field_name) {
                    let global_name = format!("{}_{}", obj_ident.name, field_name);
                    ctx.emit(Instruction::Store {
                        ptr: Value::Const(Constant::Str(global_name)),
                        value: rhs.clone(),
                    });
                    return Some(rhs);
                }
            }
        }

        // Handle this.field = value — check setter first
        if matches!(&object.value, Expr::This) {
            if let (Some(this_info), Some(class_name)) = (self.this_var.clone(), self.current_class.clone()) {
                let has_setter = self.class_info.get(&class_name)
                    .map(|ci| ci.setters.contains(&field_name.to_string()))
                    .unwrap_or(false);
                if has_setter {
                    let setter_func = format!("{}_set_{}", class_name, field_name);
                    ctx.emit(Instruction::Call {
                        dest: None,
                        func: Value::Const(Constant::Str(setter_func)),
                        args: vec![Value::Local(this_info.local_id), rhs.clone()],
                    });
                    return Some(rhs);
                }
                self.store_struct_field(ctx, Value::Local(this_info.local_id), &class_name, field_name, rhs.clone());
                return Some(rhs);
            }
        }

        // Handle obj.field = value — check setter first
        if let Expr::Ident(obj_ident) = &object.value {
            if let Some(info) = self.lookup_var(&obj_ident.name).cloned() {
                if let IrType::Struct(struct_id) = &info.ir_type {
                    if let Some((class_name, _)) = self.class_info.iter()
                        .find(|(_, ci)| ci.struct_id == *struct_id)
                        .map(|(k, v)| (k.clone(), v.clone()))
                    {
                        let has_setter = self.class_info.get(&class_name)
                            .map(|ci| ci.setters.contains(&field_name.to_string()))
                            .unwrap_or(false);
                        if has_setter {
                            let setter_func = format!("{}_set_{}", class_name, field_name);
                            ctx.emit(Instruction::Call {
                                dest: None,
                                func: Value::Const(Constant::Str(setter_func)),
                                args: vec![Value::Local(info.local_id), rhs.clone()],
                            });
                            return Some(rhs);
                        }
                        self.store_struct_field(ctx, Value::Local(info.local_id), &class_name, field_name, rhs.clone());
                        return Some(rhs);
                    }
                }
            }
        }

        None
    }

    /// Lower a method call on a class instance
    fn lower_method_call(
        &mut self,
        ctx: &mut FuncCtx,
        class_name: &str,
        method_name: &str,
        obj_info: &VarInfo,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let func_name = format!("{}_{}", class_name, method_name);

        // First arg is self (the object pointer)
        let mut arg_vals = vec![Value::Local(obj_info.local_id)];
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        // Look up return type
        let return_type = self.module.find_function(&func_name)
            .map(|f| f.return_type.clone())
            .unwrap_or(IrType::Void);

        if return_type == IrType::Void {
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str(func_name)),
                args: arg_vals,
            });
            None
        } else {
            let result = ctx.add_temp(return_type);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(result)),
                func: Value::Const(Constant::Str(func_name)),
                args: arg_vals,
            });
            Some(Value::Temp(result))
        }
    }

    /// Extract string from PropertyName
    fn property_name_to_string(&self, name: &PropertyName) -> String {
        match name {
            PropertyName::Ident(ident) => ident.value.name.clone(),
            PropertyName::String(s) => s.clone(),
            PropertyName::Number(n) => format!("{}", n),
            PropertyName::Computed(_) => "_computed".to_string(),
        }
    }

    /// Lower super(args) call — invokes parent constructor, copies parent fields to self
    fn lower_super_call(
        &mut self,
        ctx: &mut FuncCtx,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let parent_name = self.current_class_parent.clone()?;
        let this_info = self.this_var.clone()?;
        let class_name = self.current_class.clone()?;

        // Lower arguments
        let mut arg_vals = Vec::new();
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        // Call parent constructor: ParentClass_constructor(args) -> parent_ptr
        let parent_info = self.class_info.get(&parent_name)?.clone();
        let parent_ctor = format!("{}_constructor", parent_name);
        let parent_result = ctx.add_temp(IrType::Struct(parent_info.struct_id));
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(parent_result)),
            func: Value::Const(Constant::Str(parent_ctor)),
            args: arg_vals,
        });

        // Copy parent fields from parent_result to self (first N fields in the child struct)
        let child_info = self.class_info.get(&class_name)?.clone();
        for (i, (field_name, _)) in parent_info.fields.iter().enumerate() {
            // Load field from parent instance
            if let Some(val) = self.load_struct_field(ctx, Value::Temp(parent_result), &parent_name, field_name) {
                // Store into child's corresponding field (same position due to layout)
                let _ = self.store_struct_field(ctx, Value::Local(this_info.local_id), &class_name, field_name, val);
                let _ = i;
                let _ = &child_info;
            }
        }

        None // super() doesn't produce a value
    }

    // =========================================================================
    // Closure support
    // =========================================================================

    /// Lower an arrow function expression.
    /// Creates a heap-allocated environment struct for captured variables.
    /// Closure = (function_name, env_ptr) pair.
    fn lower_arrow_expr(
        &mut self,
        ctx: &mut FuncCtx,
        params: &[Param],
        return_type: Option<&Node<Type>>,
        body: &ArrowBody,
        _span: &Span,
    ) -> Option<Value> {
        let closure_id = self.next_closure_id;
        self.next_closure_id += 1;
        let func_name = format!("__closure_{}", closure_id);

        // Collect the body statements
        let body_stmts: Vec<Node<Stmt>> = match body {
            ArrowBody::Expr(expr) => {
                vec![Node::new(
                    Stmt::Return(Some((**expr).clone())),
                    expr.span,
                )]
            }
            ArrowBody::Block(block) => {
                block.value.stmts.clone()
            }
        };

        // Collect free variables
        let param_names: HashSet<String> = params.iter().filter_map(|p| {
            match &p.pattern.value {
                Pattern::Ident { name, .. } => Some(name.value.name.clone()),
                _ => None,
            }
        }).collect();
        let captured_vars = self.collect_captured_vars(&body_stmts, &param_names);

        // Detect which captured variables are mutated inside the closure body
        let mutated_captured = self.collect_mutated_captured_vars(&body_stmts, &param_names);

        // Box any captured variables that are mutated (capture-by-reference)
        // Save original types before boxing
        let mut original_types: HashMap<String, IrType> = HashMap::new();
        for cap_name in &captured_vars {
            if mutated_captured.contains(cap_name) {
                if let Some(info) = self.lookup_var(cap_name).cloned() {
                    if !info.is_boxed {
                        original_types.insert(cap_name.clone(), info.ir_type.clone());
                        // Allocate a box and store the current value
                        self.ensure_extern("zaco_box_new", vec![IrType::Ptr], IrType::Ptr);
                        let box_local = ctx.add_local(IrType::Ptr);
                        ctx.emit(Instruction::Call {
                            dest: Some(Place::from_local(box_local)),
                            func: Value::Const(Constant::Str("zaco_box_new".to_string())),
                            args: vec![Value::Local(info.local_id)],
                        });
                        // Redefine the variable to use the box pointer
                        self.define_var(cap_name, VarInfo {
                            local_id: box_local,
                            ir_type: info.ir_type,
                            is_boxed: true,
                        });
                    }
                }
            }
        }

        // Create environment struct if there are captured variables
        let env_struct_id = if !captured_vars.is_empty() {
            let env_id = self.alloc_struct_id();
            let env_fields: Vec<(String, IrType)> = captured_vars.iter().map(|name| {
                let info = self.lookup_var(name);
                let ty = info.map(|i| {
                    if i.is_boxed { IrType::Ptr } else { i.ir_type.clone() }
                }).unwrap_or(IrType::F64);
                (name.clone(), ty)
            }).collect();
            let env_struct = IrStruct::new(env_id, format!("__env_{}", closure_id), env_fields.clone());
            self.module.add_struct(env_struct);

            // Register env as a "class" so store/load_struct_field works
            let env_name = format!("__env_{}", closure_id);
            self.class_info.insert(env_name, ClassInfo {
                struct_id: env_id,
                fields: env_fields,
                methods: Vec::new(),
                parent: None,
                parent_field_count: 0,
                getters: Vec::new(),
                setters: Vec::new(),
                static_methods: Vec::new(),
                static_properties: Vec::new(),
            });

            Some(env_id)
        } else {
            None
        };

        // Allocate env struct on the heap and store captured values (in the calling context)
        let env_local = if let Some(env_id) = env_struct_id {
            let env_local = ctx.add_local(IrType::Struct(env_id));
            ctx.emit(Instruction::Alloc {
                dest: Place::from_local(env_local),
                ty: IrType::Struct(env_id),
            });

            let env_name = format!("__env_{}", closure_id);
            for cap_name in &captured_vars {
                if let Some(info) = self.lookup_var(cap_name).cloned() {
                    self.store_struct_field(
                        ctx,
                        Value::Local(env_local),
                        &env_name,
                        cap_name,
                        Value::Local(info.local_id),
                    );
                }
            }

            Some(env_local)
        } else {
            None
        };

        // Build parameter list for closure function: env_ptr (if captures), then declared params
        let mut ir_params: Vec<(LocalId, IrType)> = Vec::new();
        let mut local_idx = 0;

        if let Some(env_id) = env_struct_id {
            ir_params.push((LocalId(local_idx), IrType::Struct(env_id)));
            local_idx += 1;
        }

        for param in params {
            let ir_type = self.infer_param_type(param);
            ir_params.push((LocalId(local_idx), ir_type));
            local_idx += 1;
        }

        // Infer return type
        let ret_type = return_type
            .map(|t| self.ast_type_to_ir(&t.value))
            .unwrap_or_else(|| {
                match body {
                    ArrowBody::Expr(expr) => self.infer_expr_type(&expr.value),
                    _ => IrType::Void,
                }
            });

        let func_id = self.alloc_func_id();
        let mut ir_func = IrFunction::new(func_id, func_name.clone(), ir_params.clone(), ret_type.clone());
        let entry = ir_func.new_block();
        ir_func.entry_block = entry;

        let mut closure_ctx = FuncCtx {
            func: &mut ir_func,
            current_block: entry,
        };

        self.push_scope();

        // Load captured vars from environment struct into local variables
        if let Some(_env_id) = env_struct_id {
            let env_param_local = LocalId(0);
            let env_name = format!("__env_{}", closure_id);

            for cap_name in &captured_vars {
                if let Some(val) = self.load_struct_field(&mut closure_ctx, Value::Local(env_param_local), &env_name, cap_name) {
                    let cap_type = self.class_info.get(&env_name)
                        .and_then(|ci| ci.fields.iter().find(|(n, _)| n == cap_name))
                        .map(|(_, t)| t.clone())
                        .unwrap_or(IrType::F64);
                    let cap_local = closure_ctx.add_local(cap_type.clone());
                    closure_ctx.emit(Instruction::Assign {
                        dest: Place::from_local(cap_local),
                        value: RValue::Use(val),
                    });
                    // If this variable was boxed (mutated capture), mark it as boxed
                    // so reads/writes inside the closure go through box_get/box_set
                    let is_boxed_cap = mutated_captured.contains(cap_name);
                    let logical_type = if is_boxed_cap {
                        original_types.get(cap_name).cloned().unwrap_or(cap_type)
                    } else {
                        cap_type
                    };
                    self.define_var(cap_name, VarInfo {
                        local_id: cap_local,
                        ir_type: logical_type,
                        is_boxed: is_boxed_cap,
                    });
                }
            }
        }

        // Register declared params in scope
        let param_offset = if env_struct_id.is_some() { 1 } else { 0 };
        for (i, param) in params.iter().enumerate() {
            let param_name = match &param.pattern.value {
                Pattern::Ident { name, .. } => name.value.name.clone(),
                _ => format!("_param{}", i),
            };
            let idx = param_offset + i;
            let (local_id, ir_type) = &ir_params[idx];
            self.define_var(&param_name, VarInfo {
                local_id: *local_id,
                ir_type: ir_type.clone(),
                is_boxed: false,
            });
        }

        // Lower body
        for s in &body_stmts {
            self.lower_stmt(&mut closure_ctx, &s.value, &s.span);
        }

        // Add implicit return if needed
        if matches!(
            closure_ctx.func.block(closure_ctx.current_block).terminator,
            Terminator::Unreachable
        ) {
            if ret_type == IrType::Void {
                closure_ctx.set_terminator(Terminator::Return(None));
            } else {
                let temp = closure_ctx.add_temp(ret_type);
                closure_ctx.emit(Instruction::Assign {
                    dest: Place::from_temp(temp),
                    value: RValue::Use(Value::Const(Constant::I64(0))),
                });
                closure_ctx.set_terminator(Terminator::Return(Some(Value::Temp(temp))));
            }
        }

        self.pop_scope();
        self.module.add_function(ir_func);

        // Store closure binding
        self.closure_bindings.insert(func_name.clone(), ClosureInfo {
            func_name: func_name.clone(),
            captured_vars,
            env_struct_id,
            env_local,
        });

        // Return the function name as a string constant
        self.module.intern_string(func_name.clone());
        Some(Value::Const(Constant::Str(func_name)))
    }

    /// Lower a function expression
    fn lower_function_expr(
        &mut self,
        ctx: &mut FuncCtx,
        _name: Option<&Node<Ident>>,
        params: &[Param],
        return_type: Option<&Node<Type>>,
        body: &Node<BlockStmt>,
        span: &Span,
    ) -> Option<Value> {
        // Convert to arrow-like body and reuse arrow logic
        let arrow_body = ArrowBody::Block(Box::new(body.clone()));
        self.lower_arrow_expr(ctx, params, return_type, &arrow_body, span)
    }

    /// Lower a closure call: prepend captured variable values to args
    /// Lower a closure call: pass env_ptr as first arg, then actual args
    fn lower_closure_call(
        &mut self,
        ctx: &mut FuncCtx,
        closure_info: &ClosureInfo,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        let mut arg_vals: Vec<Value> = Vec::new();

        // First arg: environment pointer (if closure captures variables)
        if let Some(env_local) = closure_info.env_local {
            arg_vals.push(Value::Local(env_local));
        }

        // Then: add actual arguments
        for arg in args {
            if let Some(val) = self.lower_expr(ctx, &arg.value, &arg.span) {
                arg_vals.push(val);
            } else {
                return None;
            }
        }

        // Look up return type
        let return_type = self.module.find_function(&closure_info.func_name)
            .map(|f| f.return_type.clone())
            .unwrap_or(IrType::Void);

        if return_type == IrType::Void {
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str(closure_info.func_name.clone())),
                args: arg_vals,
            });
            None
        } else {
            let result = ctx.add_temp(return_type);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(result)),
                func: Value::Const(Constant::Str(closure_info.func_name.clone())),
                args: arg_vals,
            });
            Some(Value::Temp(result))
        }
    }

    // =========================================================================
    // Promise chaining methods (.then, .catch, .finally)
    // =========================================================================

    /// Lower promise.then/catch/finally(callback) → runtime function call
    fn lower_promise_chain_method(
        &mut self,
        ctx: &mut FuncCtx,
        promise_info: &VarInfo,
        method: &str,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        if args.is_empty() {
            return None;
        }

        // Lower the callback argument (should be a closure or function reference)
        let callback_arg = &args[0];
        let callback_val = self.lower_expr(ctx, &callback_arg.value, &callback_arg.span)?;

        // Get the closure info for the callback (to pass env pointer)
        let callback_closure_info = match &callback_arg.value {
            Expr::Arrow { .. } | Expr::Function { .. } => {
                let func_name = format!("__closure_{}", self.next_closure_id - 1);
                self.closure_bindings.get(&func_name).cloned()
            }
            Expr::Ident(ident) => {
                self.closure_bindings.get(&ident.name).cloned()
            }
            _ => None,
        };

        // Determine runtime function name
        let runtime_fn = match method {
            "then" => "zaco_promise_then",
            "catch" => "zaco_promise_catch",
            "finally" => "zaco_promise_finally",
            _ => return None,
        };

        // Declare the extern: (promise_ptr, callback_fn_ptr, callback_ctx_ptr) → new_promise_ptr
        self.ensure_extern(
            runtime_fn,
            vec![IrType::Ptr, IrType::Ptr, IrType::Ptr],
            IrType::Ptr,
        );

        // Build args: promise pointer, callback function pointer, callback context (env) pointer
        let promise_val = Value::Local(promise_info.local_id);
        let env_val = callback_closure_info
            .and_then(|ci| ci.env_local.map(|el| Value::Local(el)))
            .unwrap_or(Value::Const(Constant::Null));

        let result_temp = ctx.add_temp(IrType::Ptr);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(result_temp)),
            func: Value::Const(Constant::Str(runtime_fn.to_string())),
            args: vec![promise_val, callback_val, env_val],
        });

        Some(Value::Temp(result_temp))
    }

        // =========================================================================
    // Array callback methods (map, filter, forEach, etc.)
    // =========================================================================

    /// Lower array.map/filter/forEach(callback) — iterates array and calls closure
    fn lower_array_callback_method(
        &mut self,
        ctx: &mut FuncCtx,
        _array_name: &str,
        method: &str,
        array_info: &VarInfo,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Option<Value> {
        if args.is_empty() {
            return None;
        }

        // The first argument should be a closure/callback
        let callback_arg = &args[0];

        // Check if it's an inline arrow or function expr — lower it first
        let callback_closure_info = match &callback_arg.value {
            Expr::Arrow { params, return_type, body, .. } => {
                self.lower_arrow_expr(ctx, params, return_type.as_deref(), body, &callback_arg.span);
                // Get the closure info that was just registered
                let func_name = format!("__closure_{}", self.next_closure_id - 1);
                self.closure_bindings.get(&func_name).cloned()
            }
            Expr::Function { params, return_type, body, .. } => {
                let arrow_body = ArrowBody::Block(Box::new(*body.clone()));
                self.lower_arrow_expr(ctx, params, return_type.as_deref(), &arrow_body, &callback_arg.span);
                let func_name = format!("__closure_{}", self.next_closure_id - 1);
                self.closure_bindings.get(&func_name).cloned()
            }
            Expr::Ident(ident) => {
                // Look up the closure by variable name
                self.closure_bindings.get(&ident.name).cloned()
            }
            _ => None,
        };

        let closure = match callback_closure_info {
            Some(ci) => ci,
            None => return None,
        };

        // Get array length: call zaco_array_len(arr)
        self.ensure_extern("zaco_array_len", vec![IrType::Ptr], IrType::I64);
        let len_temp = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(len_temp)),
            func: Value::Const(Constant::Str("zaco_array_len".to_string())),
            args: vec![Value::Local(array_info.local_id)],
        });

        // For map: allocate result array
        let result_array = if method == "map" {
            self.ensure_extern("zaco_array_new", vec![IrType::I64], IrType::Ptr);
            let arr = ctx.add_local(IrType::Ptr);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_local(arr)),
                func: Value::Const(Constant::Str("zaco_array_new".to_string())),
                args: vec![Value::Temp(len_temp)],
            });
            Some(arr)
        } else if method == "filter" {
            self.ensure_extern("zaco_array_new", vec![IrType::I64], IrType::Ptr);
            let arr = ctx.add_local(IrType::Ptr);
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_local(arr)),
                func: Value::Const(Constant::Str("zaco_array_new".to_string())),
                args: vec![Value::Const(Constant::I64(0))],
            });
            Some(arr)
        } else {
            None
        };

        // Loop: for (let i = 0; i < len; i++) { callback(arr[i], i, arr) }
        let idx_local = ctx.add_local(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(idx_local),
            value: RValue::Use(Value::Const(Constant::I64(0))),
        });

        let loop_header = ctx.new_block();
        let loop_body = ctx.new_block();
        let loop_exit = ctx.new_block();

        ctx.set_terminator(Terminator::Jump(loop_header));
        ctx.switch_to(loop_header);

        // Condition: i < len
        let cond = ctx.add_temp(IrType::Bool);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(cond),
            value: RValue::BinaryOp {
                op: BinOp::Lt,
                left: Value::Local(idx_local),
                right: Value::Temp(len_temp),
            },
        });
        ctx.set_terminator(Terminator::Branch {
            cond: Value::Temp(cond),
            then_block: loop_body,
            else_block: loop_exit,
        });

        ctx.switch_to(loop_body);

        // Get element: arr[i]
        self.ensure_extern("zaco_array_get", vec![IrType::Ptr, IrType::I64], IrType::F64);
        let elem = ctx.add_temp(IrType::F64);
        ctx.emit(Instruction::Call {
            dest: Some(Place::from_temp(elem)),
            func: Value::Const(Constant::Str("zaco_array_get".to_string())),
            args: vec![Value::Local(array_info.local_id), Value::Local(idx_local)],
        });

        // Build callback call args: env_ptr (if any), element, index
        let mut cb_args: Vec<Value> = Vec::new();
        if let Some(env_local) = closure.env_local {
            cb_args.push(Value::Local(env_local));
        }
        cb_args.push(Value::Temp(elem));
        cb_args.push(Value::Local(idx_local));

        // Call callback
        let cb_return_type = self.module.find_function(&closure.func_name)
            .map(|f| f.return_type.clone())
            .unwrap_or(IrType::Void);

        let cb_result = if cb_return_type != IrType::Void {
            let r = ctx.add_temp(cb_return_type.clone());
            ctx.emit(Instruction::Call {
                dest: Some(Place::from_temp(r)),
                func: Value::Const(Constant::Str(closure.func_name.clone())),
                args: cb_args,
            });
            Some(r)
        } else {
            ctx.emit(Instruction::Call {
                dest: None,
                func: Value::Const(Constant::Str(closure.func_name.clone())),
                args: cb_args,
            });
            None
        };

        // For map: push result to result_array
        if method == "map" {
            if let (Some(result_arr), Some(cb_r)) = (result_array, cb_result) {
                self.ensure_extern("zaco_array_push", vec![IrType::Ptr, IrType::F64], IrType::Void);
                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str("zaco_array_push".to_string())),
                    args: vec![Value::Local(result_arr), Value::Temp(cb_r)],
                });
            }
        }

        // For filter: push element if callback returned true
        if method == "filter" {
            if let (Some(result_arr), Some(cb_r)) = (result_array, cb_result) {
                let push_block = ctx.new_block();
                let skip_block = ctx.new_block();
                ctx.set_terminator(Terminator::Branch {
                    cond: Value::Temp(cb_r),
                    then_block: push_block,
                    else_block: skip_block,
                });

                ctx.switch_to(push_block);
                self.ensure_extern("zaco_array_push", vec![IrType::Ptr, IrType::F64], IrType::Void);
                ctx.emit(Instruction::Call {
                    dest: None,
                    func: Value::Const(Constant::Str("zaco_array_push".to_string())),
                    args: vec![Value::Local(result_arr), Value::Temp(elem)],
                });
                ctx.set_terminator(Terminator::Jump(skip_block));

                ctx.switch_to(skip_block);
            }
        }

        // Increment i
        let next_i = ctx.add_temp(IrType::I64);
        ctx.emit(Instruction::Assign {
            dest: Place::from_temp(next_i),
            value: RValue::BinaryOp {
                op: BinOp::Add,
                left: Value::Local(idx_local),
                right: Value::Const(Constant::I64(1)),
            },
        });
        ctx.emit(Instruction::Assign {
            dest: Place::from_local(idx_local),
            value: RValue::Use(Value::Temp(next_i)),
        });

        ctx.set_terminator(Terminator::Jump(loop_header));
        ctx.switch_to(loop_exit);

        // Return result
        match method {
            "map" | "filter" => result_array.map(|arr| Value::Local(arr)),
            _ => None, // forEach returns void
        }
    }

    // =========================================================================
    // Free variable collection
    // =========================================================================

    /// Collect variables referenced in statements that are not locally defined
    /// (i.e., captured from enclosing scope).
    fn collect_captured_vars(&self, stmts: &[Node<Stmt>], local_names: &HashSet<String>) -> Vec<String> {
        let mut captured = Vec::new();
        let mut seen = HashSet::new();
        for stmt in stmts {
            self.collect_free_vars_in_stmt(&stmt.value, local_names, &mut captured, &mut seen);
        }
        captured
    }

    /// Collect captured variables that are mutated (assigned to) inside the closure.
    /// These need capture-by-reference (boxing) so mutations are visible outside.
    fn collect_mutated_captured_vars(&self, stmts: &[Node<Stmt>], local_names: &HashSet<String>) -> HashSet<String> {
        let mut mutated = HashSet::new();
        for stmt in stmts {
            self.collect_mutated_vars_in_stmt(&stmt.value, local_names, &mut mutated);
        }
        mutated
    }

    fn collect_mutated_vars_in_stmt(
        &self,
        stmt: &Stmt,
        local_names: &HashSet<String>,
        mutated: &mut HashSet<String>,
    ) {
        match stmt {
            Stmt::Expr(expr) => self.collect_mutated_vars_in_expr(&expr.value, local_names, mutated),
            Stmt::Return(Some(expr)) => self.collect_mutated_vars_in_expr(&expr.value, local_names, mutated),
            Stmt::VarDecl(vd) => {
                for decl in &vd.declarations {
                    if let Some(ref init) = decl.init {
                        self.collect_mutated_vars_in_expr(&init.value, local_names, mutated);
                    }
                }
            }
            Stmt::If { condition, then_stmt, else_stmt } => {
                self.collect_mutated_vars_in_expr(&condition.value, local_names, mutated);
                self.collect_mutated_vars_in_stmt(&then_stmt.value, local_names, mutated);
                if let Some(else_s) = else_stmt {
                    self.collect_mutated_vars_in_stmt(&else_s.value, local_names, mutated);
                }
            }
            Stmt::Block(block) => {
                for s in &block.stmts {
                    self.collect_mutated_vars_in_stmt(&s.value, local_names, mutated);
                }
            }
            Stmt::For { init, update, body, condition } => {
                if let Some(init_item) = init {
                    if let ForInit::Expr(expr) = init_item {
                        self.collect_mutated_vars_in_expr(&expr.value, local_names, mutated);
                    }
                }
                if let Some(cond) = condition {
                    self.collect_mutated_vars_in_expr(&cond.value, local_names, mutated);
                }
                if let Some(update_expr) = update {
                    self.collect_mutated_vars_in_expr(&update_expr.value, local_names, mutated);
                }
                self.collect_mutated_vars_in_stmt(&body.value, local_names, mutated);
            }
            Stmt::While { body, condition } => {
                self.collect_mutated_vars_in_expr(&condition.value, local_names, mutated);
                self.collect_mutated_vars_in_stmt(&body.value, local_names, mutated);
            }
            _ => {}
        }
    }

    fn collect_mutated_vars_in_expr(
        &self,
        expr: &Expr,
        local_names: &HashSet<String>,
        mutated: &mut HashSet<String>,
    ) {
        match expr {
            Expr::Assignment { target, value, .. } => {
                // Check if the target is a captured variable being mutated
                if let Expr::Ident(ident) = &target.value {
                    let name = &ident.name;
                    if !local_names.contains(name) && self.lookup_var(name).is_some() {
                        mutated.insert(name.clone());
                    }
                }
                self.collect_mutated_vars_in_expr(&value.value, local_names, mutated);
            }
            Expr::Binary { left, right, .. } => {
                self.collect_mutated_vars_in_expr(&left.value, local_names, mutated);
                self.collect_mutated_vars_in_expr(&right.value, local_names, mutated);
            }
            Expr::Unary { expr: operand, .. } => {
                self.collect_mutated_vars_in_expr(&operand.value, local_names, mutated);
            }
            Expr::Call { callee, args, .. } => {
                self.collect_mutated_vars_in_expr(&callee.value, local_names, mutated);
                for arg in args {
                    self.collect_mutated_vars_in_expr(&arg.value, local_names, mutated);
                }
            }
            Expr::Member { object, .. } => {
                self.collect_mutated_vars_in_expr(&object.value, local_names, mutated);
            }
            Expr::Paren(inner) => {
                self.collect_mutated_vars_in_expr(&inner.value, local_names, mutated);
            }
            Expr::Ternary { condition, then_expr, else_expr } => {
                self.collect_mutated_vars_in_expr(&condition.value, local_names, mutated);
                self.collect_mutated_vars_in_expr(&then_expr.value, local_names, mutated);
                self.collect_mutated_vars_in_expr(&else_expr.value, local_names, mutated);
            }
            _ => {}
        }
    }

        fn collect_free_vars_in_stmt(
        &self,
        stmt: &Stmt,
        local_names: &HashSet<String>,
        captured: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        match stmt {
            Stmt::Expr(expr) => self.collect_free_vars_in_expr(&expr.value, local_names, captured, seen),
            Stmt::Return(Some(expr)) => self.collect_free_vars_in_expr(&expr.value, local_names, captured, seen),
            Stmt::VarDecl(vd) => {
                for decl in &vd.declarations {
                    if let Some(ref init) = decl.init {
                        self.collect_free_vars_in_expr(&init.value, local_names, captured, seen);
                    }
                }
            }
            Stmt::If { condition, then_stmt, else_stmt } => {
                self.collect_free_vars_in_expr(&condition.value, local_names, captured, seen);
                self.collect_free_vars_in_stmt(&then_stmt.value, local_names, captured, seen);
                if let Some(else_s) = else_stmt {
                    self.collect_free_vars_in_stmt(&else_s.value, local_names, captured, seen);
                }
            }
            Stmt::Block(block) => {
                for s in &block.stmts {
                    self.collect_free_vars_in_stmt(&s.value, local_names, captured, seen);
                }
            }
            _ => {}
        }
    }

    fn collect_free_vars_in_expr(
        &self,
        expr: &Expr,
        local_names: &HashSet<String>,
        captured: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        match expr {
            Expr::Ident(ident) => {
                let name = &ident.name;
                if !local_names.contains(name) && !seen.contains(name) {
                    // Check if it's a variable in scope (not a global/built-in)
                    if self.lookup_var(name).is_some() {
                        seen.insert(name.clone());
                        captured.push(name.clone());
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_free_vars_in_expr(&left.value, local_names, captured, seen);
                self.collect_free_vars_in_expr(&right.value, local_names, captured, seen);
            }
            Expr::Unary { expr: operand, .. } => {
                self.collect_free_vars_in_expr(&operand.value, local_names, captured, seen);
            }
            Expr::Call { callee, args, .. } => {
                self.collect_free_vars_in_expr(&callee.value, local_names, captured, seen);
                for arg in args {
                    self.collect_free_vars_in_expr(&arg.value, local_names, captured, seen);
                }
            }
            Expr::Member { object, .. } => {
                self.collect_free_vars_in_expr(&object.value, local_names, captured, seen);
            }
            Expr::Paren(inner) => {
                self.collect_free_vars_in_expr(&inner.value, local_names, captured, seen);
            }
            Expr::Assignment { target, value, .. } => {
                self.collect_free_vars_in_expr(&target.value, local_names, captured, seen);
                self.collect_free_vars_in_expr(&value.value, local_names, captured, seen);
            }
            Expr::Ternary { condition, then_expr, else_expr } => {
                self.collect_free_vars_in_expr(&condition.value, local_names, captured, seen);
                self.collect_free_vars_in_expr(&then_expr.value, local_names, captured, seen);
                self.collect_free_vars_in_expr(&else_expr.value, local_names, captured, seen);
            }
            Expr::Template { exprs, .. } => {
                for e in exprs {
                    self.collect_free_vars_in_expr(&e.value, local_names, captured, seen);
                }
            }
            Expr::Array(elements) => {
                for elem in elements.iter().flatten() {
                    self.collect_free_vars_in_expr(&elem.value, local_names, captured, seen);
                }
            }
            Expr::Object(props) => {
                for prop in props {
                    match prop {
                        ObjectProperty::Property { value, .. } => {
                            self.collect_free_vars_in_expr(&value.value, local_names, captured, seen);
                        }
                        ObjectProperty::Spread(expr) => {
                            self.collect_free_vars_in_expr(&expr.value, local_names, captured, seen);
                        }
                        ObjectProperty::Method { .. } => {}
                    }
                }
            }
            Expr::TaggedTemplate { tag, exprs, .. } => {
                self.collect_free_vars_in_expr(&tag.value, local_names, captured, seen);
                for e in exprs {
                    self.collect_free_vars_in_expr(&e.value, local_names, captured, seen);
                }
            }
            Expr::Yield { argument, .. } => {
                if let Some(arg) = argument {
                    self.collect_free_vars_in_expr(&arg.value, local_names, captured, seen);
                }
            }
            _ => {}
        }
    }

    // =========================================================================
    // Type inference helpers
    // =========================================================================

    fn infer_expr_type(&self, expr: &Expr) -> IrType {
        match expr {
            Expr::Literal(Literal::Number(_)) => IrType::F64,
            Expr::Literal(Literal::String(_)) => IrType::Str,
            Expr::Literal(Literal::Boolean(_)) => IrType::Bool,
            Expr::Literal(Literal::Null | Literal::Undefined) => IrType::Ptr,
            Expr::Template { .. } => IrType::Str,
            Expr::TaggedTemplate { .. } => IrType::Ptr,
            Expr::Yield { .. } => IrType::Ptr,
            Expr::Binary { left, op, .. } => {
                if matches!(
                    op,
                    BinaryOp::Eq
                        | BinaryOp::NotEq
                        | BinaryOp::StrictEq
                        | BinaryOp::StrictNotEq
                        | BinaryOp::Lt
                        | BinaryOp::LtEq
                        | BinaryOp::Gt
                        | BinaryOp::GtEq
                        | BinaryOp::In
                        | BinaryOp::InstanceOf
                ) {
                    IrType::Bool
                } else {
                    // For && and ||, the result type is the operand type
                    // (they return one of the operands, not a boolean)
                    self.infer_expr_type(&left.value)
                }
            }
            Expr::Ident(ident) => {
                // __dirname and __filename are always strings
                if ident.name == "__dirname" || ident.name == "__filename" {
                    return IrType::Str;
                }
                if let Some(info) = self.lookup_var(&ident.name) {
                    info.ir_type.clone()
                } else {
                    IrType::F64 // default: TypeScript number is f64
                }
            }
            Expr::Paren(inner) => self.infer_expr_type(&inner.value),
            Expr::Array(_) => IrType::Array(Box::new(IrType::F64)),
            Expr::Object(_) => IrType::Ptr,
            Expr::Call { callee, .. } => {
                // Infer return type from known built-in calls
                if let Expr::Member { object, property, .. } = &callee.value {
                    if let Expr::Ident(obj_ident) = &object.value {
                        match obj_ident.name.as_str() {
                            "Math" => IrType::F64, // All Math methods return f64
                            "JSON" => IrType::Str, // JSON.parse/stringify return strings
                            _ if {
                                // Check if it's a Promise method call
                                if let Some(info) = self.lookup_var(&obj_ident.name) {
                                    matches!(info.ir_type, IrType::Promise(_))
                                        && matches!(property.value.name.as_str(), "then" | "catch" | "finally")
                                } else {
                                    false
                                }
                            } => IrType::Ptr, // Promise chain methods return a new promise (Ptr)
                            _ => {
                                // Check if it's a method call on a class instance
                                if let Some(info) = self.lookup_var(&obj_ident.name) {
                                    if let IrType::Struct(struct_id) = &info.ir_type {
                                        // Find the class for this struct
                                        if let Some((class_name, _)) = self.class_info.iter()
                                            .find(|(_, ci)| ci.struct_id == *struct_id)
                                        {
                                            let method_func_name = format!("{}_{}", class_name, property.value.name);
                                            if let Some(func) = self.module.find_function(&method_func_name) {
                                                return func.return_type.clone();
                                            }
                                        }
                                    }
                                }
                                IrType::F64
                            }
                        }
                    } else {
                        IrType::F64
                    }
                } else if let Expr::Ident(func_ident) = &callee.value {
                    // Look up user-defined function return type
                    // Handle renamed user main
                    let lookup_name = if func_ident.name == "main" && self.has_user_main {
                        "_user_main".to_string()
                    } else {
                        func_ident.name.clone()
                    };
                    self.module.find_function(&lookup_name)
                        .map(|f| f.return_type.clone())
                        .or_else(|| {
                            // Check if this is a recursive call to the current function
                            if let Some((ref cur_name, ref cur_ret)) = self.current_function {
                                if *cur_name == lookup_name {
                                    return Some(cur_ret.clone());
                                }
                            }
                            None
                        })
                        .or_else(|| {
                            // Check if this is an imported function call
                            if let Some(module) = self.imported_bindings.get(&func_ident.name) {
                                if let Some((_, _, ret_type)) = Self::imported_func_signature(module, &func_ident.name) {
                                    return Some(ret_type);
                                }
                            }
                            None
                        })
                        .unwrap_or(IrType::F64)
                } else {
                    IrType::F64
                }
            }
            Expr::Member { object, property, .. } => {
                // Infer type of member access (e.g., Math.PI)
                if let Expr::Ident(obj_ident) = &object.value {
                    match (obj_ident.name.as_str(), property.value.name.as_str()) {
                        ("Math", "PI" | "E") => IrType::F64,
                        ("process", "pid") => IrType::I64,
                        ("process", _) => IrType::Str,
                        _ => {
                            // Check if it's a static property on a class
                            if let Some(ci) = self.class_info.get(&obj_ident.name) {
                                if let Some((_, ty)) = ci.static_properties.iter()
                                    .find(|(n, _)| n == &property.value.name)
                                {
                                    return ty.clone();
                                }
                            }
                            // Check if it's a class instance field access
                            if let Some(info) = self.lookup_var(&obj_ident.name) {
                                if let IrType::Struct(struct_id) = &info.ir_type {
                                    if let Some((_, ci)) = self.class_info.iter()
                                        .find(|(_, ci)| ci.struct_id == *struct_id)
                                    {
                                        // Check getters first
                                        if ci.getters.contains(&property.value.name) {
                                            let getter_func = format!("{}_get_{}", ci.struct_id.0, property.value.name);
                                            if let Some(func) = self.module.find_function(&getter_func) {
                                                return func.return_type.clone();
                                            }
                                        }
                                        if let Some((_, ty)) = ci.fields.iter()
                                            .find(|(n, _)| n == &property.value.name)
                                        {
                                            return ty.clone();
                                        }
                                    }
                                }
                            }
                            IrType::F64
                        }
                    }
                } else if matches!(&object.value, Expr::This) {
                    // this.field — look up field type from current class
                    if let Some(class_name) = &self.current_class {
                        if let Some(ci) = self.class_info.get(class_name) {
                            if let Some((_, ty)) = ci.fields.iter()
                                .find(|(n, _)| n == &property.value.name)
                            {
                                return ty.clone();
                            }
                        }
                    }
                    IrType::F64
                } else {
                    IrType::F64
                }
            }
            Expr::New { callee, .. } => {
                // new ClassName() returns a class instance (struct pointer)
                if let Expr::Ident(ident) = &callee.value {
                    if let Some(ci) = self.class_info.get(&ident.name) {
                        return IrType::Struct(ci.struct_id);
                    }
                }
                IrType::Ptr
            }
            Expr::This => {
                // this refers to the current class instance
                if let Some(ref info) = self.this_var {
                    info.ir_type.clone()
                } else {
                    IrType::Ptr
                }
            }
            Expr::Arrow { .. } => {
                // Arrow function — type is a function reference stored as Ptr
                IrType::Ptr
            }
            Expr::Function { .. } => IrType::Ptr,
            Expr::Unary { op, expr: operand } => {
                match op {
                    UnaryOp::Not | UnaryOp::Delete => IrType::Bool,
                    UnaryOp::Void => IrType::Ptr,
                    _ => self.infer_expr_type(&operand.value),
                }
            }
            Expr::Ternary { then_expr, .. } => {
                // Result type is the type of the then branch
                self.infer_expr_type(&then_expr.value)
            }
            Expr::OptionalMember { object, property } => {
                self.infer_expr_type(&Expr::Member {
                    object: object.clone(),
                    property: property.clone(),
                    computed: false,
                })
            }
            Expr::OptionalCall { callee, args, .. } => {
                self.infer_expr_type(&Expr::Call {
                    callee: callee.clone(),
                    type_args: None,
                    args: args.clone(),
                })
            }
            Expr::OptionalIndex { object, .. } => {
                let obj_ty = self.infer_expr_type(&object.value);
                if let IrType::Array(elem) = obj_ty {
                    *elem
                } else {
                    IrType::F64
                }
            }
            _ => IrType::F64, // conservative default: TypeScript number is f64
        }
    }

    fn infer_param_type(&self, param: &Param) -> IrType {
        // Check Param-level type annotation first
        if let Some(ref ty) = param.type_annotation {
            return self.ast_type_to_ir(&ty.value);
        }
        // Fall back to Pattern::Ident-level type annotation
        // (the parser may place the annotation on the pattern instead of the param)
        if let Pattern::Ident { type_annotation: Some(ref ty), .. } = param.pattern.value {
            return self.ast_type_to_ir(&ty.value);
        }
        IrType::F64 // default: TypeScript number is f64
    }

    fn ast_type_to_ir(&self, ty: &Type) -> IrType {
        match ty {
            Type::Primitive(PrimitiveType::Number) => IrType::F64,
            Type::Primitive(PrimitiveType::String) => IrType::Str,
            Type::Primitive(PrimitiveType::Boolean) => IrType::Bool,
            Type::Primitive(PrimitiveType::Void) => IrType::Void,
            Type::Primitive(PrimitiveType::Null | PrimitiveType::Undefined) => IrType::Ptr,
            Type::Primitive(PrimitiveType::Any | PrimitiveType::Unknown) => IrType::Ptr,
            Type::Primitive(PrimitiveType::Never) => IrType::Void,
            Type::Array(elem) => IrType::Array(Box::new(self.ast_type_to_ir(&elem.value))),
            Type::Generic { base, type_args } => {
                // Check if this is Promise<T>
                if let Type::TypeRef { name, .. } = &base.value {
                    if name.value.name == "Promise" && !type_args.is_empty() {
                        let inner_type = self.ast_type_to_ir(&type_args[0].value);
                        return IrType::Promise(Box::new(inner_type));
                    }
                }
                IrType::Ptr
            }
            Type::TypeRef { name, type_args } => {
                // Try to resolve known types
                match name.value.name.as_str() {
                    "number" => IrType::F64,
                    "string" => IrType::Str,
                    "boolean" => IrType::Bool,
                    "void" => IrType::Void,
                    "Promise" => {
                        // Promise without type args → Promise<any>
                        if let Some(args) = type_args {
                            if !args.is_empty() {
                                let inner_type = self.ast_type_to_ir(&args[0].value);
                                return IrType::Promise(Box::new(inner_type));
                            }
                        }
                        IrType::Promise(Box::new(IrType::Ptr))
                    }
                    _ => {
                        // Check if this is a known class name
                        if let Some(ci) = self.class_info.get(name.value.name.as_str()) {
                            IrType::Struct(ci.struct_id)
                        } else {
                            IrType::Ptr // Unknown types → pointer
                        }
                    }
                }
            }
            _ => IrType::Ptr,
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    fn make_program(items: Vec<Node<ModuleItem>>) -> Program {
        Program {
            items,
            span: dummy_span(),
        }
    }

    fn make_stmt_item(stmt: Stmt) -> Node<ModuleItem> {
        Node::new(
            ModuleItem::Stmt(Node::new(stmt, dummy_span())),
            dummy_span(),
        )
    }

    fn make_decl_item(decl: Decl) -> Node<ModuleItem> {
        Node::new(
            ModuleItem::Decl(Node::new(decl, dummy_span())),
            dummy_span(),
        )
    }

    #[test]
    fn test_lower_hello_world() {
        // console.log("Hello, World!")
        let call_expr = Expr::Call {
            callee: Box::new(Node::new(
                Expr::Member {
                    object: Box::new(Node::new(
                        Expr::Ident(Ident::new("console")),
                        dummy_span(),
                    )),
                    property: Node::new(Ident::new("log"), dummy_span()),
                    computed: false,
                },
                dummy_span(),
            )),
            type_args: None,
            args: vec![Node::new(
                Expr::Literal(Literal::String("Hello, World!".to_string())),
                dummy_span(),
            )],
        };

        let program = make_program(vec![make_stmt_item(Stmt::Expr(Node::new(
            call_expr,
            dummy_span(),
        )))]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
        // Should have some instructions (print calls + return)
        let entry_block = &module.functions[0].blocks[0];
        assert!(!entry_block.instructions.is_empty());
    }

    #[test]
    fn test_lower_variable_decl() {
        // let x: number = 42;
        let program = make_program(vec![make_decl_item(Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("x"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                init: Some(Node::new(
                    Expr::Literal(Literal::Number(42.0)),
                    dummy_span(),
                )),
            }],
        }))]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        let main = &module.functions[0];
        // Should have at least one local variable (beyond the return temp)
        assert!(!main.locals.is_empty());
    }

    #[test]
    fn test_lower_function_decl() {
        // function add(a: number, b: number): number { return a + b; }
        let func_decl = Decl::Function(FunctionDecl {
            name: Node::new(Ident::new("add"), dummy_span()),
            type_params: None,
            params: vec![
                Param {
                    pattern: Node::new(
                        Pattern::Ident {
                            name: Node::new(Ident::new("a"), dummy_span()),
                            type_annotation: None,
                            ownership: None,
                        },
                        dummy_span(),
                    ),
                    type_annotation: Some(Box::new(Node::new(
                        Type::Primitive(PrimitiveType::Number),
                        dummy_span(),
                    ))),
                    ownership: None,
                    optional: false,
                    is_rest: false,
                },
                Param {
                    pattern: Node::new(
                        Pattern::Ident {
                            name: Node::new(Ident::new("b"), dummy_span()),
                            type_annotation: None,
                            ownership: None,
                        },
                        dummy_span(),
                    ),
                    type_annotation: Some(Box::new(Node::new(
                        Type::Primitive(PrimitiveType::Number),
                        dummy_span(),
                    ))),
                    ownership: None,
                    optional: false,
                    is_rest: false,
                },
            ],
            return_type: Some(Box::new(Node::new(
                Type::Primitive(PrimitiveType::Number),
                dummy_span(),
            ))),
            body: Some(Node::new(
                BlockStmt {
                    stmts: vec![Node::new(
                        Stmt::Return(Some(Node::new(
                            Expr::Binary {
                                left: Box::new(Node::new(
                                    Expr::Ident(Ident::new("a")),
                                    dummy_span(),
                                )),
                                op: BinaryOp::Add,
                                right: Box::new(Node::new(
                                    Expr::Ident(Ident::new("b")),
                                    dummy_span(),
                                )),
                            },
                            dummy_span(),
                        ))),
                        dummy_span(),
                    )],
                },
                dummy_span(),
            )),
            is_async: false,
            is_generator: false,
            is_declare: false,
        });

        let program = make_program(vec![make_decl_item(func_decl)]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        // Should have 2 functions: add (added during lowering) + main (added at end)
        assert_eq!(module.functions.len(), 2);
        let add_fn = module.find_function("add").expect("add function not found");
        assert_eq!(add_fn.params.len(), 2);
        assert!(module.find_function("main").is_some());
    }

    #[test]
    fn test_lower_math_floor() {
        // let x = Math.floor(3.7);
        let call_expr = Expr::Call {
            callee: Box::new(Node::new(
                Expr::Member {
                    object: Box::new(Node::new(
                        Expr::Ident(Ident::new("Math")),
                        dummy_span(),
                    )),
                    property: Node::new(Ident::new("floor"), dummy_span()),
                    computed: false,
                },
                dummy_span(),
            )),
            type_args: None,
            args: vec![Node::new(
                Expr::Literal(Literal::Number(3.7)),
                dummy_span(),
            )],
        };

        let var_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("x"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                init: Some(Node::new(call_expr, dummy_span())),
            }],
        });

        let program = make_program(vec![make_decl_item(var_decl)]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        // Should have extern function for zaco_math_floor
        assert!(module.extern_functions.iter().any(|f| f.name == "zaco_math_floor"));
    }

    #[test]
    fn test_lower_math_pi() {
        // let pi = Math.PI;
        let member_expr = Expr::Member {
            object: Box::new(Node::new(
                Expr::Ident(Ident::new("Math")),
                dummy_span(),
            )),
            property: Node::new(Ident::new("PI"), dummy_span()),
            computed: false,
        };

        let var_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("pi"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                init: Some(Node::new(member_expr, dummy_span())),
            }],
        });

        let program = make_program(vec![make_decl_item(var_decl)]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());
        // Math.PI should be lowered to a constant value
    }

    #[test]
    fn test_lower_import_fs() {
        // import { readFileSync } from "fs";
        // let data = readFileSync("test.txt", "utf-8");
        let import_item = ModuleItem::Import(ImportDecl {
            specifiers: vec![ImportSpecifier::Named {
                imported: Node::new(Ident::new("readFileSync"), dummy_span()),
                local: None,
                type_only: false,
            }],
            source: "fs".to_string(),
            type_only: false,
        });

        let call_expr = Expr::Call {
            callee: Box::new(Node::new(
                Expr::Ident(Ident::new("readFileSync")),
                dummy_span(),
            )),
            type_args: None,
            args: vec![
                Node::new(Expr::Literal(Literal::String("test.txt".to_string())), dummy_span()),
                Node::new(Expr::Literal(Literal::String("utf-8".to_string())), dummy_span()),
            ],
        };

        let var_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("data"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                init: Some(Node::new(call_expr, dummy_span())),
            }],
        });

        let program = make_program(vec![
            Node::new(import_item, dummy_span()),
            make_decl_item(var_decl),
        ]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        // Should have extern function for zaco_fs_read_file_sync
        assert!(module.extern_functions.iter().any(|f| f.name == "zaco_fs_read_file_sync"));
    }

    #[test]
    fn test_lower_export_function() {
        // export function greet(name: string): string { return "Hello " + name; }
        let func_decl = FunctionDecl {
            name: Node::new(Ident::new("greet"), dummy_span()),
            type_params: None,
            params: vec![Param {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("name"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                type_annotation: Some(Box::new(Node::new(
                    Type::Primitive(PrimitiveType::String),
                    dummy_span(),
                ))),
                ownership: None,
                optional: false,
                is_rest: false,
            }],
            return_type: Some(Box::new(Node::new(
                Type::Primitive(PrimitiveType::String),
                dummy_span(),
            ))),
            body: Some(Node::new(
                BlockStmt {
                    stmts: vec![Node::new(
                        Stmt::Return(Some(Node::new(
                            Expr::Binary {
                                left: Box::new(Node::new(
                                    Expr::Literal(Literal::String("Hello ".to_string())),
                                    dummy_span(),
                                )),
                                op: BinaryOp::Add,
                                right: Box::new(Node::new(
                                    Expr::Ident(Ident::new("name")),
                                    dummy_span(),
                                )),
                            },
                            dummy_span(),
                        ))),
                        dummy_span(),
                    )],
                },
                dummy_span(),
            )),
            is_async: false,
            is_generator: false,
            is_declare: false,
        };

        let export_item = ModuleItem::Export(ExportDecl::Decl(Box::new(Node::new(
            Decl::Function(func_decl),
            dummy_span(),
        ))));

        let program = make_program(vec![Node::new(export_item, dummy_span())]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        // Should have greet function marked as public
        let greet_fn = module.find_function("greet").expect("greet function not found");
        assert!(greet_fn.is_public);
    }

    #[test]
    fn test_lower_async_function() {
        // async function fetchData(): Promise<string> { return "data"; }
        let func_decl = Decl::Function(FunctionDecl {
            name: Node::new(Ident::new("fetchData"), dummy_span()),
            type_params: None,
            params: vec![],
            return_type: Some(Box::new(Node::new(
                Type::TypeRef {
                    name: Node::new(Ident::new("Promise"), dummy_span()),
                    type_args: Some(vec![Node::new(
                        Type::Primitive(PrimitiveType::String),
                        dummy_span(),
                    )]),
                },
                dummy_span(),
            ))),
            body: Some(Node::new(
                BlockStmt {
                    stmts: vec![Node::new(
                        Stmt::Return(Some(Node::new(
                            Expr::Literal(Literal::String("data".to_string())),
                            dummy_span(),
                        ))),
                        dummy_span(),
                    )],
                },
                dummy_span(),
            )),
            is_async: true,
            is_generator: false,
            is_declare: false,
        });

        let program = make_program(vec![make_decl_item(func_decl)]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        // Should have fetchData function + main function
        assert_eq!(module.functions.len(), 2);
        let fetch_fn = module.find_function("fetchData").expect("fetchData function not found");

        // Check return type is Promise<string>
        assert_eq!(fetch_fn.return_type, IrType::Promise(Box::new(IrType::Str)));

        // Should have extern functions for promise operations
        assert!(module.extern_functions.iter().any(|f| f.name == "zaco_promise_new"));
        assert!(module.extern_functions.iter().any(|f| f.name == "zaco_promise_resolve"));
    }

    #[test]
    fn test_lower_await_expression() {
        // Test a simpler case: await on a variable that holds a promise
        // async function main() { let p = promise; let result = await p; }
        let var_decl1 = VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("p"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                init: Some(Node::new(
                    Expr::Literal(Literal::String("promise".to_string())),
                    dummy_span(),
                )),
            }],
        };

        let var_decl2 = VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(
                    Pattern::Ident {
                        name: Node::new(Ident::new("result"), dummy_span()),
                        type_annotation: None,
                        ownership: None,
                    },
                    dummy_span(),
                ),
                init: Some(Node::new(
                    Expr::Await(Box::new(Node::new(
                        Expr::Ident(Ident::new("p")),
                        dummy_span(),
                    ))),
                    dummy_span(),
                )),
            }],
        };

        let async_main = Decl::Function(FunctionDecl {
            name: Node::new(Ident::new("asyncMain"), dummy_span()),
            type_params: None,
            params: vec![],
            return_type: Some(Box::new(Node::new(
                Type::TypeRef {
                    name: Node::new(Ident::new("Promise"), dummy_span()),
                    type_args: Some(vec![Node::new(
                        Type::Primitive(PrimitiveType::Void),
                        dummy_span(),
                    )]),
                },
                dummy_span(),
            ))),
            body: Some(Node::new(
                BlockStmt {
                    stmts: vec![
                        Node::new(Stmt::VarDecl(var_decl1), dummy_span()),
                        Node::new(Stmt::VarDecl(var_decl2), dummy_span()),
                    ],
                },
                dummy_span(),
            )),
            is_async: true,
            is_generator: false,
            is_declare: false,
        });

        let program = make_program(vec![make_decl_item(async_main)]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok());

        let module = result.unwrap();
        // Should have asyncMain + main functions
        assert!(module.find_function("asyncMain").is_some());

        // Should have extern function for zaco_async_block_on (called in await)
        assert!(
            module.extern_functions.iter().any(|f| f.name == "zaco_async_block_on"),
            "zaco_async_block_on should be in extern functions"
        );
    }

    #[test]
    fn test_promise_type_conversion() {
        let lowerer = Lowerer::new();

        // Test Promise<string>
        let promise_str = Type::TypeRef {
            name: Node::new(Ident::new("Promise"), dummy_span()),
            type_args: Some(vec![Node::new(
                Type::Primitive(PrimitiveType::String),
                dummy_span(),
            )]),
        };
        assert_eq!(
            lowerer.ast_type_to_ir(&promise_str),
            IrType::Promise(Box::new(IrType::Str))
        );

        // Test Promise<number>
        let promise_num = Type::TypeRef {
            name: Node::new(Ident::new("Promise"), dummy_span()),
            type_args: Some(vec![Node::new(
                Type::Primitive(PrimitiveType::Number),
                dummy_span(),
            )]),
        };
        assert_eq!(
            lowerer.ast_type_to_ir(&promise_num),
            IrType::Promise(Box::new(IrType::F64))
        );
    }

    #[test]
    fn test_lower_switch_basic() {
        let program = make_program(vec![
            make_decl_item(Decl::Var(VarDecl {
                kind: VarDeclKind::Let,
                declarations: vec![VarDeclarator {
                    pattern: Node::new(Pattern::Ident {
                        name: Node::new(Ident::new("x"), dummy_span()),
                        type_annotation: None, ownership: None,
                    }, dummy_span()),
                    init: Some(Node::new(Expr::Literal(Literal::Number(1.0)), dummy_span())),
                }],
            })),
            make_stmt_item(Stmt::Switch {
                discriminant: Node::new(Expr::Ident(Ident::new("x")), dummy_span()),
                cases: vec![
                    SwitchCase {
                        test: Some(Node::new(Expr::Literal(Literal::Number(1.0)), dummy_span())),
                        consequent: vec![Node::new(Stmt::Break(None), dummy_span())],
                    },
                    SwitchCase { test: None, consequent: vec![] },
                ],
            }),
        ]);
        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok(), "Switch lowering failed: {:?}", result.err());
        assert!(result.unwrap().functions[0].blocks.len() >= 4);
    }

    #[test]
    fn test_lower_for_in() {
        let program = make_program(vec![
            make_decl_item(Decl::Var(VarDecl {
                kind: VarDeclKind::Let,
                declarations: vec![VarDeclarator {
                    pattern: Node::new(Pattern::Ident {
                        name: Node::new(Ident::new("arr"), dummy_span()),
                        type_annotation: None, ownership: None,
                    }, dummy_span()),
                    init: Some(Node::new(Expr::Array(vec![
                        Some(Node::new(Expr::Literal(Literal::Number(1.0)), dummy_span())),
                    ]), dummy_span())),
                }],
            })),
            make_stmt_item(Stmt::ForIn {
                left: ForInLeft::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: Node::new(Pattern::Ident {
                            name: Node::new(Ident::new("i"), dummy_span()),
                            type_annotation: None, ownership: None,
                        }, dummy_span()),
                        init: None,
                    }],
                }),
                right: Node::new(Expr::Ident(Ident::new("arr")), dummy_span()),
                body: Box::new(Node::new(Stmt::Block(BlockStmt { stmts: vec![] }), dummy_span())),
            }),
        ]);
        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok(), "For-in lowering failed: {:?}", result.err());
        assert!(result.unwrap().extern_functions.iter().any(|f| f.name == "zaco_array_length"));
    }

    #[test]
    fn test_lower_for_of() {
        let program = make_program(vec![
            make_decl_item(Decl::Var(VarDecl {
                kind: VarDeclKind::Let,
                declarations: vec![VarDeclarator {
                    pattern: Node::new(Pattern::Ident {
                        name: Node::new(Ident::new("arr"), dummy_span()),
                        type_annotation: None, ownership: None,
                    }, dummy_span()),
                    init: Some(Node::new(Expr::Array(vec![
                        Some(Node::new(Expr::Literal(Literal::Number(10.0)), dummy_span())),
                    ]), dummy_span())),
                }],
            })),
            make_stmt_item(Stmt::ForOf {
                left: ForInLeft::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: Node::new(Pattern::Ident {
                            name: Node::new(Ident::new("val"), dummy_span()),
                            type_annotation: None, ownership: None,
                        }, dummy_span()),
                        init: None,
                    }],
                }),
                right: Node::new(Expr::Ident(Ident::new("arr")), dummy_span()),
                body: Box::new(Node::new(Stmt::Block(BlockStmt { stmts: vec![] }), dummy_span())),
                is_await: false,
            }),
        ]);
        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok(), "For-of lowering failed: {:?}", result.err());
        let module = result.unwrap();
        assert!(module.extern_functions.iter().any(|f| f.name == "zaco_array_length"));
        assert!(module.extern_functions.iter().any(|f| f.name == "zaco_array_get_f64"));
    }

    #[test]
    fn test_closure_capture_by_reference() {
        // let counter = 0;
        // const inc = () => { counter = counter + 1; };
        // inc();
        // console.log(counter);
        //
        // The closure mutates `counter`, so it should be boxed (capture by reference).
        let var_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(Pattern::Ident {
                    name: Node::new(Ident::new("counter"), dummy_span()),
                    type_annotation: None, ownership: None,
                }, dummy_span()),
                init: Some(Node::new(Expr::Literal(Literal::Number(0.0)), dummy_span())),
            }],
        });

        // Arrow function: () => { counter = counter + 1; }
        let closure_body = ArrowBody::Block(Box::new(Node::new(BlockStmt {
            stmts: vec![Node::new(
                Stmt::Expr(Node::new(
                    Expr::Assignment {
                        target: Box::new(Node::new(
                            Expr::Ident(Ident::new("counter")),
                            dummy_span(),
                        )),
                        op: AssignmentOp::Assign,
                        value: Box::new(Node::new(
                            Expr::Binary {
                                left: Box::new(Node::new(
                                    Expr::Ident(Ident::new("counter")),
                                    dummy_span(),
                                )),
                                op: BinaryOp::Add,
                                right: Box::new(Node::new(
                                    Expr::Literal(Literal::Number(1.0)),
                                    dummy_span(),
                                )),
                            },
                            dummy_span(),
                        )),
                    },
                    dummy_span(),
                )),
                dummy_span(),
            )],
        }, dummy_span())));

        let inc_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Const,
            declarations: vec![VarDeclarator {
                pattern: Node::new(Pattern::Ident {
                    name: Node::new(Ident::new("inc"), dummy_span()),
                    type_annotation: None, ownership: None,
                }, dummy_span()),
                init: Some(Node::new(
                    Expr::Arrow {
                        type_params: None,
                        params: vec![],
                        return_type: None,
                        body: closure_body,
                    },
                    dummy_span(),
                )),
            }],
        });

        let program = make_program(vec![
            make_decl_item(var_decl),
            make_decl_item(inc_decl),
        ]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok(), "Closure capture by reference failed: {:?}", result.err());

        let module = result.unwrap();

        // Should have extern functions for boxing
        assert!(
            module.extern_functions.iter().any(|f| f.name == "zaco_box_new"),
            "zaco_box_new should be declared for mutable capture boxing"
        );

        // Should have the closure function
        assert!(
            module.find_function("__closure_0").is_some(),
            "Closure function __closure_0 should exist"
        );
    }

    #[test]
    fn test_closure_no_boxing_for_read_only_capture() {
        // let x = 42;
        // const getX = () => x;
        //
        // The closure only reads `x`, so no boxing should occur.
        let var_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(Pattern::Ident {
                    name: Node::new(Ident::new("x"), dummy_span()),
                    type_annotation: None, ownership: None,
                }, dummy_span()),
                init: Some(Node::new(Expr::Literal(Literal::Number(42.0)), dummy_span())),
            }],
        });

        let get_x_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Const,
            declarations: vec![VarDeclarator {
                pattern: Node::new(Pattern::Ident {
                    name: Node::new(Ident::new("getX"), dummy_span()),
                    type_annotation: None, ownership: None,
                }, dummy_span()),
                init: Some(Node::new(
                    Expr::Arrow {
                        type_params: None,
                        params: vec![],
                        return_type: None,
                        body: ArrowBody::Expr(Box::new(Node::new(
                            Expr::Ident(Ident::new("x")),
                            dummy_span(),
                        ))),
                    },
                    dummy_span(),
                )),
            }],
        });

        let program = make_program(vec![
            make_decl_item(var_decl),
            make_decl_item(get_x_decl),
        ]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok(), "Read-only closure capture failed: {:?}", result.err());

        let module = result.unwrap();

        // Should NOT have box extern functions (no mutable captures)
        assert!(
            !module.extern_functions.iter().any(|f| f.name == "zaco_box_new"),
            "zaco_box_new should NOT be declared for read-only captures"
        );
    }

    #[test]
    fn test_promise_then_chaining() {
        // let p: Promise<string> = fetchData();
        // p.then((val) => { console.log(val); });
        //
        // First, declare fetchData as an async function so p has Promise type
        let fetch_decl = Decl::Function(FunctionDecl {
            name: Node::new(Ident::new("fetchData"), dummy_span()),
            type_params: None,
            params: vec![],
            return_type: Some(Box::new(Node::new(
                Type::TypeRef {
                    name: Node::new(Ident::new("Promise"), dummy_span()),
                    type_args: Some(vec![Node::new(
                        Type::Primitive(PrimitiveType::String),
                        dummy_span(),
                    )]),
                },
                dummy_span(),
            ))),
            body: Some(Node::new(BlockStmt {
                stmts: vec![Node::new(
                    Stmt::Return(Some(Node::new(
                        Expr::Literal(Literal::String("data".to_string())),
                        dummy_span(),
                    ))),
                    dummy_span(),
                )],
            }, dummy_span())),
            is_async: true,
            is_generator: false,
            is_declare: false,
        });

        // let p = fetchData();
        let p_decl = Decl::Var(VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![VarDeclarator {
                pattern: Node::new(Pattern::Ident {
                    name: Node::new(Ident::new("p"), dummy_span()),
                    type_annotation: Some(Box::new(Node::new(
                        Type::TypeRef {
                            name: Node::new(Ident::new("Promise"), dummy_span()),
                            type_args: Some(vec![Node::new(
                                Type::Primitive(PrimitiveType::String),
                                dummy_span(),
                            )]),
                        },
                        dummy_span(),
                    ))),
                    ownership: None,
                }, dummy_span()),
                init: Some(Node::new(
                    Expr::Call {
                        callee: Box::new(Node::new(
                            Expr::Ident(Ident::new("fetchData")),
                            dummy_span(),
                        )),
                        type_args: None,
                        args: vec![],
                    },
                    dummy_span(),
                )),
            }],
        });

        // p.then((val: string) => { })
        let then_call = Stmt::Expr(Node::new(
            Expr::Call {
                callee: Box::new(Node::new(
                    Expr::Member {
                        object: Box::new(Node::new(
                            Expr::Ident(Ident::new("p")),
                            dummy_span(),
                        )),
                        property: Node::new(Ident::new("then"), dummy_span()),
                        computed: false,
                    },
                    dummy_span(),
                )),
                type_args: None,
                args: vec![Node::new(
                    Expr::Arrow {
                        type_params: None,
                        params: vec![Param {
                            pattern: Node::new(
                                Pattern::Ident {
                                    name: Node::new(Ident::new("val"), dummy_span()),
                                    type_annotation: None,
                                    ownership: None,
                                },
                                dummy_span(),
                            ),
                            type_annotation: Some(Box::new(Node::new(
                                Type::Primitive(PrimitiveType::String),
                                dummy_span(),
                            ))),
                            ownership: None,
                            optional: false,
                            is_rest: false,
                        }],
                        return_type: None,
                        body: ArrowBody::Block(Box::new(Node::new(
                            BlockStmt { stmts: vec![] },
                            dummy_span(),
                        ))),
                    },
                    dummy_span(),
                )],
            },
            dummy_span(),
        ));

        let program = make_program(vec![
            make_decl_item(fetch_decl),
            make_decl_item(p_decl),
            make_stmt_item(then_call),
        ]);

        let lowerer = Lowerer::new();
        let result = lowerer.lower_program(&program);
        assert!(result.is_ok(), "Promise.then chaining failed: {:?}", result.err());

        let module = result.unwrap();

        // Should have extern function for zaco_promise_then
        assert!(
            module.extern_functions.iter().any(|f| f.name == "zaco_promise_then"),
            "zaco_promise_then should be declared"
        );
    }

}
