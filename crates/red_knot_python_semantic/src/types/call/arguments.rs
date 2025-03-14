use super::Type;

/// Typed arguments for a single call, in source order.
#[derive(Clone, Debug, Default)]
pub(crate) struct CallArguments<'a, 'db>(Vec<Argument<'a, 'db>>);

impl<'a, 'db> CallArguments<'a, 'db> {
    /// Create a [`CallArguments`] with no arguments.
    pub(crate) fn none() -> Self {
        Self(Vec::new())
    }

    /// Create a [`CallArguments`] from an iterator over non-variadic positional argument types.
    pub(crate) fn positional(positional_tys: impl IntoIterator<Item = Type<'db>>) -> Self {
        positional_tys
            .into_iter()
            .map(Argument::positional)
            .collect()
    }

    /// Prepend an extra positional argument.
    pub(crate) fn with_self(&self, self_ty: Type<'db>) -> Self {
        let mut arguments = Vec::with_capacity(self.0.len() + 1);
        arguments.push(Argument::synthetic(self_ty));
        arguments.extend_from_slice(&self.0);
        Self(arguments)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Argument<'a, 'db>> {
        self.0.iter()
    }

    // TODO this should be eliminated in favor of [`bind_call`]
    pub(crate) fn first_argument(&self) -> Option<Type<'db>> {
        self.0.first().map(Argument::argument_type)
    }

    // TODO this should be eliminated in favor of [`bind_call`]
    pub(crate) fn second_argument(&self) -> Option<Type<'db>> {
        self.0.get(1).map(Argument::argument_type)
    }

    // TODO this should be eliminated in favor of [`bind_call`]
    pub(crate) fn third_argument(&self) -> Option<Type<'db>> {
        self.0.get(2).map(Argument::argument_type)
    }
}

impl<'db, 'a, 'b> IntoIterator for &'b CallArguments<'a, 'db> {
    type Item = &'b Argument<'a, 'db>;
    type IntoIter = std::slice::Iter<'b, Argument<'a, 'db>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, 'db> FromIterator<Argument<'a, 'db>> for CallArguments<'a, 'db> {
    fn from_iter<T: IntoIterator<Item = Argument<'a, 'db>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Argument<'a, 'db> {
    kind: ArgumentKind<'a>,
    /// The inferred type of this argument.
    argument_type: Type<'db>,
}

impl<'a, 'db> Argument<'a, 'db> {
    pub(crate) fn keyword(name: &'a str, argument_type: Type<'db>) -> Self {
        Self {
            kind: ArgumentKind::Keyword(name),
            argument_type,
        }
    }

    pub(crate) fn keywords(argument_type: Type<'db>) -> Self {
        Self {
            kind: ArgumentKind::Keywords,
            argument_type,
        }
    }

    pub(crate) fn positional(argument_type: Type<'db>) -> Self {
        Self {
            kind: ArgumentKind::Positional,
            argument_type,
        }
    }

    pub(crate) fn synthetic(argument_type: Type<'db>) -> Self {
        Self {
            kind: ArgumentKind::Synthetic,
            argument_type,
        }
    }

    pub(crate) fn variadic(argument_type: Type<'db>) -> Self {
        Self {
            kind: ArgumentKind::Variadic,
            argument_type,
        }
    }

    pub(crate) fn kind(&self) -> ArgumentKind<'a> {
        self.kind
    }

    pub(crate) fn argument_type(&self) -> Type<'db> {
        self.argument_type
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ArgumentKind<'a> {
    /// The synthetic `self` or `cls` argument, which doesn't appear explicitly at the call site.
    Synthetic,
    /// A positional argument.
    Positional,
    /// A starred positional argument (e.g. `*args`).
    Variadic,
    /// A keyword argument (e.g. `a=1`).
    Keyword(&'a str),
    /// The double-starred keywords argument (e.g. `**kwargs`).
    Keywords,
}
