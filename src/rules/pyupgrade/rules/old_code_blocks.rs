use num_bigint::Sign;
use rustpython_parser::ast::{Cmpop, Constant, Expr, ExprKind, Stmt, Unaryop};
use rustpython_parser::lexer;
use rustpython_parser::lexer::Tok;

use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::fix::Fix;
use crate::registry::{Diagnostic, Rule};
use crate::settings::types::PythonVersion;
use crate::violations;

/// Checks whether the give attribute is from the given path
fn check_path(checker: &Checker, expr: &Expr, path: &[&str]) -> bool {
    checker
        .resolve_call_path(expr)
        .map_or(false, |call_path| call_path.as_slice() == path)
}

fn extract_version(elts: &[Expr]) -> Vec<u32> {
    let mut version: Vec<u32> = vec![];
    for elt in elts {
        if let ExprKind::Constant {
            value: Constant::Int(item),
            ..
        } = &elt.node
        {
            let the_number = item.to_u32_digits();
            match the_number.0 {
                // We do not have a way of handling these values, so return what was gathered
                Sign::Minus | Sign::NoSign => {
                    return version;
                }
                Sign::Plus => {
                    // Assuming that the version will never be above a 32 bit
                    version.push(*the_number.1.get(0).unwrap())
                }
            }
        } else {
            return version;
        }
    }
    version
}

/// Returns true if the user's linting version is greater than the version
/// specified in the tuple
fn compare_version(version: Vec<u32>, py_version: PythonVersion, or_equal: bool) -> bool {
    let mut ver_iter = version.iter();
    // Check the first number (the major version)
    if let Some(first) = ver_iter.next() {
        if *first < 3 {
            return true;
        } else if *first == 3 {
            // Check the second number (the minor version)
            if let Some(first) = ver_iter.next() {
                // If there is an equal, then we need to require one level higher of python
                // version
                if *first < py_version.to_tuple().1 + or_equal as u32 {
                    return true;
                }
            } else {
                // If there is no second number was assumer python 3.0, and upgrade
                return true;
            }
        }
    }
    false
}

/// Converts an if statement that has the py2 block on top
fn fix_py2_block(checker: &mut Checker, stmt: &Stmt, orelse: &[Stmt]) {
    // FOR REVIEWER: pyupgrade had a check to see if the first statement was an if
    // or an elif, and would check for an index based on this. Our parser
    // automatically only sends the start of the statement as the if or elif, so
    // I did not see that as necessary.
    let text = checker
        .locator
        .slice_source_code_range(&Range::from_located(stmt));
    let tokens = lexer::make_tokenizer(&text);
    let has_else = tokens.map(|token| token.unwrap().1 == Tok::Else).any(|x| x);
    // The statement MUST have an else
    if !has_else {
        return;
    }
    let else_statement = orelse.last().unwrap();
    let range = Range::new(stmt.location, else_statement.location);
    let mut diagnostic = Diagnostic::new(violations::OldCodeBlocks, range);
    if checker.patch(diagnostic.kind.rule()) {
        diagnostic.amend(Fix::deletion(stmt.location, else_statement.location));
    }
    checker.diagnostics.push(diagnostic);
}

/// UP037
pub fn old_code_blocks(
    checker: &mut Checker,
    stmt: &Stmt,
    test: &Expr,
    body: &[Stmt],
    orelse: &[Stmt],
) {
    // NOTE: Pyupgrade ONLY works if `sys.version_info` is on the left
    // We have to have an else statement in order to refactor
    if orelse.is_empty() {
        return;
    }
    match &test.node {
        ExprKind::Compare {
            left,
            ops,
            comparators,
        } => {
            if check_path(checker, left, &["sys", "version_info"]) {
                // We need to ensure we have only one operation and one comparison
                if ops.len() == 1 && comparators.len() == 1 {
                    // DO NOT forget to check for LT or LTE
                    if let ExprKind::Tuple { elts, ctx } = &comparators.get(0).unwrap().node {
                        let op = ops.get(0).unwrap();
                        // Here we check for the correct operator, and also adjust the desired
                        // target based on whether we are accepting equal to
                        if op == &Cmpop::Lt || op == &Cmpop::LtE {
                            let version = extract_version(elts);
                            println!("{:?}", version);
                            if compare_version(
                                version,
                                checker.settings.target_version,
                                op == &Cmpop::LtE,
                            ) {
                                fix_py2_block(checker, stmt, orelse);
                            }
                        }
                    }
                }
            }
        }
        ExprKind::Attribute { value, attr, ctx } => {
            // if six.PY2
            if check_path(checker, test, &["six", "PY2"]) {}
        }
        ExprKind::UnaryOp { op, operand } => {
            // if not six.PY3
            if check_path(checker, test, &["six", "PY3"]) && op == &Unaryop::Not {}
        }
        _ => (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(PythonVersion::Py36, vec![2], true, true; "compare-2.0")]
    #[test_case(PythonVersion::Py36, vec![2, 0], true, true; "compare-2.0-whole")]
    #[test_case(PythonVersion::Py36, vec![3], true, true; "compare-3.0")]
    #[test_case(PythonVersion::Py36, vec![3, 0], true, true; "compare-3.0-whole")]
    #[test_case(PythonVersion::Py36, vec![3, 1], true, true; "compare-3.1")]
    #[test_case(PythonVersion::Py36, vec![3, 5], true, true; "compare-3.5")]
    #[test_case(PythonVersion::Py36, vec![3, 6], true, true; "compare-3.6")]
    #[test_case(PythonVersion::Py36, vec![3, 6], false, false; "compare-3.6-not-equal")]
    #[test_case(PythonVersion::Py36, vec![3, 7], false , false; "compare-3.7")]
    #[test_case(PythonVersion::Py310, vec![3,9], true, true; "compare-3.9")]
    #[test_case(PythonVersion::Py310, vec![3, 11], true, false; "compare-3.11")]
    fn test_compare_version(version: PythonVersion, version_vec: Vec<u32>, or_equal: bool, expected: bool) {
        assert_eq!(compare_version(version_vec, version, or_equal), expected);
    }
}
