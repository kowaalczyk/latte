use regex::internal::Inst;

use crate::meta::{TypeMeta, GetType, Meta};
use crate::util::visitor::AstVisitor;
use crate::backend::compiler::compiler::Compiler;
use crate::backend::compiler::ir::{Instruction, InstructionKind, Entity, GetEntity, LLVM};
use crate::frontend::ast::*;


pub enum CompilationResult {
    Entity { entity: Entity },
    LLVM { llvm: Vec<LLVM> },
}

impl CompilationResult {
    pub fn llvm(self) -> Option<Vec<LLVM>> {
        if let CompilationResult::LLVM {llvm} = self {
            Option::Some(llvm)
        } else {
            Option::None
        }
    }
}

fn combine_instructions(r: CompilationResult, instructions: &mut Vec<LLVM>) -> Entity {
    match r {
        CompilationResult::Entity { entity } => entity,
        CompilationResult::LLVM { mut llvm } => {
            // append all instructions before getting the last entity (argument value)
            let last_ent = llvm.last().unwrap().get_entity();
            instructions.append(&mut llvm);
            last_ent
        },
    }
}

impl AstVisitor<TypeMeta, CompilationResult> for Compiler {
    fn visit_expression(&mut self, expr: &Expression<TypeMeta>) -> CompilationResult {
        match &expr.item {
            ExpressionKind::LitInt { val } => {
                CompilationResult::Entity { entity: Entity::from(*val) }
            },
            ExpressionKind::LitBool { val } => {
                CompilationResult::Entity { entity: Entity::from(*val) }
            },
            ExpressionKind::LitStr { val } => {
                // register new global constant value
                // TODO: Optimization: re-use constant if already defined
                let const_name = self.new_const();
                let const_str = LLVM::ConstStrDecl {
                    name: const_name.clone(),
                    val: val.clone(),
                    len: val.len() - 1 // -2 for brackets, +1 for null terminator
                };
                self.add_decl(const_str);

                // load the defined constant
                let instr = InstructionKind::LoadConst {
                    name: const_name,
                    len: val.len() - 1 // -2 for brackets, +1 for null terminator
                };
                let result_reg = Entity::Register {
                    n: self.new_reg(),
                    t: expr.get_type()
                };
                CompilationResult::LLVM {
                    llvm: vec![instr.with_result(result_reg)]
                }
            },
            ExpressionKind::LitNull => {
                CompilationResult::Entity { entity: Entity::Null }
            },
            ExpressionKind::App { r, args } => {
                // get name of the function / method, mapped by compiler
                let func_name = match &r.item {
                    ReferenceKind::Ident { ident } => self.get_function(ident),
                    ReferenceKind::Object { obj, field } => {
                        unimplemented!();  // TODO: virtual method call
                    },
                    ReferenceKind::ObjectSelf { field } => {
                        unimplemented!();  // TODO: virtual method call
                    },
                    r => {
                        panic!("unsupported reference type for function call: {:?}", r)
                    }
                };

                // compile argument expressions
                let mut compiled_args: Vec<CompilationResult> = args.iter()
                    .map(|a| self.visit_expression(&a))
                    .collect();

                // collect argument entities and llvm instructions necessary to compute them
                let mut arg_ents = Vec::new();
                let mut instructions = Vec::new();
                for mut compiled_arg in compiled_args.drain(..) {
                    let arg_ent = combine_instructions(compiled_arg, &mut instructions);
                    arg_ents.push(arg_ent);
                }

                // compile actual call instruction
                let instr = InstructionKind::Call {
                    func: func_name,
                    args: arg_ents
                };
                // function return type determines whether we store or forget the return value
                if let Type::Function { args: _, ret } = r.get_type() {
                    let compiled_call = match *ret {
                        Type::Void => {
                            instr.without_result()
                        },
                        t => {
                            let result_ent = Entity::Register {
                                n: self.new_reg(),
                                t
                            };
                            instr.with_result(result_ent)
                        }
                    };

                    // combine with argument instructions and return the result
                    instructions.push(compiled_call);
                    CompilationResult::LLVM {
                        llvm: instructions
                    }
                } else {
                    panic!("invalid function type in compiler: {:?}", expr)
                }
            },
            ExpressionKind::Unary { op, arg } => {
                // compile argument instructions and get the result entity
                let mut instructions = Vec::new();
                let arg_ent = combine_instructions(
                    self.visit_expression(&arg),
                    &mut instructions
                );

                // compile the operation and return the combined instructions
                let instr = InstructionKind::UnaryOp { op: op.clone(), arg: arg_ent };
                let result_ent = Entity::Register {
                    n: self.new_reg(),
                    t: expr.get_type()
                };
                instructions.push(instr.with_result(result_ent));
                CompilationResult::LLVM { llvm: instructions }
            },
            ExpressionKind::Binary { left, op, right } => {
                // compile arguments
                let mut instructions = Vec::new();
                let left_ent = combine_instructions(self.visit_expression(&left), &mut instructions);
                let right_ent = combine_instructions(self.visit_expression(&right), &mut instructions);

                // for strings, use concatenation function instead of llvm operator
                let instr = if left.get_type() == Type::Str && *op == BinaryOperator::Plus {
                    InstructionKind::Call {
                        func: String::from("__builtin_method__str__concat__"),
                        args: vec![left_ent, right_ent]
                    }
                } else {
                    InstructionKind::BinaryOp {
                        op: op.clone(),
                        l: left_ent,
                        r: right_ent
                    }
                };
                let result_ent = Entity::Register {
                    n: self.new_reg(),
                    t: expr.get_type()
                };
                instructions.push(instr.with_result(result_ent));

                CompilationResult::LLVM { llvm: instructions }
            },
            ExpressionKind::InitDefault { t } => {
                if let Type::Class { ident } = t {
                    let instr = InstructionKind::Call {
                        func: self.get_init(ident),
                        args: vec![]
                    };
                    let result_ent = Entity::Register {
                        n: self.new_reg(),
                        t: t.clone()
                    };

                    CompilationResult::LLVM {
                        llvm: vec![instr.with_result(result_ent)]
                    }
                } else {
                    panic!("Invalid type {:?} for Expression::InitDefault", t)
                }
            },
            ExpressionKind::InitArr { t, size } => {
                unimplemented!()
            },
            ExpressionKind::Reference { r } => {
                match &r.item {
                    ReferenceKind::Ident { ident } => {
                        let instr = InstructionKind::Load {
                            ptr: self.get_ptr(ident)
                        };
                        let result_ent = Entity::Register {
                            n: self.new_reg(),
                            t: expr.get_type()
                        };
                        CompilationResult::LLVM {
                            llvm: vec![instr.with_result(result_ent)]
                        }
                    },
                    ReferenceKind::Object { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load that struct field, assign type from local env
                        unimplemented!();  // TODO
                    },
                    ReferenceKind::Array { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load the desired index, assign type from local env (based on array type)
                        unimplemented!();  // TODO
                    },
                    ReferenceKind::ObjectSelf { field: _ } => {
                        unimplemented!();  // TODO
                    },
                    ReferenceKind::ArrayLen { ident: _ } => {
                        unimplemented!();  // TODO
                    }
                }
            },
            ExpressionKind::Cast { t, expr } => {
                unimplemented!()
            },
            ExpressionKind::Error => {
                unreachable!()
            },
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<TypeMeta>) -> CompilationResult {
        match &stmt.item {
            StatementKind::Block { block } => {
                let mut compiler = self.clone();
                let mut instructions = Vec::new();

                // use nested compiler to visit all statements in block, combine llvm results
                for stmt in block.item.stmts.iter() {
                    if let CompilationResult::LLVM { mut llvm } = compiler.visit_statement(&stmt) {
                        instructions.append(&mut llvm);
                    }
                }
                self.match_available_reg(&compiler);
                self.combine_declarations(&mut compiler);

                CompilationResult::LLVM { llvm: instructions }
            },
            StatementKind::Empty => {
                CompilationResult::LLVM { llvm: vec![] }
            },
            StatementKind::Decl { t, items } => {
                let mut instructions = Vec::new();
                for item in items {
                    // get identifier and entity representing the value (const with default if not provided)
                    let (entity, ident) = match &item.item {
                        DeclItemKind::NoInit { ident } => {
                            let entity = match t {
                                Type::Int => Entity::Int { v: 0 },
                                Type::Bool => Entity::Bool { v: false },
                                Type::Str => {
                                    let default_init = InstructionKind::Call {
                                        func: String::from("__builtin_method__str__init__"),
                                        args: vec![]
                                    };
                                    let call_ret_ent = Entity::Register {
                                        n: self.new_reg(),
                                        t: Type::Str
                                    };
                                    instructions.push(
                                        default_init.with_result(call_ret_ent.clone())
                                    );
                                    call_ret_ent
                                }
                                _ => unimplemented!(),
                            };
                            (entity, ident)
                        },
                        DeclItemKind::Init { ident, val } => {
                            let entity = combine_instructions(
                                self.visit_expression(&val),
                                &mut instructions
                            );
                            (entity, ident)
                        },
                    };

                    // allocate memory for the new variable
                    let alloc = InstructionKind::Alloc { t: t.clone() };
                    let alloc_ent = Entity::Register {
                        n: self.new_reg(),
                        t: Type::Reference { t: Box::new(t.clone()) }
                    };
                    let alloc_instr = alloc.with_result(alloc_ent.clone());

                    // store entity with value at the allocated memory location
                    let store = InstructionKind::Store {
                        val: entity,
                        ptr: alloc_instr.get_entity()
                    };
                    let store_instr = store.without_result();

                    // use compiler environment to remember where the variable is stored
                    self.set_ptr(&ident, &alloc_ent);

                    instructions.push(alloc_instr);
                    instructions.push(store_instr);
                }
                CompilationResult::LLVM { llvm: instructions }
            },
            StatementKind::Ass { r, expr } => {
                // collect instructions from the expression
                let mut instructions = Vec::new();
                let entity = combine_instructions(
                    self.visit_expression(&expr),
                    &mut instructions
                );

                // get entity with a pointer to the referenced variable
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident,
                    t => unimplemented!()
                };
                let ptr = self.get_ptr(var_ident);

                // store the entity at the location pointed to by the variable pointer
                let store = InstructionKind::Store {
                    val: entity,
                    ptr
                };
                instructions.push(store.without_result());

                CompilationResult::LLVM { llvm: instructions }
            },
            StatementKind::Mut { r, op } => {
                let mut instructions = Vec::new();

                // get entity with a pointer to the referenced variable
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident,
                    t => unimplemented!(),
                };
                let ptr_ent = self.get_ptr(var_ident);

                // load the value of a variable to a register
                let load = InstructionKind::Load {
                    ptr: ptr_ent.clone()
                };
                let load_ent = Entity::Register {
                    n: self.new_reg(),
                    t: Type::Int
                };
                instructions.push(load.with_result(load_ent.clone()));

                // perform the op on the extracted value
                let binary_op = match op {
                    StatementOp::Increment => BinaryOperator::Plus,
                    StatementOp::Decrement => BinaryOperator::Minus,
                };
                let mut_op = InstructionKind::BinaryOp {
                    op: binary_op,
                    l: load_ent,
                    r: Entity::Int { v: 1 }
                };
                let mut_result_ent = Entity::Register {
                    n: self.new_reg(),
                    t: Type::Int
                };
                instructions.push(mut_op.with_result(mut_result_ent.clone()));

                // store the result back to the original location
                let store = InstructionKind::Store {
                    val: mut_result_ent,
                    ptr: ptr_ent
                };
                instructions.push(store.without_result());

                CompilationResult::LLVM { llvm: instructions }
            },
            StatementKind::Return { expr } => {
                match expr {
                    None => {
                        let ret = InstructionKind::RetVoid;
                        CompilationResult::LLVM { llvm: vec![ret.without_result()] }
                    },
                    Some(e) => {
                        // compile expression
                        let mut instructions = Vec::new();
                        let result_ent = combine_instructions(
                            self.visit_expression(&e),
                            &mut instructions
                        );

                        // return the value from register containing expression result
                        let ret = InstructionKind::RetVal { val: result_ent };
                        instructions.push(ret.without_result());
                        CompilationResult::LLVM { llvm: instructions }
                    },
                }
            },
            StatementKind::Cond { expr, stmt } => {
                // TODO: This should probably be performed in preprocessor
                let empty_stmt = Box::new(Statement::from(StatementKind::Empty));
                let cond_with_empty_false = Statement::new(
                    StatementKind::CondElse {
                        expr: expr.clone(),
                        stmt_true: stmt.clone(),
                        stmt_false: empty_stmt
                    },
                    stmt.get_meta().clone()
                );
                self.visit_statement(&cond_with_empty_false)
            },
            StatementKind::CondElse { expr, stmt_true, stmt_false } => {
                let mut llvm = Vec::new();

                // first, add instructions from the expr
                let expr_ent = combine_instructions(
                    self.visit_expression(&expr),
                    &mut llvm
                );

                // create labels and the conditional jump instruction
                let suffix = self.get_label_suffix();
                let true_label = format!("__branch_true__{}", suffix);
                let false_label = format!("__branch_false__{}", suffix);
                let end_label = format!("__branch_end__{}", suffix);
                let cond = InstructionKind::JumpCond {
                    cond: expr_ent,
                    true_label: true_label.clone(),
                    false_label: false_label.clone()
                };
                llvm.push(cond.without_result());

                // true branch
                llvm.push(LLVM::Label { name: true_label });
                if let CompilationResult::LLVM { llvm: mut stmt_llvm } = self.visit_statement(&stmt_true) {
                    llvm.append(&mut stmt_llvm);
                }
                let jump_to_end = InstructionKind::Jump {
                    label: end_label.clone()
                };
                llvm.push(jump_to_end.clone().without_result());

                // false branch
                llvm.push(LLVM::Label { name: false_label });
                if let CompilationResult::LLVM { llvm: mut stmt_llvm } = self.visit_statement(&stmt_false) {
                    llvm.append(&mut stmt_llvm);
                }
                llvm.push(jump_to_end.without_result());

                // end
                llvm.push(LLVM::Label { name: end_label });
                CompilationResult::LLVM { llvm }
            },
            StatementKind::While { expr, stmt } => {
                unimplemented!()  // TODO: Duplicated registers during assignment problem
//                let mut instructions = Vec::new();
//
//                let suffix = self.get_label_suffix();
//                let cond_label = format!("__loop_cond__{}", suffix);
//                let loop_label = format!("__loop_begin__{}", suffix);
//                let end_label = format!("__loop_end__{}", suffix);
//
//                // mark beginning of condition evaluation sequence with cond label
//                let cond_label_kind = InstructionKind::Label { val: cond_label.clone() };
//                instructions.push(Instruction::from(cond_label_kind));
//
//                // compile conditional expression
//                let mut expr_result = self.visit_expression(&expr);
//                instructions.append(&mut expr_result.instructions);
//
//                if let Some(expr_ent) = expr_result.result {
//                    // perform a jump based on result of conditional expression
//                    let cond_kind = InstructionKind::JumpCond {
//                        cond: expr_ent,
//                        true_label: loop_label.clone(),
//                        false_label: end_label.clone()
//                    };
//                    instructions.push(Instruction::from(cond_kind));
//
//                    // start loop
//                    let loop_kind = InstructionKind::Label { val: loop_label.clone() };
//                    instructions.push(Instruction::from(loop_kind));
//
//                    // compile the statement
//                    let mut compiled_stmt = self.visit_statement(&stmt);
//                    instructions.append(&mut compiled_stmt.instructions);
//
//                    // end loop with a jump to conditional statement
//                    let jump_kind = InstructionKind::Jump { label: cond_label };
//                    instructions.push(Instruction::from(jump_kind));
//
//                    // mark end of loop statements with end label
//                    let end_kind = InstructionKind::Label { val: end_label };
//                    instructions.push(Instruction::from(end_kind));
//
//                    CompilationResult {
//                        instructions,
//                        result: None
//                    }
//                } else {
//                    panic!("Expression {:?} didn't return a compilation result", expr)
//                }
            },
            StatementKind::For { t, ident, arr, stmt } => {
                unimplemented!();
            },
            StatementKind::Expr { expr } => {
                if let CompilationResult::LLVM { llvm } = self.visit_expression(&expr) {
                    CompilationResult::LLVM { llvm }
                } else {
                    CompilationResult::LLVM { llvm: vec![] }
                }
            },
            StatementKind::Error => {
                unreachable!()
            },
        }
    }

    fn visit_class(&mut self, class: &Class<TypeMeta>) -> CompilationResult {
        unimplemented!()  // TODO
    }

    fn visit_function(&mut self, function: &Function<TypeMeta>) -> CompilationResult {
        let mut arg_types = Vec::new();
        let mut instructions = Vec::new();

        // add args to the env of nested compiler
        let n_args = function.item.args.len();
        let mut compiler = Compiler::with_starting_reg(n_args + 1);
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
            let var_ptr_ent = Entity::Register {
                n: compiler.new_reg(),
                t: Type::Reference { t: Box::new(arg.get_type()) }
            };

            // load passed argument value to memory allocated for variable
            let arg_val_ent = Entity::Register {
                n: arg_reg,
                t: arg.get_type()
            };
            let store = InstructionKind::Store {
                val: arg_val_ent,
                ptr: var_ptr_ent.clone()
            };

            // append instructions and set the nested compiler's env accordingly
            instructions.push(alloc.with_result(var_ptr_ent.clone()));
            instructions.push(store.without_result());
            compiler.set_ptr(&arg.item.ident, &var_ptr_ent);
        }

        // compile function instructions using the nested compiler
        for stmt in function.item.block.item.stmts.iter() {
            if let CompilationResult::LLVM { mut llvm } = compiler.visit_statement(&stmt) {
                instructions.append(&mut llvm);
            }
        }

        // no need to align registers, but we have to combine declarations
        self.combine_declarations(&mut compiler);

        // build the LLVM function
        let func = LLVM::Function {
            name: self.get_function(&function.item.ident),
            ret_type: function.item.ret.clone(),
            arg_types,
            llvm: instructions.into_iter().map(Box::new).collect()
        };
        CompilationResult::LLVM {
            llvm: vec![func]
        }
    }
}
