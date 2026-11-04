//! A function declaration which is an `if` branch has its own scope
//! (see `IfStatement::consequent_has_scope`), entered around just that branch.

use std::cell::Cell;

use oxc_allocator::{Allocator, TakeIn};
use oxc_ast::ast::*;
use oxc_ast_visit::{VisitJsMut, VisitMut, walk_js_mut, walk_mut};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_traverse::{Traverse, TraverseCtx, traverse_mut};

const SOURCE: &str = "function outer() { if (a) function f() {} else function g() {} }";

fn parse_with_scoping(allocator: &Allocator) -> (Program<'_>, Scoping) {
    let program = Parser::new(allocator, SOURCE, SourceType::script()).parse().program;
    let scoping = SemanticBuilder::new().build(&program).semantic.into_scoping();
    (program, scoping)
}

fn get_if_statement<'p, 'a>(program: &'p Program<'a>) -> (ScopeId, &'p IfStatement<'a>) {
    let Statement::FunctionDeclaration(outer) = &program.body[0] else { unreachable!() };
    let Statement::IfStatement(if_stmt) = &outer.body.as_ref().unwrap().statements[0] else {
        unreachable!()
    };
    (outer.scope_id(), if_stmt)
}

#[test]
fn semantic_creates_branch_scopes() {
    let allocator = Allocator::default();
    let (program, scoping) = parse_with_scoping(&allocator);
    let (outer_scope_id, if_stmt) = get_if_statement(&program);

    for (branch, scope_id) in [
        (&if_stmt.consequent, &if_stmt.consequent_scope_id),
        (if_stmt.alternate.as_ref().unwrap(), &if_stmt.alternate_scope_id),
    ] {
        let Statement::FunctionDeclaration(func) = branch else { unreachable!() };
        let scope_id = scope_id.get().unwrap();
        assert_eq!(scoping.scope_parent_id(scope_id), Some(outer_scope_id));
        assert_eq!(scoping.scope_parent_id(func.scope_id()), Some(scope_id));
    }
}

/// Records the scope context when entering each function, and inserts a scope above the
/// `if` statement's child scopes when leaving it.
#[derive(Default)]
struct BranchScopes {
    contexts: Vec<(ScopeId, ScopeId, ScopeId)>,
    inserted_scope_id: Option<ScopeId>,
}

impl<'a> Traverse<'a, ()> for BranchScopes {
    fn enter_function(&mut self, _func: &mut Function<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.contexts.push((
            ctx.current_scope_id(),
            ctx.current_block_scope_id(),
            ctx.current_hoist_scope_id(),
        ));
    }

    fn exit_statements(
        &mut self,
        stmts: &mut oxc_allocator::ArenaVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        if let Some(stmt @ Statement::IfStatement(_)) = stmts.first() {
            self.inserted_scope_id =
                Some(ctx.insert_scope_below_statement(stmt, ScopeFlags::empty()));
        }
    }
}

#[test]
fn traverse_enters_branch_scopes() {
    let allocator = Allocator::default();
    let (mut program, scoping) = parse_with_scoping(&allocator);
    let (outer_scope_id, if_stmt) = get_if_statement(&program);
    let consequent_scope_id = if_stmt.consequent_scope_id.get().unwrap();
    let alternate_scope_id = if_stmt.alternate_scope_id.get().unwrap();

    let mut traverser = BranchScopes::default();
    let scoping = traverse_mut(&mut traverser, &allocator, &mut program, scoping, ());

    // A branch scope is a block scope, but not a var-hoisting scope.
    assert_eq!(
        traverser.contexts[1..],
        [
            (consequent_scope_id, consequent_scope_id, outer_scope_id),
            (alternate_scope_id, alternate_scope_id, outer_scope_id),
        ]
    );

    // The branch scopes are the `if` statement's direct child scopes.
    let inserted_scope_id = traverser.inserted_scope_id.unwrap();
    assert_eq!(scoping.scope_parent_id(consequent_scope_id), Some(inserted_scope_id));
    assert_eq!(scoping.scope_parent_id(alternate_scope_id), Some(inserted_scope_id));
}

/// Counts scope events, and replaces each function declaration branch while visiting it.
#[derive(Default)]
struct ReplaceBranches<'a> {
    allocator: Option<&'a Allocator>,
    entered: Vec<ScopeId>,
    left: usize,
}

impl ReplaceBranches<'_> {
    fn enter(&mut self, scope_id: &Cell<Option<ScopeId>>) {
        self.entered.push(scope_id.get().unwrap());
    }
}

fn is_branch_function(stmt: &Statement) -> bool {
    matches!(stmt, Statement::FunctionDeclaration(func) if func.id.as_ref().unwrap().name != "outer")
}

impl<'a> VisitMut<'a> for ReplaceBranches<'a> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.enter(scope_id);
    }

    fn leave_scope(&mut self) {
        self.left += 1;
    }

    fn visit_statement(&mut self, stmt: &mut Statement<'a>) {
        walk_mut::walk_statement(self, stmt);
        if is_branch_function(stmt) {
            let _ = stmt.take_in(&self.allocator.unwrap());
        }
    }
}

impl<'a> VisitJsMut<'a> for ReplaceBranches<'a> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.enter(scope_id);
    }

    fn leave_scope(&mut self) {
        self.left += 1;
    }

    fn visit_statement(&mut self, stmt: &mut Statement<'a>) {
        walk_js_mut::walk_statement(self, stmt);
        if is_branch_function(stmt) {
            let _ = stmt.take_in(&self.allocator.unwrap());
        }
    }
}

#[test]
fn visit_mut_leaves_branch_scopes_after_replacing_branches() {
    for visit_js in [false, true] {
        let allocator = Allocator::default();
        let (mut program, _) = parse_with_scoping(&allocator);
        let (_, if_stmt) = get_if_statement(&program);
        let consequent_scope_id = if_stmt.consequent_scope_id.get().unwrap();
        let alternate_scope_id = if_stmt.alternate_scope_id.get().unwrap();

        let mut visitor = ReplaceBranches { allocator: Some(&allocator), ..Default::default() };
        if visit_js {
            VisitJsMut::visit_program(&mut visitor, &mut program);
        } else {
            VisitMut::visit_program(&mut visitor, &mut program);
        }

        // Program, `outer`, consequent branch, `f`, alternate branch, `g`.
        assert_eq!(visitor.entered.len(), 6);
        assert_eq!(visitor.entered[2], consequent_scope_id);
        assert_eq!(visitor.entered[4], alternate_scope_id);
        assert_eq!(visitor.left, visitor.entered.len());
        let (_, if_stmt) = get_if_statement(&program);
        assert!(!if_stmt.consequent_has_scope() && !if_stmt.alternate_has_scope());
    }
}
