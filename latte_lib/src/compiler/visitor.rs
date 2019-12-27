use crate::util::visitor::AstVisitor;
use crate::compiler::compiler::Compiler;
use crate::parser::ast::{Function, Expression, Statement, Class, UnaryOperator, Type, Reference};
use crate::compiler::ir::{Instruction, Entity, BasicValue, ConstComplexValue};

pub struct CompilationResult {
    /// a list of instructions that can be directly compiled to LLVM (without additional context)
    pub instructions: Vec<Instruction>,

    /// result of the compiled code (ie. where the function return value will be stored)
    pub result: Option<Entity>,
}

impl AstVisitor<CompilationResult> for Compiler {
    fn visit_expression(&mut self, expr: &Expression) -> CompilationResult {
        match expr {
            Expression::LitInt { val } => {
                let val = BasicValue::Int { v: *val };
                CompilationResult { instructions: vec![], result: Some(Entity::Const { val }) }
            },
            Expression::LitBool { val } => {
                let val = BasicValue::Bool { v: *val };
                CompilationResult { instructions: vec![], result: Some(Entity::Const { val }) }
            },
            Expression::LitStr { val } => {
                let reg = self.new_reg();
                let instr = Instruction::ConstAlloc {
                    result_reg: reg,
                    val: ConstComplexValue::Str { s: val.clone() }
                };
                CompilationResult {
                    instructions: vec![instr],
                    result: Some(Entity::Register { n: reg })
                }
            },
            Expression::LitNull => {
                CompilationResult { instructions: vec![], result: Some(Entity::NullPtr) }
            },
            Expression::App { .. } => {
                CompilationResult { instructions: vec![], result: None } // TODO
            },
            Expression::Unary { op, arg } => {
                let CompilationResult { mut instructions, result } = self.visit_expression(&arg.item);
                if let Some(result_entity) = result {
                    let reg = self.new_reg();
                    let instr = Instruction::ApplyUnaryOp {
                        result_reg: reg,
                        op: op.clone(),
                        arg_ent: result_entity,
                    };
                    instructions.push(instr);
                    CompilationResult { instructions, result: Some(Entity::Register { n: reg }) }
                } else {
                    panic!("Compilation of {:?} didn't return a result to apply unary operator to!", arg)
                }
            },
            Expression::Binary { left, op, right } => {
                let CompilationResult {
                    instructions: mut left_instructions,
                    result: left_result
                } = self.visit_expression(&left.item);
                let CompilationResult {
                    instructions: mut right_instructions,
                    result: right_result
                } = self.visit_expression(&right.item);

                if let (Some(left_ent), Some(right_ent)) = (left_result, right_result) {
                    let reg = self.new_reg();
                    let instr = Instruction::ApplyBinaryOp {
                        result_reg: reg,
                        op: op.clone(),
                        left_ent,
                        right_ent
                    };
                    let mut instructions = Vec::new();
                    instructions.append(&mut left_instructions);
                    instructions.append(&mut right_instructions);
                    instructions.push(instr);
                    CompilationResult { instructions, result: Some(Entity::Register { n: reg }) }
                } else {
                    panic!(
                        "One of expressions: {:?} or {:?} didn't return a result to apply unary binary to!",
                        left,
                        right
                    )
                }
            },
            Expression::InitDefault { t } => {
                if let Type::Class { ident } = t {
                    let reg = self.new_reg();
                    let cls = self.get_ir(&ident);
                    let val = ConstComplexValue::Obj { cls };
                    let instr = Instruction::ConstAlloc { result_reg: reg, val };
                    CompilationResult {
                        instructions: vec![instr],
                        result: Some(Entity::Register { n: reg })
                    }
                } else {
                    panic!("Invalid type {:?} for Expression::InitDefault", t)
                }
            },
            Expression::InitArr { t, size } => {
                CompilationResult { instructions: vec![], result: None } // TODO
            },
            Expression::Reference { r } => {
                match &r.item {
                    Reference::Ident { .. } => {
                        // 1. load by pointer, assign type from local env
                    },
                    Reference::Object { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load that struct field, assign type from local env
                    },
                    Reference::Array { .. } => {
                        // 1. load by pointer, assign type from local env
                        // 2. getelementptr to the struct field
                        // 3. load the desired index, assign type from local env (based on array type)
                    },
                }
                CompilationResult { instructions: vec![], result: None } // TODO: use skeleton above
            },
            Expression::Cast { t, expr } => {
                // 1. compile expression
                // 2. use bitcast to cast null (expr will always be null) to the desired type (pointer)
                CompilationResult { instructions: vec![], result: None } // TODO: use skeleton above
            },
            Expression::Error => {
                unreachable!();
            },
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) -> CompilationResult {
        unimplemented!()
    }

    fn visit_class(&mut self, class: &Class) -> CompilationResult {
        unimplemented!()
    }

    fn visit_function(&mut self, function: &Function) -> CompilationResult {
        unimplemented!()
    }
}
