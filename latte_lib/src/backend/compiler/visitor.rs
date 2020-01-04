use regex::internal::Inst;

use crate::util::visitor::AstVisitor;
use crate::meta::{TypeMeta, GetType, Meta};
use crate::frontend::ast::*;
use crate::backend::compiler::compiler::Compiler;
use crate::backend::compiler::ir::{Instruction, Entity, BasicValue, InstructionKind, InstructionMeta};


pub struct CompilationResult {
    /// a list of instructions that can be directly compiled to LLVM (without additional context)
    pub instructions: Vec<Instruction>,

    /// result of the compiled code (ie. where the function return value will be stored)
    pub result: Option<Entity>,
}

impl AstVisitor<TypeMeta, CompilationResult> for Compiler {
    fn visit_expression(&mut self, expr: &Expression<TypeMeta>) -> CompilationResult {
        match &expr.item {
            ExpressionKind::LitInt { val } => {
                let val = BasicValue::Int { v: *val };
                CompilationResult { instructions: vec![], result: Some(Entity::Const { val }) }
            },
            ExpressionKind::LitBool { val } => {
                let val = BasicValue::Bool { v: *val };
                CompilationResult { instructions: vec![], result: Some(Entity::Const { val }) }
            },
            ExpressionKind::LitStr { val } => {
                let result_reg = self.new_reg();
                let kind = InstructionKind::StrAlloc {
                    val: val.clone()
                };
                let meta = InstructionMeta {
                    reg: result_reg,
                    t: Type::Str
                };
                let instr = Instruction::new(kind, Some(meta.clone()));
                CompilationResult {
                    instructions: vec![instr],
                    result: Some(Entity::from(meta))
                }
            },
            ExpressionKind::LitNull => {
                CompilationResult { instructions: vec![], result: Some(Entity::Null) }
            },
            ExpressionKind::App { r, args } => {
                let func_name = match &r.item {
                    ReferenceKind::Ident { ident } => ident,
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

                let mut compiled_args: Vec<CompilationResult> = args.iter()
                    .map(|a| self.visit_expression(&a))
                    .collect();

                let mut arg_ents = Vec::new();
                let mut arg_instructions = Vec::new();
                for mut compiled_arg in compiled_args.drain(..) {
                    arg_ents.push(compiled_arg.result.unwrap());
                    arg_instructions.append(&mut compiled_arg.instructions)
                }

                let kind = InstructionKind::Call {
                    func_name: self.get_function(func_name),
                    ret: expr.get_type(),
                    args: arg_ents
                };
                let result_reg = self.new_reg();
                let meta = InstructionMeta {
                    reg: result_reg,
                    t: expr.get_type()
                };
                let instr = Instruction::new(kind, Some(meta.clone()));
                CompilationResult {
                    instructions: vec![instr],
                    result: Some(Entity::from(meta))
                }
            },
            ExpressionKind::Unary { op, arg } => {
                let CompilationResult { mut instructions, result } = self.visit_expression(&arg);
                if let Some(result_entity) = result {
                    let result_reg = self.new_reg();
                    let kind = InstructionKind::ApplyUnaryOp {
                        op: op.clone(),
                        arg_ent: result_entity,
                    };
                    let meta = InstructionMeta {
                        reg: result_reg,
                        t: expr.get_type()
                    };
                    let instr = Instruction::new(kind, Some(meta.clone()));
                    instructions.push(instr);
                    CompilationResult {
                        instructions,
                        result: Some(Entity::from(meta))
                    }
                } else {
                    panic!("Compilation of {:?} didn't return a result to apply unary operator to!", arg)
                }
            },
            ExpressionKind::Binary { left, op, right } => {
                let CompilationResult {
                    instructions: mut left_instructions,
                    result: left_result
                } = self.visit_expression(&left);
                let CompilationResult {
                    instructions: mut right_instructions,
                    result: right_result
                } = self.visit_expression(&right);

                // TODO: Check if type is string and call function instead (equality or concat)
                if let (Some(left_ent), Some(right_ent)) = (left_result, right_result) {
                    let mut instructions = Vec::new();
                    instructions.append(&mut left_instructions);
                    instructions.append(&mut right_instructions);

                    let kind = if left.get_type() == Type::Str && *op == BinaryOperator::Plus {
                        // string concatenation
                        InstructionKind::Call {
                            func_name: String::from("__builtin_method__str__concat__"),
                            ret: expr.get_type(),
                            args: vec![left_ent, right_ent]
                        }
                    } else {
                        // integer addition
                        InstructionKind::ApplyBinaryOp {
                            op: op.clone(),
                            left_ent,
                            right_ent
                        }
                    };
                    let result_reg = self.new_reg();
                    let meta = InstructionMeta {
                        reg: result_reg,
                        t: expr.get_type()
                    };
                    let instr = Instruction::new(kind, Some(meta.clone()));
                    instructions.push(instr);

                    CompilationResult {
                        instructions,
                        result: Some(Entity::from(meta))
                    }
                } else {
                    panic!(
                        "One of expressions: {:?} or {:?} didn't return a result to apply binary to!",
                        left,
                        right
                    )
                }
            },
            ExpressionKind::InitDefault { t } => {
                if let Type::Class { ident } = t {
                    let reg = self.new_reg();
                    let init = self.get_init(ident);
                    let kind = InstructionKind::Call {
                        func_name: init,
                        ret: t.clone(),
                        args: vec![]
                    };
                    let meta = InstructionMeta { reg, t: expr.get_type() };
                    let instr = Instruction::new(kind, Some(meta.clone()));
                    CompilationResult {
                        instructions: vec![instr],
                        result: Some(Entity::from(meta))
                    }
                } else {
                    panic!("Invalid type {:?} for Expression::InitDefault", t)
                }
            },
            ExpressionKind::InitArr { t, size } => {
                unimplemented!(); // TODO
                CompilationResult { instructions: vec![], result: None }
            },
            ExpressionKind::Reference { r } => {
                match &r.item {
                    ReferenceKind::Ident { ident } => {
                        // TODO: Load instruction for complex types
//                        let result_reg = self.new_reg();
//                        let ptr = self.get_ptr(ident);
//                        let kind = InstructionKind::Load { ptr };
//                        let instr = Instruction::new(kind, Some(meta.clone()));
                        CompilationResult {
                            instructions: vec![],
                            result: Some(self.get_ptr(ident).clone())
                        }
                    },
                    ReferenceKind::Object { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load that struct field, assign type from local env
                        unimplemented!();  // TODO
                        CompilationResult { instructions: vec![], result: None }
                    },
                    ReferenceKind::Array { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load the desired index, assign type from local env (based on array type)
                        unimplemented!();  // TODO
                        CompilationResult { instructions: vec![], result: None }
                    },
                    ReferenceKind::ObjectSelf { field: _ } => {
                        unimplemented!();  // TODO
                        CompilationResult { instructions: vec![], result: None }
                    },
                    ReferenceKind::ArrayLen { ident: _ } => {
                        unimplemented!();  // TODO
                        CompilationResult { instructions: vec![], result: None }
                    }
                }
            },
            ExpressionKind::Cast { t, expr } => {
                unimplemented!();
                // 1. compile expression
                // 2. use bitcast to cast null (expr will always be null) to the desired type (pointer)
                CompilationResult { instructions: vec![], result: None } // TODO: use skeleton above
            },
            ExpressionKind::Error => {
                unreachable!();
            },
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<TypeMeta>) -> CompilationResult {
        match &stmt.item {
            StatementKind::Block { block } => {
                let mut compiler = self.clone();
                let mut instructions = Vec::new();
                for stmt in block.item.stmts.iter() {
                    let mut res = compiler.visit_statement(&stmt);
                    instructions.append(&mut res.instructions);
                }
                self.match_available_reg(&compiler);
                CompilationResult {
                    instructions,
                    result: None
                }
            },
            StatementKind::Empty => {
                CompilationResult { instructions: vec![], result: None }
            },
            StatementKind::Decl { t, items } => {
                let mut instructions = Vec::new();
                for item in items {
                    let (entity, ident) = match &item.item {
                        DeclItemKind::NoInit { ident } => {
                            let entity = match t {
                                Type::Int => Entity::Const { val: BasicValue::Int { v: 0 } },
                                Type::Bool => Entity::Const { val: BasicValue::Bool { v: false } },
                                // TODO: Type::Str (function call to allocate memory)
                                t => {
                                    panic!("Unable to create default entity for type {:?}", t)
                                }
                            };
                            (entity, ident)
                        },
                        DeclItemKind::Init { ident, val } => {
                            let compiled_val = self.visit_expression(&val);
                            let entity = compiled_val.result.unwrap();
                            (entity, ident)
                        },
                    };
                    self.set_ptr(&ident, &entity);
                    // TODO: Store is only necessary for complex types:
//                    let kind = InstructionKind::Store {
//                        val: entity,
//                        ptr: ptr_entity,
//                    };
//                    instructions.push(Instruction::from(kind));
                }
                CompilationResult {
                    instructions,
                    result: None
                }
            },
            StatementKind::Ass { r, expr } => {
                let compiled_expr = self.visit_expression(expr);

                if let Some(val) = compiled_expr.result {
                    let var_ident = match &r.item {
                        ReferenceKind::Ident { ident } => ident,
                        t => unimplemented!() // TODO
                    };
                    self.set_ptr(var_ident, &val);  // TODO: Store for complex types
                    CompilationResult {
                        instructions: compiled_expr.instructions,
                        result: None
                    }
                } else {
                    panic!("Expression {:?} didn't return a result", expr)
                }
            },
            StatementKind::Mut { r, op } => {
                match &r.item {
                    ReferenceKind::Ident { ident } => {
                        let var_ent = self.get_ptr(ident).clone();

                        // perform the op on the extracted value
                        let op_reg = self.new_reg();
                        let binary_op = match op {
                            StatementOp::Increment => BinaryOperator::Plus,
                            StatementOp::Decrement => BinaryOperator::Minus,
                        };
                        let kind = InstructionKind::ApplyBinaryOp {
                            op: binary_op,
                            left_ent: var_ent,
                            right_ent: Entity::Const { val: BasicValue::Int { v: 1 } }
                        };

                        // store the updated register
                        self.set_ptr(ident, &Entity::Register { n: op_reg, t: Type::Int });
                        CompilationResult {
                            instructions: vec![Instruction::from(kind)],
                            result: None
                        }
                    },
                    t => unimplemented!(),  // TODO
                }
            },
            StatementKind::Return { expr } => {
                match expr {
                    None => {
                        let kind = InstructionKind::ReturnVoid;
                        let instructions = vec![Instruction::from(kind)];
                        CompilationResult {
                            instructions,
                            result: None
                        }
                    },
                    Some(e) => {
                        // compile expression
                        let mut expr_result = self.visit_expression(&e);
                        let mut instructions = Vec::new();
                        instructions.append(&mut expr_result.instructions);

                        // return the value from register containing expression result
                        if let Some(entity) = expr_result.result {
                            let kind = InstructionKind::ReturnEnt { val: entity };
                            instructions.push(Instruction::from(kind));
                            CompilationResult {
                                instructions,
                                result: None
                            }
                        } else {
                            panic!("Expression {:?} didn't return a compilation result", expr)
                        }
                    },
                }
            },
            StatementKind::Cond { expr, stmt } => {
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
//                let mut instructions = Vec::new();
//
//                let mut expr_result = self.visit_expression(&expr);
//                instructions.append(&mut expr_result.instructions);
//
//                let suffix = self.get_label_suffix();
//                let true_label = format!("__branch_true__{}", suffix);
//                let end_label = format!("__branch_end__{}", suffix);
//
//                if let Some(expr_ent) = expr_result.result {
//                    // conditional jump
//                    let cond_kind = InstructionKind::JumpCond {
//                        cond: expr_ent,
//                        true_label: true_label.clone(),
//                        false_label: end_label.clone()
//                    };
//                    instructions.push(Instruction::from(cond_kind));
//
//                    // start true branch
//                    let true_kind = InstructionKind::Label {
//                        val: true_label
//                    };
//                    instructions.push(Instruction::from(true_kind));
//
//                    // add all true branch instructions
//                    let mut compiled_stmt = self.visit_statement(stmt);
//                    instructions.append(&mut compiled_stmt.instructions);
//
//                    // add end label after all instructions from true branch
//                    let end_kind = InstructionKind::Label {
//                        val: end_label
//                    };
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
            StatementKind::CondElse { expr, stmt_true, stmt_false } => {
                let mut instructions = Vec::new();

                let mut expr_result = self.visit_expression(&expr);
                instructions.append(&mut expr_result.instructions);

                let suffix = self.get_label_suffix();
                let true_label = format!("__branch_true__{}", suffix);
                let false_label = format!("__branch_false__{}", suffix);
                let end_label = format!("__branch_end__{}", suffix);

                if let Some(expr_ent) = expr_result.result {
                    // conditional jump
                    let cond_kind = InstructionKind::JumpCond {
                        cond: expr_ent,
                        true_label: true_label.clone(),
                        false_label: false_label.clone()
                    };
                    instructions.push(Instruction::from(cond_kind));

                    // start true branch
                    let true_kind = InstructionKind::Label {
                        val: true_label
                    };
                    instructions.push(Instruction::from(true_kind));

                    // add all true branch instructions
                    let mut compiled_stmt = self.visit_statement(stmt_true);
                    instructions.append(&mut compiled_stmt.instructions);

                    // jump to end label at the end of true branch
                    let end_kind = InstructionKind::Jump {
                        label: end_label.clone()
                    };
                    instructions.push(Instruction::from(end_kind.clone()));

                    // start false branch
                    let false_kind = InstructionKind::Label {
                        val: false_label
                    };
                    instructions.push(Instruction::from(false_kind));

                    // add all false branch instructions
                    let mut compiled_stmt = self.visit_statement(stmt_false);
                    instructions.append(&mut compiled_stmt.instructions);

                    // jump to end label at the end of false branch
                    instructions.push(Instruction::from(end_kind));

                    // add end label
                    let end_kind = InstructionKind::Label {
                        val: end_label
                    };
                    instructions.push(Instruction::from(end_kind));

                    CompilationResult {
                        instructions,
                        result: None
                    }
                } else {
                    panic!("Expression {:?} didn't return a compilation result", expr)
                }
            },
            StatementKind::While { expr, stmt } => {
                let mut instructions = Vec::new();

                let suffix = self.get_label_suffix();
                let cond_label = format!("__loop_cond__{}", suffix);
                let loop_label = format!("__loop_begin__{}", suffix);
                let end_label = format!("__loop_end__{}", suffix);

                // mark beginning of condition evaluation sequence with cond label
                let cond_label_kind = InstructionKind::Label { val: cond_label.clone() };
                instructions.push(Instruction::from(cond_label_kind));

                // compile conditional expression
                let mut expr_result = self.visit_expression(&expr);
                instructions.append(&mut expr_result.instructions);

                if let Some(expr_ent) = expr_result.result {
                    // perform a jump based on result of conditional expression
                    let cond_kind = InstructionKind::JumpCond {
                        cond: expr_ent,
                        true_label: loop_label.clone(),
                        false_label: end_label.clone()
                    };
                    instructions.push(Instruction::from(cond_kind));

                    // start loop
                    let loop_kind = InstructionKind::Label { val: loop_label.clone() };
                    instructions.push(Instruction::from(loop_kind));

                    // compile the statement
                    let mut compiled_stmt = self.visit_statement(&stmt);
                    instructions.append(&mut compiled_stmt.instructions);

                    // end loop with a jump to conditional statement
                    let jump_kind = InstructionKind::Jump { label: cond_label };
                    instructions.push(Instruction::from(jump_kind));

                    // mark end of loop statements with end label
                    let end_kind = InstructionKind::Label { val: end_label };
                    instructions.push(Instruction::from(end_kind));

                    CompilationResult {
                        instructions,
                        result: None
                    }
                } else {
                    panic!("Expression {:?} didn't return a compilation result", expr)
                }
            },
            StatementKind::For { t, ident, arr, stmt } => {
                unimplemented!(); // TODO
                CompilationResult {
                    instructions: vec![],
                    result: None
                }
            },
            StatementKind::Expr { expr } => {
                let compiled_expr = self.visit_expression(expr);
                CompilationResult {
                    instructions: compiled_expr.instructions,
                    result: None
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
        let mut compiler = self.clone();
        let mut raw_args = Vec::new();

        // add args to the env of nested compiler
        for arg in function.item.args.iter() {
            let arg_ent = Entity::NamedRegister {
                n: arg.item.ident.clone(),
                t: arg.get_type()
            };
            compiler.set_ptr(&arg.item.ident, &arg_ent);
            raw_args.push(arg_ent);
        }

        // compile function instructions using the nested compiler
        let mut instructions = Vec::new();
        for stmt in function.item.block.item.stmts.iter() {
            let mut res = compiler.visit_statement(&stmt);
            let mut mapped_instr: Vec<_> = res.instructions
                .drain(..)
                .map(Box::new)
                .collect();
            instructions.append(&mut mapped_instr);
        }
        self.match_available_reg(&compiler);

        let func_name = self.get_function(&function.item.ident);
        let compiled_func = InstructionKind::FuncDef {
            ret: function.item.ret.clone(),
            name: func_name,
            args: raw_args,
            instructions
        };
        CompilationResult {
            instructions: vec![Instruction::from(compiled_func)],
            result: None
        }
    }
}
