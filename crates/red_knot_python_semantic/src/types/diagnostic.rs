use crate::types::{ClassLiteralType, Type};
use crate::Db;
use ruff_db::diagnostic::{Diagnostic, DiagnosticId, Severity};
use ruff_db::files::File;
use ruff_python_ast::{self as ast, AnyNodeRef};
use ruff_text_size::{Ranged, TextRange};
use std::borrow::Cow;
use std::fmt::Formatter;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TypeCheckDiagnostic {
    pub(super) id: DiagnosticId,
    pub(super) message: String,
    pub(super) range: TextRange,
    pub(super) file: File,
}

impl TypeCheckDiagnostic {
    pub fn id(&self) -> DiagnosticId {
        self.id
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn file(&self) -> File {
        self.file
    }
}

impl Diagnostic for TypeCheckDiagnostic {
    fn id(&self) -> DiagnosticId {
        self.id
    }

    fn message(&self) -> Cow<str> {
        TypeCheckDiagnostic::message(self).into()
    }

    fn file(&self) -> File {
        TypeCheckDiagnostic::file(self)
    }

    fn range(&self) -> Option<TextRange> {
        Some(Ranged::range(self))
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }
}

impl Ranged for TypeCheckDiagnostic {
    fn range(&self) -> TextRange {
        self.range
    }
}

/// A collection of type check diagnostics.
///
/// The diagnostics are wrapped in an `Arc` because they need to be cloned multiple times
/// when going from `infer_expression` to `check_file`. We could consider
/// making [`TypeCheckDiagnostic`] a Salsa struct to have them Arena-allocated (once the Tables refactor is done).
/// Using Salsa struct does have the downside that it leaks the Salsa dependency into diagnostics and
/// each Salsa-struct comes with an overhead.
#[derive(Default, Eq, PartialEq)]
pub struct TypeCheckDiagnostics {
    inner: Vec<std::sync::Arc<TypeCheckDiagnostic>>,
}

impl TypeCheckDiagnostics {
    pub(super) fn push(&mut self, diagnostic: TypeCheckDiagnostic) {
        self.inner.push(Arc::new(diagnostic));
    }

    pub(crate) fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }
}

impl Extend<TypeCheckDiagnostic> for TypeCheckDiagnostics {
    fn extend<T: IntoIterator<Item = TypeCheckDiagnostic>>(&mut self, iter: T) {
        self.inner.extend(iter.into_iter().map(std::sync::Arc::new));
    }
}

impl Extend<std::sync::Arc<TypeCheckDiagnostic>> for TypeCheckDiagnostics {
    fn extend<T: IntoIterator<Item = Arc<TypeCheckDiagnostic>>>(&mut self, iter: T) {
        self.inner.extend(iter);
    }
}

impl<'a> Extend<&'a std::sync::Arc<TypeCheckDiagnostic>> for TypeCheckDiagnostics {
    fn extend<T: IntoIterator<Item = &'a Arc<TypeCheckDiagnostic>>>(&mut self, iter: T) {
        self.inner
            .extend(iter.into_iter().map(std::sync::Arc::clone));
    }
}

impl std::fmt::Debug for TypeCheckDiagnostics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Deref for TypeCheckDiagnostics {
    type Target = [std::sync::Arc<TypeCheckDiagnostic>];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl IntoIterator for TypeCheckDiagnostics {
    type Item = Arc<TypeCheckDiagnostic>;
    type IntoIter = std::vec::IntoIter<std::sync::Arc<TypeCheckDiagnostic>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a TypeCheckDiagnostics {
    type Item = &'a Arc<TypeCheckDiagnostic>;
    type IntoIter = std::slice::Iter<'a, std::sync::Arc<TypeCheckDiagnostic>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

pub(super) struct TypeCheckDiagnosticsBuilder<'db> {
    db: &'db dyn Db,
    file: File,
    diagnostics: TypeCheckDiagnostics,
}

impl<'db> TypeCheckDiagnosticsBuilder<'db> {
    pub(super) fn new(db: &'db dyn Db, file: File) -> Self {
        Self {
            db,
            file,
            diagnostics: TypeCheckDiagnostics::default(),
        }
    }

    /// Emit a diagnostic declaring that the object represented by `node` is not iterable
    pub(super) fn add_not_iterable(&mut self, node: AnyNodeRef, not_iterable_ty: Type<'db>) {
        self.add(
            node,
            DiagnosticId::lint("not-iterable"),
            format_args!(
                "Object of type `{}` is not iterable",
                not_iterable_ty.display(self.db)
            ),
        );
    }

    /// Emit a diagnostic declaring that the object represented by `node` is not iterable
    /// because its `__iter__` method is possibly unbound.
    pub(super) fn add_not_iterable_possibly_unbound(
        &mut self,
        node: AnyNodeRef,
        element_ty: Type<'db>,
    ) {
        self.add(
            node,
            DiagnosticId::lint("not-iterable"),
            format_args!(
                "Object of type `{}` is not iterable because its `__iter__` method is possibly unbound",
                element_ty.display(self.db)
            ),
        );
    }

    /// Emit a diagnostic declaring that an index is out of bounds for a tuple.
    pub(super) fn add_index_out_of_bounds(
        &mut self,
        kind: &'static str,
        node: AnyNodeRef,
        tuple_ty: Type<'db>,
        length: usize,
        index: i64,
    ) {
        self.add(
            node,
            DiagnosticId::lint("index-out-of-bounds"),
            format_args!(
                "Index {index} is out of bounds for {kind} `{}` with length {length}",
                tuple_ty.display(self.db)
            ),
        );
    }

    /// Emit a diagnostic declaring that a type does not support subscripting.
    pub(super) fn add_non_subscriptable(
        &mut self,
        node: AnyNodeRef,
        non_subscriptable_ty: Type<'db>,
        method: &str,
    ) {
        self.add(
            node,
            DiagnosticId::lint("non-subscriptable"),
            format_args!(
                "Cannot subscript object of type `{}` with no `{method}` method",
                non_subscriptable_ty.display(self.db)
            ),
        );
    }

    pub(super) fn add_unresolved_module(
        &mut self,
        import_node: impl Into<AnyNodeRef<'db>>,
        level: u32,
        module: Option<&str>,
    ) {
        self.add(
            import_node.into(),
            DiagnosticId::lint("unresolved-import"),
            format_args!(
                "Cannot resolve import `{}{}`",
                ".".repeat(level as usize),
                module.unwrap_or_default()
            ),
        );
    }

    pub(super) fn add_slice_step_size_zero(&mut self, node: AnyNodeRef) {
        self.add(
            node,
            DiagnosticId::lint("zero-stepsize-in-slice"),
            format_args!("Slice step size can not be zero"),
        );
    }

    pub(super) fn add_invalid_assignment(
        &mut self,
        node: AnyNodeRef,
        declared_ty: Type<'db>,
        assigned_ty: Type<'db>,
    ) {
        match declared_ty {
            Type::ClassLiteral(ClassLiteralType { class }) => {
                self.add(node, DiagnosticId::lint("invalid-assignment"), format_args!(
                        "Implicit shadowing of class `{}`; annotate to make it explicit if this is intentional",
                        class.name(self.db)));
            }
            Type::FunctionLiteral(function) => {
                self.add(node, DiagnosticId::lint("invalid-assignment"), format_args!(
                        "Implicit shadowing of function `{}`; annotate to make it explicit if this is intentional",
                        function.name(self.db)));
            }
            _ => {
                self.add(
                    node,
                    DiagnosticId::lint("invalid-assignment"),
                    format_args!(
                        "Object of type `{}` is not assignable to `{}`",
                        assigned_ty.display(self.db),
                        declared_ty.display(self.db),
                    ),
                );
            }
        }
    }

    pub(super) fn add_possibly_unresolved_reference(&mut self, expr_name_node: &ast::ExprName) {
        let ast::ExprName { id, .. } = expr_name_node;

        self.add(
            expr_name_node.into(),
            DiagnosticId::lint("possibly-unresolved-reference"),
            format_args!("Name `{id}` used when possibly not defined"),
        );
    }

    pub(super) fn add_unresolved_reference(&mut self, expr_name_node: &ast::ExprName) {
        let ast::ExprName { id, .. } = expr_name_node;

        self.add(
            expr_name_node.into(),
            DiagnosticId::lint("unresolved-reference"),
            format_args!("Name `{id}` used when not defined"),
        );
    }

    pub(super) fn add_invalid_exception(&mut self, db: &dyn Db, node: &ast::Expr, ty: Type) {
        self.add(
            node.into(),
            DiagnosticId::lint("invalid-exception"),
            format_args!(
                "Cannot catch object of type `{}` in an exception handler \
                (must be a `BaseException` subclass or a tuple of `BaseException` subclasses)",
                ty.display(db)
            ),
        );
    }

    /// Adds a new diagnostic.
    ///
    /// The diagnostic does not get added if the rule isn't enabled for this file.
    pub(super) fn add(&mut self, node: AnyNodeRef, id: DiagnosticId, message: std::fmt::Arguments) {
        if !self.db.is_file_open(self.file) {
            return;
        }

        // TODO: Don't emit the diagnostic if:
        // * The enclosing node contains any syntax errors
        // * The rule is disabled for this file. We probably want to introduce a new query that
        //   returns a rule selector for a given file that respects the package's settings,
        //   any global pragma comments in the file, and any per-file-ignores.

        self.diagnostics.push(TypeCheckDiagnostic {
            file: self.file,
            id,
            message: message.to_string(),
            range: node.range(),
        });
    }

    pub(super) fn extend(&mut self, diagnostics: &TypeCheckDiagnostics) {
        self.diagnostics.extend(diagnostics);
    }

    pub(super) fn finish(mut self) -> TypeCheckDiagnostics {
        self.diagnostics.shrink_to_fit();
        self.diagnostics
    }
}
