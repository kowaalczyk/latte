use std::fmt::{Display, Error, Formatter};
use std::string::ToString;

use itertools::{Itertools, join};

use crate::backend::ir::{BasicBlock, Entity, FunctionDef, GetEntity, Instruction, InstructionKind, LLVM, StringDecl, StructDecl, VTableDecl};
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
            Type::Class { ident } => write!(f, "%__class__{}*", ident),
            Type::BuiltinClass { ident } => write!(f, "%{}*", ident),
            Type::Array { item_t } => {
                let formatted = format!("__builtin_struct__array_{}", item_t)
                    .replace("*", "ptr");
                write!(f, "%{}*", formatted)
            }
            Type::Function { args, ret } => {
                let formatted_args = args.iter()
                    .map(|t| format!("{}", t))
                    .join(", ");
                write!(f, "{}({})", *ret, formatted_args)
            }
            t => panic!("unexpected type: {:?}", t),
        }
    }
}

fn llvm_complex_type_name(t: &Type) -> String {
    if let Type::Class { ident } = t {
        format!("%__class__{}", ident)
    } else if let Type::BuiltinClass { ident} = t {
        format!("%__class__{}", ident)
    } else {
        panic!("dereferencing not supported on type {}", t)
    }
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            BinaryOperator::Plus => write!(f, "add"),
            BinaryOperator::Minus => write!(f, "sub"),
            BinaryOperator::Times => write!(f, "mul"),
            BinaryOperator::Divide => write!(f, "sdiv"),
            BinaryOperator::Modulo => write!(f, "srem"),
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
            Entity::Null { .. } => write!(f, "null"),
            Entity::Register { n, t: _ } => write!(f, "%{}", n),
            Entity::NamedRegister { name, t: _ } => write!(f, "%{}", name),
            Entity::Int { v, uuid: _ } => write!(f, "{}", v),
            Entity::Bool { v, uuid: _ } => write!(f, "{}", v),
            Entity::GlobalConst { name, t } => write!(f, "@{}", name),
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
            InstructionKind::BitCast { ent, to } => {
                write!(
                    f, "{} = bitcast {} {} to {}",
                    self.get_entity(),
                    ent.get_type(), ent, to
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
            InstructionKind::CallReference { func, args } => {
                let args = args.iter()
                    .map(|ent| format!("{} {}", ent.get_type(), ent))
                    .join(",");
                if self.has_result_entity() {
                    write!(
                        f, "{} = call {} {} ({})",
                        self.get_entity(), self.get_type(), func, args
                    )
                } else {
                    write!(f, "call void {} ({})", func, args)
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
            InstructionKind::Phi { args } => {
                let phi_args = args.iter()
                    .map(|(ent, label)| format!("[{}, %{}]", ent, label))
                    .join(", ");
                write!(
                    f, "{} = phi {} {}",
                    self.get_entity(),
                    self.get_type(), phi_args
                )
            }
            InstructionKind::GetStructElementPtr { container_type_name, var, idx } => {
                write!(
                    f, "{} = getelementptr {}, {} {}, i32 0, {} {}",
                    self.get_entity(), container_type_name,
                    var.get_type(), var,
                    idx.get_type(), idx
                )
            }
            InstructionKind::GetArrayElementPtr { item_t, var, idx } => {
                write!(
                    f, "{} = getelementptr {}, {} {}, {} {}",
                    self.get_entity(), item_t,
                    var.get_type(), var,
                    idx.get_type(), idx
                )
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
        let mapped_fields = self.fields.iter().map(|f| f.to_string()).join(", ");
        write!(f, "%{} = type {{ {} }}\n", self.name, mapped_fields)?;
        write!(
            f, "@{} = constant i32 ptrtoint (%{}* getelementptr (%{}, %{}* null, i32 1) to i32)",
            self.size_constant_name, self.name, self.name, self.name
        )
    }
}

impl Display for VTableDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        // declare vtable structure type
        let method_types = self.methods.iter()
            .map(|(t,n)| format!("{}*", t))
            .join(", ");
        write!(f, "%{} = type {{ {} }}\n", self.name, method_types)?;

        // define constant with actual vtable data
        let method_pointers = self.methods.iter()
            .map(|(t, n)| format!("{}* @{}", t, n))
            .join(", ");
        write!(f, "@{} = global %{} {{ {} }}", self.data_const_name, self.name, method_pointers)
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
        let f_args = self.args.iter()
            .map(|arg| format!("{} %{}", arg.t, arg.ident))
            .join(", ");
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
            LLVM::DeclFunction { decl } => write!(f, "{}\n", decl),
            LLVM::DeclStruct { decl } => write!(f, "{}\n", decl),
            LLVM::DeclString { decl } => write!(f, "{}", decl),
            LLVM::Function { def } => write!(f, "{}", def),
            LLVM::DeclVTable { decl } => {write!(f, "{}\n", decl)},
        }
    }
}
