use crate::backend::context::{BlockContext, FunctionContext, GlobalContext};
use crate::backend::builder::BlockBuilder;
use crate::frontend::ast::{Function, ArgItem, Expression, ExpressionKind, Type, ReferenceKind, BinaryOperator, Statement, StatementKind, DeclItemKind, StatementOp};
use crate::meta::{TypeMeta, GetType};
use crate::backend::ir::{FunctionDef, Entity, InstructionKind};

#[derive(Clone)]
pub struct FunctionCompiler {
    builder: BlockBuilder,
    block_context: BlockContext,
    function_context: FunctionContext,
    global_context: GlobalContext,
}

impl FunctionCompiler {
    pub fn new(global_context: &GlobalContext) -> Self {
        // TODO: Store global_context as reference to avoid unnecessary copying
        Self {
            builder: BlockBuilder::without_label(),
            block_context: BlockContext::new(),
            function_context: FunctionContext::new(),
            global_context: global_context.clone()
        }
    }

    pub fn get_global_context(&self) -> &GlobalContext {
        &self.global_context
    }

    // TODO: Use separate compiler for blocks (statements and expressions), call in FunctionCompiler

    /// construct a nested compiler for a block (without re-setting register numbers)
    fn nested_for_block(&self) -> Self {
        let mut nested = self.clone();
        nested.block_context.increase_depth();
        nested
    }

    /// merge with nested compiler that compiled a block
    fn merge_block_compiler(&mut self, nested: Self) {
        self.global_context = nested.global_context;
        self.function_context = nested.function_context;
        self.builder = nested.builder;

        self.block_context = nested.block_context;
        self.block_context.decrease_depth();
    }

    /// shortcut function for creating a new basic block
    fn next_block(&mut self, label: String) {
        let block = self.builder.build(
            &mut self.function_context
        );
        self.function_context.push_block(block);
        self.builder = BlockBuilder::with_label(label);
    }

    /// creates new basic block, but before that maps entities in the last block (loop body)
    /// using mapping for the currently built block (loop condition)
    fn complete_loop_block(&mut self, next_label: String) {
        let cond_block = self.builder.build(
            &mut self.function_context
        );

        let mapping = self.builder.get_entity_mapping(
            &mut self.function_context
        );
        self.function_context.map_entities_in_last_block(mapping);
        self.block_context.map_env(mapping);

        self.function_context.push_block(cond_block);
        self.builder = BlockBuilder::with_label(next_label);
    }

    /// clone entity, ensuring its uniqueness (new uuid) if it is a constant
    fn make_unique_entity(&mut self, ent: Entity) -> Entity {
        match ent {
            Entity::Null { uuid, t } => Entity::Null {
                uuid: self.function_context.new_uuid(),
                t
            },
            Entity::Int { v, uuid } => Entity::Int {
                v,
                uuid: self.function_context.new_uuid(),
            },
            Entity::Bool { v, uuid } => Entity::Bool {
                v,
                uuid: self.function_context.new_uuid(),
            },
            reg_entity => reg_entity,
        }
    }

    /// compiles instructions necessary to access array_ent at idx_ent as a reference entity
    fn compile_array_gep(&mut self, array_item_t: Type, array_ent: Entity, idx_ent: Entity) -> Entity {
        // get ptr to raw array from array struct
        let array_gep_instr = InstructionKind::GetStructElementPtr {
            container_type_name: self.global_context.get_or_declare_array_struct(&array_item_t).llvm_name(),
            var: array_ent,
            idx: Entity::Int { v: 1, uuid: 0 }
        };
        let array_gep_reg = self.function_context.new_register(array_item_t.reference().reference());
        self.builder.push_instruction(array_gep_instr.with_result(array_gep_reg.clone()));

        // load raw array
        let array_load_instr = InstructionKind::Load {
            ptr: array_gep_reg
        };
        let array_reg = self.function_context.new_register(array_item_t.reference());
        self.builder.push_instruction(array_load_instr.with_result(array_reg.clone()));

        // get ptr to the index in raw array
        let get_item_ptr = InstructionKind::GetArrayElementPtr {
            item_t: array_item_t.clone(),
            var: array_reg,
            idx: idx_ent
        };
        let item_ptr_ent = self.function_context.new_register(array_item_t.reference());
        self.builder.push_instruction(get_item_ptr.with_result(item_ptr_ent.clone()));

        item_ptr_ent
    }

    /// calculate size of a single value from reference
    fn get_size(t: &Type) -> Entity {
        const PTR_SIZE: i32 = 4;
        match t {
            Type::Int => Entity::Int { v: 4, uuid: 0 },
            Type::Str => Entity::Int { v: PTR_SIZE, uuid: 0 },
            Type::Bool => Entity::Int { v: 1, uuid: 0 },
            Type::Class { .. } => Entity::Int { v: PTR_SIZE, uuid: 0 },
            Type::Array { .. } => Entity::Int { v: PTR_SIZE, uuid: 0 },
            Type::Reference { .. } => Entity::Int { v: PTR_SIZE, uuid: 0 },
            _ => Entity::Int { v: 0, uuid: 0 },
        }
    }

    /// interpret type as array, get type of its element
    fn get_array_item_type(t: &Type) -> Type {
        if let Type::Array { item_t } = t {
            item_t.as_ref().clone()
        } else {
            panic!("expected array type, got {}", t)
        }
    }

    /// interpret type as function, get type of its return value
    fn get_function_return_type(t: &Type) -> Type {
        if let Type::Function { args: _, ret } = t {
            ret.as_ref().clone()
        } else {
            panic!("invalid function type in compiler: {:?}", t)
        }
    }

    pub fn compile_expression(&mut self, expr: Expression<TypeMeta>) -> Entity {
        let result_t = expr.get_type();
        match expr.item {
            ExpressionKind::LitInt { val } => {
                Entity::Int {
                    v: val,
                    uuid: self.function_context.new_uuid(),
                }
            }
            ExpressionKind::LitBool { val } => {
                Entity::Bool {
                    v: val,
                    uuid: self.function_context.new_uuid(),
                }
            }
            ExpressionKind::LitStr { val } => {
                // declare the string as global constant
                let string_decl = self.global_context.declare_string(val);

                // load the global constant in the current context
                let instr = InstructionKind::LoadConst {
                    name: string_decl.name,
                    len: string_decl.len,
                };
                let register = self.function_context.new_register(Type::Str);
                self.builder.push_instruction(instr.with_result(register.clone()));

                // immediately cast the constant to i8* to prevent type mismatch in phi expressions
                let cast_instr = InstructionKind::BitCast {
                    ent: register,
                    to: Type::Str,
                };
                let cast_register = self.function_context.new_register(Type::Str);
                self.builder.push_instruction(cast_instr.with_result(cast_register.clone()));

                cast_register
            }
            ExpressionKind::LitNull => {
                Entity::Null {
                    uuid: self.function_context.new_uuid(),
                    t: Type::Null
                }
            }
            ExpressionKind::App { r, args } => {
                // get name of the function / method, mapped by compiler
                let func_name = match &r.item {
                    ReferenceKind::Ident { ident } => self.global_context.get_function_name(&ident),
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
                match Self::get_function_return_type(&r.get_type()) {
                    Type::Void => {
                        self.builder.push_instruction(instr.without_result());
                        // typechecker guarantees we don't use this so just return a placeholder
                        Entity::Null { uuid: 0, t: Type::Null }
                    }
                    t => {
                        let result_ent = self.function_context.new_register(t);
                        self.builder.push_instruction(instr.with_result(result_ent.clone()));
                        result_ent
                    }
                }
            }
            ExpressionKind::Unary { op, arg } => {
                let arg_ent = self.compile_expression(*arg);
                let instr = InstructionKind::UnaryOp { op: op.clone(), arg: arg_ent };
                let result_reg = self.function_context.new_register(result_t);
                self.builder.push_instruction(instr.with_result(result_reg.clone()));
                result_reg
            }
            ExpressionKind::Binary { left, op, right } => {
                if op == BinaryOperator::Or || op == BinaryOperator::And {
                    // generate labels for conditional jump
                    let suffix = self.global_context.new_label_suffix();
                    let cont_label = format!("__lazy_cont__{}", suffix);
                    let end_label = format!("__lazy_end__{}", suffix);

                    // evaluate left expression
                    let left_ent = self.compile_expression(*left);

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
                        l: left_ent.clone(),
                        r: ending_value,
                    };
                    let cmp_result = self.function_context.new_register(Type::Bool);
                    self.builder.push_instruction(cmp.with_result(cmp_result.clone()));

                    // perform conditional jump to cont_label if we need to evaluate 2nd expression
                    let cond_jump = InstructionKind::JumpCond {
                        cond: cmp_result,
                        true_label: end_label.clone(),
                        false_label: cont_label.clone(),
                    };
                    self.builder.push_instruction(cond_jump.without_result());

                    // collect information for phi calculation
                    let in_label = self.builder.get_block_label();
                    let in_env = self.block_context.get_env_view();

                    // if the left branch was was not enough, evaluate right branch
                    self.next_block(cont_label);
                    let right_ent = self.compile_expression(*right);
                    let jump = InstructionKind::Jump { label: end_label.clone() };
                    self.builder.push_instruction(jump.without_result());

                    // collect information for phi calculation, we need to re-define cont_label
                    // as the end of continued evaluation branch may have different label than
                    // the beginning (eg. when there are more than 2 lazy-evaluated expressions)
                    let cont_env = self.block_context.get_env_view();
                    let cont_label = self.builder.get_block_label();

                    // at the end, load the value of the result back to a register
                    self.next_block(end_label);
                    let phi_instr = InstructionKind::Phi {
                        args: vec![(left_ent, in_label), (right_ent, cont_label)]
                    };
                    let phi_reg = self.function_context.new_register(result_t);
                    self.builder.push_instruction(phi_instr.with_result(phi_reg.clone()));
                    phi_reg
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
                    let result_ent = self.function_context.new_register(result_t);
                    self.builder.push_instruction(instr.with_result(result_ent.clone()));
                    result_ent
                }
            }
            ExpressionKind::InitDefault { t } => {
                if let Type::Class { ident } = t {
                    let instr = InstructionKind::Call {
                        func: self.global_context.get_init_name(&ident),
                        args: vec![],
                    };
                    let result_ent = self.function_context.new_register(result_t);
                    self.builder.push_instruction(instr.with_result(result_ent.clone()));
                    result_ent
                } else {
                    panic!("Invalid type {:?} for Expression::InitDefault", t)
                }
            }
            ExpressionKind::InitArr { t, size } => {
                let void_ptr_t = Type::Str; // apparently void* in C is i8* in LLVM
                let arr_item_t = Self::get_array_item_type(&t);

                // register new array type that has to be declared for our program
                let arr_struct_decl = self.global_context.get_or_declare_array_struct(&arr_item_t);

                // calculate length (number of items)
                let length_ent = self.compile_expression(*size);

                // calculate size of the array (in bytes)
                let item_size_ent = Self::get_size(&t);
                let byte_count_instr = InstructionKind::BinaryOp {
                    op: BinaryOperator::Times,
                    l: length_ent.clone(),
                    r: item_size_ent
                };
                let bytes_size_ent = self.function_context.new_register(Type::Int);
                self.builder.push_instruction(byte_count_instr.with_result(bytes_size_ent.clone()));

                // allocate memory for the array
                let raw_array_malloc = InstructionKind::Call {
                    func: String::from("__builtin_method__array__init__"),
                    args: vec![bytes_size_ent]
                };
                let malloc_result_ent = self.function_context.new_register(void_ptr_t.clone());
                self.builder.push_instruction(raw_array_malloc.with_result(malloc_result_ent.clone()));

                // cast raw array to its appropriate type
                let raw_array_t = Type::Reference { t: Box::new(arr_item_t.clone()) };
                let raw_array_cast = InstructionKind::BitCast {
                    ent: malloc_result_ent,
                    to: raw_array_t.clone()
                };
                let raw_array_ent = self.function_context.new_register(raw_array_t.clone());
                self.builder.push_instruction(raw_array_cast.with_result(raw_array_ent.clone()));

                // load size of array struct
                let load_struct_size = InstructionKind::Load {
                    ptr: Entity::GlobalConstInt { name: self.global_context.get_array_struct_size_name(&arr_item_t) }
                };
                let struct_size_ent = self.function_context.new_register(Type::Int);
                self.builder.push_instruction(load_struct_size.with_result(struct_size_ent.clone()));

                // use array init to allocate array struct as well
                let struct_init = InstructionKind::Call {
                    func: String::from("__builtin_method__array__init__"),
                    args: vec![struct_size_ent]
                };
                let array_init_ent = self.function_context.new_register(void_ptr_t);
                self.builder.push_instruction(struct_init.with_result(array_init_ent.clone()));

                // cast the result to appropriate type
                let array_struct_cast = InstructionKind::BitCast {
                    ent: array_init_ent,
                    to: t.clone()
                };
                let array_struct_ptr_ent = self.function_context.new_register(t.clone());
                self.builder.push_instruction(array_struct_cast.with_result(array_struct_ptr_ent.clone()));

                // set length
                let length_gep = InstructionKind::GetStructElementPtr {
                    container_type_name: arr_struct_decl.llvm_name(),
                    var: array_struct_ptr_ent.clone(),
                    idx: Entity::Int { v: 0, uuid: 0 }
                };
                let length_ptr = self.function_context.new_register(Type::Int.reference());
                self.builder.push_instruction(length_gep.with_result(length_ptr.clone()));

                let store_length = InstructionKind::Store {
                    val: length_ent,
                    ptr: length_ptr
                };
                self.builder.push_instruction(store_length.without_result());

                // move the allocated raw array into array struct
                let raw_array_gep = InstructionKind::GetStructElementPtr {
                    container_type_name: arr_struct_decl.llvm_name(),
                    var: array_struct_ptr_ent.clone(),
                    idx: Entity::Int { v: 1, uuid: 0 }
                };
                let raw_array_ptr = self.function_context.new_register(raw_array_t.reference());
                self.builder.push_instruction(raw_array_gep.with_result(raw_array_ptr.clone()));

                let store_array = InstructionKind::Store {
                    val: raw_array_ent,
                    ptr: raw_array_ptr
                };
                self.builder.push_instruction(store_array.without_result());

                // return array struct: {length, array}
                array_struct_ptr_ent
            }
            ExpressionKind::Reference { r } => {
                match &r.item {
                    ReferenceKind::Ident { ident } => {
                        self.block_context.get_variable(ident)
                    }
                    ReferenceKind::TypedObject { obj, cls, field } => {
                        let struct_decl = self.global_context.get_struct_decl(cls);

                        let field_idx = struct_decl.field_env.get(field).unwrap();
                        let field_t: &Type = struct_decl.fields.get(*field_idx as usize).unwrap();

                        let obj_ent = self.block_context.get_variable(obj);

                        let gep_instr = InstructionKind::GetStructElementPtr {
                            container_type_name: struct_decl.llvm_name(),
                            var: obj_ent,
                            idx: Entity::Int { v: *field_idx, uuid: 0 }
                        };
                        let gep_reg = self.function_context.new_register(field_t.reference());
                        self.builder.push_instruction(gep_instr.with_result(gep_reg.clone()));

                        let load_reg = self.function_context.new_register(field_t.clone());
                        let load_instr = InstructionKind::Load { ptr: gep_reg };
                        self.builder.push_instruction(load_instr.with_result(load_reg.clone()));

                        load_reg
                    }
                    ReferenceKind::Object { obj, field } => {
                        panic!(
                            "Reference to {} should've been converted to TypedObject before compiling",
                            obj
                        )
                    }
                    ReferenceKind::Array { arr, idx } => {
                        let array_ent = self.block_context.get_variable(arr);
                        let idx_ent = self.compile_expression(idx.as_ref().clone());
                        let gep_reg = self.compile_array_gep(
                            array_ent.get_array_item_t(),
                            array_ent,
                            idx_ent
                        );

                        let load_reg = self.function_context.new_register(result_t.clone());
                        let load_instr = InstructionKind::Load { ptr: gep_reg };
                        self.builder.push_instruction(load_instr.with_result(load_reg.clone()));

                        load_reg
                    }
                    ReferenceKind::ObjectSelf { field: _ } => {
                        unimplemented!();  // TODO
                    }
                    ReferenceKind::ArrayLen { ident } => {
                        let obj_ent = self.block_context.get_variable(ident);
                        let arr_item_t = Self::get_array_item_type(&obj_ent.get_type());

                        let gep_instr = InstructionKind::GetStructElementPtr {
                            container_type_name: self.global_context.get_or_declare_array_struct(&arr_item_t).llvm_name(),
                            var: obj_ent,
                            idx: Entity::Int { v: 0, uuid: 0 }
                        };
                        let gep_reg = self.function_context.new_register(Type::Int.reference());
                        self.builder.push_instruction(gep_instr.with_result(gep_reg.clone()));

                        let load_reg = self.function_context.new_register(Type::Int);
                        let load_instr = InstructionKind::Load { ptr: gep_reg };
                        self.builder.push_instruction(load_instr.with_result(load_reg.clone()));

                        load_reg
                    }
                }
            }
            ExpressionKind::Cast { t, expr } => {
                if let Entity::Null { uuid, t: _ } = self.compile_expression(*expr) {
                    Entity::Null { uuid, t }
                } else {
                    unimplemented!()
                }
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
                            let result_t = t.clone();
                            let entity = match &t {
                                Type::Int => Entity::Int {
                                    v: 0,
                                    uuid: self.function_context.new_uuid(),
                                },
                                Type::Bool => Entity::Bool {
                                    v: false,
                                    uuid: self.function_context.new_uuid(),
                                },
                                Type::Str => {
                                    let default_init = InstructionKind::Call {
                                        func: String::from("__builtin_method__str__init__"),
                                        args: vec![Entity::Int { v: 0, uuid: 0 }],
                                    };
                                    let call_ret_ent = self.function_context.new_register(Type::Str);
                                    self.builder.push_instruction(
                                        default_init.with_result(call_ret_ent.clone())
                                    );
                                    call_ret_ent
                                }
                                Type::Class { ident } => {
                                    let call_instr = InstructionKind::Call {
                                        func: self.global_context.get_init_name(&ident),
                                        args: vec![],
                                    };
                                    let call_result_ent = self.function_context.new_register(result_t);
                                    self.builder.push_instruction(call_instr.with_result(call_result_ent.clone()));
                                    call_result_ent
                                }
                                Type::Array { item_t } => {
                                    Entity::Null {
                                        uuid: self.function_context.new_uuid(),
                                        t: Type::Array { item_t: item_t.clone() }
                                    }
                                }
                                _ => unimplemented!(),
                            };
                            (entity, ident)
                        }
                        DeclItemKind::Init { ident, val } => {
                            // collect instructions from the expression
                            let original_ent = self.compile_expression(*val);
                            let entity = self.make_unique_entity(original_ent);
                            (entity, ident)
                        }
                    };

                    // use compiler environment to remember where the variable is stored
                    self.block_context.set_new_variable(ident, entity);
                }
            }
            StatementKind::Ass { r, expr } => {
                // collect instructions from the expression
                let original_ent = self.compile_expression(*expr);

                // get entity with a pointer to the referenced variable TODO: Refactor-out
                match &r.item {
                    ReferenceKind::Ident { ident } => {
                        // update local variable environment to reflect the change
                        let entity = self.make_unique_entity(original_ent);
                        self.block_context.update_variable(ident.clone(), entity);
                    }
                    ReferenceKind::TypedObject { obj, cls, field } => {
                        let struct_decl = self.global_context.get_struct_decl(cls);
                        let field_idx = struct_decl.field_env.get(field).unwrap();

                        let obj_ent = self.block_context.get_variable(obj);

                        // get pointer to the struct member
                        let ptr_t = r.get_type().reference();
                        let gep_instr = InstructionKind::GetStructElementPtr {
                            container_type_name: struct_decl.llvm_name(),
                            var: obj_ent,
                            idx: Entity::Int { v: *field_idx, uuid: 0 }
                        };
                        let gep_reg = self.function_context.new_register(ptr_t);
                        self.builder.push_instruction(gep_instr.with_result(gep_reg.clone()));

                        // store expression result at the pointer
                        let store_instr = InstructionKind::Store {
                            val: original_ent,
                            ptr: gep_reg
                        };
                        self.builder.push_instruction(store_instr.without_result());
                    }
                    ReferenceKind::Array { arr, idx } => {
                        // compile necessary getelementptr instructions
                        let idx_ent = self.compile_expression(idx.as_ref().clone());
                        let array_ent = self.block_context.get_variable(arr);
                        let gep_reg = self.compile_array_gep(
                            array_ent.get_array_item_t(),
                            array_ent,
                            idx_ent,
                        );

                        // store entity at the location of array index
                        let store_instruction = InstructionKind::Store {
                            val: original_ent,
                            ptr: gep_reg
                        };
                        self.builder.push_instruction(store_instruction.without_result());
                    }
                    t => unimplemented!()
                }
            }
            StatementKind::Mut { r, op } => {
                // get entity with a pointer to the referenced variable TODO: Refactor-out
                let var_ident = match &r.item {
                    ReferenceKind::Ident { ident } => ident.clone(),
                    t => unimplemented!(),
                };
                let var_ent = self.block_context.get_variable(&var_ident);

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
                let mut_result_ent = self.function_context.new_register(Type::Int);
                self.builder.push_instruction(mut_op.with_result(mut_result_ent.clone()));

                // update local variable environment to reflect the change
                self.block_context.update_variable(var_ident, mut_result_ent);
            }
            StatementKind::Return { expr } => {
                match expr {
                    None => {
                        let ret = InstructionKind::RetVoid;
                        self.builder.push_instruction(ret.without_result());
                    }
                    Some(e) => {
                        // compile expression
                        let result_ent = self.compile_expression(*e);

                        // return the value from register containing expression result
                        let ret = InstructionKind::RetVal { val: result_ent };
                        self.builder.push_instruction(ret.without_result());
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
                self.builder.push_instruction(cond_jump_instr.without_result());

                // collect information for phi calculation
                let in_label = self.builder.get_block_label();
                let in_env = self.block_context.get_env_view();

                // true branch
                self.next_block(true_label.clone());
                self.builder.add_predecessor(in_label.clone(), &in_env);
                self.compile_statement(*stmt);
                if !self.builder.block_always_returns() {
                    let jump_instr = InstructionKind::Jump { label: end_label.clone() };
                    self.builder.push_instruction(jump_instr.without_result());
                }

                // collect information for phi calculation
                let true_env = self.block_context.get_env_view();

                // end = new current block, with additional predecessor (in_block)
                self.next_block(end_label);
                self.builder.add_predecessor(in_label, &in_env);
                self.builder.add_predecessor(true_label, &true_env);
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
                self.builder.push_instruction(cond_jump_instr.without_result());

                // collect information for phi calculation
                let in_label = self.builder.get_block_label();
                let in_env = self.block_context.get_env_view();

                // prepare common jump instruction for both branches
                let end_jump_instr = InstructionKind::Jump { label: end_label.clone() }
                    .without_result();
                let mut next_block_necessary = false;

                // true branch
                self.next_block(true_label.clone());
                self.builder.add_predecessor(in_label.clone(), &in_env);
                self.compile_statement(*stmt_true);
                if !self.builder.block_always_returns() {
                    self.builder.push_instruction(end_jump_instr.clone());
                    next_block_necessary = true;
                }

                // collect information for phi calculation
                let true_env = self.block_context.get_env_view();

                // false branch
                self.next_block(false_label.clone());
                self.builder.add_predecessor(in_label.clone(), &in_env);
                self.compile_statement(*stmt_false);
                if !self.builder.block_always_returns() {
                    self.builder.push_instruction(end_jump_instr.clone());
                    next_block_necessary = true;
                }

                // collect information for phi calculation
                let false_env = self.block_context.get_env_view();

                // if either of the sides needed to jump to the end block, create it
                if next_block_necessary {
                    self.next_block(end_label);
                    self.builder.add_predecessor(true_label, &true_env);
                    self.builder.add_predecessor(false_label, &false_env);
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
                self.builder.push_instruction(cond_jump.clone());

                // collect information for phi calculation
                let in_label = self.builder.get_block_label();
                let in_env = self.block_context.get_env_view();

                // compile loop body, that also jumps back to cond at the end
                self.next_block(loop_label.clone());
                // we don't need to specify predecessor for body block
                self.compile_statement(*stmt);
                self.builder.push_instruction(cond_jump);

                // collect information for phi calculation
                let body_env = self.block_context.get_env_view();

                // begin next block by evaluating necessary phi statements based on block variables
                self.next_block(cond_label);
                self.builder.add_predecessor(in_label, &in_env);
                self.builder.add_predecessor(loop_label.clone(), &body_env);

                // evaluate condition in a new block and jump to loop body or end
                let expr_result = self.compile_expression(*expr);
                let loop_cond_jump = InstructionKind::JumpCond {
                    cond: expr_result,
                    true_label: loop_label,
                    false_label: end_label.clone(),
                };
                self.builder.push_instruction(loop_cond_jump.without_result());

                // start new block with loop end label, and fix mapping in 2 previous blocks
                self.complete_loop_block(end_label);
            }
            StatementKind::For { t, ident, arr, stmt } => {
                let suffix = self.global_context.new_label_suffix();
                let cond_label = format!("__for_cond__{}", suffix);
                let body_label = format!("__for_body__{}", suffix);
                let end_label = format!("__for_end__{}", suffix);

                // create variable i (index), dot prefix prevents conflicts with user variables
                let index_ident = format!(".__i__{}", self.function_context.new_uuid());
                let index_ent = Entity::Int { v: 0, uuid: self.function_context.new_uuid() };
                self.block_context.set_new_variable(index_ident.clone(), index_ent.clone());

                // create entity holding a pointer to the array
                let array_ent = self.compile_expression(*arr);
                let array_item_t = array_ent.get_array_item_t();

                // jump to condition evaluation sequence
                let cond_jump = InstructionKind::Jump { label: cond_label.clone() }
                    .without_result();
                self.builder.push_instruction(cond_jump.clone());

                // collect information for phi calculation
                let in_label = self.builder.get_block_label();
                let in_env = self.block_context.get_env_view();

                // compile loop body, that also jumps back to cond at the end
                // we don't need to specify predecessor for body block
                self.next_block(body_label.clone());

                // get pointer to the current item from an array
                let gep_reg = self.compile_array_gep(
                    array_item_t.clone(),
                    array_ent.clone(),
                    self.block_context.get_variable(&index_ident)
                );

                // load current item from the pointer and update environment for the loop body
                let load_reg = self.function_context.new_register(array_item_t.clone());
                let load_instr = InstructionKind::Load { ptr: gep_reg };
                self.builder.push_instruction(load_instr.with_result(load_reg.clone()));
                // TODO: Cast will be necessary here when inheritance is implemented
                self.block_context.set_new_variable(ident.clone(), load_reg);

                // compile block body
                self.compile_statement(*stmt);

                // increment index
                let increment_instr = InstructionKind::BinaryOp {
                    op: BinaryOperator::Plus,
                    l: self.block_context.get_variable(&index_ident),
                    r: Entity::Int { v: 1, uuid: 0 }
                };
                let increment_ent = self.function_context.new_register(Type::Int);
                self.builder.push_instruction(increment_instr.with_result(increment_ent.clone()));
                self.block_context.update_variable(index_ident.clone(), increment_ent);

                // jump to loop cond evaluation block
                self.builder.push_instruction(cond_jump);

                // remove item variable so that no excessive phi nodes are generated
                self.block_context.remove_variable(&ident);

                // collect information for phi calculation
                let body_env = self.block_context.get_env_view();

                // begin next block by evaluating necessary phi statements based on block variables
                self.next_block(cond_label);
                self.builder.add_predecessor(in_label, &in_env);
                self.builder.add_predecessor(body_label.clone(), &body_env);

                // get array length
                let length_gep_instr = InstructionKind::GetStructElementPtr {
                    container_type_name: self.global_context.get_or_declare_array_struct(&array_item_t).llvm_name(),
                    var: array_ent,
                    idx: Entity::Int { v: 0, uuid: 0 }
                };
                let length_gep_reg = self.function_context.new_register(Type::Int.reference());
                self.builder.push_instruction(length_gep_instr.with_result(length_gep_reg.clone()));

                let length_load_reg = self.function_context.new_register(Type::Int);
                let length_load_instr = InstructionKind::Load { ptr: length_gep_reg };
                self.builder.push_instruction(length_load_instr.with_result(length_load_reg.clone()));

                // evaluate loop condition (index < length) and jump
                let cmp_instr = InstructionKind::BinaryOp {
                    op: BinaryOperator::Less,
                    l: self.block_context.get_variable(&index_ident),
                    r: length_load_reg
                };
                let cmp_result = self.function_context.new_register(Type::Bool);
                self.builder.push_instruction(cmp_instr.with_result(cmp_result.clone()));

                let loop_cond_jump = InstructionKind::JumpCond {
                    cond: cmp_result,
                    true_label: body_label,
                    false_label: end_label.clone(),
                };
                self.builder.push_instruction(loop_cond_jump.without_result());

                // start new block with loop end label, and fix mapping in 2 previous blocks
                self.complete_loop_block(end_label);
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
        // collect function argument info
        let mut args = Vec::new();
        for (arg_reg, arg) in function.item.args.iter().enumerate() {
            // collect variable types in case the caller needs to cast the passed arguments later
            let arg_type = arg.get_type().clone();
            let mapped_arg_item = ArgItem {
                t: arg_type.clone(),
                ident: arg.item.ident.clone()
            };
            args.push(mapped_arg_item);

            // mark location of the variable in the block environment
            let arg_ent = Entity::NamedRegister {
                name: arg.item.ident.clone(),
                t: arg_type,
            };
            self.block_context.set_new_variable(arg.item.ident.clone(), arg_ent);
        }

        // compile function instructions using the nested compiler
        for stmt in function.item.block.item.stmts {
            self.compile_statement(*stmt);
        }

        // finish last block within the nested compiler
        let block = self.builder.build(&mut self.function_context);
        self.function_context.push_block(block);

        let ret_type = function.item.ret.clone();

        // build the LLVM function
        let llvm_function = FunctionDef {
            name: self.global_context.get_function_name(&function.item.ident),
            ret_type,
            args,
            body: self.function_context.conclude(),
        };

        llvm_function
    }
}
