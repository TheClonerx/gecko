use crate::{diagnostic, node, pass};

pub struct NameResolutionPass<'a> {
  module_buffer: Option<&'a node::Module<'a>>,
}

impl<'a> NameResolutionPass<'a> {
  pub fn new() -> Self {
    Self {
      module_buffer: None,
    }
  }
}

impl<'a> pass::Pass<'a> for NameResolutionPass<'a> {
  // TODO: Generated by Github code-pilot.
  // fn visit_stub(stub: &node::Stub) {
  //   let mut scope = stub.scope();
  //   let mut name = stub.name();
  //   let mut found = false;
  //   while let Some(parent) = scope.parent() {
  //     if let Some(decl) = scope.find_decl(name) {
  //       found = true;
  //       stub.set_decl(decl);
  //       break;
  //     }
  //     scope = parent;
  //   }
  //   if !found {
  //     let mut diag =
  //       diagnostic::Diagnostic::error(&format!("unresolved name `{}`", name), stub.span());
  //     diag.emit();
  //   }
  // }

  fn visit_module(&mut self, module: &'a node::Module<'a>) -> pass::PassResult {
    self.module_buffer = Some(module);

    Ok(())
  }

  fn visit_stub(&mut self, stub: &mut node::Stub<'a>) -> pass::PassResult {
    match stub {
      node::Stub::Callable { name, value } => {
        if value.is_some() {
          return Ok(());
        }

        crate::pass_assert!(self.module_buffer.is_some());

        if let Some(target) = self.module_buffer.unwrap().symbol_table.get(name) {
          *value = Some(match target {
            node::TopLevelNodeHolder::Function(function) => {
              node::StubValueTransport::Function(function)
            }
            node::TopLevelNodeHolder::External(external) => {
              node::StubValueTransport::External(external)
            }
          });
        } else {
          return Err(diagnostic::Diagnostic {
            message: format!("unresolved callee `{}`", name),
            severity: diagnostic::Severity::Error,
          });
        }
      }
    };

    Ok(())
  }
}

// TODO:
// #[cfg(test)]
// mod tests {
//   use super::*;
//   use crate::pass::Pass;
// }