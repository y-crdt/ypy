use crate::{
    type_conversions::{py_into_any, MultipleIntegrationError},
    y_array::YArray,
    y_map::YMap,
    y_text::YText,
    y_xml::{YXmlElement, YXmlText},
};
use lib0::any::Any;
use pyo3::create_exception;
use pyo3::{exceptions::PyException, prelude::*};
use std::{collections::HashMap, convert::TryFrom, fmt::Display};
use yrs::types::TYPE_REFS_XML_TEXT;
use yrs::types::{TypeRefs, TYPE_REFS_ARRAY, TYPE_REFS_MAP, TYPE_REFS_TEXT};
use yrs::{types::TYPE_REFS_XML_ELEMENT, SubscriptionId};

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
pub enum SharedType<T, P> {
    Integrated(T),
    Prelim(P),
}

impl<T, P> SharedType<T, P> {
    #[inline(always)]
    pub fn new(value: T) -> Self {
        SharedType::Integrated(value)
    }

    #[inline(always)]
    pub fn prelim(prelim: P) -> Self {
        SharedType::Prelim(prelim)
    }
}

#[derive(FromPyObject)]
pub enum Shared {
    Text(Py<YText>),
    Array(Py<YArray>),
    Map(Py<YMap>),
    XmlElement(Py<YXmlElement>),
    XmlText(Py<YXmlText>),
}

impl Shared {
    pub fn is_prelim(&self) -> bool {
        Python::with_gil(|py| match self {
            Shared::Text(v) => v.borrow(py).prelim(),
            Shared::Array(v) => v.borrow(py).prelim(),
            Shared::Map(v) => v.borrow(py).prelim(),
            Shared::XmlElement(_) | Shared::XmlText(_) => false,
        })
    }

    pub fn type_ref(&self) -> TypeRefs {
        match self {
            Shared::Text(_) => TYPE_REFS_TEXT,
            Shared::Array(_) => TYPE_REFS_ARRAY,
            Shared::Map(_) => TYPE_REFS_MAP,
            Shared::XmlElement(_) => TYPE_REFS_XML_ELEMENT,
            Shared::XmlText(_) => TYPE_REFS_XML_TEXT,
        }
    }
}

impl Display for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let info = Python::with_gil(|py| match self {
            Shared::Text(t) => t.borrow(py).__str__(),
            Shared::Array(a) => a.borrow(py).__str__(),
            Shared::Map(m) => m.borrow(py).__str__(),
            Shared::XmlElement(xml) => xml.borrow(py).__str__(),
            Shared::XmlText(xml) => xml.borrow(py).__str__(),
        });
        write!(f, "{}", info)
    }
}

impl TryFrom<PyObject> for Shared {
    type Error = PyErr;

    fn try_from(value: PyObject) -> Result<Self, Self::Error> {
        Python::with_gil(|py| {
            let value = value.as_ref(py);

            if let Ok(text) = value.extract() {
                Ok(Shared::Text(text))
            } else if let Ok(array) = value.extract() {
                Ok(Shared::Array(array))
            } else if let Ok(map) = value.extract() {
                Ok(Shared::Map(map))
            } else {
                Err(pyo3::exceptions::PyValueError::new_err(
                    "Could not extract Python value into a shared type.",
                ))
            }
        })
    }
}

impl TryFrom<Shared> for Any {
    type Error = PyErr;

    fn try_from(shared: Shared) -> Result<Self, Self::Error> {
        const integrated_message: &str = "All shared types should be preliminary at this point.";

        if shared.is_prelim() {
            Python::with_gil(|py| match shared {
                Shared::Text(text) => match text.borrow(py).0 {
                    SharedType::Prelim(text) => Ok(Any::String(text.into_boxed_str())),
                    SharedType::Integrated(_) => unreachable!(integrated_message),
                },
                Shared::Array(array) => match array.borrow(py).0 {
                    SharedType::Prelim(array) => {
                        let any_array: PyResult<Vec<Any>> =
                            array.into_iter().map(|v| py_into_any(v)).collect();
                        Ok(Any::Array(any_array?.into_boxed_slice()))
                    }
                    SharedType::Integrated(_) => unreachable!(integrated_message),
                },
                Shared::Map(dict) => match dict.borrow(py).0 {
                    SharedType::Prelim(dict) => {
                        let any_dict: PyResult<HashMap<String, Any>> = dict
                            .into_iter()
                            .map(|(k, v)| py_into_any(v).map(|v| (k, v)))
                            .collect();
                        Ok(Any::Map(Box::new(any_dict?)))
                    }
                    SharedType::Integrated(_) => unreachable!(integrated_message),
                },
                Shared::XmlElement(xml) => unimplemented!(),
                Shared::XmlText(xml) => unimplemented!(),
            })
        } else {
            Err(MultipleIntegrationError::new_err(format!(
                "Cannot integrate a nested Ypy object because is already integrated into a YDoc: {shared}"
            )))
        }
    }
}
