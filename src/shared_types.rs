use crate::{
    y_array::YArray,
    y_doc::YDocInner,
    y_map::YMap,
    y_text::YText,
    y_transaction::YTransactionInner,
    y_xml::{YXmlElement, YXmlFragment, YXmlText},
};
use pyo3::create_exception;
use pyo3::types as pytypes;
use pyo3::{exceptions::PyException, prelude::*};
use std::{
    cell::RefCell,
    fmt::Display,
    ops::{Deref, DerefMut},
    rc::Rc,
};
use yrs::types::TypeRef;
use yrs::SubscriptionId;

// Common errors
create_exception!(y_py, PreliminaryObservationException, PyException, "Occurs when an observer is attached to a Y type that is not integrated into a YDoc. Y types can only be observed once they have been added to a YDoc.");
create_exception!(y_py, IntegratedOperationException, PyException, "Occurs when a method requires a type to be integrated (embedded into a YDoc), but is called on a preliminary type.");

/// Creates a default error with a common message string for throwing a `PyErr`.
pub(crate) trait DefaultPyErr {
    /// Creates a new instance of the error with a default message.
    fn default_message() -> PyErr;
}

impl DefaultPyErr for PreliminaryObservationException {
    fn default_message() -> PyErr {
        PreliminaryObservationException::new_err(
            "Cannot observe a preliminary type. Must be added to a YDoc first",
        )
    }
}

impl DefaultPyErr for IntegratedOperationException {
    fn default_message() -> PyErr {
        IntegratedOperationException::new_err(
            "This operation requires the type to be integrated into a YDoc.",
        )
    }
}

#[pyclass]
#[derive(Clone, Copy)]
pub struct ShallowSubscription(pub SubscriptionId);
#[pyclass]
#[derive(Clone, Copy)]
pub struct DeepSubscription(pub SubscriptionId);

#[derive(FromPyObject)]
pub enum SubId {
    Shallow(ShallowSubscription),
    Deep(DeepSubscription),
}

#[derive(Clone)]
pub enum CompatiblePyType<'a> {
    Bool(&'a pytypes::PyBool),
    Int(&'a pytypes::PyInt),
    Float(&'a pytypes::PyFloat),
    String(&'a pytypes::PyString),
    List(&'a pytypes::PyList),
    Dict(&'a pytypes::PyDict),
    YType(YPyType<'a>),
    None,
}

#[derive(Clone)]
pub enum SharedType<I, P> {
    Integrated(I),
    Prelim(P),
}

impl<I, P> SharedType<I, P> {
    #[inline(always)]
    pub fn new(value: I) -> Self {
        SharedType::Integrated(value)
    }

    #[inline(always)]
    pub fn prelim(prelim: P) -> Self {
        SharedType::Prelim(prelim)
    }
}

#[derive(Clone)]
pub enum YPyType<'a> {
    Text(&'a PyCell<YText>),
    Array(&'a PyCell<YArray>),
    Map(&'a PyCell<YMap>),
    XmlElement(&'a PyCell<YXmlElement>),
    XmlText(&'a PyCell<YXmlText>),
    XmlFragment(&'a PyCell<YXmlFragment>),
}

impl<'a> YPyType<'a> {
    pub fn is_prelim(&self) -> bool {
        match self {
            YPyType::Text(v) => v.borrow().prelim(),
            YPyType::Array(v) => v.borrow().prelim(),
            YPyType::Map(v) => v.borrow().prelim(),
            YPyType::XmlElement(_) | YPyType::XmlText(_) | YPyType::XmlFragment(_) => false,
        }
    }

    pub fn type_ref(&self) -> TypeRef {
        match &self {
            YPyType::Text(_) => TypeRef::Text,
            YPyType::Array(_) => TypeRef::Array,
            YPyType::Map(_) => TypeRef::Map,
            YPyType::XmlElement(py_xml_element) => {
                TypeRef::XmlElement(py_xml_element.borrow().0.tag().clone())
            }
            YPyType::XmlText(_) => TypeRef::XmlText,
            YPyType::XmlFragment(_) => TypeRef::XmlFragment,
        }
    }
}

#[derive(Clone)]
pub struct TypeWithDoc<T> {
    pub inner: T,
    pub doc: Rc<RefCell<YDocInner>>,
}

impl<T> TypeWithDoc<T> {
    pub fn new(inner: T, doc: Rc<RefCell<YDocInner>>) -> Self {
        Self { inner, doc }
    }

    fn get_transaction(&self) -> Rc<RefCell<YTransactionInner>> {
        let doc = self.doc.clone();
        let txn = doc.borrow_mut().begin_transaction();
        txn
    }

    pub fn with_transaction<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&YTransactionInner) -> R,
    {
        let txn = self.get_transaction();
        let mut txn = txn.borrow_mut();
        f(&mut txn)
    }
}

impl<T> Deref for TypeWithDoc<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> DerefMut for TypeWithDoc<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<'a> Display for YPyType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let info = match self {
            YPyType::Text(t) => t.borrow().__str__(),
            YPyType::Array(a) => a.borrow().__str__(),
            YPyType::Map(m) => m.borrow().__str__(),
            YPyType::XmlElement(xml) => xml.borrow().__str__(),
            YPyType::XmlText(xml) => xml.borrow().__str__(),
            YPyType::XmlFragment(xml) => xml.borrow().__str__(),
        };
        write!(f, "{}", info)
    }
}
