use crate::{diagnostic, int_kind, node, pass};

pub struct EntryPointCheckPass;

pub const ENTRY_POINT_NAME: &str = "main";

impl<'a> pass::Pass<'a> for EntryPointCheckPass {
  fn visit(&mut self, node: &'a dyn node::Node) -> pass::PassResult {
    node.accept(self)
  }

  fn visit_function<'b>(&mut self, function: &node::Function<'a>) -> pass::PassResult {
    if function.prototype.name != ENTRY_POINT_NAME {
      return Ok(());
    } else if function.is_public {
      // TODO: Collect diagnostics instead (use a warning instead on this case).
      return Err(diagnostic::Diagnostic {
        message: "main function should not be annotated with `pub`, it is implied".to_string(),
        severity: diagnostic::Severity::Error,
      });
    }

    match function.prototype.return_kind_group.kind {
      node::KindHolder::IntKind(int_kind) => {
        if int_kind.size != int_kind::IntSize::Bit32 {
          return Err(diagnostic::Diagnostic {
            message: format!(
              "main function must return `i32`, but found integer size `{:?}`",
              int_kind.size
            ),
            severity: diagnostic::Severity::Error,
          });
        }
      }
      _ => {
        return Err(diagnostic::Diagnostic {
          message: format!(
            "main function must return `i32`, but returns `{:?}`",
            function.prototype.return_kind_group.kind
          ),
          severity: diagnostic::Severity::Error,
        });
      }
    };

    if function.prototype.parameters.len() != 2 {
      return Err(diagnostic::Diagnostic {
        message: format!(
          "main function must have 2 parameters, but found `{}`",
          function.prototype.parameters.len()
        ),
        severity: diagnostic::Severity::Error,
      });
    } else if function.prototype.parameters[0].1.kind
      != node::KindHolder::IntKind(int_kind::IntKind {
        size: int_kind::IntSize::Bit32,
        is_signed: true,
      })
    {
      return Err(diagnostic::Diagnostic {
        message: format!(
          "main function's first parameter must be `i32`, but found `{:?}`",
          function.prototype.parameters[0].1.kind
        ),
        severity: diagnostic::Severity::Error,
      });
    } else if function.prototype.parameters[1].1.kind
      != node::KindHolder::IntKind(int_kind::IntKind {
        size: int_kind::IntSize::Bit32,
        is_signed: true,
      })
    {
      // TODO: Should be an array of i32 instead of i32.
      return Err(diagnostic::Diagnostic {
        message: format!(
          "main function's second parameter must be `i32`, but found `{:?}`",
          function.prototype.parameters[1].1.kind
        ),
        severity: diagnostic::Severity::Error,
      });
    }

    Ok(())
  }
}
