use crate::{ast, cache, diagnostic, type_check::TypeCheck};

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum SymbolKind {
  Definition,
  // TODO: Can be more than that. Do not lie.
  /// A global type. Can be a struct, or enum.
  Type,
}

pub type Symbol = (String, SymbolKind);

type Scope = std::collections::HashMap<Symbol, cache::UniqueId>;

pub trait Resolve {
  fn declare(&self, _resolver: &mut NameResolver, _cache: &cache::Cache) {
    //
  }

  fn resolve(&mut self, _resolver: &mut NameResolver) {
    //
  }
}

impl Resolve for ast::Type {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    match self {
      ast::Type::Stub(stub_type) => stub_type.resolve(resolver),
      ast::Type::Pointer(pointee_type) => pointee_type.resolve(resolver),
      ast::Type::Array(element_type, _) => element_type.resolve(resolver),
      ast::Type::This(this_type) => this_type.resolve(resolver),
      // TODO: Are there any other types that may need to be resolved?
      _ => {}
    };
  }
}

impl Resolve for ast::Node {
  // TODO: This `dispatch` may actually only apply for top-level nodes, so there might be room for simplification.

  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    crate::dispatch!(&self.kind, Resolve::declare, resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    crate::dispatch!(&mut self.kind, Resolve::resolve, resolver);
  }
}

impl Resolve for ast::Trait {
  // TODO: Implement.
}

impl Resolve for ast::ThisType {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    if let Some(this_type_id) = resolver.current_struct_type_id {
      self.target_id = Some(this_type_id);
    } else {
      resolver
        .diagnostic_builder
        .error("type `This` cannot be used outside of a struct implementation".to_string());
    }
  }
}

impl Resolve for ast::StructImpl {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    resolver.push_scope();

    for method in &self.methods {
      method.declare(resolver, cache);
    }

    resolver.force_pop_scope();
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.target_struct_pattern.resolve(resolver);

    if let Some(trait_pattern) = &mut self.trait_pattern {
      trait_pattern.resolve(resolver);
    }

    // FIXME: [!] Investigate: We can't unwrap here because the lookup
    // might have failed. Is this done in other parts? Certain resolve methods
    // depend on other things being resolved already, this could be dangerous.
    let struct_type_id_result = self.target_struct_pattern.target_id;

    if let Some(struct_type_id) = struct_type_id_result {
      resolver.current_struct_type_id = Some(struct_type_id);

      for method in &mut self.methods {
        method.resolve(resolver);
      }

      resolver.current_struct_type_id = None;
    }
  }
}

impl Resolve for ast::MemberAccess {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.base_expr.resolve(resolver);
  }
}

impl Resolve for ast::Closure {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    // TODO: Here, captures should be force-declared.
    // FIXME: The body is resolved within a virtual environment. This means that declarations from here may not be accessible. To solve this, perhaps we may virtualize all but the last scope (this declaration's body's scope).

    self.prototype.declare(resolver, cache);
    self.body.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    // FIXME: Continue implementation.

    for (_index, capture) in self.captures.iter_mut().enumerate() {
      let symbol = (capture.0.clone(), SymbolKind::Definition);

      capture.1 = resolver.relative_lookup_or_error(&symbol);

      // FIXME: Anything else needs to be done here?
    }

    // Cache the existing relative scopes, and create a new, empty
    // environment within the resolver, then restore the cached scopes
    // after the body has been resolved. This is done to encapsulate the
    // closure's environment.
    let relative_scopes_cache = resolver.relative_scopes.clone();

    resolver.relative_scopes.clear();
    self.body.resolve(resolver);

    // FIXME: [!] Investigate: Should this closing of relative scopes occur
    // before or after the return type is possibly inferred?
    resolver.relative_scopes = relative_scopes_cache;

    self.prototype.resolve(resolver);
  }
}

impl Resolve for ast::TypeAlias {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.ty.resolve(resolver);
  }
}

// TODO: This might be getting too complicated. Maybe we should keep it simple in this case?
impl Resolve for ast::Pattern {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    // TODO: Consider extending this as a function of `Pattern` (via `impl`).
    let symbol = (self.base_name.clone(), self.symbol_kind.clone());
    let lookup_result = resolver.relative_lookup(&symbol);

    if lookup_result.is_none() {
      return resolver.produce_lookup_error(&symbol.0);
    }

    self.target_id = Some(lookup_result.unwrap().clone());
  }
}

impl Resolve for ast::IntrinsicCall {
  //
}

impl Resolve for ast::ExternStatic {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.1.resolve(resolver);
  }
}

impl Resolve for ast::StubType {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    // TODO: A bit misleading, since `lookup_or_error` returns `Option<>`.
    self.target_id = resolver.relative_lookup_or_error(&(self.name.clone(), SymbolKind::Type));
  }
}

impl Resolve for ast::StructValue {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    // TODO: A bit misleading, since `lookup_or_error` returns `Option<>`.
    self.target_id =
      resolver.relative_lookup_or_error(&(self.struct_name.clone(), SymbolKind::Type));

    for field in self.fields.iter_mut() {
      field.resolve(resolver);
    }
  }
}

impl Resolve for ast::Prototype {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    // FIXME: [!!] Bug: Cloning the parameter before it has been resolved.
    if self.accepts_instance {
      let _this_parameter = self.this_parameter.as_ref().unwrap();

      // TODO: Re-implement.
      // ast::Definition {
      //   symbol: Some((this_parameter.0.clone(), SymbolKind::Definition)),
      //   node_ref_cell: cache::create_cached_node(ast::Node {
      //     // TODO: Cloning parameter.
      //     kind: ast::NodeKind::Parameter(this_parameter.clone()),
      //     // TODO: Span.
      //     span: 0..0,
      //     unique_id: cache.create_unique_id(),
      //   }),
      //   // TODO: Will this `declare` function ever be called more than once? If so, this could be a problem.
      //   unique_id: cache.create_unique_id(),
      // }
      // .declare(resolver, cache);
    }

    // FIXME: [!!] Investigate: This might be causing the unwrap problem, probably because of the cloning?
    for parameter in &self.parameters {
      parameter.declare(resolver, cache);
    }
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    for parameter in &mut self.parameters {
      parameter.resolve(resolver);
    }

    if self.accepts_instance {
      self.this_parameter.as_mut().unwrap().resolve(resolver);
    }

    // TODO: Be a bit more specific.
    // NOTE: The prototype's return type is manually resolved after its body is resolved.
  }
}

impl Resolve for ast::StructType {
  fn declare(&self, _resolver: &mut NameResolver, _cache: &cache::Cache) {
    // TODO: Implement?
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    for field in &mut self.fields {
      field.1.resolve(resolver);
    }
  }
}

impl Resolve for ast::UnaryExpr {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.expr.resolve(resolver);
  }
}

impl Resolve for ast::Enum {
  //
}

impl Resolve for ast::AssignStmt {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.assignee_expr.resolve(resolver);
    self.value.resolve(resolver);
  }
}

impl Resolve for ast::ContinueStmt {
  //
}

impl Resolve for ast::ArrayIndexing {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.index_expr.resolve(resolver);

    self.target_id =
      resolver.relative_lookup_or_error(&(self.name.clone(), SymbolKind::Definition));
  }
}

impl Resolve for ast::ArrayValue {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    for element in &mut self.elements {
      element.resolve(resolver);
    }
  }
}

impl Resolve for ast::UnsafeBlockStmt {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    self.0.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.0.resolve(resolver);
  }
}

impl Resolve for ast::Parameter {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.ty.resolve(resolver);
  }
}

impl Resolve for ast::Reference {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.0.resolve(resolver);
  }
}

impl Resolve for ast::BreakStmt {
  //
}

impl Resolve for ast::LoopStmt {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    if let Some(condition) = &self.condition {
      condition.declare(resolver, cache);
    }

    self.body.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    if let Some(condition) = &mut self.condition {
      condition.resolve(resolver);
    }

    self.body.resolve(resolver);
  }
}

impl Resolve for ast::IfStmt {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    self.condition.declare(resolver, cache);
    self.then_block.declare(resolver, cache);

    if let Some(else_block) = &self.else_block {
      else_block.declare(resolver, cache);
    }
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.condition.resolve(resolver);
    self.then_block.resolve(resolver);

    if let Some(else_block) = &mut self.else_block {
      else_block.resolve(resolver);
    }
  }
}

impl Resolve for ast::LetStmt {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    self.value.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.value.resolve(resolver);
    self.ty.resolve(resolver);
  }
}

impl Resolve for ast::ReturnStmt {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    if let Some(value) = &self.value {
      value.declare(resolver, cache);
    }
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    if let Some(value) = &mut self.value {
      value.resolve(resolver);
    }
  }
}

impl Resolve for ast::Block {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    resolver.push_scope();

    for statement in &self.statements {
      statement.declare(resolver, cache);
    }

    resolver.register_scope_tree(self.unique_id);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    resolver.current_block_unique_id = Some(self.unique_id);

    // TODO:
    for statement in &mut self.statements {
      statement.resolve(resolver);
    }
  }
}

impl Resolve for ast::Literal {
  //
}

impl Resolve for ast::Function {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    // FIXME: [!] Revise: Ensure the order is correct (test nested parameter, references, etc.).

    // Parameter scope.
    resolver.push_scope();

    // NOTE: The scope tree won't be overwritten by the block's, nor the
    // prototype's scope tree, instead they will be merged, as expected.
    resolver.register_scope_tree(self.body.unique_id);

    self.prototype.declare(resolver, cache);
    self.body.declare(resolver, cache);
  }

  // FIXME: This resolve step may need to be repeated for closure.
  fn resolve(&mut self, resolver: &mut NameResolver) {
    // FIXME: [!] Investigate: Do we need to resolve the prototype first?
    // If so, doesn't the prototype's inferred type depend on the body being
    // resolved first?

    // TODO: Do we need scope management here, for the prototype's parameters?
    self.prototype.resolve(resolver);
    self.prototype.return_type.resolve(resolver);

    // Finally, after both the prototype and its return type have been resolved,
    // proceed to resolve the body.
    self.body.resolve(resolver);
  }
}

impl Resolve for ast::ExternFunction {
  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.prototype.resolve(resolver);
  }
}

impl Resolve for ast::Definition {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    // TODO: In the future, we'd like to warn against shadowed names. This can be accomplished simply, by using `resolver.contains_relative`.
    if let Some(symbol) = &self.symbol {
      // Check for existing definitions.
      if resolver.contains_current_scope(&symbol) {
        resolver
          .diagnostic_builder
          .error(format!("re-definition of `{}`", symbol.0));

        // TODO: What about calling the child's declare function?
        return;
      }

      // Bind the symbol to the current scope for name resolution lookup.
      resolver.bind(symbol.clone(), self.unique_id);
    }

    self.node.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.node.resolve(resolver);
  }
}

impl Resolve for ast::CallExpr {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    // Declare any possible `Definition` nodes in the arguments
    // (such as inline closures, etc.).
    for argument in &self.arguments {
      argument.declare(resolver, cache);
    }
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.callee_expr.resolve(resolver);

    for argument in &mut self.arguments {
      argument.resolve(resolver);
    }
  }
}

impl Resolve for ast::InlineExprStmt {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    self.expr.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.expr.resolve(resolver);
  }
}

impl Resolve for ast::BinaryExpr {
  fn declare(&self, resolver: &mut NameResolver, cache: &cache::Cache) {
    self.left.declare(resolver, cache);
    self.right.declare(resolver, cache);
  }

  fn resolve(&mut self, resolver: &mut NameResolver) {
    self.left.resolve(resolver);
    self.right.resolve(resolver);
  }
}

pub struct NameResolver {
  pub diagnostic_builder: diagnostic::DiagnosticBuilder,
  current_module_name: Option<String>,
  /// Contains the modules with their respective top-level definitions.
  global_scopes: std::collections::HashMap<String, Scope>,
  /// Contains volatile, relative scopes. Only used during the declare step.
  /// This is reset when the module changes, although by that time, all the
  /// relative scopes should have been popped automatically.
  relative_scopes: Vec<Scope>,
  /// A mapping of a scope's unique key to its own scope, and all visible parent
  /// relative scopes, excluding the global scope.
  scope_map: std::collections::HashMap<cache::UniqueId, Vec<Scope>>,
  /// The unique id of the current block's scope. Used in the resolve step.
  current_block_unique_id: Option<cache::UniqueId>,
  current_struct_type_id: Option<cache::UniqueId>,
}

impl NameResolver {
  pub fn new() -> Self {
    Self {
      diagnostic_builder: diagnostic::DiagnosticBuilder::new(),
      current_module_name: None,
      global_scopes: std::collections::HashMap::new(),
      relative_scopes: Vec::new(),
      scope_map: std::collections::HashMap::new(),
      current_block_unique_id: None,
      current_struct_type_id: None,
    }
  }

  /// Set per-file. A new global scope is created per-module.
  pub fn create_module(&mut self, name: String) {
    // TODO: Can the module name possibly collide with an existing one?

    self.current_module_name = Some(name.clone());

    self
      .global_scopes
      .insert(name, std::collections::HashMap::new());

    self.relative_scopes.clear();
  }

  pub fn set_active_module(&mut self, name: String) -> bool {
    // TODO: Implement checks (that module exists, etc.).
    // TODO: Shouldn't we reset buffers here? This might prevent the re-definition bug.
    self.current_module_name = Some(name.clone());

    true
  }

  fn infer_return_type(body: &ast::Block, cache: &cache::Cache) -> ast::Type {
    let mut return_type = body.infer_type(cache);

    if return_type.is_unit() && !body.statements.is_empty() {
      let first_return_stmt = body
        .statements
        .iter()
        .find(|x| matches!(x.kind, ast::NodeKind::ReturnStmt(_)));

      // TODO: Maybe could use some minor simplification.
      if let Some(first_return_stmt) = first_return_stmt {
        if let ast::NodeKind::ReturnStmt(return_stmt) = &first_return_stmt.kind {
          if let Some(return_value) = &return_stmt.value {
            return_type = return_value.infer_type(cache);
          }
        }
      }
    }

    return_type
  }

  /// Retrieve the last pushed relative scope, or if there are none,
  /// the global scope of the current module.
  fn get_current_scope(&mut self) -> &mut Scope {
    if self.relative_scopes.is_empty() {
      self
        .global_scopes
        .get_mut(self.current_module_name.as_ref().unwrap())
        .unwrap()
    } else {
      self.relative_scopes.last_mut().unwrap()
    }
  }

  // TODO: Consider returning the pushed scope? Unless it's not actually used.
  fn push_scope(&mut self) {
    self.relative_scopes.push(std::collections::HashMap::new());
  }

  /// Pop the last scope off the relatives scopes stack, and return it.
  ///
  /// Will panic if there are no relative scopes.
  fn force_pop_scope(&mut self) -> Scope {
    self.relative_scopes.pop().unwrap()
  }

  /// Force-pop the last scope off the relatives scopes stack, and create
  /// a scope tree. This tree will then be inserted into the scope map. If
  /// an entry with the same unique id already exists, the scope tree will
  /// be appended onto the existing definition.
  fn register_scope_tree(&mut self, unique_id: cache::UniqueId) {
    let mut scope_tree = vec![self.force_pop_scope()];

    // Clone the relative scope tree.
    scope_tree.extend(self.relative_scopes.iter().rev().cloned());

    let mut final_value_buffer = scope_tree;

    // Append to the existing definition, if applicable.
    if self.scope_map.contains_key(&unique_id) {
      final_value_buffer.extend(self.scope_map.remove(&unique_id).unwrap());
    }

    self.scope_map.insert(unique_id, final_value_buffer);
  }

  /// Register a name on the last scope for name resolution lookups.
  ///
  /// If there are no relative scopes, the symbol is registered in the global scope.
  fn bind(&mut self, symbol: Symbol, unique_id: cache::UniqueId) {
    self.get_current_scope().insert(symbol, unique_id);
  }

  fn produce_lookup_error(&mut self, name: &String) {
    self
      .diagnostic_builder
      .error(format!("undefined reference to `{}`", name));
  }

  /// Lookup a symbol starting from the nearest scope, all the way to the global scope
  /// of the current module.
  fn relative_lookup(&mut self, symbol: &Symbol) -> Option<&cache::UniqueId> {
    // If applicable, lookup on the relative scopes. This may not
    // be the case for when resolving global entities such as struct
    // types that reference other structs in their fields (in such case,
    // the relative scopes will be empty and the `current_block_unique_id`
    // buffer would be `None`).
    if let Some(current_block_unique_id) = self.current_block_unique_id {
      let scope_tree = self.scope_map.get(&current_block_unique_id).unwrap();

      // First attempt to find the symbol in the relative scopes.
      for scope in scope_tree {
        if let Some(unique_id) = scope.get(&symbol) {
          return Some(unique_id);
        }
      }
    }

    // Otherwise, attempt to find the symbol in the current module's global scope.
    let global_scope = self
      .global_scopes
      .get(self.current_module_name.as_ref().unwrap())
      .unwrap();

    if let Some(unique_id) = global_scope.get(&symbol) {
      return Some(unique_id);
    }

    None
  }

  fn relative_lookup_or_error(&mut self, symbol: &Symbol) -> Option<cache::UniqueId> {
    if let Some(unique_id) = self.relative_lookup(symbol) {
      return Some(unique_id.clone());
    }

    self.produce_lookup_error(&symbol.0);

    None
  }

  fn contains_current_scope(&mut self, key: &Symbol) -> bool {
    self.get_current_scope().contains_key(key)
  }
}

// TODO: Add essential tests.
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn proper_initial_values() {
    let name_resolver = NameResolver::new();

    assert!(name_resolver.current_module_name.is_none());
    assert!(name_resolver.relative_scopes.is_empty());
    assert!(name_resolver.global_scopes.is_empty());
  }

  #[test]
  fn push_pop_scope() {
    let mut name_resolver = NameResolver::new();

    assert!(name_resolver.relative_scopes.is_empty());
    name_resolver.push_scope();
    assert_eq!(1, name_resolver.relative_scopes.len());
    name_resolver.force_pop_scope();
    assert!(name_resolver.relative_scopes.is_empty());
  }
}
