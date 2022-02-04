use pyo3::prelude::*;
use yrs::Text;

use crate::shared_types::SharedType;
use crate::y_transaction::YTransaction;

/// A shared data type used for collaborative text editing. It enables multiple users to add and
/// remove chunks of text in efficient manner. This type is internally represented as a mutable
/// double-linked list of text chunks - an optimization occurs during `YTransaction.commit`, which
/// allows to squash multiple consecutively inserted characters together as a single chunk of text
/// even between transaction boundaries in order to preserve more efficient memory model.
///
/// `YText` structure internally uses UTF-8 encoding and its length is described in a number of
/// bytes rather than individual characters (a single UTF-8 code point can consist of many bytes).
///
/// Like all Yrs shared data types, `YText` is resistant to the problem of interleaving (situation
/// when characters inserted one after another may interleave with other peers concurrent inserts
/// after merging all updates together). In case of Yrs conflict resolution is solved by using
/// unique document id to determine correct and consistent ordering.
#[pyclass(unsendable)]
#[derive(Clone)]
pub struct YText(pub SharedType<Text, String>);
impl From<Text> for YText {
    fn from(v: Text) -> Self {
        YText(SharedType::new(v))
    }
}

#[pymethods]
impl YText {
    /// Creates a new preliminary instance of a `YText` shared data type, with its state initialized
    /// to provided parameter.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into y-py
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[new]
    pub fn new(init: Option<String>) -> Self {
        YText(SharedType::prelim(init.unwrap_or_default()))
    }

    /// Returns true if this is a preliminary instance of `YText`.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into y-py
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[getter]
    pub fn prelim(&self) -> bool {
        match self.0 {
            SharedType::Prelim(_) => true,
            _ => false,
        }
    }

    /// Returns length of an underlying string stored in this `YText` instance,
    /// understood as a number of UTF-8 encoded bytes.
    #[getter]
    pub fn length(&self) -> u32 {
        match &self.0 {
            SharedType::Integrated(v) => v.len(),
            SharedType::Prelim(v) => v.len() as u32,
        }
    }

    /// Returns an underlying shared string stored in this data type.
    pub fn to_string(&self, txn: &YTransaction) -> String {
        match &self.0 {
            SharedType::Integrated(v) => v.to_string(txn),
            SharedType::Prelim(v) => v.clone(),
        }
    }

    /// Returns an underlying shared string stored in this data type.
    pub fn to_json(&self, txn: &YTransaction) -> String {
        match &self.0 {
            SharedType::Integrated(v) => v.to_string(txn),
            SharedType::Prelim(v) => v.clone(),
        }
    }

    /// Inserts a given `chunk` of text into this `YText` instance, starting at a given `index`.
    pub fn insert(&mut self, txn: &mut YTransaction, index: u32, chunk: &str) {
        match &mut self.0 {
            SharedType::Integrated(v) => v.insert(txn, index, chunk),
            SharedType::Prelim(v) => v.insert_str(index as usize, chunk),
        }
    }

    /// Appends a given `chunk` of text at the end of current `YText` instance.
    pub fn push(&mut self, txn: &mut YTransaction, chunk: &str) {
        match &mut self.0 {
            SharedType::Integrated(v) => v.push(txn, chunk),
            SharedType::Prelim(v) => v.push_str(chunk),
        }
    }

    /// Deletes a specified range of of characters, starting at a given `index`.
    /// Both `index` and `length` are counted in terms of a number of UTF-8 character bytes.
    pub fn delete(&mut self, txn: &mut YTransaction, index: u32, length: u32) {
        match &mut self.0 {
            SharedType::Integrated(v) => v.remove_range(txn, index, length),
            SharedType::Prelim(v) => {
                v.drain((index as usize)..(index + length) as usize);
            }
        }
    }
}
