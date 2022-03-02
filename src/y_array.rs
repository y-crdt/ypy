use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::type_conversions::insert_at;
use crate::y_transaction::YTransaction;

use super::shared_types::SharedType;
use crate::type_conversions::ToPython;
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use yrs::types::array::{ArrayEvent, ArrayIter};
use yrs::{Array, Subscription, Transaction};

/// A collection used to store data in an indexed sequence structure. This type is internally
/// implemented as a double linked list, which may squash values inserted directly one after another
/// into single list node upon transaction commit.
///
/// Reading a root-level type as an YArray means treating its sequence components as a list, where
/// every countable element becomes an individual entity:
///
/// - JSON-like primitives (booleans, numbers, strings, JSON maps, arrays etc.) are counted
///   individually.
/// - Text chunks inserted by [Text] data structure: each character becomes an element of an
///   array.
/// - Embedded and binary values: they count as a single element even though they correspond of
///   multiple bytes.
///
/// Like all Yrs shared data types, YArray is resistant to the problem of interleaving (situation
/// when elements inserted one after another may interleave with other peers concurrent inserts
/// after merging all updates together). In case of Yrs conflict resolution is solved by using
/// unique document id to determine correct and consistent ordering.
#[pyclass(unsendable)]
pub struct YArray(pub SharedType<Array, Vec<PyObject>>);

impl From<Array> for YArray {
    fn from(v: Array) -> Self {
        YArray(SharedType::new(v))
    }
}

#[pymethods]
impl YArray {
    /// Creates a new preliminary instance of a `YArray` shared data type, with its state
    /// initialized to provided parameter.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[new]
    pub fn new(init: Option<Vec<PyObject>>) -> Self {
        YArray(SharedType::prelim(init.unwrap_or_default()))
    }

    /// Returns true if this is a preliminary instance of `YArray`.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[getter]
    pub fn prelim(&self) -> bool {
        match &self.0 {
            SharedType::Prelim(_) => true,
            _ => false,
        }
    }

    /// Returns a number of elements stored within this instance of `YArray`.
    #[getter]
    pub fn length(&self) -> u32 {
        match &self.0 {
            SharedType::Integrated(v) => v.len(),
            SharedType::Prelim(v) => v.len() as u32,
        }
    }

    /// Converts an underlying contents of this `YArray` instance into their JSON representation.
    pub fn to_json(&self, txn: &YTransaction) -> PyObject {
        Python::with_gil(|py| match &self.0 {
            SharedType::Integrated(v) => v.to_json(txn).into_py(py),
            SharedType::Prelim(v) => {
                let py_ptrs: Vec<PyObject> = v.iter().cloned().collect();
                py_ptrs.into_py(py)
            }
        })
    }

    /// Inserts a given range of `items` into this `YArray` instance, starting at given `index`.
    pub fn insert(&mut self, txn: &mut YTransaction, index: u32, items: Vec<PyObject>) {
        let mut j = index;
        match &mut self.0 {
            SharedType::Integrated(array) => {
                insert_at(array, txn, index, items);
            }
            SharedType::Prelim(vec) => {
                for el in items {
                    vec.insert(j as usize, el);
                    j += 1;
                }
            }
        }
    }

    /// Appends a range of `items` at the end of this `YArray` instance.
    pub fn push(&mut self, txn: &mut YTransaction, items: Vec<PyObject>) {
        let index = self.length();
        self.insert(txn, index, items);
    }

    /// Deletes a range of items of given `length` from current `YArray` instance,
    /// starting from given `index`.
    pub fn delete(&mut self, txn: &mut YTransaction, index: u32, length: u32) {
        match &mut self.0 {
            SharedType::Integrated(v) => v.remove_range(txn, index, length),
            SharedType::Prelim(v) => {
                v.drain((index as usize)..(index + length) as usize);
            }
        }
    }

    /// Returns an element stored under given `index`.
    pub fn get(&self, txn: &YTransaction, index: u32) -> PyResult<PyObject> {
        match &self.0 {
            SharedType::Integrated(v) => {
                if let Some(value) = v.get(txn, index) {
                    Ok(Python::with_gil(|py| value.into_py(py)))
                } else {
                    Err(PyIndexError::new_err(
                        "Index outside the bounds of an YArray",
                    ))
                }
            }
            SharedType::Prelim(v) => {
                if let Some(value) = v.get(index as usize) {
                    Ok(value.clone())
                } else {
                    Err(PyIndexError::new_err(
                        "Index outside the bounds of an YArray",
                    ))
                }
            }
        }
    }

    /// Returns an iterator that can be used to traverse over the values stored withing this
    /// instance of `YArray`.
    ///
    /// Example:
    ///
    /// ```python
    /// from y_py import YDoc
    ///
    /// # document on machine A
    /// doc = YDoc()
    /// array = doc.get_array('name')
    ///
    /// with doc.begin_transaction() as txn:
    ///     array.push(txn, ['hello', 'world'])
    ///     for item in array.values(txn)):
    ///         print(item)
    /// ```
    pub fn values(&self, txn: &YTransaction) -> YArrayIterator {
        let inner_iter = match &self.0 {
            SharedType::Integrated(v) => unsafe {
                let this: *const Array = v;
                let tx: *const Transaction = txn.deref() as *const _;
                InnerYArrayIter::Integrated((*this).iter(tx.as_ref().unwrap()))
            },
            SharedType::Prelim(v) => unsafe {
                let this: *const Vec<PyObject> = v;
                InnerYArrayIter::Prelim((*this).iter())
            },
        };
        YArrayIterator(ManuallyDrop::new(inner_iter))
    }

    /// Subscribes to all operations happening over this instance of `YArray`. All changes are
    /// batched and eventually triggered during transaction commit phase.
    /// Returns an `YObserver` which, when free'd, will unsubscribe current callback.
    pub fn observe(&mut self, f: PyObject) -> YArrayObserver {
        match &mut self.0 {
            SharedType::Integrated(v) => v
                .observe(move |txn, e| {
                    Python::with_gil(|py| {
                        let event = YArrayEvent::new(e, txn);
                        f.call1(py, (event,)).unwrap();
                    })
                })
                .into(),
            SharedType::Prelim(_) => {
                panic!("YArray.observe is not supported on preliminary type.")
            }
        }
    }
}

enum InnerYArrayIter {
    Integrated(ArrayIter<'static>),
    Prelim(std::slice::Iter<'static, PyObject>),
}

#[pyclass(unsendable)]
pub struct YArrayIterator(ManuallyDrop<InnerYArrayIter>);

impl Drop for YArrayIterator {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.0) }
    }
}

#[pymethods]
impl YArrayIterator {
    pub fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<PyObject> {
        match slf.0.deref_mut() {
            InnerYArrayIter::Integrated(iter) => {
                Python::with_gil(|py| iter.next().map(|v| v.into_py(py)))
            }
            InnerYArrayIter::Prelim(iter) => iter.next().cloned(),
        }
    }
}

/// Event generated by `YArray.observe` method. Emitted during transaction commit phase.
#[pyclass(unsendable)]
pub struct YArrayEvent {
    inner: *const ArrayEvent,
    txn: *const Transaction,
    target: Option<PyObject>,
    delta: Option<PyObject>,
}

impl YArrayEvent {
    fn new(event: &ArrayEvent, txn: &Transaction) -> Self {
        let inner = event as *const ArrayEvent;
        let txn = txn as *const Transaction;
        YArrayEvent {
            inner,
            txn,
            target: None,
            delta: None,
        }
    }

    fn inner(&self) -> &ArrayEvent {
        unsafe { self.inner.as_ref().unwrap() }
    }

    fn txn(&self) -> &Transaction {
        unsafe { self.txn.as_ref().unwrap() }
    }
}

#[pymethods]
impl YArrayEvent {
    /// Returns a current shared type instance, that current event changes refer to.
    #[getter]
    pub fn target(&mut self) -> PyObject {
        if let Some(target) = self.target.as_ref() {
            target.clone()
        } else {
            let target: PyObject =
                Python::with_gil(|py| YArray::from(self.inner().target().clone()).into_py(py));
            self.target = Some(target.clone());
            target
        }
    }

    /// Returns an array of keys and indexes creating a path from root type down to current instance
    /// of shared type (accessible via `target` getter).
    pub fn path(&self) -> PyObject {
        Python::with_gil(|py| self.inner().path(self.txn()).into_py(py))
    }

    /// Returns a list of text changes made over corresponding `YArray` collection within
    /// bounds of current transaction. These changes follow a format:
    ///
    /// - { insert: any[] }
    /// - { delete: number }
    /// - { retain: number }
    #[getter]
    pub fn delta(&mut self) -> PyObject {
        if let Some(delta) = &self.delta {
            delta.clone()
        } else {
            let delta: PyObject = Python::with_gil(|py| {
                let delta = self
                    .inner()
                    .delta(self.txn())
                    .into_iter()
                    .map(|change| Python::with_gil(|py| change.into_py(py)));
                PyList::new(py, delta).into()
            });
            self.delta = Some(delta.clone());
            delta
        }
    }
}

#[pyclass(unsendable)]
pub struct YArrayObserver(Subscription<ArrayEvent>);

impl From<Subscription<ArrayEvent>> for YArrayObserver {
    fn from(o: Subscription<ArrayEvent>) -> Self {
        YArrayObserver(o)
    }
}
