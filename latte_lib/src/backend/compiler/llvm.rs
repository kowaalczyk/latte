use itertools::Itertools;

use crate::frontend::ast::{Type, UnaryOperator, BinaryOperator};
use crate::backend::compiler::ir::{Entity, BasicValue, Instruction, InstructionKind};
use crate::backend::compiler::visitor::CompilationResult;


pub trait ToLLVM {
    fn to_llvm(&self) -> String;
}

impl ToLLVM for Type {
    fn to_llvm(&self) -> String {
        match self {
            Type::Int => String::from("i32"),
            Type::Str => String::from("char*"),
            Type::Bool => String::from("i1"),
            Type::Void => String::from("void"),
            Type::Null => unimplemented!(),
            Type::Class { .. } => unimplemented!(),
            Type::Array { .. } => unimplemented!(),
            Type::Function { .. } => unimplemented!(),
            t => panic!("unexpected type: {:?}", t),
        }
    }
}

impl ToLLVM for Entity {
    fn to_llvm(&self) -> String {
        match self {
            Entity::Null => String::from("null"),
            Entity::Register { n, t } => format!("%{}", n),
            Entity::NamedRegister { n, t } => format!("%{}", n),
            Entity::Const { val } => {
                match val {
                    BasicValue::Bool { v } => (*v as i8).to_string(),
                    BasicValue::Int { v } => v.to_string(),
                }
            },
        }
    }
}

impl ToLLVM for Instruction {
    fn to_llvm(&self) -> String {
        match &self.item {
            InstructionKind::StrAlloc { val } => {
                unimplemented!()
            },
            InstructionKind::ApplyUnaryOp { op, arg_ent } => {
                if let Some(meta) = self.get_meta() {
                    let result_reg = Entity::from(meta.clone());
                    match op {
                        UnaryOperator::Neg => {
                            format!("\t{} = sub i32 0, {}", result_reg.to_llvm(), arg_ent.to_llvm())
                        },
                        UnaryOperator::Not => {
                            format!("\t{} = add i1 {}, 1", result_reg.to_llvm(), arg_ent.to_llvm())
                        },
                    }
                } else {
                    panic!("expected meta for instruction: {:?}", self.item)
                }
            },
            InstructionKind::ApplyBinaryOp { op, left_ent, right_ent } => {
                let llvm_op = match op {
                    BinaryOperator::Plus => String::from("add i32"),
                    BinaryOperator::Minus => String::from("sub i32"),
                    BinaryOperator::Times => String::from("mul i32"),
                    BinaryOperator::Divide => String::from("div i32"),
                    BinaryOperator::Modulo => String::from("srem i32"),  // TODO: Make sure this is is fine
                    BinaryOperator::Less => String::from("icmp slt i32"),
                    BinaryOperator::LessEqual => String::from("icmp sle i32"),
                    BinaryOperator::Greater => String::from("icmp sgt i32"),
                    BinaryOperator::GreaterEqual => String::from("icmp sge i32"),
                    BinaryOperator::Equal => format!("icmp eq {}", left_ent.get_type().to_llvm()),
                    BinaryOperator::NotEqual => format!("icmp eq {}", left_ent.get_type().to_llvm()),
                    BinaryOperator::And => String::from("and i1"),
                    BinaryOperator::Or => String::from("or i1"),
                };
                if let Some(meta) = self.get_meta() {
                    let result_reg = Entity::from(meta.clone());
                    format!(
                        "\t{} = {} {}, {}",
                        result_reg.to_llvm(),
                        llvm_op,
                        left_ent.to_llvm(),
                        right_ent.to_llvm()
                    )
                } else {
                    panic!("insufficient metadata for instruction: {:?}", self)
                }
            },
            InstructionKind::Call { func_name, ret, args } => {
                if let Some(meta) = self.get_meta() {
                    let result_reg = Entity::from(meta.clone());
                    let formatted_args = args.iter()
                        .map(|arg| format!("{} {}", arg.get_type().to_llvm(), arg.to_llvm()))
                        .join(",");
                    let result_fmt = if let Type::Void = ret {
                        String::new()
                    } else {
                        format!("{} = ", result_reg.to_llvm())
                    };
                    format!(
                        "\t{}call {} @{} ({})",
                        result_fmt,
                        ret.to_llvm(),
                        func_name,
                        formatted_args
                    )
                } else {
                    panic!("insufficient metadata for instruction: {:?}", self)
                }
            },
            InstructionKind::ReturnEnt { val } => {
                format!("\tret {} {}", val.get_type().to_llvm(), val.to_llvm())
            },
            InstructionKind::ReturnVoid => String::from("ret void"),
            InstructionKind::JumpCond { cond, true_label, false_label } => {
                format!("\tbr i1 {}, label %{}, label %{}", cond.to_llvm(), true_label, false_label)
            },
            InstructionKind::Jump { label } => {
                format!("\tbr label %{}", label)
            },
            InstructionKind::Label { val } => {
                format!("{}:", val)
            },
            InstructionKind::FuncDef { ret, name, args, instructions } => {
                let f_args = args.iter()
                    .map(|arg| format!("{} {}", arg.get_type().to_llvm(), arg.to_llvm()))
                    .join(",");
                let f_instrs = instructions.iter()
                    .map(|instr| instr.to_llvm())
                    .join("\n");
                format!("define {} @{}( {} ){{\n{}\n}}", ret.to_llvm(), name, f_args, f_instrs)
            },
        }
    }
}

impl ToLLVM for CompilationResult {
    fn to_llvm(&self) -> String {
        self.instructions.iter()
            .map(|i| i.to_llvm())
            .join("\n")
    }
}
