use super::{ClassType, KnownClass, Type};
use crate::Db;
use itertools::Itertools;
use rustc_hash::FxHashSet;
use std::borrow::Cow;
use std::collections::{hash_set, VecDeque};
use std::iter::{FusedIterator, Once};
use std::ops::Deref;

/// The resolved possible [method resolution order]s for a single class.
///
/// [method resolution order]: https://docs.python.org/3/glossary.html#term-method-resolution-order
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum MroPossibilities<'db> {
    /// It can be statically determined that there is only 1 possible `__mro__`
    /// outcome for this class and the outcome is that class creation always succeeds
    /// with the same MRO. Here is the successful MRO:
    SingleSuccess(Mro<'db>),

    /// There are multiple possible `__mro__` values for this class, but they would
    /// all lead to the class being successfully created. Here are the different
    /// possibilities:
    MultipleSuccesses(FxHashSet<Mro<'db>>),

    /// It can be statically determined that the `__mro__` possibilities for this class
    /// (possibly one, possibly many) always fail. Here are the various possible
    /// bases that all lead to class creation failing:
    CertainFailure(FxHashSet<BasesList<'db>>),

    /// There are multiple possible `__mro__`s for this class. Some of these
    /// possibilities result in the class being successfully created; some of them
    /// result in class creation failure.
    PossibleSuccess {
        possible_mros: FxHashSet<Mro<'db>>,
        failure_cases: FxHashSet<BasesList<'db>>,
    },
}

impl<'db> MroPossibilities<'db> {
    /// Return the possible Method Resolution Orders ("MRO"s) for this class.
    ///
    /// See [`ClassType::mro_possibilities`] for more details.
    pub(super) fn of_class(db: &'db dyn Db, class: ClassType<'db>) -> Self {
        let bases = class.bases(db);

        // Start with some fast paths for some common occurrences:
        if !bases.iter().any(Type::is_union) {
            if let Some(short_circuit) = mro_of_class_fast_path(db, class, &bases) {
                return short_circuit;
            }
        }

        mro_of_class_slow_path(db, class, &bases)
    }

    pub(super) fn iter<'s>(
        &'s self,
        db: &'db dyn Db,
        class: ClassBase<'db>,
    ) -> MroPossibilityIterator<'s, 'db> {
        match self {
            Self::CertainFailure { .. } => {
                MroPossibilityIterator::SingleFailure(std::iter::once(Mro::from_error(db, class)))
            }
            Self::SingleSuccess(single_mro) => {
                MroPossibilityIterator::SingleSuccess(std::iter::once(single_mro))
            }
            Self::MultipleSuccesses(multiple_mros) => {
                MroPossibilityIterator::MultipleSuccesses(multiple_mros.iter())
            }
            Self::PossibleSuccess {
                possible_mros,
                failure_cases: _,
            } => MroPossibilityIterator::SuccessesAndFailures {
                successes: possible_mros.iter(),
                failures: std::iter::once(Mro::from_error(db, class)),
            },
        }
    }

    /// Return a `HashSet` representing various possible ways in which
    /// construction of the class might have raised `TypeError` at runtime.
    /// Return `None` if construction of the class always succeeds at runtime.
    ///
    /// Each element in the returned `HashSet` is a [`BasesList`] representing
    /// one possible value of the class's `__bases__` at runtime. The set is
    /// only made up of [`BasesList`]s that would cause the class creation to
    /// fail due to it being impossible to construct a consistent MRO from said
    /// list of bases.
    pub(super) fn possible_errors(&self) -> Option<&FxHashSet<BasesList<'db>>> {
        match self {
            Self::CertainFailure(failure_cases) | Self::PossibleSuccess { failure_cases, .. } => {
                Some(failure_cases)
            }
            Self::SingleSuccess(_) | Self::MultipleSuccesses(_) => None,
        }
    }

    /// Create an [`MroPossibilities`] instance that reflects the fact
    /// that the given class can have only one possible MRO at runtime.
    fn single(mro: impl Into<Mro<'db>>) -> Self {
        Self::SingleSuccess(mro.into())
    }

    /// Create an [`MroPossibilities`] instance that reflects the fact
    /// that the given class could potentially have multiple possible MROs at
    /// runtime (and/or that the class creation could possibly fail
    /// altogether).
    fn possibly_many(
        mut possibilities: FxHashSet<Mro<'db>>,
        mut errors: FxHashSet<BasesList<'db>>,
    ) -> Self {
        debug_assert_ne!(
            possibilities.len().saturating_add(errors.len()),
            0,
            "There should always be at least one possible mro outcome"
        );
        match (possibilities.len(), errors.len()) {
            (1, 0) => Self::SingleSuccess(possibilities.into_iter().next().unwrap()),
            (_, 0) => {
                possibilities.shrink_to_fit();
                Self::MultipleSuccesses(possibilities)
            }
            (0, _) => {
                errors.shrink_to_fit();
                Self::CertainFailure(errors)
            }
            _ => {
                possibilities.shrink_to_fit();
                errors.shrink_to_fit();
                Self::PossibleSuccess {
                    possible_mros: possibilities,
                    failure_cases: errors,
                }
            }
        }
    }
}

/// Fast path for calculating a class's MRO.
///
/// This fast path is only valid if we know that none of the bases is a
/// [`Type::Union`]
fn mro_of_class_fast_path<'db>(
    db: &'db dyn Db,
    class: ClassType<'db>,
    bases: &[Type<'db>],
) -> Option<MroPossibilities<'db>> {
    match bases {
        // 0 bases means that it must be `object` itself.
        //
        // The case for `object` itself isn't really that common,
        // but we may as well handle it here, since it's known and easy:
        [] => {
            debug_assert_eq!(
                Type::Class(class),
                KnownClass::Object.to_class(db),
                "Only `object` should have 0 bases in Python"
            );
            Some(MroPossibilities::single([class]))
        }

        // The class has a single base.
        //
        // That could be an explicit base (`class A(B): pass`),
        // or an implicit base (which is always `object`: `class A: pass`).
        [single_base] => {
            let object = KnownClass::Object.to_class(db);
            let mro = if single_base == &object {
                MroPossibilities::single([class, object.expect_class()])
            } else {
                let mut possibilities = FxHashSet::default();
                let base = ClassBase::from(single_base);
                for possibility in base.mro_possibilities(db).iter(db, base) {
                    possibilities.insert(
                        std::iter::once(ClassBase::Class(class))
                            .chain(possibility.iter().copied())
                            .collect(),
                    );
                }
                MroPossibilities::possibly_many(possibilities, FxHashSet::default())
            };
            Some(mro)
        }

        // The class has multiple bases.
        //
        // At this point, whatever we do isn't really going to be "fast",
        // so we may as well fallback to the slow path below.
        // Even though we know that none of our direct bases is a union type,
        // that doesn't mean that none of our indirect bases is a union type...
        _ => None,
    }
}

/// Slow path for calculating the MRO of a class.
///
/// We fall back to this path if the class has multiple bases
/// (of which any might be a union type),
/// or it has a single base, and the base is a union type.
fn mro_of_class_slow_path<'db>(
    db: &'db dyn Db,
    class: ClassType<'db>,
    bases: &[Type<'db>],
) -> MroPossibilities<'db> {
    let bases_possibilities = fork_bases(db, bases);
    debug_assert_ne!(bases_possibilities.len(), 0);
    let mut mro_possibilities = FxHashSet::default();
    let mut mro_errors = FxHashSet::default();

    for bases_possibility in bases_possibilities {
        match &*bases_possibility {
            [] => panic!("Only `object` should ever have 0 bases, which should have been handled in a fast path"),

            // fast path for a common case: only inherits from a single base
            [single_base] => {
                let object = ClassBase::builtins_object(db);
                if *single_base == object {
                    mro_possibilities.insert(Mro::from([ClassBase::Class(class), object]));
                } else {
                    for possibility in single_base.mro_possibilities(db).iter(db, *single_base) {
                        mro_possibilities.insert(
                            std::iter::once(ClassBase::Class(class))
                                .chain(possibility.iter().copied())
                                .collect(),
                        );
                    }
                }
            }

            // slow path of the slow path: fall back to full C3 linearisation algorithm
            // as described in https://docs.python.org/3/howto/mro.html#python-2-3-mro
            //
            // For a Python-3 translation of the algorithm described in that document,
            // see https://gist.github.com/AlexWaygood/674db1fce6856a90f251f63e73853639
            _ => {
                let possible_mros_per_base: Vec<(ClassBase, Cow<MroPossibilities>)> = bases_possibility
                    .iter()
                    .map(|base| (*base, base.mro_possibilities(db)))
                    .collect();

                let mro_cartesian_product = possible_mros_per_base
                    .iter()
                    .map(|(base, mro_possibilities)| mro_possibilities.iter(db, *base))
                    .multi_cartesian_product();

                // Each `possible_mros_of_bases` is a concrete possibility of the list of mros of all of the bases:
                // where the bases are `[B1, B2, B..N]`, `possible_mros_of_bases` represents one possibility of
                // `[mro_of_B1, mro_of_B2, mro_of_B..N]`
                for possible_mros_of_bases in mro_cartesian_product {
                    let possible_mros_of_bases: Vec<VecDeque<ClassBase>> = possible_mros_of_bases
                        .into_iter()
                        .map(|mro|mro.iter().copied().collect())
                        .collect();
                    let linearized = c3_merge(
                        std::iter::once(VecDeque::from([ClassBase::Class(class)]))
                            .chain(possible_mros_of_bases)
                            .chain(std::iter::once(bases_possibility.iter().copied().collect()))
                            .collect(),
                    );
                    match linearized {
                        Some(mro) => {
                            mro_possibilities.insert(mro);
                        },
                        None => {
                            if !mro_errors.contains(&bases_possibility) {
                                mro_errors.insert(bases_possibility.clone());
                            }
                        },
                    };
                }
            }
        }
    }

    MroPossibilities::possibly_many(mro_possibilities, mro_errors)
}

/// Given a list of types representing the bases of a class,
/// of which one or more types could be a [`Type::Union`] variant,
/// resolve the list into a "union of bases lists", where each list in the union
/// is guaranteed not to hold any bases that are a [`Type::Union`].
fn fork_bases<'db>(db: &'db dyn Db, bases: &[Type<'db>]) -> BasesPossibilities<'db> {
    // Fast path for the common case, where none of the bases is a [`Type::Union`]:
    if !bases.iter().any(Type::is_union) {
        return BasesPossibilities::Single(bases.iter().map(ClassBase::from).collect());
    }

    // Slow path: one or more of the bases is a [`Type::Union`]
    let mut possibilities = FxHashSet::from_iter([BasesList::default()]);
    for base in bases {
        possibilities = add_next_base(db, &possibilities, *base);
    }
    BasesPossibilities::Multiple(possibilities)
}

fn add_next_base<'db>(
    db: &'db dyn Db,
    bases_possibilities: &FxHashSet<BasesList<'db>>,
    next_base: Type<'db>,
) -> FxHashSet<BasesList<'db>> {
    let mut new_possibilities = FxHashSet::default();
    let mut add_non_union_base = |fork: &[ClassBase<'db>], base: Type<'db>| {
        new_possibilities.insert(
            fork.iter()
                .copied()
                .chain(std::iter::once(ClassBase::from(base)))
                .collect(),
        );
    };
    match next_base {
        Type::Union(union) => {
            for element in union.elements(db) {
                for existing_possibility in bases_possibilities {
                    add_non_union_base(existing_possibility, *element);
                }
            }
        }
        _ => {
            for possibility in bases_possibilities {
                add_non_union_base(possibility, next_base);
            }
        }
    }
    debug_assert_ne!(new_possibilities.len(), 0);
    new_possibilities
}

/// The possible value of `__bases__` for a given class.
///
/// Whereas [`ClassType::bases`] returns a list of types in which any type
/// might be a [`Type::Union`], this enum transforms the list of types so that we
/// have a union of possible `__bases__` lists rather than a single list
/// that could contain a union.
enum BasesPossibilities<'db> {
    /// There is only one possible value for the class's `__bases__`; here it is
    Single(BasesList<'db>),

    /// There are multiple possible values for the class's `__bases__` tuple
    Multiple(FxHashSet<BasesList<'db>>),
}

impl<'db> BasesPossibilities<'db> {
    fn len(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Multiple(possibilities) => possibilities.len(),
        }
    }
}

impl<'db> IntoIterator for BasesPossibilities<'db> {
    type IntoIter = BasesPossibilityIterator<'db>;
    type Item = BasesList<'db>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Single(bases) => BasesPossibilityIterator::Single(std::iter::once(bases)),
            Self::Multiple(bases) => BasesPossibilityIterator::Multiple(bases.into_iter()),
        }
    }
}

/// Iterates over the various possible [`BasesList`]s for a given class
enum BasesPossibilityIterator<'db> {
    Single(std::iter::Once<BasesList<'db>>),
    Multiple(hash_set::IntoIter<BasesList<'db>>),
}

impl<'db> Iterator for BasesPossibilityIterator<'db> {
    type Item = BasesList<'db>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(iter) => iter.next(),
            Self::Multiple(iter) => iter.next(),
        }
    }
}

impl FusedIterator for BasesPossibilityIterator<'_> {}

/// Iterates over the various possible [`Mro`]s for a given class.
#[derive(Clone)]
pub(super) enum MroPossibilityIterator<'a, 'db> {
    SingleSuccess(Once<&'a Mro<'db>>),
    SingleFailure(Once<Mro<'db>>),
    MultipleSuccesses(hash_set::Iter<'a, Mro<'db>>),
    SuccessesAndFailures {
        successes: hash_set::Iter<'a, Mro<'db>>,
        failures: Once<Mro<'db>>,
    },
}

impl<'a, 'db> Iterator for MroPossibilityIterator<'a, 'db> {
    type Item = Cow<'a, Mro<'db>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::SingleSuccess(iter) => iter.next().map(Cow::Borrowed),
            Self::SingleFailure(iter) => iter.next().map(Cow::Owned),
            Self::MultipleSuccesses(iter) => iter.next().map(Cow::Borrowed),
            Self::SuccessesAndFailures {
                successes,
                failures,
            } => successes
                .next()
                .map(Cow::Borrowed)
                .or_else(|| failures.next().map(Cow::Owned)),
        }
    }
}

impl<'a, 'db> FusedIterator for MroPossibilityIterator<'a, 'db> {}

/// Enumeration of the possible kinds of types we allow in class bases.
///
/// This is much more limited than the [`Type`] enum:
/// all types that would be invalid to have as a class base are
/// transformed into [`ClassBase::Unknown`]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum ClassBase<'db> {
    Class(ClassType<'db>),
    Any,
    Todo,
    Unknown,
}

impl<'db> ClassBase<'db> {
    /// Return a `ClassBase` representing the `builtins.object` class.
    fn builtins_object(db: &'db dyn Db) -> Self {
        Self::Class(KnownClass::Object.to_class(db).expect_class())
    }

    /// Access an attribute on this `ClassBase` ignoring the possibility of any
    /// superclasses.
    ///
    /// (For a `ClassBase::Class` variant, this is equivalent to looking up the
    /// attribute directly in the class's dictionary at runtime.)
    pub(super) fn own_class_member(self, db: &'db dyn Db, member: &str) -> Type<'db> {
        match self {
            Self::Any => Type::Any,
            Self::Todo => Type::Todo,
            Self::Unknown => Type::Unknown,
            Self::Class(class) => class.own_class_member(db, member),
        }
    }

    pub(super) fn display(self, db: &'db dyn Db) -> Cow<'static, str> {
        match self {
            Self::Any => Cow::Borrowed("Any"),
            Self::Todo => Cow::Borrowed("Todo"),
            Self::Unknown => Cow::Borrowed("Unknown"),
            Self::Class(class) => Cow::Owned(format!("<class '{}'>", class.name(db))),
        }
    }

    /// Return the various [`MroPossibilities`] for this base.
    fn mro_possibilities(self, db: &'db dyn Db) -> Cow<MroPossibilities<'db>> {
        match self {
            ClassBase::Class(class) => Cow::Borrowed(class.mro_possibilities(db)),
            ClassBase::Any | ClassBase::Todo | ClassBase::Unknown => {
                let object = ClassBase::builtins_object(db);
                Cow::Owned(MroPossibilities::single([self, object]))
            }
        }
    }
}

impl<'db> From<Type<'db>> for ClassBase<'db> {
    fn from(value: Type<'db>) -> Self {
        match value {
            Type::Any => ClassBase::Any,
            Type::Todo => ClassBase::Todo,
            Type::Unknown => ClassBase::Unknown,
            Type::Class(class) => ClassBase::Class(class),
            // TODO support `__mro_entries__`?? --Alex
            Type::Instance(_) => ClassBase::Todo,
            // TODO: ??
            Type::Intersection(_) => ClassBase::Todo,
            Type::Union(_) => {
                panic!(
                    "Should never call `ClassBase::from` on a `Type::Union` variant; \
                    unions have custom handling throughout"
                )
            }
            // These are all errors:
            Type::Unbound
            | Type::BooleanLiteral(_)
            | Type::BytesLiteral(_)
            | Type::Function(_)
            | Type::IntLiteral(_)
            | Type::LiteralString
            | Type::Module(_)
            | Type::Never
            | Type::None
            | Type::StringLiteral(_)
            | Type::Tuple(_) => ClassBase::Unknown,
        }
    }
}

impl<'db> From<&Type<'db>> for ClassBase<'db> {
    fn from(value: &Type<'db>) -> Self {
        Self::from(*value)
    }
}

impl<'db> From<ClassBase<'db>> for Type<'db> {
    fn from(value: ClassBase<'db>) -> Self {
        match value {
            ClassBase::Class(class) => Type::Class(class),
            ClassBase::Any => Type::Any,
            ClassBase::Todo => Type::Todo,
            ClassBase::Unknown => Type::Unknown,
        }
    }
}

impl<'db> From<&ClassBase<'db>> for Type<'db> {
    fn from(value: &ClassBase<'db>) -> Self {
        Self::from(*value)
    }
}

/// Represents a class's bases list.
///
/// At runtime, this list of classes can be found
/// in the `__bases__` attribute on a class.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub(super) struct BasesList<'db>(Box<[ClassBase<'db>]>);

impl<'db> Deref for BasesList<'db> {
    type Target = [ClassBase<'db>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'db> FromIterator<ClassBase<'db>> for BasesList<'db> {
    fn from_iter<T: IntoIterator<Item = ClassBase<'db>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A single possible method resolution order of a given class.
///
/// See [`ClassType::mro_possibilities`] for more details.
#[derive(PartialEq, Eq, Default, Hash, Clone, Debug)]
pub(super) struct Mro<'db>(Box<[ClassBase<'db>]>);

impl<'db> Mro<'db> {
    fn new(mro: Vec<ClassBase<'db>>) -> Self {
        Self(mro.into_boxed_slice())
    }

    /// In the event that a possible list of bases would (or could) lead to a
    /// `TypeError` being raised at runtime due to an unresolvable MRO, we
    /// infer one of the "possible MROs" of the class as being
    /// `[<the class in question>, Unknown, object]`. This seems most likely
    /// to reduce the possibility of cascading errors elsewhere.
    ///
    /// (We emit a diagnostic warning about the runtime `TypeError` in
    /// [`super::infer::TypeInferenceBuilder::infer_region_scope`].)
    fn from_error(db: &'db dyn Db, class: ClassBase<'db>) -> Self {
        Self::from([class, ClassBase::Unknown, ClassBase::builtins_object(db)])
    }

    pub(super) fn iter(&self) -> std::slice::Iter<'_, ClassBase<'db>> {
        self.0.iter()
    }
}

impl<'db, const N: usize> From<[ClassBase<'db>; N]> for Mro<'db> {
    fn from(value: [ClassBase<'db>; N]) -> Self {
        Self(Box::new(value))
    }
}

impl<'db, const N: usize> From<[ClassType<'db>; N]> for Mro<'db> {
    fn from(value: [ClassType<'db>; N]) -> Self {
        value.into_iter().map(ClassBase::Class).collect()
    }
}

impl<'db> FromIterator<ClassBase<'db>> for Mro<'db> {
    fn from_iter<T: IntoIterator<Item = ClassBase<'db>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a, 'db> IntoIterator for &'a Mro<'db> {
    type IntoIter = std::slice::Iter<'a, ClassBase<'db>>;
    type Item = &'a ClassBase<'db>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Implementation of the [C3-merge algorithm] for calculating a Python class's
/// [method resolution order].
///
/// [C3-merge algorithm]: https://docs.python.org/3/howto/mro.html#python-2-3-mro
/// [method resolution order]: https://docs.python.org/3/glossary.html#term-method-resolution-order
fn c3_merge(mut sequences: Vec<VecDeque<ClassBase>>) -> Option<Mro> {
    // Most MROs aren't that long...
    let mut mro = Vec::with_capacity(8);

    loop {
        sequences.retain(|sequence| !sequence.is_empty());

        if sequences.is_empty() {
            return Some(Mro::new(mro));
        }

        // Iterator over all potential candidates to be the next MRO entry:
        let mut mro_entry_candidate_iter = sequences.iter().map(|sequence| sequence[0]);

        // If the candidate exists "deeper down" in the inheritance hierarchy,
        // we should refrain from adding it to the MRO for now. Add the first candidate
        // for which this does not hold true. If this holds true for all candidates,
        // return `None`; it will be impossible to find a consistent MRO for the class
        // with the given bases.
        let mro_entry = mro_entry_candidate_iter.find(|candidate| {
            sequences
                .iter()
                .all(|sequence| sequence.iter().skip(1).all(|base| base != candidate))
        })?;

        mro.push(mro_entry);

        // Make sure we don't try to add the candidate to the MRO twice:
        for sequence in &mut sequences {
            if sequence[0] == mro_entry {
                sequence.pop_front();
            }
        }
    }
}
