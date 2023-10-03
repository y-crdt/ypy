use crate::shared_types::SubId;
use crate::y_doc::{YDocInner, WithDoc, WithTransaction};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::rc::Rc;
use yrs::types::xml::{Xml, XmlEvent, XmlTextEvent, TreeWalker};
use yrs::types::{DeepObservable, EntryChange, Path, PathSegment};
use yrs::{XmlElementRef, XmlElementPrelim, XmlTextPrelim, GetString};
use yrs::XmlFragmentRef;
use yrs::XmlTextRef;
use yrs::{Observable, SubscriptionId, Text, TransactionMut, XmlFragment, XmlNode};

use crate::shared_types::{DeepSubscription, ShallowSubscription};
use crate::type_conversions::{events_into_py, ToPython, WithDocToPython};
use crate::y_transaction::{YTransactionInner, YTransaction};

/// XML element data type. It represents an XML node, which can contain key-value attributes
/// (interpreted as strings) as well as other nested XML elements or rich text (represented by
/// `YXmlText` type).
///
/// In terms of conflict resolution, `YXmlElement` uses following rules:
///
/// - Attribute updates use logical last-write-wins principle, meaning the past updates are
///   automatically overridden and discarded by newer ones, while concurrent updates made by
///   different peers are resolved into a single value using document id seniority to establish
///   an order.
/// - Child node insertion uses sequencing rules from other Yrs collections - elements are inserted
///   using interleave-resistant algorithm, where order of concurrent inserts at the same index
///   is established using peer's document id seniority.
#[pyclass(unsendable)]
pub struct YXmlElement {
    pub inner: XmlElementRef,
    doc: Option<Rc<RefCell<YDocInner>>>,
}

impl From<XmlElementRef> for YXmlElement {
    fn from(v: XmlElementRef) -> Self {
        YXmlElement{
            inner: v,
            doc: None
        }
    }
}


impl WithDoc<YXmlElement> for XmlElementRef {
    fn with_doc(self, doc: Rc<RefCell<YDocInner>>) -> YXmlElement {
        YXmlElement {
            inner: self,
            doc: Some(doc),
        }
    }
}

impl WithTransaction for YXmlElement {
    fn get_doc(&self) -> Rc<RefCell<YDocInner>> {
        self.doc.as_ref().unwrap().clone()
    }
}

impl YXmlElement {
    fn new(v: XmlElementRef, doc: Rc<RefCell<YDocInner>>) -> Self {
        YXmlElement{
            inner: v,
            doc: Some(doc)
        }
    }
}

#[pymethods]
impl YXmlElement {
    /// Returns a tag name of this XML node.
    #[getter]
    pub fn name(&self) -> String {
        self.inner.tag().to_string()
    }

    pub fn __len__(&self) -> usize {
        self.with_transaction(|txn| self._len(txn))
    }

    fn _len(&self, txn: &YTransactionInner) -> usize {
        self.inner.len(txn) as usize
    }

    /// Inserts a new instance of `YXmlElement` as a child of this XML node and returns it.
    pub fn insert_xml_element(
        &self,
        txn: &mut YTransaction,
        index: u32,
        name: &str,
    ) -> PyResult<YXmlElement> {
        txn.transact(|txn| self._insert_xml_element(txn, index, name))
    }

    fn _insert_xml_element(
        &self,
        txn: &mut YTransactionInner,
        index: u32,
        name: &str,
    ) -> YXmlElement {
        let inner_node = self.inner.insert(txn, index, XmlElementPrelim::empty(name));
        YXmlElement {
            inner: inner_node,
            doc: self.doc.clone(),
        }
    }

    // /// Inserts a new instance of `YXmlText` as a child of this XML node and returns it.
    pub fn insert_xml_text(&self, txn: &mut YTransaction, index: u32) -> PyResult<YXmlText> {
        txn.transact(|txn| self._insert_xml_text(txn, index))
    }

    fn _insert_xml_text(&self, txn: &mut YTransactionInner, index: u32) -> YXmlText {
        let inner_node = self.inner.insert(txn, index, XmlTextPrelim::new(""));
        YXmlText {
            inner: inner_node,
            doc: self.doc.clone(),
        }
    }

    /// Removes a range of children XML nodes from this `YXmlElement` instance,
    /// starting at given `index`.
    pub fn delete(&self, txn: &mut YTransaction, index: u32, length: u32) -> PyResult<()> {
        txn.transact(|txn| self._delete(txn, index, length))
    }

    fn _delete(&self, txn: &mut YTransactionInner, index: u32, length: u32) {
        self.inner.remove_range(txn, index, length)
    }

    /// Appends a new instance of `YXmlElement` as the last child of this XML node and returns it.
    pub fn push_xml_element(&self, txn: &mut YTransaction, name: &str) -> PyResult<YXmlElement> {
        txn.transact(|txn| self._push_xml_element(txn, name))
    }
    fn _push_xml_element(&self, txn: &mut YTransactionInner, name: &str) -> YXmlElement {
        let index = self._len(txn) as u32;
        self._insert_xml_element(txn, index, name)
    }

    /// Appends a new instance of `YXmlText` as the last child of this XML node and returns it.
    pub fn push_xml_text(&self, txn: &mut YTransaction) -> PyResult<YXmlText> {
        txn.transact(|txn| self._push_xml_text(txn))
    }
    fn _push_xml_text(&self, txn: &mut YTransactionInner) -> YXmlText {
        let index = self._len(txn) as u32;
        self._insert_xml_text(txn, index)
    }

    /// Returns a first child of this XML node.
    /// It can be either `YXmlElement`, `YXmlText` or `undefined` if current node has not children.
    #[getter]
    pub fn first_child(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| {
            self.inner
                .first_child()
                .map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py))
        })
    }

    /// Returns a next XML sibling node of this XMl node.
    /// It can be either `YXmlElement`, `YXmlText` or `undefined` if current node is a last child of
    /// parent XML node.
    #[getter]
    pub fn next_sibling(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                self.inner.siblings(txn).next().map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py))
            })
        })
    }

    /// Returns a previous XML sibling node of this XMl node.
    /// It can be either `YXmlElement`, `YXmlText` or `undefined` if current node is a first child
    /// of parent XML node.
    #[getter]
    pub fn prev_sibling(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                self.inner
                    .siblings(txn)
                    .next_back()
                    .map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py))
            })
        })
    }

    /// Returns a parent `YXmlElement` node or `undefined` if current node has no parent assigned.
    #[getter]
    pub fn parent(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| self.inner.parent().map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py)))
    }

    /// Returns a string representation of this XML node.
    pub fn __str__(&self) -> String {
        self.with_transaction(|txn| self.inner.get_string(txn))
    }

    pub fn __repr__(&self) -> String {
        format!("YXmlElement({})", self.__str__())
    }

    /// Sets a `name` and `value` as new attribute for this XML node. If an attribute with the same
    /// `name` already existed on that node, its value with be overridden with a provided one.
    pub fn set_attribute(&self, txn: &mut YTransaction, name: &str, value: &str) -> PyResult<()> {
        txn.transact(|txn| self.inner.insert_attribute(txn, name, value))
    }

    /// Returns a value of an attribute given its `name`. If no attribute with such name existed,
    /// `null` will be returned.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.with_transaction(|txn| self.inner.get_attribute(txn, name))
    }

    pub fn remove_attribute(&self, txn: &mut YTransaction, name: &str) -> PyResult<()> {
        txn.transact(|txn| self.inner.remove_attribute(txn, &name))
    }

    /// Returns the attributes of this XML node as a Python list of tuples
    pub fn attributes(&self) -> PyObject {
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                let attributes = self.inner.attributes(txn);
                attributes
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<Vec<_>>()
            }).into_py(py)
        })
    }

    /// Returns an iterator that enables a deep traversal of this XML node - starting from first
    /// child over this XML node successors using depth-first strategy.
    pub fn tree_walker(&self) -> YXmlTreeWalker {
        YXmlTreeWalker::from(self)
    }

    /// Subscribes to all operations happening over this instance of `YXmlElement`. All changes are
    /// batched and eventually triggered during transaction commit phase.
    /// Returns an `SubscriptionId` which, can be used to unsubscribe the observer.
    pub fn observe(&mut self, f: PyObject) -> ShallowSubscription {
        let doc = self.get_doc();
        let sub_id = self
            .inner
            .observe(move |txn, e| {
                Python::with_gil(|py| {
                    let event = YXmlEvent::new(e, txn, doc.clone());
                    if let Err(err) = f.call1(py, (event,)) {
                        err.restore(py)
                    }
                })
            })
            .into();

        ShallowSubscription(sub_id)
    }

    /// Subscribes to all operations happening over this instance of `YXmlElement` and all of its children.
    /// All changes are batched and eventually triggered during transaction commit phase.
    /// Returns an `SubscriptionId` which, can be used to unsubscribe the observer.
    pub fn observe_deep(&mut self, f: PyObject) -> DeepSubscription {
        let doc = self.get_doc();
        let sub_id = self
            .inner
            .observe_deep(move |txn, events| {
                Python::with_gil(|py| {
                    let events = events_into_py(txn, events, Some(doc.clone()));
                    if let Err(err) = f.call1(py, (events,)) {
                        err.restore(py)
                    }
                })
            })
            .into();
        DeepSubscription(sub_id)
    }

    /// Cancels the observer callback associated with the `subscripton_id`.
    pub fn unobserve(&mut self, subscription_id: SubId) {
        match subscription_id {
            SubId::Shallow(ShallowSubscription(id)) => self.inner.unobserve(id),
            SubId::Deep(DeepSubscription(id)) => self.inner.unobserve_deep(id),
        }
    }
}

/// A shared data type used for collaborative text editing, that can be used in a context of
/// `YXmlElement` node. It enables multiple users to add and remove chunks of text in efficient
/// manner. This type is internally represented as a mutable double-linked list of text chunks
/// - an optimization occurs during `YTransaction.commit`, which allows to squash multiple
/// consecutively inserted characters together as a single chunk of text even between transaction
/// boundaries in order to preserve more efficient memory model.
///
/// Just like `YXmlElement`, `YXmlText` can be marked with extra metadata in form of attributes.
///
/// `YXmlText` structure internally uses UTF-8 encoding and its length is described in a number of
/// bytes rather than individual characters (a single UTF-8 code point can consist of many bytes).
///
/// Like all Yrs shared data types, `YXmlText` is resistant to the problem of interleaving (situation
/// when characters inserted one after another may interleave with other peers concurrent inserts
/// after merging all updates together). In case of Yrs conflict resolution is solved by using
/// unique document id to determine correct and consistent ordering.
#[pyclass(unsendable)]
pub struct YXmlText {
    pub inner: XmlTextRef,
    doc: Option<Rc<RefCell<YDocInner>>>
}

impl From<XmlTextRef> for YXmlText {
    fn from(v: XmlTextRef) -> Self {
        YXmlText {
            inner: v,
            doc: None
        }
    }
}

impl WithDoc<YXmlText> for XmlTextRef {
    fn with_doc(self, doc: Rc<RefCell<YDocInner>>) -> YXmlText {
        YXmlText {
            inner: self,
            doc: Some(doc),
        }
    }
}

impl WithTransaction for YXmlText {
    fn get_doc(&self) -> Rc<RefCell<YDocInner>> {
        self.doc.as_ref().unwrap().clone()
    }
}

impl YXmlText {
    fn new(v: XmlTextRef, doc: Rc<RefCell<YDocInner>>) -> Self {
        YXmlText{
            inner: v,
            doc: Some(doc)
        }
    }
}

#[pymethods]
impl YXmlText {
    /// Returns length of an underlying string stored in this `YXmlText` instance,
    /// understood as a number of UTF-8 encoded bytes.
    pub fn __len__(&self) -> usize {
        self.with_transaction(|txn| self._len(txn))
    }

    fn _len(&self, txn: &YTransactionInner) -> usize {
        self.inner.len(txn) as usize
    }

    /// Inserts a given `chunk` of text into this `YXmlText` instance, starting at a given `index`.
    pub fn insert(&self, txn: &mut YTransaction, index: i32, chunk: &str) -> PyResult<()> {
        txn.transact(|txn| self._insert(txn, index, chunk))
    }
    fn _insert(&self, txn: &mut YTransactionInner, index: i32, chunk: &str) {
        self.inner.insert(txn, index as u32, chunk)
    }

    /// Appends a given `chunk` of text at the end of `YXmlText` instance.
    pub fn push(&self, txn: &mut YTransaction, chunk: &str) -> PyResult<()> {
        txn.transact(|txn| self._push(txn, chunk))
    }

    fn _push(&self, txn: &mut YTransactionInner, chunk: &str) {
        self.inner.push(txn, chunk)
    }

    /// Deletes a specified range of of characters, starting at a given `index`.
    /// Both `index` and `length` are counted in terms of a number of UTF-8 character bytes.
    pub fn delete(&self, txn: &mut YTransaction, index: u32, length: u32) -> PyResult<()> {
        txn.transact(|txn| self._delete(txn, index, length))
    }
    fn _delete(&self, txn: &mut YTransactionInner, index: u32, length: u32) {
        self.inner.remove_range(txn, index, length)
    }

    /// Returns a next XML sibling node of this XMl node.
    /// It can be either `YXmlElement`, `YXmlText` or `undefined` if current node is a last child of
    /// parent XML node.
    #[getter]
    pub fn next_sibling(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                self.inner.siblings(txn).next().map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py))
            })
        })
    }

    /// Returns a previous XML sibling node of this XMl node.
    /// It can be either `YXmlElement`, `YXmlText` or `undefined` if current node is a first child
    /// of parent XML node.
    #[getter]
    pub fn prev_sibling(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                self.inner
                    .siblings(txn)
                    .next_back()
                    .map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py))
            })
        })
    }

    /// Returns a parent `YXmlElement` node or `undefined` if current node has no parent assigned.
    #[getter]
    pub fn parent(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| self.inner.parent().map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py)))
    }

    /// Returns an underlying string stored in this `YXmlText` instance.
    pub fn __str__(&self) -> String {
        self.with_transaction(|txn| self.inner.get_string(txn))
    }

    pub fn __repr__(&self) -> String {
        format!("YXmlText({})", self.__str__())
    }

    /// Sets a `name` and `value` as new attribute for this XML node. If an attribute with the same
    /// `name` already existed on that node, its value with be overridden with a provided one.
    pub fn set_attribute(&self, txn: &mut YTransaction, name: &str, value: &str) -> PyResult<()> {
        txn.transact(|txn| self.inner.insert_attribute(txn, name, value))
    }

    /// Returns a value of an attribute given its `name`. If no attribute with such name existed,
    /// `null` will be returned.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.with_transaction(|txn| self.inner.get_attribute(txn, name))
    }

    /// Removes an attribute from this XML node, given its `name`.
    pub fn remove_attribute(&self, txn: &mut YTransaction, name: &str) -> PyResult<()> {
        txn.transact(|txn| self.inner.remove_attribute(txn, &name))
    }

    /// Returns an iterator that enables to traverse over all attributes of this XML node in
    /// unspecified order.
    pub fn attributes(&self) -> PyObject {
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                let attributes = self.inner.attributes(txn);
                attributes
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<Vec<_>>()
            }).into_py(py)
        })
    }

    /// Subscribes to all operations happening over this instance of `YXmlText`. All changes are
    /// batched and eventually triggered during transaction commit phase.
    /// Returns an `SubscriptionId` which, which can be used to unsubscribe the callback function.
    pub fn observe(&mut self, f: PyObject) -> ShallowSubscription {
        let doc = self.get_doc();
        let sub_id: SubscriptionId = self
            .inner
            .observe(move |txn, e| {
                Python::with_gil(|py| {
                    let e = YXmlTextEvent::new(e, txn, doc.clone());
                    if let Err(err) = f.call1(py, (e,)) {
                        err.restore(py)
                    }
                })
            })
            .into();
        ShallowSubscription(sub_id)
    }

    /// Subscribes to all operations happening over this instance of `YXmlText` and its child elements. All changes are
    /// batched and eventually triggered during transaction commit phase.
    /// Returns an `SubscriptionId` which, which can be used to unsubscribe the callback function.
    pub fn observe_deep(&mut self, f: PyObject) -> DeepSubscription {
        let doc = self.get_doc();
        let sub_id: SubscriptionId = self
            .inner
            .observe_deep(move |txn, events| {
                Python::with_gil(|py| {
                    let e = events_into_py(txn, events, Some(doc.clone()));
                    if let Err(err) = f.call1(py, (e,)) {
                        err.restore(py)
                    }
                })
            })
            .into();
        DeepSubscription(sub_id)
    }

    /// Cancels the observer callback associated with the `subscripton_id`.
    pub fn unobserve(&mut self, subscription_id: SubId) {
        match subscription_id {
            SubId::Shallow(ShallowSubscription(id)) => self.inner.unobserve(id),
            SubId::Deep(DeepSubscription(id)) => self.inner.unobserve_deep(id),
        }
    }
}

#[pyclass(unsendable)]
pub struct YXmlFragment {
    pub inner: XmlFragmentRef,
    doc: Option<Rc<RefCell<YDocInner>>>
}


impl From<XmlFragmentRef> for YXmlFragment {
    fn from(v: XmlFragmentRef) -> Self {
        YXmlFragment{
            inner: v,
            doc: None
        }
    }
}


impl WithDoc<YXmlFragment> for XmlFragmentRef {
    fn with_doc(self, doc: Rc<RefCell<YDocInner>>) -> YXmlFragment {
        YXmlFragment {
            inner: self,
            doc: Some(doc),
        }
    }
}

impl WithTransaction for YXmlFragment {
    fn get_doc(&self) -> Rc<RefCell<YDocInner>> {
        self.doc.as_ref().unwrap().clone()
    }
}

impl YXmlFragment {
    fn new(v: XmlFragmentRef, doc: Rc<RefCell<YDocInner>>) -> Self {
        YXmlFragment{
            inner: v,
            doc: Some(doc)
        }
    }
}

#[pymethods]
impl YXmlFragment {
    /// Returns a number of child XML nodes stored within this `YXMlElement` instance.
    pub fn __len__(&self) -> usize {
        self.with_transaction(|txn| self._len(txn))
    }

    fn _len(&self, txn: &YTransactionInner) -> usize {
        self.inner.len(txn) as usize
    }

    pub fn __str__(&self) -> String {
        self.with_transaction(|txn| self.inner.get_string(txn))
    }

    pub fn __repr__(&self) -> String {
        format!("YXmlElement({})", self.__str__())
    }

    /// Returns a first child of this XML node.
    /// It can be either `YXmlElement`, `YXmlText` or `undefined` if current node has not children.
    #[getter]
    pub fn first_child(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| {
            self.inner
                .first_child()
                .map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py))
        })
    }

    #[getter]
    pub fn parent(&self) -> PyObject {
        let doc = self.get_doc();
        Python::with_gil(|py| self.inner.parent().map_or(py.None(), |xml| xml.with_doc_into_py(doc.clone(), py)))
    }

    /// Inserts a new instance of `YXmlElement` as a child of this XML node and returns it.
    pub fn insert_xml_element(
        &self,
        txn: &mut YTransaction,
        index: u32,
        name: &str,
    ) -> PyResult<YXmlElement> {
        txn.transact(|txn| self._insert_xml_element(txn, index, name))
    }

    fn _insert_xml_element(
        &self,
        txn: &mut YTransactionInner,
        index: u32,
        name: &str,
    ) -> YXmlElement {
        let inner_node = self.inner.insert(txn, index, XmlElementPrelim::empty(name));
        YXmlElement {
            inner: inner_node,
            doc: self.doc.clone(),
        }
    }

    /// Removes a single element at provided `index`.
    pub fn remove(&self, txn: &mut YTransaction, index: u32) -> PyResult<()> {
        txn.transact(|txn| self._remove(txn, index))
    }

    fn _remove(&self, txn: &mut YTransactionInner, index: u32) {
        self.inner.remove_range(txn, index, 1)
    }

    /// Retrieves a value stored at a given `index`. Returns `None` when provided index was out
    /// of the range of a current array.
    pub fn get(&self, index: u32) -> Option<PyObject> {
        Python::with_gil(|py| {
            self.with_transaction(|txn| {
                self.inner.get(txn, index).map(|xml| xml.with_doc_into_py(self.get_doc(), py))
            })
        })
    }

    pub fn tree_walker(&self) -> YXmlTreeWalker {
        YXmlTreeWalker::from(self)
    }

}

#[pyclass(unsendable)]
pub struct YXmlTreeWalker {
    walker: ManuallyDrop<TreeWalker<'static, &'static YTransactionInner, YTransactionInner>>,
    doc: Rc<RefCell<YDocInner>>,
}

impl From<&YXmlElement> for YXmlTreeWalker {
    fn from(xml_element: &YXmlElement) -> Self {
        // HACK: get rid of lifetime
        let xml_element = xml_element as *const YXmlElement;
        let xml_element = unsafe { &*xml_element };

        let walker = xml_element.with_transaction(|txn| {
            // HACK: get rid of lifetime
            let txn = txn as *const YTransactionInner;
            unsafe { 
                xml_element.inner.successors(&*txn)
            }
        });
        YXmlTreeWalker {
            walker: ManuallyDrop::new(walker),
            doc: xml_element.get_doc(),
        }
    }
}


impl From<&YXmlFragment> for YXmlTreeWalker {
    fn from(xml_fragment: &YXmlFragment) -> Self {
        // HACK: get rid of lifetime
        let xml_fragment = xml_fragment as *const YXmlFragment;
        let xml_fragment = unsafe { &*xml_fragment };

        let walker = xml_fragment.with_transaction(|txn| {
            // HACK: get rid of lifetime
            let txn = txn as *const YTransactionInner;
            unsafe { 
                xml_fragment.inner.successors(&*txn)
            }
        });
        YXmlTreeWalker {
            walker: ManuallyDrop::new(walker),
            doc: xml_fragment.get_doc(),
        }
    }
}

impl Drop for YXmlTreeWalker {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.walker) }
    }
}

#[pymethods]
impl YXmlTreeWalker {
    pub fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<PyObject> {
        Python::with_gil(|py| {
            slf.walker.next().map(|v| v.with_doc_into_py(slf.doc.clone(), py))
        })
    }
}

#[pyclass(unsendable)]
pub struct YXmlEvent {
    inner: *const XmlEvent,
    doc: Rc<RefCell<YDocInner>>,
    txn: *const TransactionMut<'static>,

    target: Option<PyObject>,
    delta: Option<PyObject>,
    keys: Option<PyObject>,
}
impl YXmlEvent {
    pub fn new(event: &XmlEvent, txn: &TransactionMut, doc: Rc<RefCell<YDocInner>>) -> Self {
        let inner = event as *const XmlEvent;
        // HACK: get rid of lifetime
        let txn = unsafe { std::mem::transmute::<&TransactionMut, &TransactionMut<'static>>(txn) };
        let txn = txn as *const TransactionMut;
        YXmlEvent {
            inner,
            doc,
            txn,
            target: None,
            delta: None,
            keys: None,
        }
    }

    fn inner(&self) -> &XmlEvent {
        unsafe { self.inner.as_ref().unwrap() }
    }

    fn txn(&self) -> &TransactionMut {
        unsafe { self.txn.as_ref().unwrap() }
    }
}

#[pymethods]
impl YXmlEvent {
    /// Returns a current shared type instance, that current event changes refer to.
    #[getter]
    pub fn target(&mut self) -> PyObject {
        if let Some(target) = self.target.as_ref() {
            target.clone()
        } else {
            let target: PyObject = Python::with_gil(|py| {
                let target = self.inner().target().clone();
                match target {
                    XmlNode::Element(v) => YXmlElement::new(v, self.doc.clone()).into_py(py),
                    XmlNode::Text(v) => YXmlText::new(v, self.doc.clone()).into_py(py),
                    XmlNode::Fragment(v) => YXmlFragment::new(v, self.doc.clone()).into_py(py),
                }
            });
            self.target = Some(target.clone());
            target
        }
    }

    fn __repr__(&mut self) -> String {
        let target = self.target();
        let delta = self.delta();
        let keys = self.keys();
        let path = self.path();

        format!("YXmlEvent(target={target}, delta={delta}, keys={keys}, path={path})")
    }

    /// Returns an array of keys and indexes creating a path from root type down to current instance
    /// of shared type (accessible via `target` getter).
    pub fn path(&self) -> PyObject {
        Python::with_gil(|py| self.inner().path().into_py(py))
    }

    /// Returns all changes done upon map component of a current shared data type (which can be
    /// accessed via `target`) within a bounds of corresponding transaction `txn`. These
    /// changes are done in result of operations made on `YMap` data type or attribute changes of
    /// `YXmlElement` and `YXmlText` types.
    #[getter]
    pub fn keys(&mut self) -> PyObject {
        if let Some(keys) = &self.keys {
            keys.clone()
        } else {
            Python::with_gil(|py| {
                let keys = self.inner().keys(self.txn());
                let result = PyDict::new(py);
                for (key, value) in keys.iter() {
                    result.set_item(key.deref(), value.into_py(py)).unwrap();
                }
                let keys = PyObject::from(result);
                self.keys = Some(keys.clone());
                keys
            })
        }
    }

    /// Returns collection of all changes done over an array component of a current shared data
    /// type (which can be accessed via `target` property). These changes are usually done in result
    /// of operations done on `YArray` and `YText`/`XmlText` types, but also whenever `XmlElement`
    /// children nodes list is modified.
    #[getter]
    pub fn delta(&mut self) -> PyObject {
        if let Some(delta) = &self.delta {
            delta.clone()
        } else {
            Python::with_gil(|py| {
                let delta = self
                    .inner()
                    .delta(self.txn())
                    .into_iter()
                    .map(|d| Python::with_gil(|py| d.into_py(py)));
                let result = pyo3::types::PyList::new(py, delta);
                let delta: PyObject = result.into();
                self.delta = Some(delta.clone());
                delta
            })
        }
    }
}

#[pyclass(unsendable)]
pub struct YXmlTextEvent {
    inner: *const XmlTextEvent,
    doc: Rc<RefCell<YDocInner>>,
    txn: *const TransactionMut<'static>,
  
    target: Option<PyObject>,
    delta: Option<PyObject>,
    keys: Option<PyObject>,
}

impl YXmlTextEvent {
    pub fn new(event: &XmlTextEvent, txn: &TransactionMut, doc: Rc<RefCell<YDocInner>>) -> Self {
        let inner = event as *const XmlTextEvent;
        // HACK: get rid of lifetime
        let txn = unsafe { std::mem::transmute::<&TransactionMut, &TransactionMut<'static>>(txn) };
        let txn = txn as *const TransactionMut;
        YXmlTextEvent {
            inner,
            doc,
            txn,
            target: None,
            delta: None,
            keys: None,
        }
    }

    fn inner(&self) -> &XmlTextEvent {
        unsafe { self.inner.as_ref().unwrap() }
    }

    fn txn(&self) -> &TransactionMut {
        unsafe { self.txn.as_ref().unwrap() }
    }
}

#[pymethods]
impl YXmlTextEvent {
    /// Returns a current shared type instance, that current event changes refer to.
    #[getter]
    pub fn target(&mut self) -> PyObject {
        if let Some(target) = self.target.as_ref() {
            target.clone()
        } else {
            let target = Python::with_gil(|py| {
                let target = self.inner().target().clone();
                target.with_doc(self.doc.clone()).into_py(py)
            });
            self.target = Some(target.clone());
            target
        }
    }

    fn __repr__(&mut self) -> String {
        let target = self.target();
        let delta = self.delta();
        let keys = self.keys();
        let path = self.path();

        format!("YXmlEvent(target={target}, delta={delta}, keys={keys}, path={path})")
    }

    /// Returns a current shared type instance, that current event changes refer to.
    pub fn path(&self) -> PyObject {
        Python::with_gil(|py| self.inner().path().into_py(py))
    }

    /// Returns all changes done upon map component of a current shared data type (which can be
    /// accessed via `target`) within a bounds of corresponding transaction `txn`. These
    /// changes are done in result of operations made on `YMap` data type or attribute changes of
    /// `YXmlElement` and `YXmlText` types.
    #[getter]
    pub fn keys(&mut self) -> PyObject {
        if let Some(keys) = &self.keys {
            keys.clone()
        } else {
            Python::with_gil(|py| {
                let keys = self.inner().keys(self.txn());
                let result = PyDict::new(py);
                for (key, value) in keys.iter() {
                    result.set_item(key.deref(), value.into_py(py)).unwrap();
                }
                let keys = PyObject::from(result);
                self.keys = Some(keys.clone());
                keys
            })
        }
    }

    /// Returns a list of text changes made over corresponding `YXmlText` collection within
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
            Python::with_gil(|py| {
                let delta = self
                    .inner()
                    .delta(self.txn())
                    .into_iter()
                    .map(|d| Python::with_gil(|py| d.clone().into_py(py)));
                let result = pyo3::types::PyList::new(py, delta);
                let delta: PyObject = result.into();
                self.delta = Some(delta.clone());
                delta
            })
        }
    }
}

// XML Type Conversions

impl ToPython for XmlNode {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            XmlNode::Element(v) => YXmlElement::from(v).into_py(py),
            XmlNode::Text(v) => YXmlText::from(v).into_py(py),
            XmlNode::Fragment(v) => YXmlFragment::from(v).into_py(py),
        }
    }
}

impl WithDocToPython for XmlNode {
    fn with_doc_into_py(self,  doc: Rc<RefCell<YDocInner>>, py: Python) -> PyObject {
        match self {
            XmlNode::Element(v) => v.with_doc(doc).into_py(py),
            XmlNode::Text(v) => v.with_doc(doc).into_py(py),
            XmlNode::Fragment(v) => v.with_doc(doc).into_py(py),
        }
    }
}

impl<'a> ToPython for &EntryChange {
    fn into_py(self, py: Python) -> PyObject {
        let result = PyDict::new(py);
        let action = "action";
        match self {
            EntryChange::Inserted(new) => {
                let new_value = new.clone().into_py(py);
                result.set_item(action, "add").unwrap();
                result.set_item("newValue", new_value).unwrap();
            }
            EntryChange::Updated(old, new) => {
                let old_value = old.clone().into_py(py);
                let new_value = new.clone().into_py(py);
                result.set_item(action, "update").unwrap();
                result.set_item("oldValue", old_value).unwrap();
                result.set_item("newValue", new_value).unwrap();
            }
            EntryChange::Removed(old) => {
                let old_value = old.clone().into_py(py);
                result.set_item(action, "delete").unwrap();
                result.set_item("oldValue", old_value).unwrap();
            }
        }
        result.into()
    }
}

impl ToPython for Path {
    fn into_py(self, py: Python) -> PyObject {
        let result = PyList::empty(py);
        for segment in self {
            match segment {
                PathSegment::Key(key) => {
                    result.append(key.as_ref()).unwrap();
                }
                PathSegment::Index(idx) => {
                    result.append(idx).unwrap();
                }
            }
        }
        result.into()
    }
}
