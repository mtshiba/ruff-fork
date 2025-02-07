use crate::{
    types::{Type, UnionType},
    Db,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Boundness {
    Bound,
    PossiblyUnbound,
}

/// Indicates whether a symbol is re-exported using the [import conventions].
///
/// [import conventions]: https://typing.readthedocs.io/en/latest/spec/distributing.html#import-conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReExport {
    /// Symbol is either defined by an import statement and is re-exported, or it could be defined
    /// by any other statement or expression.
    ///
    /// For example, in the following code:
    /// ```py
    /// import foo as foo
    /// from foo import Bar as Bar
    ///
    /// baz = 1
    /// ```
    ///
    /// All the symbols (`foo`, `Bar`, and `baz`) are re-exported.
    Yes,

    /// Symbol is defined by an import statement and is not re-exported.
    ///
    /// For example, in the following code:
    /// ```py
    /// import foo
    /// from foo import Bar
    /// ```
    ///
    /// Both `foo` (module) and `Bar` are not re-exported.
    No,

    /// Symbol is maybe re-exported.
    ///
    /// For example, in the following code:
    /// ```py
    /// if flag:
    ///     import foo
    /// else:
    ///     import foo as foo
    /// ```
    ///
    /// The `foo` symbol is maybe re-exported, depending on the value of `flag`.
    ///
    /// Or, in the following code:
    /// ```py
    /// import foo
    ///
    /// if flag:
    ///     foo: int = 1
    /// ```
    ///
    /// The `foo` symbol is maybe re-exported if the truthiness of `flag` is ambiguous.
    Maybe,
}

impl ReExport {
    pub(crate) fn or(self, other: ReExport) -> ReExport {
        match (self, other) {
            (ReExport::Yes, ReExport::Yes) => ReExport::Yes,
            (ReExport::No, ReExport::No) => ReExport::No,
            _ => ReExport::Maybe,
        }
    }
}

/// The result of a symbol lookup, which can either be a (possibly unbound) type
/// or a completely unbound symbol.
///
/// Consider this example:
/// ```py
/// bound = 1
///
/// if flag:
///     possibly_unbound = 2
/// ```
///
/// If we look up symbols in this scope, we would get the following results:
/// ```rs
/// bound:             Symbol::Type(Type::IntLiteral(1), Boundness::Bound),
/// possibly_unbound:  Symbol::Type(Type::IntLiteral(2), Boundness::PossiblyUnbound),
/// non_existent:      Symbol::Unbound,
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Symbol<'db> {
    Type(Type<'db>, ReExport, Boundness),
    Unbound,
}

impl<'db> Symbol<'db> {
    pub(crate) fn is_unbound(&self) -> bool {
        matches!(self, Symbol::Unbound)
    }

    /// Returns the type of the symbol, ignoring possible unboundness.
    ///
    /// If the symbol is *definitely* unbound, this function will return `None`. Otherwise,
    /// if there is at least one control-flow path where the symbol is bound, return the type.
    pub(crate) fn ignore_possibly_unbound(&self) -> Option<Type<'db>> {
        match self {
            Symbol::Type(ty, _, _) => Some(*ty),
            Symbol::Unbound => None,
        }
    }

    #[cfg(test)]
    #[track_caller]
    pub(crate) fn expect_type(self) -> Type<'db> {
        self.ignore_possibly_unbound()
            .expect("Expected a (possibly unbound) type, not an unbound symbol")
    }

    /// Fallback (partially or fully) to another symbol if `self` is partially or fully unbound.
    ///
    /// 1. If `self` is definitely bound, return `self` without evaluating `fallback_fn()`.
    /// 2. Else, evaluate `fallback_fn()`:
    ///    a. If `self` is definitely unbound, return the result of `fallback_fn()`.
    ///    b. Else, if `fallback` is definitely unbound, return `self`.
    ///    c. Else, if `self` is possibly unbound and `fallback` is definitely bound,
    ///       return `Symbol(<union of self-type and fallback-type>, Boundness::Bound)`
    ///    d. Else, if `self` is possibly unbound and `fallback` is possibly unbound,
    ///       return `Symbol(<union of self-type and fallback-type>, Boundness::PossiblyUnbound)`
    #[must_use]
    pub(crate) fn or_fall_back_to(
        self,
        db: &'db dyn Db,
        fallback_fn: impl FnOnce() -> Self,
    ) -> Self {
        match self {
            Symbol::Type(_, _, Boundness::Bound) => self,
            Symbol::Unbound => fallback_fn(),
            Symbol::Type(self_ty, _, Boundness::PossiblyUnbound) => match fallback_fn() {
                Symbol::Unbound => self,
                Symbol::Type(fallback_ty, fallback_re_export, fallback_boundness) => Symbol::Type(
                    UnionType::from_elements(db, [self_ty, fallback_ty]),
                    fallback_re_export,
                    fallback_boundness,
                ),
            },
        }
    }

    #[must_use]
    pub(crate) fn map_type(self, f: impl FnOnce(Type<'db>) -> Type<'db>) -> Symbol<'db> {
        match self {
            Symbol::Type(ty, re_export, boundness) => Symbol::Type(f(ty), re_export, boundness),
            Symbol::Unbound => Symbol::Unbound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::setup_db;

    #[test]
    fn test_symbol_or_fall_back_to() {
        use Boundness::{Bound, PossiblyUnbound};

        let db = setup_db();
        let ty1 = Type::IntLiteral(1);
        let ty2 = Type::IntLiteral(2);

        // Start from an unbound symbol
        assert_eq!(
            Symbol::Unbound.or_fall_back_to(&db, || Symbol::Unbound),
            Symbol::Unbound
        );
        assert_eq!(
            Symbol::Unbound.or_fall_back_to(&db, || Symbol::Type(
                ty1,
                ReExport::Yes,
                PossiblyUnbound
            )),
            Symbol::Type(ty1, ReExport::Yes, PossiblyUnbound)
        );
        assert_eq!(
            Symbol::Unbound.or_fall_back_to(&db, || Symbol::Type(ty1, ReExport::Yes, Bound)),
            Symbol::Type(ty1, ReExport::Yes, Bound)
        );

        // Start from a possibly unbound symbol
        assert_eq!(
            Symbol::Type(ty1, ReExport::Yes, PossiblyUnbound)
                .or_fall_back_to(&db, || Symbol::Unbound),
            Symbol::Type(ty1, ReExport::Yes, PossiblyUnbound)
        );
        assert_eq!(
            Symbol::Type(ty1, ReExport::Yes, PossiblyUnbound)
                .or_fall_back_to(&db, || Symbol::Type(ty2, ReExport::Yes, PossiblyUnbound)),
            Symbol::Type(
                UnionType::from_elements(&db, [ty1, ty2]),
                ReExport::Yes,
                PossiblyUnbound
            )
        );
        assert_eq!(
            Symbol::Type(ty1, ReExport::Yes, PossiblyUnbound)
                .or_fall_back_to(&db, || Symbol::Type(ty2, ReExport::Yes, Bound)),
            Symbol::Type(
                UnionType::from_elements(&db, [ty1, ty2]),
                ReExport::Yes,
                Bound
            )
        );

        // Start from a definitely bound symbol
        assert_eq!(
            Symbol::Type(ty1, ReExport::Yes, Bound).or_fall_back_to(&db, || Symbol::Unbound),
            Symbol::Type(ty1, ReExport::Yes, Bound)
        );
        assert_eq!(
            Symbol::Type(ty1, ReExport::Yes, Bound).or_fall_back_to(&db, || Symbol::Type(
                ty2,
                ReExport::Yes,
                PossiblyUnbound
            )),
            Symbol::Type(ty1, ReExport::Yes, Bound)
        );
        assert_eq!(
            Symbol::Type(ty1, ReExport::Yes, Bound).or_fall_back_to(&db, || Symbol::Type(
                ty2,
                ReExport::Yes,
                Bound
            )),
            Symbol::Type(ty1, ReExport::Yes, Bound)
        );
    }
}
