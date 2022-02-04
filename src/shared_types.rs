use crate::{
    y_array::YArray,
    y_map::YMap,
    y_text::YText,
    y_xml::{YXmlElement, YXmlText},
};
use pyo3::prelude::*;
use std::convert::TryFrom;
use yrs::types::TYPE_REFS_XML_ELEMENT;
use yrs::types::TYPE_REFS_XML_TEXT;
use yrs::types::{TypeRefs, TYPE_REFS_ARRAY, TYPE_REFS_MAP, TYPE_REFS_TEXT};

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
