use crate::{ast, context, diagnostic, dispatch};

pub type TypeCheckResult = Option<Vec<diagnostic::Diagnostic>>;

pub struct TypeCheckContext {
  pub diagnostics: diagnostic::DiagnosticBuilder,
  in_loop: bool,
  in_unsafe_block: bool,
  does_function_return: bool,
}

impl TypeCheckContext {
  pub fn new() -> Self {
    Self {
      diagnostics: diagnostic::DiagnosticBuilder::new(),
      in_loop: false,
      in_unsafe_block: false,
      does_function_return: false,
    }
  }

  fn _unify_types() {
    // TODO: Implement.
  }
}

pub trait TypeCheck {
  fn infer_type(&self, _context: &context::Context) -> Option<ast::Type> {
    None
  }

  fn type_check(&self, _type_context: &mut TypeCheckContext, _context: &mut context::Context) {
    //
  }
}

impl TypeCheck for ast::Node {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    dispatch!(self, TypeCheck::type_check, type_context, context);
  }

  fn infer_type(&self, context: &context::Context) -> Option<ast::Type> {
    dispatch!(self, TypeCheck::infer_type, context)
  }
}

impl TypeCheck for ast::Prototype {
  fn type_check(&self, _type_context: &mut TypeCheckContext, _context: &mut context::Context) {
    // TODO: Implement?
  }
}

impl TypeCheck for ast::StructDef {
  // TODO: Implement.
}

impl TypeCheck for ast::UnaryExpr {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    match self.operator {
      ast::OperatorKind::MultiplyOrDereference => {
        if !type_context.in_unsafe_block {
          type_context
            .diagnostics
            .error("can only dereference inside an unsafe block".to_string());
        }
      }
      ast::OperatorKind::Not => {
        let expr_type = self.expr.infer_type(context);

        if expr_type != Some(ast::Type::Primitive(ast::PrimitiveType::Bool)) {
          type_context
            .diagnostics
            .error("can only negate boolean expressions".to_string());
        }
      }
      ast::OperatorKind::SubtractOrNegate => {
        let expr_type = self.expr.infer_type(context);

        // TODO: Include floats.
        if !matches!(
          expr_type,
          Some(ast::Type::Primitive(ast::PrimitiveType::Int(_)))
        ) {
          // TODO: Error message too similar to the boolean negation case.
          type_context
            .diagnostics
            .error("can only negate integers or float expressions".to_string());
        }
      }
      ast::OperatorKind::AddressOf => {
        // TODO: Implement.
        todo!();
      }
      _ => unreachable!(),
    };
  }

  fn infer_type(&self, context: &context::Context) -> Option<ast::Type> {
    self.expr.infer_type(context)
  }
}

impl TypeCheck for ast::Enum {
  //
}

impl TypeCheck for ast::LValueAssignStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    let lvalue_type = self.lvalue_expr.infer_type(context);

    if !matches!(lvalue_type, Some(ast::Type::Pointer(_))) {
      type_context
        .diagnostics
        .error("assignment lvalue must be a pointer".to_string());
    }
  }
}

impl TypeCheck for ast::ContinueStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, _context: &mut context::Context) {
    if !type_context.in_loop {
      type_context
        .diagnostics
        .error("continue statement may only occur inside loops".to_string());
    }
  }
}

impl TypeCheck for ast::ArrayIndexing {
  // TODO: Infer type.

  fn type_check(&self, _type_context: &mut TypeCheckContext, _context: &mut context::Context) {
    // TODO: Implement.
  }
}

impl TypeCheck for ast::ArrayValue {
  // TODO: Implement. Ensure all values are of the same type.
}

impl TypeCheck for ast::UnsafeBlockStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    // TODO: To avoid problems with nested cases, save a buffer here, then restore?
    type_context.in_unsafe_block = true;
    self.0.type_check(type_context, context);
    type_context.in_unsafe_block = false;
  }
}

impl TypeCheck for ast::Extern {
  //
}

impl TypeCheck for ast::Parameter {
  //
}

impl TypeCheck for ast::Block {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    for statement in &self.statements {
      statement.type_check(type_context, context);
    }
  }
}

impl TypeCheck for ast::VariableRef {
  fn infer_type(&self, context: &context::Context) -> Option<ast::Type> {
    let target_variable = &*context
      .declarations
      .get(&self.definition_key.unwrap())
      .unwrap()
      .as_ref()
      .borrow();

    // TODO: Simplify code.
    let variable_type = match target_variable {
      ast::Node::LetStmt(let_stmt) => &let_stmt.ty,
      ast::Node::Parameter(parameter) => &parameter.1,
      _ => unreachable!(),
    };

    Some(variable_type.clone())
  }
}

impl TypeCheck for ast::Literal {
  fn infer_type(&self, _context: &context::Context) -> Option<ast::Type> {
    Some(ast::Type::Primitive(match self {
      ast::Literal::Bool(_) => ast::PrimitiveType::Bool,
      ast::Literal::Char(_) => ast::PrimitiveType::Char,
      ast::Literal::Int(_, size) => ast::PrimitiveType::Int(size.clone()),
      ast::Literal::String(_) => ast::PrimitiveType::String,
    }))
  }
}

impl TypeCheck for ast::IfStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    if self.condition.infer_type(context) != Some(ast::Type::Primitive(ast::PrimitiveType::Bool)) {
      type_context
        .diagnostics
        .error("if statement condition must evaluate to a boolean".to_string());
    }
  }
}

impl TypeCheck for ast::BinaryExpr {
  fn infer_type(&self, context: &context::Context) -> Option<ast::Type> {
    // TODO: What if the binary expression is comparing? Then it would be bool, not the type of the left arm.
    self.left.infer_type(context)
  }

  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    let left_type = self.left.infer_type(context);
    let right_type = self.right.infer_type(context);

    // TODO: Also add checks for when using operators with wrong values (ex. less-than or greater-than comparison of booleans).

    // FIXME: Does this equality check work as expected? Research & ensure.
    if left_type != right_type {
      type_context
        .diagnostics
        .error("binary expression operands must be the same type".to_string());

      return;
    }

    // TODO: Check for mixed operators that don't make sense (ex. addition, then a comparison operator).

    // NOTE: By this point, it is assumed that both operands are of the same type.
    match self.operator {
      ast::OperatorKind::Add
      | ast::OperatorKind::SubtractOrNegate
      | ast::OperatorKind::MultiplyOrDereference
      | ast::OperatorKind::Divide
      | ast::OperatorKind::LessThan
      | ast::OperatorKind::GreaterThan => {
        // TODO: What about floats?
        if !matches!(
          left_type,
          Some(ast::Type::Primitive(ast::PrimitiveType::Int(_)))
        ) {
          type_context
            .diagnostics
            .error("binary expression operands must be both integers".to_string());
        }
      }
      // TODO: Equality operator, and others?
      _ => todo!(),
    };

    self.left.type_check(type_context, context);
    self.right.type_check(type_context, context);
  }
}

impl TypeCheck for ast::BreakStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, _context: &mut context::Context) {
    if !type_context.in_loop {
      type_context
        .diagnostics
        .error("break statement may only occur inside loops".to_string());
    }
  }
}

impl TypeCheck for ast::Definition {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    self.node.borrow().type_check(type_context, context);
  }
}

impl TypeCheck for ast::ExprStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    self.expr.type_check(type_context, context);
  }
}

impl TypeCheck for ast::LetStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    let value_type = self.value.infer_type(context);

    // TODO: Ensure this comparison works as expected (especially for complex types).
    // TODO: Comparing references?
    if Some(&self.ty) != value_type.as_ref() {
      type_context
        .diagnostics
        .error("let statement value and type mismatch".to_string());
    }

    self.value.type_check(type_context, context);
  }
}

impl TypeCheck for ast::ReturnStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    if type_context.does_function_return && self.value.is_none() {
      type_context
        .diagnostics
        .error("return statement must return a value".to_string());
    } else if !type_context.does_function_return && self.value.is_some() {
      type_context
        .diagnostics
        .error("return statement must not return a value".to_string());
    }

    if let Some(value) = &self.value {
      // TODO: Unify prototype return type with the value's return type.

      value.type_check(type_context, context);
    }
  }
}

impl TypeCheck for ast::Function {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    type_context.does_function_return = self.prototype.return_type.is_some();

    // NOTE: No need to type-check parameters.
    self.body.type_check(type_context, context);

    // TODO: Special case for the `main` function. Unify expected signature.
    // TODO: If it must return value, ensure a value was returned.
  }
}

impl TypeCheck for ast::FunctionCall {
  fn infer_type(&self, context: &context::Context) -> Option<ast::Type> {
    // TODO: Is this cloning? Simplify this messy code section.
    let function_or_extern = &*context
      .declarations
      .get(&self.callee_key.unwrap())
      .unwrap()
      .as_ref()
      .borrow();

    let prototype = match function_or_extern {
      ast::Node::Function(function) => &function.prototype,
      ast::Node::Extern(extern_) => &extern_.prototype,
      _ => unreachable!(),
    };

    prototype.return_type.clone()
  }

  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    // TODO: Need access to the current function.
    // TODO: Ensure externs and unsafe function are only called from unsafe functions.

    let callee = context
      .declarations
      .get(self.callee_key.as_ref().unwrap())
      .unwrap();

    // TODO: Cleanup.
    let callee_final = &*callee.as_ref().borrow();

    // TODO: Better, simpler way of doing this?
    let prototype: &ast::Prototype = match callee_final {
      ast::Node::Extern(extern_) => {
        if !type_context.in_unsafe_block {
          type_context
            .diagnostics
            .error("extern function calls may only occur inside an unsafe block".to_string());
        }

        &extern_.prototype
      }
      ast::Node::Function(function) => &function.prototype,
      _ => unreachable!(),
    };

    let min_arg_count = prototype.parameters.len();
    let actual_arg_count = self.arguments.len();
    const ARG_COUNT_MISMATCH: &str = "function call argument count mismatch";

    // Verify argument count.
    if !prototype.is_variadic && actual_arg_count != min_arg_count {
      type_context
        .diagnostics
        .error(ARG_COUNT_MISMATCH.to_string());
    } else if prototype.is_variadic && actual_arg_count < min_arg_count {
      type_context
        .diagnostics
        .error(ARG_COUNT_MISMATCH.to_string());
    }

    // FIXME: Different amount of arguments and parameters (due to variadic parameters) may affect this.
    // Unify argument and parameter types.
    for (parameter, argument) in prototype.parameters.iter().zip(self.arguments.iter()) {
      let parameter_type = parameter.infer_type(context);
      let argument_type = argument.infer_type(context);

      if argument_type != parameter_type {
        type_context.diagnostics.error(format!(
          "function call argument and parameter `{}` type mismatch",
          parameter.0
        ));
      }
    }
  }
}

impl TypeCheck for ast::WhileStmt {
  fn type_check(&self, type_context: &mut TypeCheckContext, context: &mut context::Context) {
    if self.condition.infer_type(context) != Some(ast::Type::Primitive(ast::PrimitiveType::Bool)) {
      type_context
        .diagnostics
        .error("while statement condition must evaluate to a boolean".to_string());
    }

    // TODO: To avoid problems with nested cases, save a buffer here, then restore.
    type_context.in_loop = true;
    self.condition.type_check(type_context, context);
    self.body.type_check(type_context, context);
    type_context.in_loop = false;
  }
}
