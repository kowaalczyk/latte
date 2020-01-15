use std::cmp::max;

use crate::backend::context::{BlockContext, FunctionContext, GlobalContext};
use crate::backend::ir::{Entity, FunctionDef, Instruction, InstructionKind, LLVM};
use crate::frontend::ast::{BinaryOperator, Block, DeclItemKind, Expression, ExpressionKind, Function, ReferenceKind, Statement, StatementKind, StatementOp, Type, Program};
use crate::meta::{GetType, TypeMeta};
use crate::util::env::Env;
use itertools::Itertools;

#[derive(Clone)]
pub struct Compiler {
    block_context: Option<BlockContext>,
    function_context: Option<FunctionContext>,
    global_context: GlobalContext,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            block_context: None,
            function_context: None,
            global_context: GlobalContext::new(),
        }
    }

    /// construct a compiler with a pre-defined vector of function declarations
    pub fn with_builtin_functions(declarations: &mut Vec<String>) -> Self {
        let mut compiler = Self::new();
        compiler.global_context.append_function_declarations(declarations);
        compiler
    }

    /// construct a nested compiler for function compilation
    fn nested_for_function(&self, starting_reg: usize) -> Self {
        // TODO: Nesting logic would be much nicer with some smart references instead of copying
        let mut compiler = Compiler::new();
        compiler.block_context = Some(BlockContext::new());
        compiler.function_context = Some(FunctionContext::new(starting_reg));
        compiler.global_context = self.global_context.clone();
        compiler
    }

    /// merge with nested compiler that compiled a function
    fn merge_function_compiler(&mut self, nested: Self) {
        self.global_context = nested.global_context;
    }

    /// construct a nested compiler for a block (without re-setting register numbers)
    fn nested_for_block(&self) -> Self {
        self.clone()
    }

    /// merge with nested compiler that compiled a block
    fn merge_block_compiler(&mut self, nested: Self) {
        self.global_context = nested.global_context;
        self.function_context = nested.function_context;
    }

    /// shortcut function for getting a new register entity
    fn new_register(&mut self, t: Type) -> Entity {
        Entity::Register {
            n: self.function_context.as_mut().unwrap().new_register(),
            t,
        }
    }

    /// shortcut function for puhsing instructions to current block in function context
    fn push_instruction(&mut self, i: Instruction) {
        self.function_context.as_mut().unwrap().push_instruction(i)
    }

    /// shortcut function for creating a new basic block
    fn next_block(&mut self, label: String) {
        self.function_context.as_mut().unwrap().next_block(label)
    }

    pub fn compile_expression(&mut self, expr: Expression<TypeMeta>) -> Entity {
        let result_t = expr.get_type();
        match expr.item {
            ExpressionKind::LitInt { val } => Entity::from(val),
            ExpressionKind::LitBool { val } => Entity::from(val),
            ExpressionKind::LitStr { val } => {
                // declare the string as global constant
                let string_decl = self.global_context.declare_string(val);

                // load the global constant in the current context
                let instr = InstructionKind::LoadConst {
                    name: string_decl.name,
                    len: string_decl.len,
                };
                let register = self.new_register(Type::Str);
                self.push_instruction(instr.with_result(register.clone()));
                register
            }
            ExpressionKind::LitNull => Entity::Null,
            ExpressionKind::App { r, args } => {
                // get name of the function / method, mapped by compiler
                let func_name = match &r.item {
                    ReferenceKind::Ident { ident } => self.global_context.get_function(&ident),
                    ReferenceKind::Object { obj, field } => {
                        unimplemented!();  // TODO: virtual method call
                    }
                    ReferenceKind::ObjectSelf { field } => {
                        unimplemented!();  // TODO: virtual method call
                    }
                    r => {
                        panic!("unsupported reference type for function call: {:?}", r)
                    }
                };

                // compile argument expressions
                let mut arg_entities: Vec<Entity> = args.iter()
                    .map(|a| self.compile_expression(*a.clone()))
                    .collect();

                // compile actual call instruction
                let instr = InstructionKind::Call {
                    func: func_name,
                    args: arg_entities,
                };

                // function return type determines whether we store or forget the return value
                // TODO: Consider using nightly version with syntax sugar for box destructuring
                if let Type::Function { args: _, ret } = r.get_type() {
                    match *ret {
                        Type::Void => {
                            self.push_instruction(instr.without_result());
                            // typechecker guarantees we don't use this so just return a placeholder
                            Entity::Null
                        }
                        t => {
                            let result_ent = self.new_register(t);
                            self.push_instruction(instr.with_result(result_ent.clone()));
                            result_ent
                        }
                    }
                } else {
                    panic!("invalid function type in compiler: {:?}", r.get_type())
                }
            }
            ExpressionKind::Unary { op, arg } => {
                let arg_ent = self.compile_expression(*arg);
                let instr = InstructionKind::UnaryOp { op: op.clone(), arg: arg_ent };
                let result_reg = self.new_register(result_t);
                self.push_instruction(instr.with_result(result_reg.clone()));
                result_reg
            }
            ExpressionKind::Binary { left, op, right } => {
                if op == BinaryOperator::Or || op == BinaryOperator::And {
                    // perform lazy evaluation by storing the result in allocated memory TODO: PHI
                    let alloc = InstructionKind::Alloc { t: Type::Bool };
                    let result_reg = self.new_register(
                        Type::Reference { t: Box::new(Type::Bool) }
                    );
                    self.push_instruction(alloc.with_result(result_reg.clone()));

                    // generate labels for conditional jump
                    let suffix = self.global_context.new_label_suffix();
                    let false_label = format!("__lazy_cont__{}", suffix);
                    let end_label = format!("__lazy_end__{}", suffix);

                    // evaluate left expression, store the result
                    let left_ent = self.compile_expression(*left);
                    let store = InstructionKind::Store { val: left_ent.clone(), ptr: result_reg.clone() };
                    self.push_instruction(store.without_result());

                    // check if left result is enough to determine the entire expression result
                    let ending_value = if let BinaryOperator::Or = op {
                        // for OR, if left expression was true, entire expression is also true
                        Entity::Bool { v: true }
                    } else {
                        // for AND, if left expression was false, entire expression is also false
                        Entity::Bool { v: false }
                    };
                    let cmp = InstructionKind::BinaryOp {
                        op: BinaryOperator::Equal,
                        l: left_ent,
                        r: ending_value,
                    };
                    let cmp_result = self.new_register(Type::Bool);
                    self.push_instruction(cmp.with_result(cmp_result.clone()));
                    let cond_jump = InstructionKind::JumpCond {
                        cond: cmp_result,
                        true_label: end_label.clone(),
                        false_label: false_label.clone(),
                    };
                    self.push_instruction(cond_jump.without_result());

                    // if the left branch was was not enough, evaluate right branch and store result
                    self.next_block(false_label);
                    let right_ent = self.compile_expression(*right);
                    let store = InstructionKind::Store { val: right_ent, ptr: result_reg.clone() };
                    self.push_instruction(store.without_result());
                    let jump = InstructionKind::Jump { label: end_label.clone() };
                    self.push_instruction(jump.without_result());

                    // at the end, load the value of the result back to a register
                    self.next_block(end_label);
                    let load = InstructionKind::Load { ptr: result_reg };
                    let load_reg = self.new_register(Type::Bool);
                    self.push_instruction(load.with_result(load_reg.clone()));
                    load_reg
                } else {
                    // evaluate both sides of expression before performing the operation
                    let t = left.get_type();
                    let left_ent = self.compile_expression(*left);
                    let right_ent = self.compile_expression(*right);

                    // for strings, use concatenation function instead of llvm operator
                    let instr = if t == Type::Str && op == BinaryOperator::Plus {
                        InstructionKind::Call {
                            func: String::from("__builtin_method__str__concat__"),
                            args: vec![left_ent, right_ent],
                        }
                    } else {
                        InstructionKind::BinaryOp {
                            op: op.clone(),
                            l: left_ent,
                            r: right_ent,
                        }
                    };
                    let result_ent = self.new_register(result_t);
                    self.push_instruction(instr.with_result(result_ent.clone()));
                    result_ent
                }
            }
            ExpressionKind::InitDefault { t } => {
                if let Type::Class { ident } = t {
                    let instr = InstructionKind::Call {
                        func: self.global_context.get_init(&ident),
                        args: vec![],
                    };
                    let result_ent = self.new_register(result_t);
                    self.push_instruction(instr.with_result(result_ent.clone()));
                    result_ent
                } else {
                    panic!("Invalid type {:?} for Expression::InitDefault", t)
                }
            }
            ExpressionKind::InitArr { t, size } => {
                unimplemented!()
            }
            ExpressionKind::Reference { r } => {
                match &r.item {
                    ReferenceKind::Ident { ident } => {
                        let instr = InstructionKind::Load {
                            ptr: self.block_context.as_mut().unwrap().get_ptr(ident)
                        };
                        let result_ent = self.new_register(result_t);
                        self.push_instruction(instr.with_result(result_ent.clone()));
                        result_ent
                    }
                    ReferenceKind::Object { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load that struct field, assign type from local env
                        unimplemented!();  // TODO
                    }
                    ReferenceKind::Array { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load the desired index, assign type from local env (based on array type)
                        unimplemented!();  // TODO
                    }
                    ReferenceKind::ObjectSelf { field: _ } => {
                        unimplemented!();  // TODO
                    }
                    ReferenceKind::ArrayLen { ident: _ } => {
                        unimplemented!();  // TODO
                    }
                }
            }
            ExpressionKind::Cast { t, expr } => {
                unimplemented!()
            }
            ExpressionKind::Error => {
                unreachable!()
            }
        }
    }

    pub fn compile_statement(&mut self, stmt: Statement<TypeMeta>) {
        match stmt.item {
            StatementKind::Block { block } => {
                // use nested compiler to visit all statements in block, combine llvm results
                let mut compiler = self.nested_for_block();
                for stmt in block.item.stmts {
                    compiler.compile_statement(*stmt)
                }
                self.merge_block_compiler(compiler);
            }
            StatementKind::Empty => {}
            StatementKind::Decl { t, items } => {
                for item in items {
                    // get identifier and entity representing the value (const with default if not provided)
                    let (entity, ident) = match item.item {
                        DeclItemKind::NoInit { ident } => {
                            let entity = match t {
                                Type::Int => Entity::Int { v: 0 },
                                Type::Bool => Entity::Bool { v: false },
                                Type::Str => {
                                    let default_init = InstructionKind::Call {
                                        func: String::from("__builtin_method__str__init__"),
                                        args: vec![Entity::Int { v: 0 }],
                                    };
                                    let call_ret_ent = self.new_register(Type::Str);
                                    self.push_instruction(
                                        default_init.with_result(call_ret_ent.clone())
                                    );
                                    call_ret_ent
                                }
                                _ => unimplemented!(),
                            };
                            (entity, ident)
                        }
                        DeclItemKind::Init { ident, val } => {
                            let entity = self.compile_expression(*val);
                            (entity, ident)
                        }
                    };

                    // allocate memory for the new variable
                    let alloc = InstructionKind::Alloc { t: t.clone() };
                    let alloc_ent = self.new_register(
                        Type::Reference { t: Box::new(t.clone()) }
                    );
                    self.push_instruction(alloc.with_result(alloc_ent.clone()));

                    // store entity with value at the allocated memory location
                    let store = InstructionKind::Store {
                        val: entity,
                        ptr: alloc_ent.clone(),
                    };
                    self.push_instruction(store.without_result());

                    // use compiler environment to remember where the variable is stored
                    self.block_context.as_mut().unwrap().set_ptr(&ident, &alloc_ent);
                }
            }
            StatementKind::Ass { r, expr } => {
                // collect instructions from the expression
                let entity = self.compile_expression(*expr);

                // get entity with a pointer to the referenced variable
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident,
                    t => unimplemented!()
                };
                let ptr = self.block_context.as_mut().unwrap().get_ptr(var_ident);

                // store the entity at the location pointed to by the variable pointer
                let store = InstructionKind::Store {
                    val: entity,
                    ptr,
                };
                self.push_instruction(store.without_result());
            }
            StatementKind::Mut { r, op } => {
                // get entity with a pointer to the referenced variable
                // TODO: This part needs to be refactored-out to independent function
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident,
                    t => unimplemented!(),
                };
                let ptr_ent = self.block_context.as_mut().unwrap().get_ptr(var_ident);

                // load the value of a variable to a register
                let load = InstructionKind::Load {
                    ptr: ptr_ent.clone()
                };
                let load_ent = self.new_register(Type::Int);
                self.push_instruction(load.with_result(load_ent.clone()));

                // perform the op on the extracted value
                let binary_op = match op {
                    StatementOp::Increment => BinaryOperator::Plus,
                    StatementOp::Decrement => BinaryOperator::Minus,
                };
                let mut_op = InstructionKind::BinaryOp {
                    op: binary_op,
                    l: load_ent,
                    r: Entity::Int { v: 1 },
                };
                let mut_result_ent = self.new_register(Type::Int);
                self.push_instruction(mut_op.with_result(mut_result_ent.clone()));

                // store the result back to the original location
                let store = InstructionKind::Store {
                    val: mut_result_ent,
                    ptr: ptr_ent,
                };
                self.push_instruction(store.without_result());
            }
            StatementKind::Return { expr } => {
                match expr {
                    None => {
                        let ret = InstructionKind::RetVoid;
                        self.push_instruction(ret.without_result());
                    }
                    Some(e) => {
                        // compile expression
                        let result_ent = self.compile_expression(*e);

                        // return the value from register containing expression result
                        let ret = InstructionKind::RetVal { val: result_ent };
                        self.push_instruction(ret.without_result());
                    }
                }
            }
            StatementKind::Cond { expr, stmt } => {
                // create labels for all branches
                let suffix = self.global_context.new_label_suffix();
                let true_label = format!("__cond__true__{}", suffix);
                let end_label = format!("__cond__false__{}", suffix);

                // compile conditional expression, jump based on result
                let cond_result = self.compile_expression(*expr);
                let cond_jump_instr = InstructionKind::JumpCond {
                    cond: cond_result,
                    true_label: true_label.clone(),
                    false_label: end_label.clone(),
                };
                self.push_instruction(cond_jump_instr.without_result());

                // true branch
                self.function_context.as_mut().unwrap().next_block(true_label);
                self.compile_statement(*stmt);
                if !self.function_context.as_ref().unwrap().current_block_always_returns() {
                    let jump_instr = InstructionKind::Jump { label: end_label.clone() };
                    self.push_instruction(jump_instr.without_result());
                }

                // end = new current block
                self.function_context.as_mut().unwrap().next_block(end_label);
            }
            StatementKind::CondElse { expr, stmt_true, stmt_false } => {
                // create labels for all branches
                let suffix = self.global_context.new_label_suffix();
                let true_label = format!("__cond_else__true__{}", suffix);
                let false_label = format!("__cond_else__false__{}", suffix);
                let end_label = format!("__cond_else__end__{}", suffix);

                // evaluate conditional expression and perform conditional jump
                let expr_ent = self.compile_expression(*expr);
                let cond_jump_instr = InstructionKind::JumpCond {
                    cond: expr_ent,
                    true_label: true_label.clone(),
                    false_label: false_label.clone(),
                };
                self.push_instruction(cond_jump_instr.without_result());

                // prepare common jump instruction for both branches
                let end_jump_instr = InstructionKind::Jump { label: end_label.clone() }
                    .without_result();
                let mut next_block_necessary = false;

                // true branch
                self.function_context.as_mut().unwrap().next_block(true_label);
                self.compile_statement(*stmt_true);
                if !self.function_context.as_ref().unwrap().current_block_always_returns() {
                    self.push_instruction(end_jump_instr.clone());
                    next_block_necessary = true;
                }

                // false branch
                self.function_context.as_mut().unwrap().next_block(false_label);
                self.compile_statement(*stmt_false);
                if !self.function_context.as_ref().unwrap().current_block_always_returns() {
                    self.push_instruction(end_jump_instr.clone());
                    next_block_necessary = true;
                }

                // if either of the sides needed to jump to the end block, create it
                if next_block_necessary {
                    self.function_context.as_mut().unwrap().next_block(end_label);
                }
            }
            StatementKind::While { expr, stmt } => {
                // get labels with unique suffix
                let suffix = self.global_context.new_label_suffix();
                let cond_label = format!("__loop_cond__{}", suffix);
                let loop_label = format!("__loop_begin__{}", suffix);
                let end_label = format!("__loop_end__{}", suffix);

                // jump to condition evaluation sequence
                let cond_jump = InstructionKind::Jump { label: cond_label.clone() }
                    .without_result();
                self.push_instruction(cond_jump.clone());

                // evaluate condition in a new block and jump to loop body or end
                self.function_context.as_mut().unwrap().next_block(cond_label);
                let expr_result = self.compile_expression(*expr);
                let loop_cond_jump = InstructionKind::JumpCond {
                    cond: expr_result,
                    true_label: loop_label.clone(),
                    false_label: end_label.clone(),
                };
                self.push_instruction(loop_cond_jump.without_result());

                // compile loop body that jumps back to cond at the end
                self.function_context.as_mut().unwrap().next_block(loop_label);
                self.compile_statement(*stmt);
                self.push_instruction(cond_jump);

                // start new block with loop end label
                self.function_context.as_mut().unwrap().next_block(end_label);
            }
            StatementKind::For { t, ident, arr, stmt } => {
                unimplemented!();
            }
            StatementKind::Expr { expr } => {
                self.compile_expression(*expr);
            }
            StatementKind::Error => {
                unreachable!()
            }
        }
    }

    pub fn compile_function(&mut self, function: Function<TypeMeta>) -> FunctionDef {
        let mut arg_types = Vec::new();

        // add args to the env of nested compiler
        let n_args = function.item.args.len();
        let mut compiler = self.nested_for_function(n_args + 1);
        for (arg_reg, arg) in function.item.args.iter().enumerate() {
            // collect variable types in case the caller needs to cast the passed arguments later
            // TODO: This is not necessary at the moment, re-visit before implementing method calls
            arg_types.push(arg.get_type());

            // for consistent way of accessing variables (via self.get_ptr()),
            // function stores the value of every variable before executing instructions

            // allocate memory for the variable
            let alloc = InstructionKind::Alloc {
                t: arg.get_type()
            };
            let var_ptr_ent = compiler.new_register(
                Type::Reference { t: Box::new(arg.get_type()) }
            );
            compiler.push_instruction(alloc.with_result(var_ptr_ent.clone()));

            // load passed argument value to memory allocated for variable
            let arg_val_ent = Entity::Register {
                n: arg_reg,
                t: arg.get_type(),
            };
            let store = InstructionKind::Store {
                val: arg_val_ent,
                ptr: var_ptr_ent.clone(),
            };
            compiler.push_instruction(store.without_result());

            // mark location of the variable in the block environment
            compiler.block_context.as_mut().unwrap().set_ptr(&arg.item.ident, &var_ptr_ent);
        }

        // compile function instructions using the nested compiler
        for stmt in function.item.block.item.stmts {
            compiler.compile_statement(*stmt);
        }

        // build the LLVM function
        let llvm_function = FunctionDef {
            name: self.global_context.get_function(&function.item.ident),
            ret_type: function.item.ret.clone(),
            arg_types,
            body: compiler.function_context.as_mut().unwrap().conclude(),
        };

        // merge necessary data from nested compiler (ie. new global constants) into current one
        self.merge_function_compiler(compiler);

        llvm_function
    }

    pub fn compile_program(&mut self, program: Program<TypeMeta>) -> Vec<LLVM> {
        // compile all functions
        let mut compiled_functions: Vec<LLVM> = program.functions.values()
            .map(|func| {
                self.compile_function(func.clone())
            })
            .map(|def| LLVM::Function { def })
            .collect();

        // get all global declarations after compilation (so that they contain const string literals)
        let mut declarations_after_compilation = self.global_context.get_declarations();

        // return combined result
        let mut compiled = Vec::new();
        compiled.append(&mut declarations_after_compilation);
        compiled.append(&mut compiled_functions);
        compiled
    }
}
