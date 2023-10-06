use crate::shared_types::{
    CompatiblePyType, DeepSubscription, DefaultPyErr, IntegratedOperationException,
    PreliminaryObservationException, ShallowSubscription, SharedType, SubId, TypeWithDoc,
};
use crate::type_conversions::{events_into_py, ToPython, WithDocToPython};
use crate::y_doc::{WithDoc, YDocInner};
use crate::y_transaction::{YTransaction, YTransactionInner};
use lib0::any::Any;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::rc::Rc;
use std::sync::Arc;
use yrs::types::text::TextEvent;
use yrs::types::Attrs;
use yrs::types::DeepObservable;
use yrs::{GetString, Observable, Text, TextRef, TransactionMut};

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
pub struct YText(pub SharedType<TypeWithDoc<TextRef>, String>);

impl WithDoc<YText> for TextRef {
    fn with_doc(self, doc: Rc<RefCell<YDocInner>>) -> YText {
        YText(SharedType::new(TypeWithDoc::new(self, doc)))
    }
}

#[pymethods]
impl YText {
    /// Creates a new preliminary instance of a `YText` shared data type, with its state initialized
    /// to provided parameter.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[new]
    pub fn new(init: Option<String>) -> Self {
        YText(SharedType::prelim(init.unwrap_or_default()))
    }

    /// Returns true if this is a preliminary instance of `YText`.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[getter]
    pub fn prelim(&self) -> bool {
        matches!(self.0, SharedType::Prelim(_))
    }

    /// Returns an underlying shared string stored in this data type.
    pub fn __str__(&self) -> String {
        match &self.0 {
            SharedType::Integrated(v) => v.with_transaction(|txn| v.get_string(txn)),
            SharedType::Prelim(v) => v.clone(),
        }
    }

    pub fn __repr__(&self) -> String {
        format!("YText({})", self.__str__())
    }

    /// Returns length of an underlying string stored in this `YText` instance,
    /// understood as a number of UTF-8 encoded bytes.
    pub fn __len__(&self) -> usize {
        match &self.0 {
            SharedType::Integrated(v) => v.with_transaction(|txn| v.len(txn)) as usize,
            SharedType::Prelim(v) => v.len(),
        }
    }

    /// Returns an underlying shared string stored in this data type.
    pub fn to_json(&self) -> String {
        format!("\"{}\"", self.__str__())
    }

    pub fn insert(
        &mut self,
        txn: &mut YTransaction,
        index: u32,
        chunk: &str,
        attributes: Option<HashMap<String, PyObject>>,
    ) -> PyResult<()> {
        txn.transact(|txn| self._insert(txn, index, chunk, attributes))?
    }

    /// Inserts a given `chunk` of text into this `YText` instance, starting at a given `index`.
    fn _insert(
        &mut self,
        txn: &mut YTransactionInner,
        index: u32,
        chunk: &str,
        attributes: Option<HashMap<String, PyObject>>,
    ) -> PyResult<()> {
        let attributes: Option<PyResult<Attrs>> = attributes.map(Self::parse_attrs);

        if let Some(Ok(attributes)) = attributes {
            match &mut self.0 {
                SharedType::Integrated(text) => {
                    text.insert_with_attributes(txn, index, chunk, attributes);
                    Ok(())
                }
                SharedType::Prelim(_) => Err(IntegratedOperationException::default_message()),
            }
        } else if let Some(Err(error)) = attributes {
            Err(error)
        } else {
            match &mut self.0 {
                SharedType::Integrated(text) => text.insert(txn, index, chunk),
                SharedType::Prelim(prelim_string) => {
                    prelim_string.insert_str(index as usize, chunk)
                }
            }
            Ok(())
        }
    }

    /// Inserts a given `embed` object into this `YText` instance, starting at a given `index`.
    ///
    /// Optional object with defined `attributes` will be used to wrap provided `embed`
    /// with a formatting blocks.`attributes` are only supported for a `YText` instance which
    /// already has been integrated into document store.
    pub fn insert_embed(
        &mut self,
        txn: &mut YTransaction,
        index: u32,
        embed: PyObject,
        attributes: Option<HashMap<String, PyObject>>,
    ) -> PyResult<()> {
        txn.transact(|txn| self._insert_embed(txn, index, embed, attributes))?
    }

    fn _insert_embed(
        &mut self,
        txn: &mut YTransactionInner,
        index: u32,
        embed: PyObject,
        attributes: Option<HashMap<String, PyObject>>,
    ) -> PyResult<()> {
        match &mut self.0 {
            SharedType::Integrated(text) => {
                let content: PyResult<Any> = Python::with_gil(|py| {
                    let py_type: CompatiblePyType = embed.extract(py)?;
                    py_type.try_into()
                });
                if let Some(Ok(attrs)) = attributes.map(Self::parse_attrs) {
                    text.insert_embed_with_attributes(txn, index, content?, attrs);
                } else {
                    text.insert_embed(txn, index, content?);
                }
                Ok(())
            }
            SharedType::Prelim(_) => Err(IntegratedOperationException::default_message()),
        }
    }

    /// Wraps an existing piece of text within a range described by `index`-`length` parameters with
    /// formatting blocks containing provided `attributes` metadata. This method only works for
    /// `YText` instances that already have been integrated into document store.
    pub fn format(
        &mut self,
        txn: &mut YTransaction,
        index: u32,
        length: u32,
        attributes: HashMap<String, PyObject>,
    ) -> PyResult<()> {
        txn.transact(|txn| self._format(txn, index, length, attributes))?
    }

    fn _format(
        &mut self,
        txn: &mut YTransactionInner,
        index: u32,
        length: u32,
        attributes: HashMap<String, PyObject>,
    ) -> PyResult<()> {
        match Self::parse_attrs(attributes) {
            Ok(attrs) => match &mut self.0 {
                SharedType::Integrated(text) => {
                    text.format(txn, index, length, attrs);
                    Ok(())
                }
                SharedType::Prelim(_) => Err(IntegratedOperationException::default_message()),
            },
            Err(err) => Err(err),
        }
    }

    /// Appends a given `chunk` of text at the end of current `YText` instance.
    pub fn extend(&mut self, txn: &mut YTransaction, chunk: &str) -> PyResult<()> {
        txn.transact(|txn| self._extend(txn, chunk))
    }
    fn _extend(&mut self, txn: &mut YTransactionInner, chunk: &str) {
        match &mut self.0 {
            SharedType::Integrated(v) => v.push(txn, chunk),
            SharedType::Prelim(v) => v.push_str(chunk),
        }
    }
    /// Deletes character at the specified index.
    pub fn delete(&mut self, txn: &mut YTransaction, index: u32) -> PyResult<()> {
        self.delete_range(txn, index, 1)
    }

    /// Deletes a specified range of of characters, starting at a given `index`.
    /// Both `index` and `length` are counted in terms of a number of UTF-8 character bytes.
    pub fn delete_range(
        &mut self,
        txn: &mut YTransaction,
        index: u32,
        length: u32,
    ) -> PyResult<()> {
        txn.transact(|txn| self._delete_range(txn, index, length))
    }

    fn _delete_range(&mut self, txn: &mut YTransactionInner, index: u32, length: u32) {
        match &mut self.0 {
            SharedType::Integrated(v) => v.remove_range(txn, index, length),
            SharedType::Prelim(v) => {
                v.drain((index as usize)..(index + length) as usize);
            }
        }
    }

    /// Observes updates from the `YText` instance.
    pub fn observe(&mut self, f: PyObject) -> PyResult<ShallowSubscription> {
        match &mut self.0 {
            SharedType::Integrated(text) => {
                let doc = text.doc.clone();
                let sub_id = text
                    .inner
                    .observe(move |txn, e| {
                        let e = YTextEvent::new(e, txn, doc.clone());
                        Python::with_gil(|py| {
                            if let Err(err) = f.call1(py, (e,)) {
                                err.restore(py)
                            }
                        });
                    })
                    .into();
                Ok(ShallowSubscription(sub_id))
            }
            SharedType::Prelim(_) => Err(PreliminaryObservationException::default_message()),
        }
    }

    /// Observes updates from the `YText` instance and all of its nested children.
    pub fn observe_deep(&mut self, f: PyObject) -> PyResult<DeepSubscription> {
        match &mut self.0 {
            SharedType::Integrated(text) => {
                let doc = text.doc.clone();
                let sub = text
                    .inner
                    .observe_deep(move |txn, events| {
                        Python::with_gil(|py| {
                            let events = events_into_py(txn, events, doc.clone());
                            if let Err(err) = f.call1(py, (events,)) {
                                err.restore(py)
                            }
                        })
                    })
                    .into();
                Ok(DeepSubscription(sub))
            }
            SharedType::Prelim(_) => Err(PreliminaryObservationException::default_message()),
        }
    }
    /// Cancels the observer callback associated with the `subscripton_id`.
    pub fn unobserve(&mut self, subscription_id: SubId) -> PyResult<()> {
        match &mut self.0 {
            SharedType::Integrated(text) => {
                match subscription_id {
                    SubId::Shallow(ShallowSubscription(id)) => text.unobserve(id),
                    SubId::Deep(DeepSubscription(id)) => text.unobserve_deep(id),
                }
                Ok(())
            }
            SharedType::Prelim(_) => Err(PreliminaryObservationException::default_message()),
        }
    }
}

impl YText {
    fn parse_attrs(attrs: HashMap<String, PyObject>) -> PyResult<Attrs> {
        Python::with_gil(|py| {
            attrs
                .into_iter()
                .map(|(k, v)| {
                    let key = Arc::from(k);
                    let value: CompatiblePyType = v.extract(py)?;
                    Ok((key, value.try_into()?))
                })
                .collect()
        })
    }
}

/// Event generated by `YYText.observe` method. Emitted during transaction commit phase.
#[pyclass(unsendable)]
pub struct YTextEvent {
    inner: *const TextEvent,
    doc: Rc<RefCell<YDocInner>>,
    txn: *const TransactionMut<'static>,
    target: Option<PyObject>,
    delta: Option<PyObject>,
}

impl YTextEvent {
    pub fn new(event: &TextEvent, txn: &TransactionMut, doc: Rc<RefCell<YDocInner>>) -> Self {
        let inner = event as *const TextEvent;
        // HACK: get rid of lifetime
        let txn = unsafe { std::mem::transmute::<&TransactionMut, &TransactionMut<'static>>(txn) };
        let txn = txn as *const TransactionMut;

        YTextEvent {
            inner,
            doc,
            txn,
            target: None,
            delta: None,
        }
    }

    fn inner(&self) -> &TextEvent {
        unsafe { self.inner.as_ref().unwrap() }
    }

    fn txn(&self) -> &TransactionMut {
        unsafe { self.txn.as_ref().unwrap() }
    }
}

#[pymethods]
impl YTextEvent {
    /// Returns a current shared type instance, that current event changes refer to.
    #[getter]
    pub fn target(&mut self) -> PyObject {
        if let Some(target) = self.target.as_ref() {
            target.clone()
        } else {
            let target: PyObject = Python::with_gil(|py| {
                let target = self.inner().target().clone();
                target.with_doc(self.doc.clone()).into_py(py)
            });
            self.target = Some(target.clone());
            target
        }
    }

    /// Returns an array of keys and indexes creating a path from root type down to current instance
    /// of shared type (accessible via `target` getter).
    pub fn path(&self) -> PyObject {
        Python::with_gil(|py| self.inner().path().into_py(py))
    }

    /// Returns a list of text changes made over corresponding `YText` collection within
    /// bounds of current transaction. These changes follow a format:
    ///
    /// - { insert: string, attributes: any|undefined }
    /// - { delete: number }
    /// - { retain: number, attributes: any|undefined }
    #[getter]
    pub fn delta(&mut self) -> PyObject {
        if let Some(delta) = &self.delta {
            delta.clone()
        } else {
            let delta: PyObject = Python::with_gil(|py| {
                let delta = {
                    self.inner()
                        .delta(self.txn())
                        .iter()
                        .map(|d| d.clone().with_doc_into_py(self.doc.clone(), py))
                };
                PyList::new(py, delta).into()
            });
            self.delta = Some(delta.clone());
            delta
        }
    }

    fn __repr__(&mut self) -> String {
        let target = self.target();
        let delta = self.delta();
        let path = self.path();
        format!("YTextEvent(target={target}, delta={delta}, path={path})")
    }
}
