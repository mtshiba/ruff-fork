use rustpython_ast::{Constant as RpAstConstant, KeywordData};
use rustpython_parser::ast::{ArgData, Expr, ExprKind, Stmt, StmtKind};

use crate::ast::helpers;
use crate::ast::types::{Binding, BindingKind, Range, Scope, ScopeKind};
use crate::checks::{Check, CheckKind};
use crate::pyupgrade::types::Primitive;
use crate::settings::types::PythonVersion;

pub const PY33_PLUS_REMOVE_FUTURES: &[&str] = &[
    "nested_scopes",
    "generators",
    "with_statement",
    "division",
    "absolute_import",
    "with_statement",
    "print_function",
    "unicode_literals",
];

pub const PY37_PLUS_REMOVE_FUTURES: &[&str] = &[
    "nested_scopes",
    "generators",
    "with_statement",
    "division",
    "absolute_import",
    "with_statement",
    "print_function",
    "unicode_literals",
    "generator_stop",
];

/// U008
pub fn super_args(
    scope: &Scope,
    parents: &[&Stmt],
    expr: &Expr,
    func: &Expr,
    args: &[Expr],
) -> Option<Check> {
    if !helpers::is_super_call_with_arguments(func, args) {
        return None;
    }

    // Check: are we in a Function scope?
    if !matches!(scope.kind, ScopeKind::Function { .. }) {
        return None;
    }

    let mut parents = parents.iter().rev();

    // For a `super` invocation to be unnecessary, the first argument needs to match
    // the enclosing class, and the second argument needs to match the first
    // argument to the enclosing function.
    if let [first_arg, second_arg] = args {
        // Find the enclosing function definition (if any).
        if let Some(StmtKind::FunctionDef {
            args: parent_args, ..
        }) = parents
            .find(|stmt| matches!(stmt.node, StmtKind::FunctionDef { .. }))
            .map(|stmt| &stmt.node)
        {
            // Extract the name of the first argument to the enclosing function.
            if let Some(ArgData {
                arg: parent_arg, ..
            }) = parent_args.args.first().map(|expr| &expr.node)
            {
                // Find the enclosing class definition (if any).
                if let Some(StmtKind::ClassDef {
                    name: parent_name, ..
                }) = parents
                    .find(|stmt| matches!(stmt.node, StmtKind::ClassDef { .. }))
                    .map(|stmt| &stmt.node)
                {
                    if let (
                        ExprKind::Name {
                            id: first_arg_id, ..
                        },
                        ExprKind::Name {
                            id: second_arg_id, ..
                        },
                    ) = (&first_arg.node, &second_arg.node)
                    {
                        if first_arg_id == parent_name && second_arg_id == parent_arg {
                            return Some(Check::new(
                                CheckKind::SuperCallWithParameters,
                                Range::from_located(expr),
                            ));
                        }
                    }
                }
            }
        }
    }

    None
}

/// U001
pub fn useless_metaclass_type(targets: &[Expr], value: &Expr, location: Range) -> Option<Check> {
    if targets.len() == 1 {
        if let ExprKind::Name { id, .. } = targets.first().map(|expr| &expr.node).unwrap() {
            if id == "__metaclass__" {
                if let ExprKind::Name { id, .. } = &value.node {
                    if id == "type" {
                        return Some(Check::new(CheckKind::UselessMetaclassType, location));
                    }
                }
            }
        }
    }
    None
}

/// U002
pub fn unnecessary_abspath(func: &Expr, args: &[Expr], location: Range) -> Option<Check> {
    // Validate the arguments.
    if args.len() == 1 {
        if let ExprKind::Name { id, .. } = &args[0].node {
            if id == "__file__" {
                match &func.node {
                    ExprKind::Attribute { attr: id, .. } | ExprKind::Name { id, .. } => {
                        if id == "abspath" {
                            return Some(Check::new(CheckKind::UnnecessaryAbspath, location));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// U004
pub fn useless_object_inheritance(name: &str, bases: &[Expr], scope: &Scope) -> Option<Check> {
    for expr in bases {
        if let ExprKind::Name { id, .. } = &expr.node {
            if id == "object" {
                match scope.values.get(id) {
                    None
                    | Some(Binding {
                        kind: BindingKind::Builtin,
                        ..
                    }) => {
                        return Some(Check::new(
                            CheckKind::UselessObjectInheritance(name.to_string()),
                            Range::from_located(expr),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

/// U003
pub fn type_of_primitive(func: &Expr, args: &[Expr], location: Range) -> Option<Check> {
    // Validate the arguments.
    if args.len() == 1 {
        match &func.node {
            ExprKind::Attribute { attr: id, .. } | ExprKind::Name { id, .. } => {
                if id == "type" {
                    if let ExprKind::Constant { value, .. } = &args[0].node {
                        if let Some(primitive) = Primitive::from_constant(value) {
                            return Some(Check::new(
                                CheckKind::TypeOfPrimitive(primitive),
                                location,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    None
}

/// U010
pub fn unnecessary_future_import(
    version: PythonVersion,
    name: &str,
    location: Range,
) -> Option<Check> {
    if (version >= PythonVersion::Py33 && PY33_PLUS_REMOVE_FUTURES.contains(&name))
        || (version >= PythonVersion::Py37 && PY37_PLUS_REMOVE_FUTURES.contains(&name))
    {
        return Some(Check::new(
            CheckKind::UnnecessaryFutureImport(name.to_string()),
            location,
        ));
    }
    None
}

/// U011
pub fn unnecessary_lru_cache_params(
    version: PythonVersion,
    decorator_list: &[Expr],
) -> Option<Check> {
    for expr in decorator_list.iter() {
        if let ExprKind::Call {
            func,
            args,
            keywords,
        } = &expr.node
        {
            if args.is_empty() {
                match &func.node {
                    ExprKind::Name { id, .. } => {
                        if id == "lru_cache" {
                            if keywords.is_empty() {
                                let check = Some(Check::new(
                                    CheckKind::UnnecessaryLRUCacheParams,
                                    Range::from_located(expr),
                                ));
                                return check;
                            }
                            if version >= PythonVersion::Py39 && keywords.len() == 1 {
                                let keyword = &keywords[0];
                                let KeywordData { arg, value } = &keyword.node;
                                if arg.as_ref().unwrap().as_str() == "maxsize" {
                                    if let ExprKind::Constant {
                                        value: RpAstConstant::None,
                                        kind: None,
                                    } = &value.node
                                    {
                                        let check = Some(Check::new(
                                            CheckKind::UnnecessaryLRUCacheParams,
                                            Range::from_located(expr),
                                        ));
                                        return check;
                                    }
                                }
                            }
                        }
                    }
                    ExprKind::Attribute { value, attr, .. } => {
                        if let ExprKind::Name { id, .. } = &value.node {
                            if attr == "lru_cache" && id == "functools" {
                                if keywords.is_empty() {
                                    let check = Some(Check::new(
                                        CheckKind::UnnecessaryLRUCacheParams,
                                        Range::from_located(expr),
                                    ));
                                    return check;
                                }
                                if version >= PythonVersion::Py39 && keywords.len() == 1 {
                                    let keyword = &keywords[0];
                                    let KeywordData { arg, value } = &keyword.node;
                                    if arg.as_ref().unwrap().as_str() == "maxsize" {
                                        if let ExprKind::Constant {
                                            value: RpAstConstant::None,
                                            kind: None,
                                        } = &value.node
                                        {
                                            let check = Some(Check::new(
                                                CheckKind::UnnecessaryLRUCacheParams,
                                                Range::from_located(expr),
                                            ));
                                            return check;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}
