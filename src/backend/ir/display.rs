use std::fmt::{Display, Error, Formatter};
use std::string::ToString;

use itertools::Itertools;

use crate::backend::ir::{Entity, GetEntity, Instruction, InstructionKind, LLVM, BasicBlock, StructDecl, StringDecl, FunctionDef};
use crate::frontend::ast::{BinaryOperator, Type, UnaryOperator};
use crate::meta::GetType;

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Type::Int => write!(f, "i32"),
            Type::Str => write!(f, "i8*"),
            Type::Bool => write!(f, "i1"),
            Type::Void => write!(f, "void"),
            Type::Reference { t } => {
                write!(f, "{}*", t)
            }
            Type::Class { .. } => unimplemented!(),
            Type::Array { .. } => unimplemented!(),
            t => panic!("unexpected type: {:?}", t),
        }
    }
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            BinaryOperator::Plus => write!(f, "add"),
            BinaryOperator::Minus => write!(f, "sub"),
            BinaryOperator::Times => write!(f, "mul"),
            BinaryOperator::Divide => write!(f, "sdiv"),
            BinaryOperator::Modulo => write!(f, "srem"), // TODO: Make sure it's ok
            BinaryOperator::Less => write!(f, "icmp slt"),
            BinaryOperator::LessEqual => write!(f, "icmp sle"),
            BinaryOperator::Greater => write!(f, "icmp sgt"),
            BinaryOperator::GreaterEqual => write!(f, "icmp sge"),
            BinaryOperator::Equal => write!(f, "icmp eq"),
            BinaryOperator::NotEqual => write!(f, "icmp ne"),
            BinaryOperator::And => write!(f, "and"),
            BinaryOperator::Or => write!(f, "or"),
        }
    }
}

impl Display for Entity {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Entity::Null => write!(f, "null"),
            Entity::Register { n, t } => write!(f, "%{}", n),
            Entity::Int { v } => write!(f, "{}", v),
            Entity::Bool { v } => write!(f, "{}", v),
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match &self.item {
            InstructionKind::Alloc { t } => {
                write!(
                    f, "{} = alloca {}",
                    self.get_entity(), t
                )
            }
            InstructionKind::Load { ptr } => {
                write!(
                    f, "{} = load {}, {} {}",
                    self.get_entity(),
                    self.get_type(),
                    ptr.get_type(),
                    ptr
                )
            }
            InstructionKind::Store { val, ptr } => {
                write!(
                    f, "store {} {}, {} {}",
                    val.get_type(),
                    val,
                    ptr.get_type(),
                    ptr
                )
            }
            InstructionKind::LoadConst { name, len } => {
                write!(
                    f, "{} = getelementptr inbounds [{} x i8], [{} x i8]* @{}, i32 0, i32 0",
                    self.get_entity(),
                    len, len, name
                )
            }
            InstructionKind::UnaryOp { op, arg } => {
                match op {
                    UnaryOperator::Neg => write!(f, "{} = sub i32 0, {}", self.get_entity(), arg),
                    UnaryOperator::Not => write!(f, "{} = add i1 {}, 1", self.get_entity(), arg),
                }
            }
            InstructionKind::BinaryOp { op, l, r } => {
                write!(
                    f, "{} = {} {} {}, {}",
                    self.get_entity(),
                    op, l.get_type(),
                    l, r
                )
            }
            InstructionKind::Call { func, args } => {
                let args = args.iter()
                    .map(|ent| format!("{} {}", ent.get_type(), ent))
                    .join(",");
                if self.has_result_entity() {
                    write!(
                        f, "{} = call {} @{} ({})",
                        self.get_entity(), self.get_type(), func, args
                    )
                } else {
                    write!(f, "call void @{} ({})", func, args)
                }
            }
            InstructionKind::RetVal { val } => {
                write!(f, "ret {} {}", val.get_type(), val)
            }
            InstructionKind::RetVoid => {
                write!(f, "ret void")
            }
            InstructionKind::JumpCond { cond, true_label, false_label } => {
                write!(
                    f, "br {} {}, label %{}, label %{}",
                    cond.get_type(), cond,
                    true_label, false_label
                )
            }
            InstructionKind::Jump { label } => {
                write!(f, "br label %{}", label)
            }
            InstructionKind::Phi { args: _ } => {
                unimplemented!()
            }
        }
    }
}

impl Display for BasicBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(label) = &self.label {
            write!(f, "{}:\n", label)?
        }
        let instructions = self.instructions.iter()
            .map(Instruction::to_string)
            .map(|i| format!("\t{}", i))
            .join("\n");
        write!(f, "{}\n", instructions)
    }
}

impl Display for StructDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        unimplemented!()
    }
}

impl Display for StringDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        // string already contains quotes inside
        let mut s = self.val.clone();
        s.insert(s.len() - 1, '\\');
        s.insert(s.len() - 1, '0');
        s.insert(s.len() - 1, '0');
        write!(
            f, "@{} = private unnamed_addr constant [{} x i8] c{}",
            self.name, self.len, s
        )
    }
}

impl Display for FunctionDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let f_args = self.arg_types.iter()
            .map(Type::to_string)
            .join(",");
        let f_instrs = self.body.iter()
            .map(BasicBlock::to_string)
            .join("\n");
        write!(
            f, "define {} @{} ({}) {{\n {} \n}}\n",
            self.ret_type, self.name, f_args, f_instrs
        )
    }
}

impl Display for LLVM {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            LLVM::DeclFunction { decl } => write!(f, "{}", decl),
            LLVM::DeclStruct { decl } => write!(f, "{}", decl),
            LLVM::DeclString { decl } => write!(f, "{}", decl),
            LLVM::Function { def } => write!(f, "{}", def),
        }
    }
}
