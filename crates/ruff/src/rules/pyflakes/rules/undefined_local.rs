use std::string::ToString;

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;

#[violation]
pub struct UndefinedLocal {
    pub name: String,
}

impl Violation for UndefinedLocal {
    #[derive_message_formats]
    fn message(&self) -> String {
        let UndefinedLocal { name } = self;
        format!("Local variable `{name}` referenced before assignment")
    }
}

/// F823
pub fn undefined_local(checker: &mut Checker, name: &str) {
    let current = &checker.ctx.scopes[checker.ctx.scope_id];

    // If the name hasn't already been defined in the current scope...
    if !current.kind.is_function() || current.defines(name) {
        return;
    }

    let Some(parent_scope_id) = checker.ctx.scopes.parent(current.id) else {
        return;
    };

    // For every function and module scope above us...
    for scope_id in checker.ctx.scopes.ancestors(parent_scope_id) {
        let scope = &checker.ctx.scopes[scope_id];
        if !(scope.kind.is_function() || scope.kind.is_module()) {
            continue;
        }

        // If the name was defined in that scope...
        if let Some(binding) = scope.get(name).map(|index| &checker.ctx.bindings[*index]) {
            // And has already been accessed in the current scope...
            if let Some((scope_id, location)) = binding.runtime_usage {
                if scope_id == current.id {
                    // Then it's probably an error.
                    checker.diagnostics.push(Diagnostic::new(
                        UndefinedLocal {
                            name: name.to_string(),
                        },
                        location,
                    ));
                    return;
                }
            }
        }
    }
}
