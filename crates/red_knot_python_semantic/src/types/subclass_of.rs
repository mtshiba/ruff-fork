use super::{ClassBase, ClassLiteralType, Db, KnownClass, Symbol, Type};

/// A type that represents `type[C]`, i.e. the class literal `C` and class literals that are subclasses of `C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub struct SubclassOfType<'db> {
    // Keep this field private, so that the only way of constructing the struct is through the `from` method.
    subclass_of: ClassBase<'db>,
}

impl<'db> SubclassOfType<'db> {
    pub fn from(db: &'db dyn Db, subclass_of: impl Into<ClassBase<'db>>) -> Type<'db> {
        let subclass_of = subclass_of.into();
        match subclass_of {
            ClassBase::Any | ClassBase::Unknown | ClassBase::Todo(_) => {
                Type::SubclassOf(Self { subclass_of })
            }
            ClassBase::Class(class) => {
                if class.is_final(db) {
                    Type::ClassLiteral(ClassLiteralType { class })
                } else if class.is_known(db, KnownClass::Object) {
                    KnownClass::Type.to_instance(db)
                } else {
                    Type::SubclassOf(Self { subclass_of })
                }
            }
        }
    }

    pub const fn subclass_of(self) -> ClassBase<'db> {
        self.subclass_of
    }

    pub const fn is_dynamic(self) -> bool {
        // Unpack `self` so that we're forced to update this method if any more fields are added in the future.
        let Self { subclass_of } = self;
        subclass_of.is_dynamic()
    }

    pub const fn is_fully_static(self) -> bool {
        !self.is_dynamic()
    }

    pub(crate) fn member(self, db: &'db dyn Db, name: &str) -> Symbol<'db> {
        Type::from(self.subclass_of).member(db, name)
    }

    pub fn is_subtype_of(self, db: &'db dyn Db, other: SubclassOfType<'db>) -> bool {
        match (self.subclass_of, other.subclass_of) {
            // Non-fully-static types do not participate in subtyping
            (ClassBase::Any | ClassBase::Unknown | ClassBase::Todo(_), _)
            | (_, ClassBase::Any | ClassBase::Unknown | ClassBase::Todo(_)) => false,

            // For example, `type[bool]` describes all possible runtime subclasses of the class `bool`,
            // and `type[int]` describes all possible runtime subclasses of the class `int`.
            // The first set is a subset of the second set, because `bool` is itself a subclass of `int`.
            (ClassBase::Class(self_class), ClassBase::Class(other_class)) => {
                // N.B. The subclass relation is fully static
                self_class.is_subclass_of(db, other_class)
            }
        }
    }
}
