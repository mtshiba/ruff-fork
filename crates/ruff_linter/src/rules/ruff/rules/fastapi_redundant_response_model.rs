use ruff_diagnostics::{AlwaysFixableViolation, Diagnostic, Fix};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::{Decorator, Expr, ExprCall, Keyword, StmtFunctionDef};
use ruff_python_semantic::{Modules, SemanticModel};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::fix::edits::{remove_argument, Parentheses};
use crate::rules::ruff::fastapi::is_fastapi_route_decorator;

/// ## What it does
/// Checks for FastAPI routes that use the optional `response_model` parameter
/// with the same type as the return type.
///
/// ## Why is this bad?
/// FastAPI routes automatically infer the response model type from the return
/// type, so specifying it explicitly is redundant.
///
/// The `response_model` parameter is used to override the default response
/// model type. For example, `response_model` can be used to specify that
/// a non-serializable response type should instead be serialized via an
/// alternative type.
///
/// For more information, see the [FastAPI documentation](https://fastapi.tiangolo.com/tutorial/response-model/).
///
/// ## Example
///
/// ```python
/// from fastapi import FastAPI
/// from pydantic import BaseModel
///
/// app = FastAPI()
///
///
/// class Item(BaseModel):
///     name: str
///
///
/// @app.post("/items/", response_model=Item)
/// async def create_item(item: Item) -> Item:
///     return item
/// ```
///
/// Use instead:
///
/// ```python
/// from fastapi import FastAPI
/// from pydantic import BaseModel
///
/// app = FastAPI()
///
///
/// class Item(BaseModel):
///     name: str
///
///
/// @app.post("/items/")
/// async def create_item(item: Item) -> Item:
///     return item
/// ```

#[violation]
pub struct FastApiRedundantResponseModel;

impl AlwaysFixableViolation for FastApiRedundantResponseModel {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("FastAPI route with redundant `response_model` argument")
    }

    fn fix_title(&self) -> String {
        "Remove argument".to_string()
    }
}

/// RUF102
pub(crate) fn fastapi_redundant_response_model(
    checker: &mut Checker,
    function_def: &StmtFunctionDef,
) {
    if !checker.semantic().seen_module(Modules::FASTAPI) {
        return;
    }
    for decorator in &function_def.decorator_list {
        let Some((call, response_model_arg)) =
            check_decorator(function_def, decorator, checker.semantic())
        else {
            continue;
        };
        let mut diagnostic =
            Diagnostic::new(FastApiRedundantResponseModel, response_model_arg.range());
        diagnostic.try_set_fix(|| {
            remove_argument(
                response_model_arg,
                &call.arguments,
                Parentheses::Preserve,
                checker.locator().contents(),
            )
            .map(Fix::unsafe_edit)
        });
        checker.diagnostics.push(diagnostic);
    }
}

fn check_decorator<'a>(
    function_def: &StmtFunctionDef,
    decorator: &'a Decorator,
    semantic: &'a SemanticModel,
) -> Option<(&'a ExprCall, &'a Keyword)> {
    let call = is_fastapi_route_decorator(decorator, semantic)?;
    let response_model_arg = call.arguments.find_keyword("response_model")?;
    let return_value = function_def.returns.as_ref()?;
    if is_identical_types(&response_model_arg.value, return_value, semantic) {
        Some((call, response_model_arg))
    } else {
        None
    }
}

fn is_identical_types(
    response_model_arg: &Expr,
    return_value: &Expr,
    semantic: &SemanticModel,
) -> bool {
    if let (Some(response_mode_name_expr), Some(return_value_name_expr)) = (
        response_model_arg.as_name_expr(),
        return_value.as_name_expr(),
    ) {
        return semantic.resolve_name(response_mode_name_expr)
            == semantic.resolve_name(return_value_name_expr);
    }
    if let (Some(response_mode_subscript), Some(return_value_subscript)) = (
        response_model_arg.as_subscript_expr(),
        return_value.as_subscript_expr(),
    ) {
        return is_identical_types(
            &response_mode_subscript.value,
            &return_value_subscript.value,
            semantic,
        ) && is_identical_types(
            &response_mode_subscript.slice,
            &return_value_subscript.slice,
            semantic,
        );
    }
    if let (Some(response_mode_tuple), Some(return_value_tuple)) = (
        response_model_arg.as_tuple_expr(),
        return_value.as_tuple_expr(),
    ) {
        return response_mode_tuple.elts.len() == return_value_tuple.elts.len()
            && response_mode_tuple
                .elts
                .iter()
                .zip(return_value_tuple.elts.iter())
                .all(|(x, y)| is_identical_types(x, y, semantic));
    }
    false
}
