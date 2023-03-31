use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{collections::hash_map::Iter as HashMapIter, ops::Deref};

use rustpython_parser::ast::{Expr, Located, Location, Stmt};

#[derive(Clone)]
pub enum Node<'a> {
    Stmt(&'a Stmt),
    Expr(&'a Expr),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Range {
    pub location: Location,
    pub end_location: Location,
}

impl Range {
    pub const fn new(location: Location, end_location: Location) -> Self {
        Self {
            location,
            end_location,
        }
    }
}

impl<T> From<&Located<T>> for Range {
    fn from(located: &Located<T>) -> Self {
        Range::new(located.location, located.end_location.unwrap())
    }
}

impl<T> From<&Box<Located<T>>> for Range {
    fn from(located: &Box<Located<T>>) -> Self {
        Range::new(located.location, located.end_location.unwrap())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RefEquality<'a, T>(pub &'a T);

impl<'a, T> std::hash::Hash for RefEquality<'a, T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        (self.0 as *const T).hash(state);
    }
}

impl<'a, 'b, T> PartialEq<RefEquality<'b, T>> for RefEquality<'a, T> {
    fn eq(&self, other: &RefEquality<'b, T>) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<'a, T> Eq for RefEquality<'a, T> {}

impl<'a, T> Deref for RefEquality<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a> From<&RefEquality<'a, Stmt>> for &'a Stmt {
    fn from(r: &RefEquality<'a, Stmt>) -> Self {
        r.0
    }
}

impl<'a> From<&RefEquality<'a, Expr>> for &'a Expr {
    fn from(r: &RefEquality<'a, Expr>) -> Self {
        r.0
    }
}

pub type CallPath<'a> = smallvec::SmallVec<[&'a str; 8]>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub name: String,
    location: Location,
    end_location: Location,
}

impl Import {
    pub fn new(name: &str, location: Location, end_location: Location) -> Self {
        Self {
            name: name.to_string(),
            location,
            end_location,
        }
    }
}

impl From<&Import> for Range {
    fn from(import: &Import) -> Range {
        Range::new(import.location, import.end_location)
    }
}

impl From<Import> for Range {
    fn from(import: Import) -> Range {
        Range::new(import.location, import.end_location)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imports {
    pub imports_per_module: FxHashMap<String, Vec<Import>>,
}

impl Imports {
    pub fn insert(&mut self, module: &str, imports_vec: Vec<Import>) {
        self.imports_per_module
            .insert(module.to_owned(), imports_vec);
    }

    pub fn extend(&mut self, other: Self) {
        self.imports_per_module.extend(other.imports_per_module);
    }
}

impl<'a> IntoIterator for &'a Imports {
    type Item = (&'a String, &'a Vec<Import>);
    type IntoIter = HashMapIter<'a, String, Vec<Import>>;

    fn into_iter(self) -> Self::IntoIter {
        self.imports_per_module.iter()
    }
}
