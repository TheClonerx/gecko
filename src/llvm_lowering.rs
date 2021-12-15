use crate::{ast, context, dispatch};
use inkwell::values::BasicValue;

const ENTRY_POINT_NAME: &str = "main";

trait Lower {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx>;
}

impl Lower for ast::Node {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    dispatch!(self, Lower::lower, generator, context)
  }
}

impl Lower for ast::Literal {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    _: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    match self {
      ast::Literal::Int(value, integer_kind) => {
        let llvm_int_type = generator
          .llvm_context
          .custom_width_int_type(match integer_kind {
            ast::IntSize::I8 | ast::IntSize::U8 => 8,
            ast::IntSize::I16 | ast::IntSize::U16 => 16,
            ast::IntSize::I32 | ast::IntSize::U32 => 32,
            ast::IntSize::I64 | ast::IntSize::U64 => 64,
            ast::IntSize::Isize | ast::IntSize::Usize => 128,
          });

        llvm_int_type
          .const_int(
            // TODO: Is this cloning?
            *value,
            match integer_kind {
              ast::IntSize::I8 => true,
              _ => false,
            },
          )
          .as_basic_value_enum()
      }
      ast::Literal::Char(character) => generator
        .llvm_context
        .i8_type()
        // TODO: Is this cloning?
        .const_int(*character as u64, false)
        .as_basic_value_enum(),
      // TODO: Process all literals.
      ast::Literal::Bool(value) => generator
        .llvm_context
        .bool_type()
        // TODO: Is this cloning?
        .const_int(*value as u64, false)
        .as_basic_value_enum(),
      _ => todo!(),
    }
  }
}

impl Lower for ast::Function {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    let llvm_function_type = generator
      .lower_type(self.prototype)
      .into_pointer_type()
      .get_element_type();

    assert!(llvm_function_type.is_function_type());

    // let llvm_function_name = if self.prototype.name == ENTRY_POINT_NAME {
    //   self.prototype.name.clone()
    // } else {
    //   mangle_name(&self.module_buffer.unwrap().name, &self.prototype.name)
    // };

    let llvm_function = generator.llvm_module.add_function(
      "todo_name_fn",
      llvm_function_type.into_function_type(),
      // Some(if self.prototype.name == ENTRY_POINT_NAME {
      //   inkwell::module::Linkage::External
      // } else {
      //   inkwell::module::Linkage::Private
      // }),
      Some(inkwell::module::Linkage::Private),
    );

    match self.prototype {
      ast::Type::Prototype(parameters, _, _) => {
        // TODO: Find a way to use only one loop to process both local parameters and LLVM's names.
        for (i, ref mut llvm_parameter) in llvm_function.get_param_iter().enumerate() {
          // TODO: Ensure this access is safe and checked.
          let (parameter_name, _) = parameters.get(i).unwrap();

          llvm_parameter.set_name(parameter_name.as_str());
        }
      }
      _ => unreachable!(),
    };

    self.body.lower(generator, context);
    assert!(llvm_function.verify(false));

    llvm_function.as_global_value().as_basic_value_enum()
  }
}

impl Lower for ast::Extern {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    let llvm_function_type = generator
      .lower_type(self.prototype)
      .into_pointer_type()
      .get_element_type();

    assert!(llvm_function_type.is_function_type());

    let llvm_external_function = generator.llvm_module.add_function(
      "todo_name",
      llvm_function_type.into_function_type(),
      Some(inkwell::module::Linkage::External),
    );

    assert!(llvm_external_function.verify(false));

    generator.make_unit_value()
  }
}

impl Lower for ast::Block {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    let llvm_basic_block = generator
      .llvm_context
      .append_basic_block(generator.get_current_function(), self.llvm_name.as_str());

    generator.llvm_builder.position_at_end(llvm_basic_block);

    for statement in &self.statements {
      // TODO: Invoke dispatch for the statement.
      // dispatch!(statement, Lower::, generator, context);
    }

    // FIXME: What if there are no statements (i.e. empty block)?
    self.statements.last().unwrap().lower(generator, context)
  }
}

impl Lower for ast::ReturnStmt {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    if let Some(return_value) = self.value {
      let llvm_return_value = return_value.lower(generator, context);

      return llvm_return_value;
    }

    // TODO: Shouldn't be returning unit value here, instead there should always be a value returned.
    generator.make_unit_value()
  }
}

impl Lower for ast::LetStmt {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    let llvm_type = generator.lower_type(self.ty);

    // TODO: Finish implementing.
    let llvm_alloca_inst_ptr = generator
      .llvm_builder
      .build_alloca(llvm_type, self.name.as_str());

    let llvm_value = self.value.lower(generator, context);

    generator
      .llvm_builder
      .build_store(llvm_alloca_inst_ptr, llvm_value);

    todo!();
    // Ok(llvm_alloca_inst_ptr)
  }
}

impl Lower for ast::CallExpr {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    match *self.callee {
      // TODO: Need to stop callee from lowering more than once. Also, watch out for buffers being overwritten.
      ast::Node::Function(function) => function.lower(generator, context),
      ast::Node::External(external) => external.lower(generator, context),
      // TODO: Emit I.C.E. instead?
      _ => unreachable!(),
    };

    let mut arguments = Vec::new();

    arguments.reserve(self.arguments.len());

    for argument in &self.arguments {
      // TODO: Cloning argument.
      let llvm_value = argument.lower(generator, context);

      arguments.push(match llvm_value {
        // TODO: Is this check necessary?
        // TODO: Add support for missing basic values.
        inkwell::values::BasicValueEnum::IntValue(int_value) => {
          inkwell::values::BasicMetadataValueEnum::IntValue(int_value)
        }
        _ => unreachable!(),
      });
    }

    let llvm_call_value = generator.llvm_builder.build_call(
      generator.get_current_function(),
      arguments.as_slice(),
      "call_result",
    );

    let llvm_call_basic_value_result = llvm_call_value.try_as_basic_value();

    assert!(llvm_call_basic_value_result.is_left());

    let llvm_call_basic_value = llvm_call_basic_value_result.left().unwrap();

    // FIXME: Awaiting checks?
    llvm_call_basic_value
  }
}

// impl Lower for ast::WhileStmt {
//   fn lower<'ctx>(
//     &self,
//     generator: LlvmGenerator<'ctx>,
//     context: &mut context::Context,
//   ) -> inkwell::values::BasicValueEnum<'ctx> {
//     let llvm_parent_block = self.llvm_builder_buffer.get_insert_block().unwrap();
//     let llvm_condition_basic_value = self.condition.lower(generator, context);
//     let llvm_condition_int_value = llvm_condition_basic_value.into_int_value();

//     // TODO: What if even tho. we set the buffer to the after block, there isn't any instructions lowered? This would leave the block without even a `ret void` (which is required).
//     let llvm_after_block = self
//       .generator
//       .llvm_context
//       .append_basic_block(self.llvm_function_like_buffer.unwrap(), "while_after");

//     self
//       .llvm_basic_block_fallthrough_stack
//       .push(llvm_after_block);

//     // FIXME: `BasicValueEnum` is not associated with `BasicBlock`.
//     let llvm_then_block = self.body.lower(generator, context);
//     let llvm_parent_builder = self.llvm_context.create_builder();

//     llvm_parent_builder.position_at_end(llvm_parent_block);

//     llvm_parent_builder.build_conditional_branch(
//       llvm_condition_int_value,
//       llvm_then_block,
//       llvm_after_block,
//     );

//     // TODO: Assert condition is both an int value and that it has 1 bit width.

//     self.llvm_builder_buffer.position_at_end(llvm_after_block);

//     if llvm_then_block.get_terminator().is_none() {
//       let llvm_temporary_builder = self.llvm_context.create_builder();

//       llvm_temporary_builder.position_at_end(llvm_then_block);

//       llvm_temporary_builder.build_conditional_branch(
//         llvm_condition_int_value,
//         llvm_then_block,
//         llvm_after_block,
//       );
//     }

//     Ok(())
//   }
// }

// impl Lower for ast::IfStmt<'_> {
//   fn lower<'ctx>(
//     &self,
//     generator: LlvmGenerator<'ctx>,
//     context: &mut context::Context,
//   ) -> inkwell::values::BasicValueEnum<'ctx> {
//     crate::diagnostic_assert!(self.llvm_function_like_buffer.is_some());
//     crate::diagnostic_assert!(self.llvm_builder_buffer.get_insert_block().is_some());

//     let llvm_function = self.llvm_function_like_buffer.unwrap();

//     // TODO: Verify builder is in the correct/expected block.
//     // TODO: What if the buffer was intended to be for an external?

//     let llvm_parent_block = self.llvm_builder_buffer.get_insert_block().unwrap();

//     let llvm_after_block = self
//       .llvm_context
//       .append_basic_block(llvm_function, "if_after");

//     self
//       .llvm_basic_block_fallthrough_stack
//       .push(llvm_after_block);

//     let llvm_then_block = self.then_block.lower(generator, context);

//     // Position the builder on the `after` block, for the next statement(s) (if any).
//     // It is important to do this after visiting the `then` block.
//     self.llvm_builder_buffer.position_at_end(llvm_after_block);

//     // TODO: Does it matter if the condition is visited before the `then` block? (Remember that the code is not being executed).
//     let llvm_condition_basic_value = self.condition.lower(generator, context);

//     crate::diagnostic_assert!(llvm_condition_basic_value.is_int_value());

//     let llvm_condition_int_value = llvm_condition_basic_value.into_int_value();

//     crate::diagnostic_assert!(llvm_condition_int_value.get_type().get_bit_width() == 1);

//     let llvm_temporary_builder = self.llvm_context.create_builder();

//     llvm_temporary_builder.position_at_end(llvm_parent_block);

//     llvm_temporary_builder.build_conditional_branch(
//       llvm_condition_int_value,
//       // TODO: Cloning LLVM basic block.
//       llvm_then_block.to_owned(),
//       llvm_after_block,
//     );

//     // If there is no terminator instruction on the `then` LLVM basic block,
//     // build a link to continue the code after the if statement.
//     if llvm_then_block.get_terminator().is_none() {
//       // TODO: Cloning LLVM basic block.
//       llvm_temporary_builder.position_at_end(llvm_then_block.to_owned());
//       llvm_temporary_builder.build_unconditional_branch(llvm_after_block);
//     }

//     // TODO: At this point not all instructions have been lowered (only up to the `if` statement itself), which means that a terminator can exist afterwards, but that case is being ignored here.
//     // If the `after` block has no terminator instruction, there might be a
//     // possibility for fallthrough. If after visiting the `then` block, the
//     // length of the LLVM basic block stack is larger than the cached length,
//     // then there is a fallthrough.
//     if llvm_after_block.get_terminator().is_none() {
//       llvm_temporary_builder.position_at_end(llvm_after_block);
//     }

//     // FIXME: Complete implementation.
//     todo!();
//   }
// }

// impl Lower for ast::BlockStmt<'_> {
//   fn lower<'ctx>(
//     &self,
//     generator: LlvmGenerator<'ctx>,
//     context: &mut context::Context,
//   ) -> inkwell::values::BasicValueEnum<'ctx> {
//     crate::diagnostic_assert!(self.llvm_function_like_buffer.is_some());
//     crate::diagnostic_assert!(self.llvm_builder_buffer.get_insert_block().is_some());

//     let llvm_parent_block = self.llvm_builder_buffer.get_insert_block().unwrap();
//     let llvm_temporary_builder = self.llvm_context.create_builder();

//     llvm_temporary_builder.position_at_end(llvm_parent_block);

//     let llvm_after_block = self
//       .llvm_context
//       .append_basic_block(self.llvm_function_like_buffer.unwrap(), "block_stmt_after");

//     // Push the `after` block before visiting the statement's body.
//     self
//       .llvm_basic_block_fallthrough_stack
//       .push(llvm_after_block);

//     // TODO: Cloning LLVM basic block.
//     let llvm_new_block = lower_or_retrieve_block!(self, &self.block)?.to_owned();

//     if llvm_parent_block.get_terminator().is_none() {
//       llvm_temporary_builder.build_unconditional_branch(llvm_new_block);
//     }

//     llvm_temporary_builder.position_at_end(llvm_new_block);

//     if llvm_new_block.get_terminator().is_none() {
//       llvm_temporary_builder.build_unconditional_branch(llvm_after_block);
//     }

//     self.llvm_builder_buffer.position_at_end(llvm_after_block);

//     Ok(())
//   }
// }

impl Lower for ast::BreakStmt {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    // TODO: Must ensure that the current block is a loop block.
    todo!();
  }
}

impl Lower for ast::Module {
  fn lower<'ctx>(
    &self,
    generator: LlvmGenerator<'ctx>,
    context: &mut context::Context,
  ) -> inkwell::values::BasicValueEnum<'ctx> {
    // for top_level_node in module.symbol_table.values() {
    //   match top_level_node {
    //     ast::Node::Function(function) => self.lower_function(function)?,
    //     ast::Node::External(external) => self.lower_external(external)?,
    //     // TODO: Emit I.C.E. instead?
    //     _ => unreachable!(),
    //   };
    // }

    // Ok(())
    todo!();
  }
}

struct LlvmGenerator<'ctx> {
  llvm_context: &'ctx inkwell::context::Context,
  llvm_module: inkwell::module::Module<'ctx>,
  llvm_builder: inkwell::builder::Builder<'ctx>,
  definitions:
    std::collections::HashMap<context::DefinitionKey, inkwell::values::BasicValueEnum<'ctx>>,
}

impl<'ctx> LlvmGenerator<'ctx> {
  fn new(
    llvm_context: &'ctx inkwell::context::Context,
    llvm_module: inkwell::module::Module<'ctx>,
  ) -> Self {
    Self {
      llvm_context,
      llvm_module,
      llvm_builder: llvm_context.create_builder(),
      definitions: std::collections::HashMap::new(),
    }
  }

  fn lower_type(&self, ty: ast::Type) -> inkwell::types::BasicTypeEnum<'ctx> {
    use inkwell::types::BasicType;

    match ty {
      ast::Type::PrimitiveType(primitive_type) => match primitive_type {
        ast::PrimitiveType::BooleanType => self.llvm_context.bool_type().as_basic_type_enum(),
        // TODO: Take into account size.
        ast::PrimitiveType::IntType(size) => self.llvm_context.i32_type().as_basic_type_enum(),
        ast::PrimitiveType::CharType => self.llvm_context.i8_type().as_basic_type_enum(),
      },
      ast::Type::Prototype(parameter_types, return_type, is_variadic) => {
        let llvm_parameter_types = parameter_types
          .iter()
          .map(|parameter_type| self.lower_type(parameter_type.1))
          .collect::<Vec<_>>();

        let llvm_return_type = self.lower_type(*return_type);

        llvm_return_type
          .fn_type(&[], is_variadic)
          .ptr_type(inkwell::AddressSpace::Generic)
          .into()
      }
    }
  }

  fn make_unit_value(&self) -> inkwell::values::BasicValueEnum<'ctx> {
    self.llvm_context.bool_type().const_int(0, false).into()
  }

  fn get_current_block(&self) -> inkwell::basic_block::BasicBlock<'ctx> {
    // TODO: What if the insertion point for the builder is not set?
    self.llvm_builder.get_insert_block().unwrap()
  }

  fn get_current_function(&self) -> inkwell::values::FunctionValue<'ctx> {
    self.get_current_block().get_parent().unwrap()
  }
}

fn mangle_name(scope_name: &String, name: &String) -> String {
  format!(".{}.{}", scope_name, name)
}

// #[cfg(test)]
// mod tests {
//   use super::*;

//   #[test]
//   fn proper_initial_values<'a>() {
//     let llvm_context = inkwell::context::Context::create();
//     let llvm_module: inkwell::module::Module<'_> = llvm_context.create_module("test");

//     assert_eq!(
//       true,
//       LlvmLowering::new(&llvm_context, &llvm_module)
//         .llvm_type_map
//         .is_empty()
//     );
//   }

//   // TODO:
//   // #[test]
//   // fn visit_or_retrieve_type() {
//   //   let llvm_context = inkwell::context::Context::create();
//   //   let llvm_module = llvm_context.create_module("test");
//   //   let mut llvm_lowering_pass = LlvmLoweringPass::new(&llvm_context, &llvm_module);

//   //   let int_kind = node::KindTransport::IntKind(&int_kind::IntKind {
//   //     size: int_kind::IntSize::Bit32,
//   //     is_signed: true,
//   //   });

//   //   let visit_or_retrieve_result = visit_or_retrieve_type!(llvm_lowering_pass, &int_kind);

//   //   assert_eq!(true, visit_or_retrieve_result.is_ok());
//   //   assert_eq!(true, visit_or_retrieve_result.ok().is_some());
//   //   assert_eq!(1, llvm_lowering_pass.llvm_type_map.len());
//   // }

//   #[test]
//   fn visit_int_kind() {
//     let llvm_context = inkwell::context::Context::create();
//     let llvm_module = llvm_context.create_module("test");
//     let mut llvm_lowering_pass = LlvmLowering::new(&llvm_context, &llvm_module);

//     let visit_int_kind_result = llvm_lowering_pass.lower_int_kind(&int_kind::IntKind {
//       size: int_kind::IntSize::Size32,
//       is_signed: true,
//     });

//     assert_eq!(true, visit_int_kind_result.is_ok());
//     assert_eq!(llvm_lowering_pass.llvm_type_map.len(), 1);
//   }

//   #[test]
//   fn visit_function() {
//     let llvm_context = inkwell::context::Context::create();
//     let llvm_module = llvm_context.create_module("test");
//     let mut llvm_lowering_pass = LlvmLowering::new(&llvm_context, &llvm_module);
//     let module = ast::Module::new("test");

//     llvm_lowering_pass.module_buffer = Some(&module);

//     let function = ast::Function {
//       prototype: ast::Prototype {
//         name: "foo".to_string(),
//         return_kind_group: None,
//         parameters: vec![],
//         is_variadic: false,
//       },
//       body: ast::Block {
//         llvm_name: "entry".to_string(),

//         statements: vec![ast::AnyStmtNode::ReturnStmt(ast::ReturnStmt {
//           value: None,
//         })],
//       },
//     };

//     let visit_function_result = llvm_lowering_pass.lower_function(&function);

//     assert_eq!(true, visit_function_result.is_ok());
//     assert_eq!(true, llvm_lowering_pass.llvm_function_like_buffer.is_some());
//   }

//   // TODO: Add more tests: `visit_prototype()`, etc.
// }
