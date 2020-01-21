use std::iter::FromIterator;

use crate::frontend::ast::*;
use crate::frontend::error::{FrontendError, FrontendErrorKind};
use crate::frontend::typechecker::typechecker::TypeChecker;
use crate::frontend::typechecker::util::ToTypeEnv;
use crate::meta::{GetLocation, GetType, LocationMeta, TypeMeta};
use crate::util::env::Env;
use crate::util::mapper::AstMapper;

pub type TypeCheckResult<AstT> = Result<AstT, Vec<FrontendError<LocationMeta>>>;

impl AstMapper<LocationMeta, TypeMeta, FrontendError<LocationMeta>> for TypeChecker<'_> {
    fn map_var_reference(&mut self, r: &Reference<LocationMeta>) -> TypeCheckResult<Reference<TypeMeta>> {
        let loc = r.get_meta();
        let typecheck_result = match &r.item {
            ReferenceKind::Ident { ident } => {
                let var_t = self.get_variable(ident, loc)?;
                // if the variable actually refers to current class member, mark it explicitly
                // this is necessary for compiler backend to assign correct operation later
                if self.is_class_variable(ident) {
                    // we use unwrap on current class because is_class_variable implies it exists
                    let typed_self_reference = ReferenceKind::TypedObject {
                        obj: String::from("self"),
                        cls: self.get_current_class().unwrap().item.get_key().clone(),
                        field: ident.clone()
                    };
                    Ok((typed_self_reference, var_t.clone()))
                } else {
                    Ok((ReferenceKind::Ident { ident: ident.clone() }, var_t.clone()))
                }
            }
            ReferenceKind::Object { obj, field } => {
                let var_t = self.get_variable(obj, loc)?;
                if let Type::Array { item_t } = var_t {
                    // manually check field name and convert ReferenceKind to ArrayLen
                    if field == "length" {
                        Ok((ReferenceKind::ArrayLen { ident: obj.clone() }, Type::Int))
                    } else {
                        let kind = FrontendErrorKind::EnvError {
                            message: format!("Invalid instance variable for array: {}", field)
                        };
                        Err(vec![FrontendError::new(kind, loc.clone())])
                    }
                } else {
                    // interpret var_t as object and try to get the `field` instance variable
                    let cls = self.get_class(var_t, loc)?;
                    let field_t = self.get_instance_variable(cls, field, loc)?;

                    // map reference to TypedObject, backend needs to know the name of the class
                    let mapped_ref = ReferenceKind::TypedObject {
                        obj: obj.clone(),
                        cls: cls.item.get_key().clone(),
                        field: field.clone()
                    };
                    Ok((mapped_ref, field_t.clone()))
                }
            }
            ReferenceKind::ObjectSelf { field } => {
                if let Some(cls) = self.get_current_class() {
                    let field_t = self.get_instance_variable(cls, field, loc)?;
                    let typed_self_reference = ReferenceKind::TypedObject {
                        obj: String::from("self"),
                        cls: cls.item.get_key().clone(),
                        field: field.clone()
                    };
                    Ok((typed_self_reference, field_t.clone()))
                } else {
                    let kind = FrontendErrorKind::EnvError {
                        message: String::from("No object in the current context")
                    };
                    Err(vec![FrontendError::new(kind, loc.clone())])
                }
            }
            ReferenceKind::Array { arr, idx } => {
                // TODO: Refactor to 2 simpler methods (like class & member)
                let var_t = self.get_variable(arr, loc)?;
                if let Type::Array { item_t } = var_t {
                    let item_t = *item_t.clone();
                    let mapped_expr = self.map_expression(idx)?;
                    let mapped_t = &mapped_expr.get_meta().t;
                    if *mapped_t == Type::Int {
                        Ok((ReferenceKind::Array { arr: arr.clone(), idx: Box::new(mapped_expr) }, item_t))
                    } else {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Int,
                            actual: mapped_t.clone(),
                        };
                        Err(vec![FrontendError::new(kind, loc.clone())])
                    }
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: Type::Array { item_t: Box::new(Type::Any) },
                        actual: var_t.clone(),
                    };
                    Err(vec![FrontendError::new(kind, loc.clone())])
                }
            }
            ReferenceKind::ArrayLen { ident } => {
                // parser never creates these, ArrayLen can only be a result of a transformation
                unreachable!();
            }
            ReferenceKind::TypedObject { obj: _, cls: _, field: _ } => { unreachable!() }
        };
        match typecheck_result {
            Ok((kind, t)) => {
                let mapped: Reference<TypeMeta> = Reference::new(kind, TypeMeta { t });
                Ok(mapped)
            }
            Err(err_vec) => Err(err_vec),
        }
    }

    fn map_func_reference(&mut self, r: &Reference<LocationMeta>) -> TypeCheckResult<Reference<TypeMeta>> {
        let loc = r.get_meta();
        let typecheck_result = match &r.item {
            ReferenceKind::Ident { ident } => {
                let func_t = self.get_func(ident, loc)?;
                Ok((ReferenceKind::Ident { ident: ident.clone() }, func_t.clone()))
            }
            ReferenceKind::Object { obj, field } => {
                let var_t = self.get_variable(obj, loc)?;
                let cls = self.get_class(var_t, loc)?;
                let method_t = self.get_method(cls, field, loc)?;

                let typed_reference = ReferenceKind::TypedObject {
                    obj: obj.clone(),
                    cls: cls.item.get_key().clone(),
                    field: field.clone()
                };
                Ok((typed_reference, method_t.clone()))
            }
            ReferenceKind::ObjectSelf { field } => {
                if let Some(cls) = self.get_current_class() {
                    let method_t = self.get_method(cls, field, loc)?;
                    let typed_self_reference = ReferenceKind::TypedObject {
                        obj: String::from("self"),
                        cls: cls.item.get_key().clone(),
                        field: field.clone()
                    };
                    Ok((typed_self_reference, method_t.clone()))
                } else {
                    let kind = FrontendErrorKind::EnvError {
                        message: String::from("No object in the current context")
                    };
                    Err(vec![FrontendError::new(kind, loc.clone())])
                }
            }
            r => {
                let kind = FrontendErrorKind::ArgumentError {
                    message: format!("Expected function or method, got: {:?}", r)
                };
                Err(vec![FrontendError::new(kind, loc.clone())])
            }
        };
        // TODO: Refactor to separate function (or better: From trait)
        match typecheck_result {
            Ok((kind, t)) => {
                let mapped: Reference<TypeMeta> = Reference::new(kind, TypeMeta { t });
                Ok(mapped)
            }
            Err(err_vec) => Err(err_vec),
        }
    }

    fn map_block(&mut self, block: &Block<LocationMeta>) -> TypeCheckResult<Block<TypeMeta>> {
        let mut mapped_stmts = Vec::new();
        let mut errors = Vec::new();
        for block_stmt in block.item.stmts.iter() {
            match self.map_statement(&block_stmt) {
                Ok(mapped_stmt) => {
                    mapped_stmts.push(Box::new(mapped_stmt));
                }
                Err(mut v) => {
                    errors.append(&mut v);
                }
            }
        }
        if errors.is_empty() {
            // return type is always determined by last statement thanks to the BlockOrganizer
            let return_t = match mapped_stmts.last() {
                Some(stmt) => stmt.get_meta().clone(),
                None => TypeMeta { t: Type::Void },
            };
            let item = BlockItem::<TypeMeta> { stmts: mapped_stmts };
            Ok(Block::new(item, return_t.clone()))
        } else {
            Err(errors)
        }
    }

    fn map_expression(&mut self, expr: &Expression<LocationMeta>) -> TypeCheckResult<Expression<TypeMeta>> {
        let typecheck_result = match &expr.item {
            ExpressionKind::LitInt { val } => {
                Ok((ExpressionKind::LitInt { val: val.clone() }, Type::Int))
            }
            ExpressionKind::LitBool { val } => {
                Ok((ExpressionKind::LitBool { val: val.clone() }, Type::Bool))
            }
            ExpressionKind::LitStr { val } => {
                Ok((ExpressionKind::LitStr { val: val.clone() }, Type::Str))
            }
            ExpressionKind::LitNull => {
                Ok((ExpressionKind::LitNull, Type::Null))
            }
            ExpressionKind::App { r, args } => {
                let mapped_r = self.map_func_reference(&r)?;
                match &mapped_r.get_meta().t {
                    Type::Function { args: exp_args, ret } => {
                        let mut mapped_args = Vec::new();
                        let mut errors: Vec<FrontendError<LocationMeta>> = Vec::new();
                        if exp_args.len() != args.len() {
                            let kind = FrontendErrorKind::ArgumentError {
                                message: format!(
                                    "Incorrect argument count, expected {} got {}",
                                    exp_args.len(),
                                    args.len()
                                )
                            };
                            errors.push(FrontendError::new(kind, r.get_location()));
                        } else {
                            for (expected_arg_type, arg_expr) in exp_args.iter().zip(args.iter()) {
                                match self.map_expression(&arg_expr) {
                                    Ok(mapped_arg) => {
                                        let assignment_check = self.check_assignment(
                                            &expected_arg_type,
                                            &mapped_arg.get_meta().t,
                                        );
                                        if let Err(kind) = assignment_check {
                                            errors.push(FrontendError::new(kind, arg_expr.get_location()));
                                        } else {
                                            mapped_args.push(Box::new(mapped_arg));
                                        }
                                    }
                                    Err(mut err_vec) => {
                                        errors.append(&mut err_vec);
                                    }
                                }
                            }
                        }
                        if errors.is_empty() {
                            let t = *ret.clone();
                            Ok((ExpressionKind::App { r: mapped_r, args: mapped_args }, t))
                        } else {
                            Err(errors)
                        }
                    }
                    t => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Function { args: vec![], ret: Box::new(Type::Any) },
                            actual: t.clone(),
                        };
                        Err(vec![FrontendError::new(kind, r.get_location())])
                    }
                }
            }
            ExpressionKind::Unary { op, arg } => {
                let op_t = match op {
                    UnaryOperator::Neg => Type::Int,
                    UnaryOperator::Not => Type::Bool,
                };
                let mapped_arg = self.map_expression(&arg)?;
                let t = mapped_arg.get_type();
                if t == op_t {
                    Ok((ExpressionKind::Unary { op: op.clone(), arg: Box::new(mapped_arg) }, t))
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: op_t,
                        actual: mapped_arg.get_type(),
                    };
                    Err(vec![FrontendError::new(kind, arg.get_location())])
                }
            }
            ExpressionKind::Binary { left, op, right } => {
                // TODO: Collect errors from both sides before terminating
                let mapped_l = self.map_expression(&left)?;
                let mapped_r = self.map_expression(&right)?;

                if mapped_l.get_meta() == mapped_r.get_meta() {
                    let left_t = mapped_l.get_type();
                    let op_result_t = match op {
                        BinaryOperator::Equal | BinaryOperator::NotEqual => {
                            Option::Some(Type::Bool)
                        }
                        BinaryOperator::Plus => {
                            if left_t == Type::Str || left_t == Type::Int {
                                Option::Some(left_t.clone())
                            } else {
                                Option::None
                            }
                        }
                        BinaryOperator::And | BinaryOperator::Or => {
                            if left_t == Type::Bool {
                                Option::Some(Type::Bool)
                            } else {
                                Option::None
                            }
                        }
                        BinaryOperator::Greater
                        | BinaryOperator::GreaterEqual
                        | BinaryOperator::LessEqual
                        | BinaryOperator::Less => {
                            if left_t == Type::Int {
                                Option::Some(Type::Bool)
                            } else {
                                Option::None
                            }
                        }
                        _ => {
                            if left_t == Type::Int {
                                Option::Some(Type::Int)
                            } else {
                                Option::None
                            }
                        }
                    };
                    if let Some(result_t) = op_result_t {
                        let mapped_expr = ExpressionKind::Binary {
                            left: Box::new(mapped_l),
                            op: op.clone(),
                            right: Box::new(mapped_r),
                        };
                        Ok((mapped_expr, result_t))
                    } else {
                        let kind = FrontendErrorKind::ArgumentError {
                            message: format!(
                                "Invalid argument type {:?} for operator {:?}",
                                mapped_l.get_type(),
                                op
                            )
                        };
                        Err(vec![FrontendError::new(kind, left.get_location())])
                    }
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: mapped_l.get_type(),
                        actual: mapped_r.get_type(),
                    };
                    Err(vec![FrontendError::new(kind, right.get_location())])
                }
            }
            ExpressionKind::InitDefault { t } => {
                Ok((ExpressionKind::InitDefault { t: t.clone() }, t.clone()))
            }
            ExpressionKind::InitArr { t, size } => {
                let mapped_size = self.map_expression(&size)?;
                if mapped_size.get_meta().t == Type::Int {
                    // in this scope, t is always an array (by parser definition)
                    // + the type that we return will always be checked by the caller
                    Ok((ExpressionKind::InitArr { t: t.clone(), size: Box::new(mapped_size) }, t.clone()))
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: Type::Int,
                        actual: mapped_size.get_type(),
                    };
                    Err(vec![FrontendError::new(kind, size.get_location())])
                }
            }
            ExpressionKind::Reference { r } => {
                let mapped_ref = self.map_var_reference(r)?;
                let t = mapped_ref.get_type();
                Ok((ExpressionKind::Reference { r: mapped_ref }, t))
            }
            ExpressionKind::Cast { t, expr } => {
                let mapped_expr = self.map_expression(&expr)?;
                if mapped_expr.get_meta().t == Type::Null {
                    let kind = ExpressionKind::Cast { t: t.clone(), expr: Box::new(mapped_expr) };
                    Ok((kind, t.clone()))
                } else {
                    // for now we only allow the null type to be casted
                    let kind = FrontendErrorKind::TypeError {
                        expected: Type::Null,
                        actual: mapped_expr.get_type(),
                    };
                    Err(vec![FrontendError::new(kind, expr.get_location())])
                }
            }
            ExpressionKind::Error => {
                unreachable!()
            }
        };
        match typecheck_result {
            Ok((kind, t)) => {
                let mapped: Expression<TypeMeta> = Expression::new(kind, TypeMeta { t });
                Ok(mapped)
            }
            Err(err_vec) => Err(err_vec),
        }
    }

    fn map_statement(&mut self, stmt: &Statement<LocationMeta>) -> TypeCheckResult<Statement<TypeMeta>> {
        match &stmt.item {
            StatementKind::Block { block } => {
                let mut typechecker = self.with_nested_env(Env::new());
                let mapped_block = typechecker.map_block(&block)?;
                let meta = mapped_block.get_meta().clone();
                let kind = StatementKind::Block { block: mapped_block };
                Ok(Statement::new(kind, meta))
            }
            StatementKind::Empty => {
                Ok(Statement::new(StatementKind::Empty, TypeMeta { t: Type::Void }))
            }
            StatementKind::Decl { items, t } => {
                let mut errors = Vec::new();
                let mut mapped_declitems = Vec::new();
                for declitem in items.iter() {
                    // TODO: Refactor to separate function: map declitem?
                    match &declitem.item {
                        DeclItemKind::NoInit { ident } => {
                            // check for duplicate variable declaration
                            if self.local_decl.contains(ident) {
                                let err = FrontendErrorKind::EnvError {
                                    message: format!("Duplicated declaration of {}", ident)
                                };
                                errors.push(FrontendError::new(err, declitem.get_location()));
                            } else {
                                // define the variable
                                self.local_env.insert(ident.clone(), t.clone());
                                self.local_decl.insert(ident.clone());

                                let kind = DeclItemKind::NoInit { ident: ident.clone() };
                                mapped_declitems.push(DeclItem::new(kind, TypeMeta { t: t.clone() }))
                            }
                        }
                        DeclItemKind::Init { ident, val } => {
                            let loc = val.get_location();

                            // check for duplicate variable declaration
                            if self.local_decl.contains(ident) {
                                let err = FrontendErrorKind::EnvError {
                                    message: format!("Duplicated declaration of {}", ident)
                                };
                                errors.push(FrontendError::new(err, declitem.get_location()));
                                continue;
                            }

                            // check expression and define the variable
                            match self.map_expression(&val) {
                                Ok(mapped_expr) => {
                                    let expr_t = &mapped_expr.get_meta().t;
                                    match self.check_assignment(&t, expr_t) {
                                        Ok(_) => {
                                            self.local_env.insert(ident.clone(), t.clone());
                                            let kind = DeclItemKind::Init {
                                                ident: ident.clone(),
                                                val: Box::new(mapped_expr.clone()),
                                            };
                                            mapped_declitems.push(DeclItem::new(
                                                kind,
                                                TypeMeta { t: t.clone() },
                                            ));
                                        }
                                        Err(kind) => {
                                            errors.push(FrontendError::new(kind, loc));
                                        }
                                    }
                                }
                                Err(mut v) => {
                                    errors.append(&mut v);
                                }
                            }
                        }
                    };
                }
                if errors.is_empty() {
                    let kind = StatementKind::Decl { t: t.clone(), items: mapped_declitems };
                    Ok(Statement::new(kind, TypeMeta { t: Type::Void }))
                } else {
                    Err(errors)
                }
            }
            StatementKind::Ass { r, expr } => {
                let expr_loc = expr.get_location();
                // TODO: Collect errors from both expression and the reference before failing
                let mapped_expr = self.map_expression(&expr)?;
                let mapped_ref = self.map_var_reference(&r)?;

                let ref_t = &mapped_ref.get_meta().t;
                let expr_t = &mapped_expr.get_meta().t;
                match self.check_assignment(&ref_t, &expr_t) {
                    Ok(_) => {
                        // assignment is not an expression, doesn't have a return value
                        let kind = StatementKind::Ass { r: mapped_ref, expr: Box::new(mapped_expr) };
                        let meta = TypeMeta { t: Type::Void };
                        Ok(Statement::new(kind, meta))
                    }
                    Err(kind) => {
                        Err(vec![FrontendError::new(kind, expr_loc)])
                    }
                }
            }
            StatementKind::Mut { r, op } => {
                let mapped_ref = self.map_var_reference(r)?;
                let target_t = &mapped_ref.get_meta().t;

                // ++ and -- expressions can only be performed on integer types
                match target_t {
                    Type::Int => {
                        // ++ and -- are not expressions, they don't have a return value
                        let kind = StatementKind::Mut { r: mapped_ref, op: op.clone() };
                        let meta = TypeMeta { t: Type::Void };
                        Ok(Statement::new(kind, meta))
                    }
                    t => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Int,
                            actual: t.clone(),
                        };
                        Err(vec![FrontendError::new(kind, r.get_location())])
                    }
                }
            }
            StatementKind::Return { expr } => {
                match expr {
                    None => {
                        let kind = StatementKind::Return { expr: Option::None };
                        let meta = TypeMeta { t: Type::Void };
                        Ok(Statement::new(kind, meta))
                    }
                    Some(expr) => {
                        let mapped_expr = self.map_expression(&expr)?;
                        let t = mapped_expr.get_type();
                        let kind = StatementKind::Return { expr: Some(Box::new(mapped_expr)) };
                        let meta = TypeMeta { t };
                        Ok(Statement::new(kind, meta))
                    }
                }
            }
            StatementKind::Cond { expr, stmt } => {
                // TODO: Collect errors from both expression and statement before failing
                let mapped_expr = self.map_expression(&expr)?;
                match &mapped_expr.get_meta().t {
                    Type::Bool => {
                        let mapped_stmt = self.map_statement(&stmt)?;
                        let t = mapped_stmt.get_type();
                        let kind = StatementKind::Cond {
                            expr: Box::new(mapped_expr),
                            stmt: Box::new(mapped_stmt),
                        };
                        let meta = TypeMeta { t };
                        Ok(Statement::new(kind, meta))
                    }
                    t => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Bool,
                            actual: t.clone(),
                        };
                        Err(vec![FrontendError::new(kind, expr.get_location())])
                    }
                }
            }
            StatementKind::CondElse { expr, stmt_true, stmt_false } => {
                let mapped_expr = self.map_expression(&expr)?;
                match &mapped_expr.get_meta().t {
                    Type::Bool => {
                        // TODO: Collect errors from both statements before failing
                        let mapped_true = self.map_statement(&stmt_true)?;
                        let mapped_false = self.map_statement(&stmt_false)?;

                        let true_t = &mapped_true.get_meta().t;
                        let false_t = &mapped_false.get_meta().t;
                        match self.get_types_lca(&true_t, &false_t) {
                            Some(lca_t) => {
                                let kind = StatementKind::CondElse {
                                    expr: Box::new(mapped_expr),
                                    stmt_true: Box::new(mapped_true),
                                    stmt_false: Box::new(mapped_false),
                                };
                                let meta = TypeMeta { t: lca_t };
                                Ok(Statement::new(kind, meta))
                            }
                            None => {
                                // technically, if this is not the last statement in block we could
                                // allow retuning from one branch and not returning from other branch
                                // ie: if (cond) { return 10; } else {} return 20;
                                // but this is not a good practice and we want to encourage either:
                                // int i; if (cond) { i = 10; } else {i = 20;} return i;
                                // or:
                                // if (cond) {return 10;} else {return 20;}
                                // so I decided to report it as a typing error
                                let kind = FrontendErrorKind::TypeError {
                                    expected: true_t.clone(),
                                    actual: false_t.clone(),
                                };
                                Err(vec![FrontendError::new(kind, stmt_false.get_location())])
                            }
                        }
                    }
                    t => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Bool,
                            actual: t.clone(),
                        };
                        Err(vec![FrontendError::new(kind, expr.get_location())])
                    }
                }
            }
            StatementKind::While { expr, stmt } => {
                let mapped_expr = self.map_expression(&expr)?;
                let mut typechecker = self.with_nested_env(Env::new());
                let mapped_stmt = typechecker.map_statement(&stmt)?;
                let kind = StatementKind::While {
                    expr: Box::new(mapped_expr),
                    stmt: Box::new(mapped_stmt),
                };
                Ok(Statement::new(kind, TypeMeta { t: Type::Void }))
            }
            StatementKind::For { t, ident, arr, stmt } => {
                let mapped_arr = self.map_expression(&arr)?;
                let arr_t = &mapped_arr.get_meta().t;
                // check if arr is an array
                match arr_t {
                    Type::Array { item_t } => {
                        // check array item type
                        match self.check_assignment(&t, &item_t) {
                            Ok(_) => {
                                // check loop statement with nested environemnt
                                let mut loop_env = Env::new();
                                loop_env.insert(ident.clone(), t.clone());
                                let mut typechecker = self.with_nested_env(loop_env);
                                let mapped_stmt = typechecker.map_statement(&stmt)?;

                                let kind = StatementKind::For {
                                    t: t.clone(),
                                    ident: ident.clone(),
                                    arr: Box::new(mapped_arr),
                                    stmt: Box::new(mapped_stmt),
                                };
                                Ok(Statement::new(kind, TypeMeta { t: Type::Void }))
                            }
                            Err(kind) => {
                                Err(vec![FrontendError::new(kind, arr.get_location())])
                            }
                        }
                    }
                    invalid_arr_t => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Array { item_t: Box::new(t.clone()) },
                            actual: invalid_arr_t.clone(),
                        };
                        Err(vec![FrontendError::new(kind, arr.get_location())])
                    }
                }
            }
            StatementKind::Expr { expr } => {
                let mapped_expr = self.map_expression(&expr)?;
                let kind = StatementKind::Expr { expr: Box::new(mapped_expr) };
                Ok(Statement::new(kind, TypeMeta { t: Type::Void }))
            }
            StatementKind::Error => {
                unreachable!()
            }
        }
    }

    fn map_class(&mut self, class: &Class<LocationMeta>) -> TypeCheckResult<Class<TypeMeta>> {
        let mut typechecker = self.with_class(class);

        // variables are mapped by swapping meta, no errors can happen here
        let mut mapped_vars: Vec<_> = class.item.vars
            .values()
            .map(|cv| {
                let meta = TypeMeta { t: cv.item.t.clone() };
                ClassVar::new(cv.item.clone(), meta)
            })
            .collect();

        // every method needs to be checked for type correctness
        let (mapped_methods, errors): (Vec<_>, Vec<_>) = class.item.methods
            .values()
            .map(|func| {
                typechecker.map_function(&func)
            })
            .partition(Result::is_ok);
        let mut mapped_methods: Vec<Function<TypeMeta>> = mapped_methods
            .into_iter()
            .map(Result::unwrap)
            .collect();
        let errors: Vec<FrontendError<LocationMeta>> = errors
            .into_iter()
            .map(Result::unwrap_err)
            .flatten()
            .collect();

        if errors.is_empty() {
            if let Ok(mut item) = ClassItem::new(
                class.item.get_key().clone(),
                &mut mapped_vars,
                &mut mapped_methods,
            ) {
                if let Some(parent) = &class.item.parent {
                    item = item.with_parent(parent);
                }
                Ok(Class::new(item, TypeMeta { t: Type::Class { ident: class.item.get_key().clone() } }))
            } else {
                // because we only transformed metadata in envs, we know creation cannot fail
                unreachable!()
            }
        } else {
            Err(errors)
        }
    }

    fn map_function(&mut self, function: &Function<LocationMeta>) -> TypeCheckResult<Function<TypeMeta>> {
        let mapped_args: Vec<Arg<TypeMeta>> = function.item.args.iter()
            .map(|arg| {
                Arg::new(arg.item.clone(), TypeMeta { t: arg.item.t.clone() })
            })
            .collect();

        let mut typechecker = self.with_nested_env(function.to_type_env());
        let mapped_block = typechecker.map_block(&function.item.block)?;

        match self.check_assignment(&function.item.ret, &mapped_block.get_meta().t) {
            Ok(_) => {
                if let Ok(item) = FunctionItem::new(
                    function.item.ret.clone(),
                    function.item.ident.clone(),
                    mapped_args,
                    mapped_block,
                ) {
                    Ok(Function::new(item.clone(), TypeMeta { t: item.get_type() }))
                } else {
                    // because we only transformed metadata in envs, we know creation cannot fail
                    unreachable!()
                }
            }
            Err(kind) => {
                Err(vec![FrontendError::new(kind, function.get_location())])
            }
        }
    }
}
