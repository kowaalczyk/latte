use std::cmp::max;

use crate::backend::context::{BlockContext, FunctionContext, GlobalContext};
use crate::backend::ir::{Entity, FunctionDef, Instruction, InstructionKind, LLVM};
use crate::frontend::ast::{BinaryOperator, Block, DeclItemKind, Expression, ExpressionKind, Function, ReferenceKind, Statement, StatementKind, StatementOp, Type, Program};
use crate::meta::{GetType, TypeMeta};
use crate::util::env::Env;
use itertools::Itertools;
use std::collections::HashSet;
use std::iter::FromIterator;
use crate::backend::builder::BlockBuilder;

#[derive(Clone)]
pub struct Compiler {
    block_context: Option<BlockContext>,
    function_context: Option<FunctionContext>,
    builder: Option<BlockBuilder>,
    global_context: GlobalContext,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            block_context: None,
            function_context: None,
            builder: None,
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
        compiler.builder = Some(BlockBuilder::without_label());
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
        self.builder = nested.builder;
        self.block_context = nested.block_context; // TODO: Nested envs are necessary, fix this (!!!)
    }

    /// shortcut function for getting a new register entity
    fn new_register(&mut self, t: Type) -> Entity {
        // TODO: Create a one function returning mutable reference to the context
        self.function_context.as_mut().unwrap().new_register(t)
    }

    /// shortcut function for puhsing instructions to current block in function context
    fn push_instruction(&mut self, i: Instruction) {
        self.builder.as_mut().unwrap().push_instruction(i);
    }

    /// shortcut function for creating a new basic block
    fn next_block(&mut self, label: String) {
        let block = self.builder.as_mut().unwrap().build(
            self.function_context.as_mut().unwrap()
        );
        self.function_context.as_mut().unwrap().push_block(block);
        self.builder = Some(BlockBuilder::with_label(label));
    }

    /// creates new basic block, but before that maps entities in the last block (loop body)
    /// using mapping for the currently built block (loop condition)
    fn complete_loop_block(&mut self, next_label: String) {
        let cond_block = self.builder.as_mut().unwrap().build(
            self.function_context.as_mut().unwrap()
        );

        let mapping = self.builder.as_mut().unwrap().get_entity_mapping(
            self.function_context.as_mut().unwrap()
        );
        self.function_context.as_mut().unwrap().map_entities_in_last_block(mapping);
        self.block_context.as_mut().unwrap().map_env(mapping);

        self.function_context.as_mut().unwrap().push_block(cond_block);
        self.builder = Some(BlockBuilder::with_label(next_label));
    }

    /// shortcut function for getting the label from currently constructed basic block
    fn get_current_block_label(&self) -> String {
        self.builder.as_ref().unwrap().get_block_label()
    }

    /// shortcut for getting environment from currently used block context
    fn get_current_env(&self) -> Env<Entity> {
        self.block_context.as_ref().unwrap().get_env().clone()
    }

    /// shortcut for setting a variable and updating gen for current basic block
    fn set_variable(&mut self, ident: String, ent: Entity) {
        self.block_context.as_mut().unwrap().set_ptr(&ident, &ent);
//        self.builder.as_mut().unwrap().set_gen(ident, ent);
    }

    pub fn compile_expression(&mut self, expr: Expression<TypeMeta>) -> Entity {
        let result_t = expr.get_type();
        match expr.item {
            ExpressionKind::LitInt { val } => {
                Entity::Int {
                    v: val,
                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                }
            },
            ExpressionKind::LitBool { val } => {
                Entity::Bool {
                    v: val,
                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                }
            },
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

                // immediately cast the constant to i8* to prevent type mismatch in phi expressions
                let cast_instr = InstructionKind::BitCast {
                    ent: register,
                    to: Type::Str
                };
                let cast_register = self.new_register(Type::Str);
                self.push_instruction(cast_instr.with_result(cast_register.clone()));

                cast_register
            }
            ExpressionKind::LitNull => {
                Entity::Null {
                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                }
            },
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
                            Entity::Null { uuid: 0 }
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
                        Entity::Bool { v: true, uuid: 0 }
                    } else {
                        // for AND, if left expression was false, entire expression is also false
                        Entity::Bool { v: false, uuid: 0 }
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
                        self.block_context.as_mut().unwrap().get_ptr(ident)
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
                            let entity = match &t {
                                Type::Int => Entity::Int {
                                    v: 0,
                                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                                },
                                Type::Bool => Entity::Bool {
                                    v: false,
                                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                                },
                                Type::Str => {
                                    let default_init = InstructionKind::Call {
                                        func: String::from("__builtin_method__str__init__"),
                                        args: vec![Entity::Int { v: 0, uuid: 0 }],
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
                            // collect instructions from the expression TODO: Refactor-out
                            let entity = match self.compile_expression(*val) {
                                Entity::Null { uuid } => Entity::Null {
                                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                                },
                                Entity::Int { v, uuid } => Entity::Int {
                                    v,
                                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                                },
                                Entity::Bool { v, uuid } => Entity::Bool {
                                    v,
                                    uuid: self.function_context.as_mut().unwrap().new_uuid()
                                },
                                reg_entity => reg_entity,
                            };
                            (entity, ident)
                        }
                    };

                    // use compiler environment to remember where the variable is stored
                    self.set_variable(ident, entity);
                }
            }
            StatementKind::Ass { r, expr } => {
                // collect instructions from the expression TODO: Refactor-out
                let entity = match self.compile_expression(*expr) {
                    Entity::Null { uuid } => Entity::Null {
                        uuid: self.function_context.as_mut().unwrap().new_uuid()
                    },
                    Entity::Int { v, uuid } => Entity::Int {
                        v,
                        uuid: self.function_context.as_mut().unwrap().new_uuid()
                    },
                    Entity::Bool { v, uuid } => Entity::Bool {
                        v,
                        uuid: self.function_context.as_mut().unwrap().new_uuid()
                    },
                    reg_entity => reg_entity,
                };

                // get entity with a pointer to the referenced variable
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident.clone(),
                    t => unimplemented!()
                };

                // update local variable environment to reflect the change
                self.set_variable(var_ident, entity)
            }
            StatementKind::Mut { r, op } => {
                // get entity with a pointer to the referenced variable
                // TODO: This part needs to be refactored-out to independent function
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident.clone(),
                    t => unimplemented!(),
                };
                let var_ent = self.block_context.as_mut().unwrap().get_ptr(&var_ident);

                // perform the op on the extracted value
                let binary_op = match op {
                    StatementOp::Increment => BinaryOperator::Plus,
                    StatementOp::Decrement => BinaryOperator::Minus,
                };
                let mut_op = InstructionKind::BinaryOp {
                    op: binary_op,
                    l: var_ent,
                    r: Entity::Int { v: 1, uuid: 0 },
                };
                let mut_result_ent = self.new_register(Type::Int);
                self.push_instruction(mut_op.with_result(mut_result_ent.clone()));

                // update local variable environment to reflect the change
                self.set_variable(var_ident, mut_result_ent)
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

                // collect information for phi calculation
                let in_label = self.get_current_block_label();
                let in_env = self.get_current_env();

                // true branch
                self.next_block(true_label.clone());
                self.builder.as_mut().unwrap().add_predecessor(in_label.clone(), &in_env);
                self.compile_statement(*stmt);
                if !self.builder.as_ref().unwrap().block_always_returns() {
                    let jump_instr = InstructionKind::Jump { label: end_label.clone() };
                    self.push_instruction(jump_instr.without_result());
                }

                // collect information for phi calculation
                let true_env = self.get_current_env();

                // end = new current block, with additional predecessor (in_block)
                self.next_block(end_label);
                self.builder.as_mut().unwrap().add_predecessor(in_label, &in_env);
                self.builder.as_mut().unwrap().add_predecessor(true_label, &true_env);
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

                // collect information for phi calculation
                let in_label = self.get_current_block_label();
                let in_env = self.get_current_env();

                // prepare common jump instruction for both branches
                let end_jump_instr = InstructionKind::Jump { label: end_label.clone() }
                    .without_result();
                let mut next_block_necessary = false;

                // true branch
                self.next_block(true_label.clone());
                self.builder.as_mut().unwrap().add_predecessor(in_label.clone(), &in_env);
                self.compile_statement(*stmt_true);
                if !self.builder.as_ref().unwrap().block_always_returns() {
                    self.push_instruction(end_jump_instr.clone());
                    next_block_necessary = true;
                }

                // collect information for phi calculation
                let true_env = self.get_current_env();

                // false branch
                self.next_block(false_label.clone());
                self.builder.as_mut().unwrap().add_predecessor(in_label.clone(), &in_env);
                self.compile_statement(*stmt_false);
                if !self.builder.as_ref().unwrap().block_always_returns() {
                    self.push_instruction(end_jump_instr.clone());
                    next_block_necessary = true;
                }

                // collect information for phi calculation
                let false_env = self.get_current_env();

                // if either of the sides needed to jump to the end block, create it
                if next_block_necessary {
                    self.next_block(end_label);
//                    self.builder.as_mut().unwrap().add_predecessor(in_label, &in_env);
                    self.builder.as_mut().unwrap().add_predecessor(true_label, &true_env);
                    self.builder.as_mut().unwrap().add_predecessor(false_label, &false_env);
                }
            }
            StatementKind::While { expr, stmt } => {
                // get labels with unique suffix
                let suffix = self.global_context.new_label_suffix();
                let cond_label = format!("__loop_cond__{}", suffix);
                let loop_label = format!("__loop_body__{}", suffix);
                let end_label = format!("__loop_end__{}", suffix);

                // jump to condition evaluation sequence
                let cond_jump = InstructionKind::Jump { label: cond_label.clone() }
                    .without_result();
                self.push_instruction(cond_jump.clone());

                // collect information for phi calculation
                let in_label = self.get_current_block_label();
                let in_env = self.get_current_env();

                // compile loop body, that also jumps back to cond at the end
                self.next_block(loop_label.clone());
                // we don't need to specify predecessor for body block
                self.compile_statement(*stmt);
                self.push_instruction(cond_jump);

                // collect information for phi calculation
                let body_env = self.get_current_env();

                // begin next block by evaluating necessary phi statements based on block variables
                self.next_block(cond_label);
                self.builder.as_mut().unwrap().add_predecessor(in_label, &in_env);
                self.builder.as_mut().unwrap().add_predecessor(loop_label.clone(), &body_env);

                // evaluate condition in a new block and jump to loop body or end
                let expr_result = self.compile_expression(*expr);
                let loop_cond_jump = InstructionKind::JumpCond {
                    cond: expr_result,
                    true_label: loop_label,
                    false_label: end_label.clone(),
                };
                self.push_instruction(loop_cond_jump.without_result());

                // start new block with loop end label, and fix mapping in 2 previous blocks
                self.complete_loop_block(end_label);
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
        let mut args = Vec::new();

        // add args to the env of nested compiler
        let n_args = function.item.args.len();
        let mut compiler = self.nested_for_function(1); // TODO: This parameter is no longer needed
        for (arg_reg, arg) in function.item.args.iter().enumerate() {
            // collect variable types in case the caller needs to cast the passed arguments later
            // TODO: This is not necessary at the moment, re-visit before implementing method calls
            args.push(arg.item.clone());

            // mark location of the variable in the block environment
            let arg_ent = Entity::NamedRegister {
                name: arg.item.ident.clone(),
                t: arg.get_type()
            };
            compiler.set_variable(arg.item.ident.clone(), arg_ent);
        }

        // compile function instructions using the nested compiler
        for stmt in function.item.block.item.stmts {
            compiler.compile_statement(*stmt);
        }

        // finish last block within the nested compiler
        let block = compiler.builder.as_mut().unwrap().build(
            compiler.function_context.as_mut().unwrap()
        );
        compiler.function_context.as_mut().unwrap().push_block(block);

        // build the LLVM function
        let llvm_function = FunctionDef {
            name: self.global_context.get_function(&function.item.ident),
            ret_type: function.item.ret.clone(),
            args,
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
