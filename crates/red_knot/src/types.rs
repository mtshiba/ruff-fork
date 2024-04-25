#![allow(dead_code)]
use crate::ast_ids::NodeKey;
use crate::module::Module;
use crate::symbols::SymbolId;
use crate::{FxDashMap, FxIndexSet, Name};
use ruff_index::{newtype_index, IndexVec};
use rustc_hash::FxHashMap;

/// unique ID for a type
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    /// the dynamic or gradual type: a statically-unknown set of values
    Any,
    /// the empty set of values
    Never,
    /// unknown type (no annotation)
    /// equivalent to Any, or to object in strict mode
    Unknown,
    /// name is not bound to any value
    Unbound,
    /// a specific function
    Function(FunctionTypeId),
    /// the set of Python objects with a given class in their __class__'s method resolution order
    Class(ClassTypeId),
    Union(UnionTypeId),
    Intersection(IntersectionTypeId),
    // TODO protocols, callable types, overloads, generics, type vars
}

impl Type {
    fn display<'a>(&'a self, store: &'a TypeStore) -> DisplayType<'a> {
        DisplayType { ty: self, store }
    }
}

#[derive(Debug, Default)]
pub(crate) struct TypeStore {
    modules: FxDashMap<Module, ModuleTypeStore>,
}

impl TypeStore {
    fn add_or_get_module(&mut self, module: Module) -> ModuleStoreRefMut {
        self.modules
            .entry(module)
            .or_insert_with(|| ModuleTypeStore::new(module))
    }

    fn get_module(&self, module: Module) -> ModuleStoreRef {
        self.modules.get(&module).expect("module should exist")
    }

    fn add_function(&mut self, module: Module, name: &str) -> Type {
        self.add_or_get_module(module).add_function(name)
    }

    fn add_class(&mut self, module: Module, name: &str) -> Type {
        self.add_or_get_module(module).add_class(name)
    }

    fn add_union(&mut self, module: Module, elems: &[Type]) -> Type {
        self.add_or_get_module(module).add_union(elems)
    }

    fn add_intersection(&mut self, module: Module, positive: &[Type], negative: &[Type]) -> Type {
        self.add_or_get_module(module)
            .add_intersection(positive, negative)
    }

    fn get_function(&self, id: FunctionTypeId) -> FunctionTypeRef {
        FunctionTypeRef {
            module_store: self.get_module(id.module),
            function_id: id.func_id,
        }
    }

    fn get_class(&self, id: ClassTypeId) -> ClassTypeRef {
        ClassTypeRef {
            module_store: self.get_module(id.module),
            class_id: id.class_id,
        }
    }

    fn get_union(&self, id: UnionTypeId) -> UnionTypeRef {
        UnionTypeRef {
            module_store: self.get_module(id.module),
            union_id: id.union_id,
        }
    }

    fn get_intersection(&self, id: IntersectionTypeId) -> IntersectionTypeRef {
        IntersectionTypeRef {
            module_store: self.get_module(id.module),
            intersection_id: id.intersection_id,
        }
    }
}

type ModuleStoreRef<'a> = dashmap::mapref::one::Ref<
    'a,
    Module,
    ModuleTypeStore,
    std::hash::BuildHasherDefault<rustc_hash::FxHasher>,
>;

type ModuleStoreRefMut<'a> = dashmap::mapref::one::RefMut<
    'a,
    Module,
    ModuleTypeStore,
    std::hash::BuildHasherDefault<rustc_hash::FxHasher>,
>;

#[derive(Debug)]
pub(crate) struct FunctionTypeRef<'a> {
    module_store: ModuleStoreRef<'a>,
    function_id: ModuleFunctionTypeId,
}

impl<'a> std::ops::Deref for FunctionTypeRef<'a> {
    type Target = FunctionType;

    fn deref(&self) -> &Self::Target {
        self.module_store.get_function(self.function_id)
    }
}

#[derive(Debug)]
pub(crate) struct ClassTypeRef<'a> {
    module_store: ModuleStoreRef<'a>,
    class_id: ModuleClassTypeId,
}

impl<'a> std::ops::Deref for ClassTypeRef<'a> {
    type Target = ClassType;

    fn deref(&self) -> &Self::Target {
        self.module_store.get_class(self.class_id)
    }
}

#[derive(Debug)]
pub(crate) struct UnionTypeRef<'a> {
    module_store: ModuleStoreRef<'a>,
    union_id: ModuleUnionTypeId,
}

impl<'a> std::ops::Deref for UnionTypeRef<'a> {
    type Target = UnionType;

    fn deref(&self) -> &Self::Target {
        self.module_store.get_union(self.union_id)
    }
}

#[derive(Debug)]
pub(crate) struct IntersectionTypeRef<'a> {
    module_store: ModuleStoreRef<'a>,
    intersection_id: ModuleIntersectionTypeId,
}

impl<'a> std::ops::Deref for IntersectionTypeRef<'a> {
    type Target = IntersectionType;

    fn deref(&self) -> &Self::Target {
        self.module_store.get_intersection(self.intersection_id)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct FunctionTypeId {
    module: Module,
    func_id: ModuleFunctionTypeId,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct ClassTypeId {
    module: Module,
    class_id: ModuleClassTypeId,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct UnionTypeId {
    module: Module,
    union_id: ModuleUnionTypeId,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct IntersectionTypeId {
    module: Module,
    intersection_id: ModuleIntersectionTypeId,
}

#[newtype_index]
struct ModuleFunctionTypeId;

#[newtype_index]
struct ModuleClassTypeId;

#[newtype_index]
struct ModuleUnionTypeId;

#[newtype_index]
struct ModuleIntersectionTypeId;

#[derive(Debug)]
struct ModuleTypeStore {
    module: Module,
    /// arena of all function types defined in this module
    functions: IndexVec<ModuleFunctionTypeId, FunctionType>,
    /// arena of all class types defined in this module
    classes: IndexVec<ModuleClassTypeId, ClassType>,
    /// arenda of all union types created in this module
    unions: IndexVec<ModuleUnionTypeId, UnionType>,
    /// arena of all intersection types created in this module
    intersections: IndexVec<ModuleIntersectionTypeId, IntersectionType>,
    /// cached types of symbols in this module
    symbol_types: FxHashMap<SymbolId, Type>,
    /// cached types of AST nodes in this module
    node_types: FxHashMap<NodeKey, Type>,
}

impl ModuleTypeStore {
    fn new(module: Module) -> Self {
        Self {
            module,
            functions: IndexVec::default(),
            classes: IndexVec::default(),
            unions: IndexVec::default(),
            intersections: IndexVec::default(),
            symbol_types: FxHashMap::default(),
            node_types: FxHashMap::default(),
        }
    }

    fn add_function(&mut self, name: &str) -> Type {
        let func_id = self.functions.push(FunctionType {
            name: Name::new(name),
        });
        Type::Function(FunctionTypeId {
            module: self.module,
            func_id,
        })
    }

    fn add_class(&mut self, name: &str) -> Type {
        let class_id = self.classes.push(ClassType {
            name: Name::new(name),
        });
        Type::Class(ClassTypeId {
            module: self.module,
            class_id,
        })
    }

    fn add_union(&mut self, elems: &[Type]) -> Type {
        let union_id = self.unions.push(UnionType {
            elements: FxIndexSet::from_iter(elems.iter().copied()),
        });
        Type::Union(UnionTypeId {
            module: self.module,
            union_id,
        })
    }

    fn add_intersection(&mut self, positive: &[Type], negative: &[Type]) -> Type {
        let intersection_id = self.intersections.push(IntersectionType {
            positive: FxIndexSet::from_iter(positive.iter().copied()),
            negative: FxIndexSet::from_iter(negative.iter().copied()),
        });
        Type::Intersection(IntersectionTypeId {
            module: self.module,
            intersection_id,
        })
    }

    fn get_function(&self, func_id: ModuleFunctionTypeId) -> &FunctionType {
        &self.functions[func_id]
    }

    fn get_class(&self, class_id: ModuleClassTypeId) -> &ClassType {
        &self.classes[class_id]
    }

    fn get_union(&self, union_id: ModuleUnionTypeId) -> &UnionType {
        &self.unions[union_id]
    }

    fn get_intersection(&self, intersection_id: ModuleIntersectionTypeId) -> &IntersectionType {
        &self.intersections[intersection_id]
    }
}

#[derive(Copy, Clone, Debug)]
struct DisplayType<'a> {
    ty: &'a Type,
    store: &'a TypeStore,
}

impl std::fmt::Display for DisplayType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ty {
            Type::Any => f.write_str("Any"),
            Type::Never => f.write_str("Never"),
            Type::Unknown => f.write_str("Unknown"),
            Type::Unbound => f.write_str("Unbound"),
            Type::Class(class_id) => f.write_str(self.store.get_class(*class_id).name()),
            Type::Function(func_id) => f.write_str(self.store.get_function(*func_id).name()),
            Type::Union(union_id) => self
                .store
                .get_module(union_id.module)
                .get_union(union_id.union_id)
                .display(f, self.store),
            Type::Intersection(int_id) => self
                .store
                .get_module(int_id.module)
                .get_intersection(int_id.intersection_id)
                .display(f, self.store),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ClassType {
    name: Name,
}

impl ClassType {
    fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Debug)]
pub(crate) struct FunctionType {
    name: Name,
}

impl FunctionType {
    fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Debug)]
pub(crate) struct UnionType {
    // the union type includes values in any of these types
    elements: FxIndexSet<Type>,
}

impl UnionType {
    fn display(&self, f: &mut std::fmt::Formatter<'_>, store: &TypeStore) -> std::fmt::Result {
        f.write_str("(")?;
        let mut first = true;
        for ty in self.elements.iter() {
            if !first {
                f.write_str(" | ")?;
            };
            first = false;
            write!(f, "{}", ty.display(store))?;
        }
        f.write_str(")")
    }
}

// Negation types aren't expressible in annotations, and are most likely to arise from type
// narrowing along with intersections (e.g. `if not isinstance(...)`), so we represent them
// directly in intersections rather than as a separate type. This sacrifices some efficiency in the
// case where a Not appears outside an intersection (unclear when that could even happen, but we'd
// have to represent it as a single-element intersection if it did) in exchange for better
// efficiency in the not-within-intersection case.
#[derive(Debug)]
pub(crate) struct IntersectionType {
    // the intersection type includes only values in all of these types
    positive: FxIndexSet<Type>,
    // negated elements of the intersection, e.g.
    negative: FxIndexSet<Type>,
}

impl IntersectionType {
    fn display(&self, f: &mut std::fmt::Formatter<'_>, store: &TypeStore) -> std::fmt::Result {
        f.write_str("(")?;
        let mut first = true;
        for (neg, ty) in self
            .positive
            .iter()
            .map(|ty| (false, ty))
            .chain(self.negative.iter().map(|ty| (true, ty)))
        {
            if !first {
                f.write_str(" & ")?;
            };
            first = false;
            if neg {
                f.write_str("~")?;
            };
            write!(f, "{}", ty.display(store))?;
        }
        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use crate::module::test_module;
    use crate::types::{Type, TypeStore};
    use crate::FxIndexSet;

    #[test]
    fn add_class() {
        let mut store = TypeStore::default();
        let module = test_module(0);
        let class = store.add_class(module, "C");
        if let Type::Class(id) = class {
            assert_eq!(store.get_class(id).name(), "C");
        } else {
            panic!("not a class");
        }
        assert_eq!(format!("{}", class.display(&store)), "C");
    }

    #[test]
    fn add_function() {
        let mut store = TypeStore::default();
        let module = test_module(0);
        let func = store.add_function(module, "func");
        if let Type::Function(id) = func {
            assert_eq!(store.get_function(id).name(), "func");
        } else {
            panic!("not a function");
        }
        assert_eq!(format!("{}", func.display(&store)), "func");
    }

    #[test]
    fn add_union() {
        let mut store = TypeStore::default();
        let module = test_module(0);
        let c1 = store.add_class(module, "C1");
        let c2 = store.add_class(module, "C2");
        let elems = vec![c1, c2];
        let union = store.add_union(module, &elems);
        if let Type::Union(id) = union {
            assert_eq!(
                store.get_union(id).elements,
                FxIndexSet::from_iter(elems.iter().copied())
            );
        } else {
            panic!("not a union");
        }
        assert_eq!(format!("{}", union.display(&store)), "(C1 | C2)");
    }

    #[test]
    fn add_intersection() {
        let mut store = TypeStore::default();
        let module = test_module(0);
        let c1 = store.add_class(module, "C1");
        let c2 = store.add_class(module, "C2");
        let c3 = store.add_class(module, "C3");
        let pos = vec![c1, c2];
        let neg = vec![c3];
        let intersection = store.add_intersection(module, &pos, &neg);
        if let Type::Intersection(id) = intersection {
            assert_eq!(
                store.get_intersection(id).positive,
                FxIndexSet::from_iter(pos.iter().copied())
            );
            assert_eq!(
                store.get_intersection(id).negative,
                FxIndexSet::from_iter(neg.iter().copied())
            );
        } else {
            panic!("not an intersection");
        }
        assert_eq!(
            format!("{}", intersection.display(&store)),
            "(C1 & C2 & ~C3)"
        );
    }
}
