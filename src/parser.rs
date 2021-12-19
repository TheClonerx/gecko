use crate::{ast, diagnostic, token};

macro_rules! skip_past {
  ($self:expr, $token:expr) => {
    // FIXME: Something's wrong, we get an error when no input is provided (eof at index 0).
    // if $self.is_eof() {
    //   return Err(diagnostic::Diagnostic {
    //     message: format!("expected token `{}` but reached eof", $token),
    //     severity: diagnostic::Severity::Error,
    //   });
    // }

    if !$self.is($token) {
      return Err(diagnostic::Diagnostic {
        message: format!(
          "expected token `{}`, but got `{}`",
          $token, $self.tokens[$self.index]
        ),
        severity: diagnostic::Severity::Error,
      });
    }

    $self.skip();
  };
}

// TODO: Add more test cases for larger numbers than `0`. Also, is there a need for a panic here? If so, consider using `unreachable!()`. Additionally, should `unreachabe!()` panics even be reported on the documentation?
/// Determine the minimum bit-size in which a number can fit.
fn minimum_int_size_of(number: &u64) -> ast::IntSize {
  let log2_result = f64::log2(*number as f64 + 1_f64);
  let minimum_bit_size = f64::floor(log2_result) as u64;

  if minimum_bit_size <= 8 {
    ast::IntSize::I8
  } else if minimum_bit_size <= 16 {
    ast::IntSize::I16
  } else if minimum_bit_size <= 32 {
    ast::IntSize::I32
  } else if minimum_bit_size <= 64 {
    ast::IntSize::I64
  } else {
    panic!("expected calculated minimum bit-size to be smaller than 64");
  }
}

type ParserResult<T> = Result<T, diagnostic::Diagnostic>;

pub struct Parser {
  tokens: Vec<token::Token>,
  index: usize,
}

impl Parser {
  pub fn new(tokens: Vec<token::Token>) -> Self {
    Self { tokens, index: 0 }
  }

  /// Parse all top-level definitions.
  pub fn parse_all(&mut self) -> ParserResult<Vec<ast::Node>> {
    let mut result = Vec::new();

    while !self.is_eof() {
      result.push(self.parse_top_level_node()?);

      // FIXME: Nothing being done to the parsed top-level node.
    }

    Ok(result)
  }

  fn is(&self, token: token::Token) -> bool {
    if self.index >= self.tokens.len() {
      return false;
    }

    self.tokens[self.index] == token
  }

  fn skip(&mut self) -> bool {
    // FIXME: Address out of bounds problem (if any at this state).
    if self.index + 1 >= self.tokens.len() {
      return false;
    }

    self.index += 1;

    true
  }

  fn peek(&self) -> Option<&token::Token> {
    match self.tokens.get(self.index + 1) {
      Some(value) => Some(value),
      None => None,
    }
  }

  fn peek_is(&self, token: token::Token) -> bool {
    let next_token = self.peek();

    if next_token.is_none() {
      return false;
    }

    token == *next_token.unwrap()
  }

  /// Whether the parser has reached the end of the input.
  ///
  /// Will return true if the tokens vector provided is empty,
  /// or if the index is at the end of the tokens vector.
  pub fn is_eof(&self) -> bool {
    self.tokens.is_empty() || self.index == self.tokens.len() - 1
  }

  /// %name
  fn parse_name(&mut self) -> ParserResult<String> {
    // TODO: Illegal/unrecognized tokens are also represented under 'Identifier'.

    // TODO: Wrong error message.
    crate::diagnostic_assert!(match &self.tokens[self.index] {
      token::Token::Identifier(_) => true,
      _ => false,
    });

    let name = {
      match &self.tokens[self.index] {
        token::Token::Identifier(value) => Some(value.clone()),
        _ => None,
      }
    };

    crate::diagnostic_assert!(name.is_some());
    self.skip();

    Ok(name.unwrap())
  }

  /// TODO: Be specific, which statements? Also include lonely expression.
  /// '{' (%statement+) '}'
  fn parse_block(&mut self, llvm_name: &str) -> ParserResult<ast::Block> {
    // TODO: Have a symbol table for blocks, and check for re-declarations here.

    skip_past!(self, token::Token::SymbolBraceL);

    let mut statements = vec![];

    while !self.is(token::Token::SymbolBraceR) && !self.is_eof() {
      statements.push(Box::new(match self.tokens[self.index] {
        token::Token::KeywordReturn => ast::Node::ReturnStmt(self.parse_return_stmt()?),
        token::Token::KeywordLet => ast::Node::LetStmt(self.parse_let_stmt()?),
        token::Token::KeywordIf => ast::Node::IfStmt(self.parse_if_stmt()?),
        token::Token::KeywordWhile => ast::Node::WhileStmt(self.parse_while_stmt()?),
        token::Token::SymbolBraceL => ast::Node::BlockStmt(self.parse_block_stmt()?),
        token::Token::KeywordBreak => ast::Node::BreakStmt(self.parse_break_stmt()?),
        // TODO: Throw error/diagnostic instead.
        _ => todo!(),
      }));
    }

    skip_past!(self, token::Token::SymbolBraceR);

    Ok(ast::Block {
      llvm_name: llvm_name.to_string(),
      statements,
    })
  }

  /// (unsigned) {i8 | i16 | i32 | i64}
  fn parse_int_type(&mut self) -> ParserResult<ast::Type> {
    // TODO: Accessing index unsafely.
    let current_token = &self.tokens[self.index];

    let size = match current_token {
      token::Token::TypeInt16 => ast::IntSize::I16,
      token::Token::TypeInt32 => ast::IntSize::I32,
      token::Token::TypeInt64 => ast::IntSize::I64,
      // TODO: Add unsigned type tokens.
      _ => {
        return Err(diagnostic::Diagnostic {
          message: format!(
            "unexpected token `{}`, expected integer type",
            current_token
          ),
          severity: diagnostic::Severity::Error,
        })
      }
    };

    self.skip();

    Ok(ast::Type::PrimitiveType(ast::PrimitiveType::Int(size)))
  }

  /// bool
  fn parse_bool_type(&mut self) -> ParserResult<ast::Type> {
    skip_past!(self, token::Token::TypeBool);

    Ok(ast::Type::PrimitiveType(ast::PrimitiveType::Bool))
  }

  /// %type
  fn parse_type(&mut self) -> ParserResult<ast::Type> {
    // TODO: Check if the index is valid?
    // TODO: Support for more types.
    match self.tokens[self.index] {
      // TODO: Other types as well.
      token::Token::TypeInt16 | token::Token::TypeInt32 | token::Token::TypeInt64 => {
        self.parse_int_type()
      }
      token::Token::TypeBool => self.parse_bool_type(),
      _ => {
        return Err(diagnostic::Diagnostic {
          message: format!(
            "unexpected token `{:?}`, expected type",
            // TODO: Check if the index is valid?
            self.tokens[self.index]
          ),
          severity: diagnostic::Severity::Error,
        });
      }
    }
  }

  /// %name ':' %kind_group
  fn parse_parameter(&mut self) -> ParserResult<ast::Parameter> {
    let name = self.parse_name()?;

    skip_past!(self, token::Token::SymbolColon);

    let kind_group = self.parse_type()?;

    Ok((name, kind_group))
  }

  /// %name '(' {%parameter* (,)} (+) ')' '~' %kind_group
  fn parse_prototype(&mut self) -> ParserResult<ast::Type> {
    skip_past!(self, token::Token::SymbolParenthesesL);

    let mut parameters = vec![];
    let mut is_variadic = false;

    // TODO: Analyze, and remove possibility of lonely comma.
    while !self.is(token::Token::SymbolParenthesesR) && !self.is_eof() {
      if self.is(token::Token::SymbolPlus) {
        is_variadic = true;
        self.skip();

        break;
      }

      parameters.push(self.parse_parameter()?);

      if !self.is(token::Token::SymbolComma) {
        break;
      }

      self.skip();
    }

    skip_past!(self, token::Token::SymbolParenthesesR);

    let mut return_type = None;

    if self.is(token::Token::SymbolTilde) {
      self.skip();
      return_type = Some(Box::new(self.parse_type()?));
    }

    Ok(ast::Type::Prototype(parameters, return_type, is_variadic))
  }

  /// (pub) fn %prototype %block
  fn parse_function(&mut self) -> ParserResult<ast::Function> {
    skip_past!(self, token::Token::KeywordFn);

    let name = self.parse_name()?;
    let prototype = self.parse_prototype()?;
    let mut body = self.parse_block("fn_body")?;

    // Insert a return void instruction if the function body is empty.
    if body.statements.is_empty() {
      body
        .statements
        .push(Box::new(ast::Node::ReturnStmt(ast::ReturnStmt {
          value: None,
        })));
    }

    Ok(ast::Function {
      name,
      prototype,
      body,
    })
  }

  /// (pub) extern fn %prototype
  fn parse_extern(&mut self) -> ParserResult<ast::Extern> {
    // TODO: Support for visibility.

    skip_past!(self, token::Token::KeywordExtern);
    skip_past!(self, token::Token::KeywordFn);

    let name = self.parse_name()?;
    let prototype = self.parse_prototype()?;

    Ok(ast::Extern { name, prototype })
  }

  fn parse_top_level_node(&mut self) -> ParserResult<ast::Node> {
    let mut token = self.tokens.get(self.index);

    if self.is(token::Token::KeywordPub) {
      token = self.peek();
    }

    if token.is_none() {
      return Err(diagnostic::Diagnostic {
        message: "expected top-level construct but got end of file".to_string(),
        severity: diagnostic::Severity::Error,
      });
    }

    Ok(match token.unwrap() {
      token::Token::KeywordFn => ast::Node::Function(self.parse_function()?),
      token::Token::KeywordExtern => ast::Node::Extern(self.parse_extern()?),
      _ => {
        return Err(diagnostic::Diagnostic {
          message: format!(
            "unexpected token `{}`, expected top-level construct",
            token.unwrap()
          ),
          severity: diagnostic::Severity::Error,
        })
      }
    })
  }

  /// return (%expr)
  fn parse_return_stmt(&mut self) -> ParserResult<ast::ReturnStmt> {
    skip_past!(self, token::Token::KeywordReturn);

    let mut value = None;

    // TODO: Does this cover all cases?
    if !self.is(token::Token::SymbolBraceR) {
      value = Some(Box::new(ast::Node::Literal(self.parse_literal()?)));
    }

    Ok(ast::ReturnStmt { value })
  }

  /// let %name (':' %kind_group) '=' %expr
  fn parse_let_stmt(&mut self) -> ParserResult<ast::LetStmt> {
    skip_past!(self, token::Token::KeywordLet);

    let name = self.parse_name()?;
    let mut ty = None;

    if self.is(token::Token::SymbolColon) {
      self.skip();
      ty = Some(self.parse_type()?);
    }

    skip_past!(self, token::Token::SymbolEqual);

    let value = self.parse_expr()?;

    // Infer the kind of the variable, based on its value.
    if ty.is_none() {
      // FIXME: Implement.
      todo!();
    }

    Ok(ast::LetStmt {
      name,
      ty: ty.unwrap(),
      value: Box::new(value),
    })
  }

  /// if %expr %block (else %block)
  fn parse_if_stmt(&mut self) -> ParserResult<ast::IfStmt> {
    skip_past!(self, token::Token::KeywordIf);

    let condition = self.parse_expr()?;
    let then_block = self.parse_block("if_then")?;
    let mut else_block = None;

    if self.is(token::Token::KeywordElse) {
      self.skip();
      else_block = Some(self.parse_block("if_else")?);
    }

    Ok(ast::IfStmt {
      condition: Box::new(condition),
      then_block,
      else_block,
    })
  }

  fn parse_while_stmt(&mut self) -> ParserResult<ast::WhileStmt> {
    skip_past!(self, token::Token::KeywordWhile);

    let condition = self.parse_expr()?;
    let body = self.parse_block("while_then")?;

    Ok(ast::WhileStmt {
      condition: Box::new(condition),
      body,
    })
  }

  fn parse_block_stmt(&mut self) -> ParserResult<ast::BlockStmt> {
    Ok(ast::BlockStmt {
      block: self.parse_block("block_stmt")?,
    })
  }

  /// break
  fn parse_break_stmt(&mut self) -> ParserResult<ast::BreakStmt> {
    skip_past!(self, token::Token::KeywordBreak);

    Ok(ast::BreakStmt {})
  }

  /// {true | false}
  fn parse_bool_literal(&mut self) -> ParserResult<ast::Literal> {
    Ok(match self.tokens[self.index] {
      token::Token::LiteralBool(value) => {
        self.skip();

        ast::Literal::Bool(value)
      }
      // TODO: Better error.
      _ => {
        return Err(diagnostic::Diagnostic {
          message: "unexpected token, expected boolean literal".to_string(),
          severity: diagnostic::Severity::Error,
        })
      }
    })
  }

  fn parse_int_literal(&mut self) -> ParserResult<ast::Literal> {
    // TODO:
    // Ok(match self.tokens.get(self.index) {
    //   Some(token::Token::LiteralInt(value)) => {
    //     self.skip();

    //     let size = int_kind::calculate_int_size_of(value);

    //     node::IntLiteral {
    //       value: *value,
    //       kind: int_kind::IntKind { size, is_signed: true },
    //     }
    //   }
    //   _ => return Err(diagnostic::error_unexpected_eof("integer literal")),
    // })
    Ok(match self.tokens[self.index] {
      token::Token::LiteralInt(value) => {
        self.skip();

        let size = minimum_int_size_of(&value);

        // Default size to 32 bit-width.
        // FIXME:
        // if size < ast::IntSize::I32 {
        //   size = ast::IntSize::I32;
        // }

        ast::Literal::Int(value, size)
      }
      _ => {
        return Err(diagnostic::Diagnostic {
          message: "expected integer literal but got end of file".to_string(),
          severity: diagnostic::Severity::Error,
        })
      }
    })
  }

  fn parse_literal(&mut self) -> ParserResult<ast::Literal> {
    Ok(match self.tokens[self.index] {
      token::Token::LiteralBool(_) => self.parse_bool_literal()?,
      token::Token::LiteralInt(_) => self.parse_int_literal()?,
      _ => {
        return Err(diagnostic::Diagnostic {
          // TODO: Show the actual token.
          message: "unexpected token, expected literal".to_string(),
          severity: diagnostic::Severity::Error,
        });
      }
    })
  }

  fn parse_expr(&mut self) -> ParserResult<ast::Node> {
    Ok(match self.tokens[self.index] {
      token::Token::Identifier(_) => {
        if self.peek_is(token::Token::SymbolParenthesesL) {
          ast::Node::CallExpr(self.parse_call_expr()?)
        } else {
          return Err(diagnostic::Diagnostic {
            // TODO: Show the actual token.
            message: "unexpected token, expected expression".to_string(),
            severity: diagnostic::Severity::Error,
          });
        }
      }
      // Default to a literal if nothing else matched.
      _ => ast::Node::Literal(self.parse_literal()?),
    })
  }

  /// %name '(' (%expr (,))* ')'
  fn parse_call_expr(&mut self) -> ParserResult<ast::CallExpr> {
    let _callee_name = self.parse_name()?;

    skip_past!(self, token::Token::SymbolParenthesesL);

    // TODO: Shouldn't it be a `Vec<node::ExprTransport<'_>>`?
    let _arguments: Vec<ast::Node> = vec![];

    // TODO: Parse arguments.

    skip_past!(self, token::Token::SymbolParenthesesR);

    // FIXME: Fix implementation.
    todo!();

    // Ok(ast::CallExpr {
    //   callee: ast::CalleeStub {
    //     name: callee_name,
    //     value: None,
    //   },
    //   arguments,
    // })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn proper_initial_values() {
    let parser = Parser::new(vec![]);

    assert_eq!(0, parser.index);
  }

  #[test]
  fn is() {
    let parser = Parser::new(vec![token::Token::KeywordFn]);

    assert_eq!(true, parser.is(token::Token::KeywordFn));
  }

  #[test]
  fn is_empty() {
    let parser = Parser::new(vec![]);

    assert_eq!(false, parser.is(token::Token::KeywordFn));
  }

  #[test]
  fn skip() {
    let mut parser = Parser::new(vec![token::Token::KeywordFn, token::Token::KeywordFn]);

    parser.skip();
    assert_eq!(1, parser.index);
  }

  #[test]
  fn skip_out_of_bounds() {
    let mut parser = Parser::new(vec![token::Token::KeywordFn]);

    parser.skip();
    assert_eq!(0, parser.index);
  }

  #[test]
  fn is_eof() {
    let mut parser = Parser::new(vec![]);

    assert_eq!(true, parser.is_eof());
    parser.tokens.push(token::Token::KeywordFn);
    assert_eq!(true, parser.is_eof());
    parser.tokens.push(token::Token::KeywordFn);
    assert_eq!(false, parser.is_eof());
    parser.skip();
    assert_eq!(true, parser.is_eof());
  }

  // TODO: Add more tests.
}
