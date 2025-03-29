use crate::goto::{find_goto_target, GotoTarget};
use crate::{Db, NavigationTargets, RangeInfo};
use red_knot_python_semantic::types::Type;
use red_knot_python_semantic::{HasType, SemanticModel};
use ruff_db::files::{File, FileRange};
use ruff_db::parsed::parsed_module;
use ruff_text_size::{Ranged, TextSize};

pub fn hover(db: &dyn Db, file: File, offset: TextSize) -> Option<RangeInfo<Hover>> {
    let parsed = parsed_module(db.upcast(), file);
    let goto_target = find_goto_target(parsed, offset)?;

    let model = SemanticModel::new(db.upcast(), file);

    let ty = match goto_target {
        GotoTarget::Expression(expression) => expression.inferred_type(&model),
        GotoTarget::FunctionDef(function) => function.inferred_type(&model),
        GotoTarget::ClassDef(class) => class.inferred_type(&model),
        GotoTarget::Parameter(parameter) => parameter.inferred_type(&model),
        GotoTarget::Alias(alias) => alias.inferred_type(&model),
        GotoTarget::ExceptVariable(except) => except.inferred_type(&model),
        GotoTarget::KeywordArgument(argument) => {
            // TODO: Pyright resolves the declared type of the matching parameter. This seems more accurate
            // than using the inferred value.
            argument.value.inferred_type(&model)
        }

        // TODO: Better support for go to type definition in match pattern.
        // This may require improving type inference (e.g. it currently doesn't handle `...rest`)
        // but it also requires a new API to query the type because implementing `HasType` for `PatternMatchMapping`
        // is ambiguous.
        GotoTarget::PatternMatchRest(_)
        | GotoTarget::PatternKeywordArgument(_)
        | GotoTarget::PatternMatchStarName(_)
        | GotoTarget::PatternMatchAsName(_) => return None,

        // TODO: Resolve the module; The type inference already does all the work
        // but type isn't stored anywhere. We should either extract the logic
        // for resolving the module from a ImportFromStmt or store the type during semantic analysis
        GotoTarget::ImportedModule(_) => return None,

        // Targets without a type definition.
        GotoTarget::TypeParamTypeVarName(_)
        | GotoTarget::TypeParamParamSpecName(_)
        | GotoTarget::TypeParamTypeVarTupleName(_) => return None,
    };

    // TODO: Most LSPs show the symbol declaration: e.g. `class Foo: x: str` instead of just the name of the type
    // It also seems possible to return more than one markdown element and clients then display all of the
    // markdown blocks.
    tracing::debug!(
        "Inferred type of covering node is {}",
        ty.display(db.upcast())
    );

    Some(RangeInfo {
        file_range: FileRange::new(file, goto_target.range()),
        info: Hover::Type(ty),
    })
}

pub enum Hover<'db> {
    Type(Type<'db>),
}

impl Hover<'_> {
    pub fn to_string(&self, db: &dyn Db) -> String {
        match self {
            Hover::Type(ty) => ty.display(db.upcast()).to_string(),
        }
    }
}
