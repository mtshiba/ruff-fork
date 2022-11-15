use fnv::FnvHashMap;
use rustpython_ast::{Expr, ExprKind};

use crate::ast::types::Range;
use crate::ast::visitor;
use crate::ast::visitor::Visitor;
use crate::check_ast::Checker;
use crate::checks::{Check, CheckKind};

struct NameFinder<'a> {
    names: FnvHashMap<&'a str, &'a Expr>,
}

impl NameFinder<'_> {
    fn new() -> Self {
        NameFinder {
            names: Default::default(),
        }
    }
}

impl<'a, 'b> Visitor<'b> for NameFinder<'a>
where
    'b: 'a,
{
    fn visit_expr(&mut self, expr: &'b Expr) {
        match &expr.node {
            ExprKind::Name { id, .. } => {
                self.names.insert(id, expr);
            }
            ExprKind::ListComp { generators, .. }
            | ExprKind::DictComp { generators, .. }
            | ExprKind::SetComp { generators, .. }
            | ExprKind::GeneratorExp { generators, .. } => {
                for comp in generators {
                    self.visit_expr(&comp.iter);
                }
            }
            ExprKind::Lambda { args, body } => {
                visitor::walk_expr(self, body);
                for arg in args.args.iter() {
                    self.names.remove(arg.node.arg.as_str());
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

/// B020
pub fn for_loop_target_overrides_iter(checker: &mut Checker, target: &Expr, iter: &Expr) {
    let target_names = {
        let mut target_finder = NameFinder::new();
        target_finder.visit_expr(target);
        target_finder.names
    };
    let iter_names = {
        let mut iter_finder = NameFinder::new();
        iter_finder.visit_expr(iter);
        iter_finder.names
    };

    for (name, expr) in target_names {
        if iter_names.contains_key(name) {
            checker.add_check(Check::new(
                CheckKind::ForLoopTargetOverridesIter(name.to_string()),
                Range::from_located(expr),
            ));
        }
    }
}
