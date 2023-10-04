use lib0::any::Any;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types as pytypes;
use pyo3::types::PyList;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::ops::Deref;
use std::rc::Rc;
use yrs::block::Unused;
use yrs::block::{ItemContent, Prelim};
use yrs::types::Events;
use yrs::types::{Attrs, Branch, BranchPtr, Change, Delta, Value};
use yrs::ArrayRef;
use yrs::MapRef;
use yrs::TextRef;
use yrs::TransactionMut;
use yrs::{Array, Map, Text};

use crate::shared_types::CompatiblePyType;
use crate::shared_types::TypeWithDoc;
use crate::shared_types::{SharedType, YPyType};
use crate::y_array::YArray;
use crate::y_array::YArrayEvent;
use crate::y_doc::WithDoc;
use crate::y_doc::YDocInner;
use crate::y_map::YMapEvent;
use crate::y_text::YTextEvent;
use crate::y_xml::{YXmlEvent, YXmlTextEvent};

create_exception!(y_py, MultipleIntegrationError, PyException, "A Ypy data type instance cannot be integrated into multiple YDocs or the same YDoc multiple times");

pub trait ToPython {
    fn into_py(self, py: Python) -> PyObject;
}

impl<T: ToPython> ToPython for Vec<T> {
    fn into_py(self, py: Python) -> PyObject {
        let elements = self.into_iter().map(|v| v.into_py(py));
        pyo3::types::PyList::new(py, elements).into()
    }
}

impl<K, V> ToPython for HashMap<K, V>
where
    K: ToPyObject,
    V: ToPython,
{
    fn into_py(self, py: Python) -> PyObject {
        let py_dict = pytypes::PyDict::new(py);
        for (k, v) in self.into_iter() {
            py_dict.set_item(k, v.into_py(py)).unwrap();
        }
        py_dict.into_py(py)
    }
}

impl<'a> TryFrom<&'a PyAny> for CompatiblePyType<'a> {
    type Error = PyErr;

    fn try_from(py_any: &'a PyAny) -> Result<Self, Self::Error> {
        if let Ok(b) = py_any.downcast::<pytypes::PyBool>() {
            Ok(Self::Bool(b))
        } else if let Ok(i) = py_any.downcast::<pytypes::PyInt>() {
            Ok(Self::Int(i))
        } else if py_any.is_none() {
            Ok(Self::None)
        } else if let Ok(f) = py_any.downcast::<pytypes::PyFloat>() {
            Ok(Self::Float(f))
        } else if let Ok(s) = py_any.downcast::<pytypes::PyString>() {
            Ok(Self::String(s))
        } else if let Ok(list) = py_any.downcast::<pytypes::PyList>() {
            Ok(Self::List(list))
        } else if let Ok(dict) = py_any.downcast::<pytypes::PyDict>() {
            Ok(Self::Dict(dict))
        } else if let Ok(v) = YPyType::try_from(py_any) {
            Ok(Self::YType(v))
        } else {
            Err(PyTypeError::new_err(format!(
                "Cannot integrate this type into a YDoc: {py_any}"
            )))
        }
    }
}

impl<'a> FromPyObject<'a> for CompatiblePyType<'a> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        Self::try_from(ob)
    }
}

pub trait WithDocToPython {
    fn with_doc_into_py(self, doc: Rc<RefCell<YDocInner>>, py: Python) -> PyObject;
}

impl WithDocToPython for Delta {
    fn with_doc_into_py(self, doc: Rc<RefCell<YDocInner>>, py: Python) -> PyObject {
        let result = pytypes::PyDict::new(py);
        match self {
            Delta::Inserted(value, attrs) => {
                let value = value.clone().with_doc_into_py(doc.clone(), py);
                result.set_item("insert", value).unwrap();

                if let Some(attrs) = attrs {
                    let attrs = attrs.with_doc_into_py(doc.clone(), py);
                    result.set_item("attributes", attrs).unwrap();
                }
            }
            Delta::Retain(len, attrs) => {
                result.set_item("retain", len).unwrap();

                if let Some(attrs) = attrs {
                    let attrs = attrs.with_doc_into_py(doc.clone(), py);
                    result.set_item("attributes", attrs).unwrap();
                }
            }
            Delta::Deleted(len) => {
                result.set_item("delete", len).unwrap();
            }
        }
        result.into()
    }
}

impl WithDocToPython for &Attrs {
    fn with_doc_into_py(self, doc: Rc<RefCell<YDocInner>>, py: Python) -> PyObject {
        let o = pytypes::PyDict::new(py);
        for (key, value) in self.iter() {
            let key = key.as_ref();
            let value = Value::Any(value.clone()).with_doc_into_py(doc.clone(), py);
            o.set_item(key, value).unwrap();
        }
        o.into()
    }
}

impl WithDocToPython for &Change {
    fn with_doc_into_py(self, doc: Rc<RefCell<YDocInner>>, py: Python) -> PyObject {
        let result = pytypes::PyDict::new(py);
        match self {
            Change::Added(values) => {
                let values: Vec<PyObject> = values
                    .iter()
                    .map(|v| v.clone().with_doc_into_py(doc.clone(), py))
                    .collect();
                result.set_item("insert", values).unwrap();
            }
            Change::Removed(len) => {
                result.set_item("delete", len).unwrap();
            }
            Change::Retain(len) => {
                result.set_item("retain", len).unwrap();
            }
        }
        result.into()
    }
}

pub(crate) struct PyObjectWrapper(pub TypeWithDoc<PyObject>);

impl PyObjectWrapper {
    pub fn new(inner: PyObject, doc: Rc<RefCell<YDocInner>>) -> Self {
        Self(TypeWithDoc::new(inner, doc))
    }
}

impl Deref for PyObjectWrapper {
    type Target = PyObject;

    fn deref(&self) -> &Self::Target {
        &self.0.inner
    }
}

impl Prelim for PyObjectWrapper {
    type Return = Unused;

    fn into_content(self, txn: &mut TransactionMut) -> (ItemContent, Option<Self>) {
        Python::with_gil(|py| {
            let valid_type: CompatiblePyType = self.0.extract(py).unwrap_or_else(|err| {
                err.restore(py);
                CompatiblePyType::None
            });
            let (item_content, py_any) = valid_type.into_content(txn);
            let wrapper: Option<Self> =
                py_any.map(|py_type| PyObjectWrapper::new(py_type.into(), self.0.doc.clone()));
            (item_content, wrapper)
        })
    }

    fn integrate(self, txn: &mut TransactionMut, inner_ref: BranchPtr) {
        Python::with_gil(|py| {
            let valid_type: CompatiblePyType = self.0.extract(py).unwrap_or_else(|err| {
                err.restore(py);
                CompatiblePyType::None
            });

            match valid_type {
                CompatiblePyType::YType(y_type) if y_type.is_prelim() => {
                    match y_type {
                        YPyType::Text(v) => {
                            let text = TextRef::from(inner_ref);
                            let mut y_text = v.borrow_mut();

                            if let SharedType::Prelim(v) = y_text.0.to_owned() {
                                text.push(txn, v.as_str());
                            }
                            y_text.0 = SharedType::Integrated(TypeWithDoc::new(text.clone(), self.0.doc.clone()));
                        }
                        YPyType::Array(v) => {
                            let array = ArrayRef::from(inner_ref);
                            let mut y_array = v.borrow_mut();
                            if let SharedType::Prelim(items) = y_array.0.to_owned() {
                                let len = array.len(txn);
                                YArray::insert_multiple_at(&array, txn, self.0.doc.clone(), len, items).unwrap();
                            }
                            y_array.0 = SharedType::Integrated(TypeWithDoc::new(array.clone(), self.0.doc.clone()));
                        }
                        YPyType::Map(v) => {
                            let map = MapRef::from(inner_ref);
                            let mut y_map = v.borrow_mut();
                            Python::with_gil(|py| {
                                if let SharedType::Prelim(ref entries) = y_map.0 {
                                    for (k, v) in entries {
                                        let x: CompatiblePyType = v.extract(py).unwrap_or_else(|err| {
                                            err.restore(py);
                                            CompatiblePyType::None
                                        });
                                        if let CompatiblePyType::YType(y_type) = x {
                                            let wrapped = PyObjectWrapper::new(y_type.into(), self.0.doc.clone());
                                            map.insert(txn, k.to_owned(), wrapped);
                                        } else {
                                            map.insert(txn, k.to_owned(), x);
                                        }
                                    }
                                }
                            });
                            y_map.0 = SharedType::Integrated(TypeWithDoc::new(map.clone(), self.0.doc.clone()));
                        }
                        YPyType::XmlElement(_) | YPyType::XmlText(_) | YPyType::XmlFragment(_) => unreachable!("As defined in Shared::is_prelim(), neither XML type can ever exist outside a YDoc"),
                    }
                }
                _ => ()
            }
        })
    }
}

impl<'a> From<CompatiblePyType<'a>> for PyObject {
    fn from(value: CompatiblePyType<'a>) -> Self {
        match value {
            CompatiblePyType::Bool(b) => b.into(),
            CompatiblePyType::Int(i) => i.into(),
            CompatiblePyType::Float(f) => f.into(),
            CompatiblePyType::String(s) => s.into(),
            CompatiblePyType::List(list) => list.into(),
            CompatiblePyType::Dict(dict) => dict.into(),
            CompatiblePyType::YType(y_type) => y_type.into(),
            CompatiblePyType::None => Python::with_gil(|py| py.None()),
        }
    }
}

impl<'a> Prelim for CompatiblePyType<'a> {
    type Return = Unused;

    fn into_content(self, _txn: &mut TransactionMut) -> (ItemContent, Option<Self>) {
        let content = match self.clone() {
            CompatiblePyType::YType(y_type) if y_type.is_prelim() => {
                let branch = Branch::new(y_type.type_ref());
                Ok(ItemContent::Type(branch))
            }
            py_value => Any::try_from(py_value).map(|any| ItemContent::Any(vec![any])),
        };

        let content = content.unwrap_or_else(|err| {
            Python::with_gil(|py| err.restore(py));
            ItemContent::Any(vec![])
        });

        let this = if let ItemContent::Type(_) = &content {
            Some(self)
        } else {
            None
        };

        (content, this)
    }

    fn integrate(self, _txn: &mut TransactionMut, _inner_ref: BranchPtr) {
        match self {
            CompatiblePyType::YType(y_type) if y_type.is_prelim() => {
                match y_type {
                    YPyType::Text(_) | YPyType::Array(_) | YPyType::Map(_) => panic!("Trying to integrate ypytype without PyObjectWrapper!"),
                    YPyType::XmlElement(_) | YPyType::XmlText(_) | YPyType::XmlFragment(_) => unreachable!("As defined in Shared::is_prelim(), neither XML type can ever exist outside a YDoc"),
                }
            }
            _ => ()
        }
    }
}

impl<'a> TryFrom<CompatiblePyType<'a>> for Any {
    type Error = PyErr;

    fn try_from(py_type: CompatiblePyType<'a>) -> Result<Self, Self::Error> {
        const MAX_JS_NUMBER: i64 = 2_i64.pow(53) - 1;
        match py_type {
            CompatiblePyType::Bool(b) => Ok(Any::Bool(b.extract()?)),
            CompatiblePyType::String(s) => Ok(Any::String(s.extract::<String>()?.into_boxed_str())),
            CompatiblePyType::Int(i) => {
                let num: i64 = i.extract()?;
                if num > MAX_JS_NUMBER {
                    Ok(Any::BigInt(num))
                } else {
                    Ok(Any::Number(num as f64))
                }
            }
            CompatiblePyType::Float(f) => Ok(Any::Number(f.extract()?)),
            CompatiblePyType::List(l) => {
                let result: PyResult<Vec<Any>> = l
                    .into_iter()
                    .map(|py_any|CompatiblePyType::try_from(py_any)?.try_into())
                    .collect();
                result.map(|res| Any::Array(res.into_boxed_slice()))
            },
            CompatiblePyType::Dict(d) => {
                let result: PyResult<HashMap<String, Any>> = d
                    .iter()
                    .map(|(k, v)| {
                        let key: String = k.extract()?;
                        let value = CompatiblePyType::try_from(v)?.try_into()?;
                        Ok((key, value))
                    })
                    .collect();
                result.map(|res| Any::Map(Box::new(res)))
            },
            CompatiblePyType::None => Ok(Any::Null),
            CompatiblePyType::YType(v) => Err(MultipleIntegrationError::new_err(format!(
                    "Cannot integrate a nested Ypy object because is already integrated into a YDoc: {v}"
                ))),
        }
    }
}

impl<'a> FromPyObject<'a> for YPyType<'a> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        Self::try_from(ob)
    }
}

impl<'a> From<YPyType<'a>> for PyObject {
    fn from(value: YPyType<'a>) -> Self {
        match value {
            YPyType::Text(text) => text.into(),
            YPyType::Array(array) => array.into(),
            YPyType::Map(map) => map.into(),
            YPyType::XmlElement(xml) => xml.into(),
            YPyType::XmlText(xml) => xml.into(),
            YPyType::XmlFragment(xml) => xml.into(),
        }
    }
}

impl<'a> TryFrom<&'a PyAny> for YPyType<'a> {
    type Error = PyErr;

    fn try_from(value: &'a PyAny) -> Result<Self, Self::Error> {
        if let Ok(text) = value.extract() {
            Ok(YPyType::Text(text))
        } else if let Ok(array) = value.extract() {
            Ok(YPyType::Array(array))
        } else if let Ok(map) = value.extract() {
            Ok(YPyType::Map(map))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Could not extract a Ypy type from this object: {value}"
            )))
        }
    }
}

impl ToPython for Any {
    fn into_py(self, py: Python) -> pyo3::PyObject {
        match self {
            Any::Null | Any::Undefined => py.None(),
            Any::Bool(v) => v.into_py(py),
            Any::Number(v) => v.into_py(py),
            Any::BigInt(v) => v.into_py(py),
            Any::String(v) => v.into_py(py),
            Any::Buffer(v) => {
                let byte_array = pytypes::PyByteArray::new(py, v.as_ref());
                byte_array.into()
            }
            Any::Array(v) => {
                let mut a = Vec::new();
                for value in v.iter() {
                    let value = value.to_owned();
                    a.push(value);
                }
                a.into_py(py)
            }
            Any::Map(v) => {
                let mut m = HashMap::new();
                for (k, v) in v.iter() {
                    let value = v.to_owned();
                    m.insert(k, value);
                }
                m.into_py(py)
            }
        }
    }
}

impl WithDocToPython for Value {
    fn with_doc_into_py(self, doc: Rc<RefCell<YDocInner>>, py: Python) -> PyObject {
        match self {
            Value::Any(v) => v.into_py(py),
            Value::YText(v) => v.with_doc(doc).into_py(py),
            Value::YArray(v) => v.with_doc(doc).into_py(py),
            Value::YMap(v) => v.with_doc(doc).into_py(py),
            Value::YXmlElement(v) => v.with_doc(doc).into_py(py),
            Value::YXmlText(v) => v.with_doc(doc).into_py(py),
            Value::YXmlFragment(v) => v.with_doc(doc).into_py(py),
            Value::YDoc(_) => py.None(),
        }
    }
}

pub(crate) fn events_into_py(
    txn: &TransactionMut,
    events: &Events,
    doc: Rc<RefCell<YDocInner>>,
) -> PyObject {
    Python::with_gil(|py| {
        let py_events = events.iter().map(|event| match event {
            yrs::types::Event::Text(e_txt) => YTextEvent::new(e_txt, txn, doc.clone()).into_py(py),
            yrs::types::Event::Array(e_arr) => {
                YArrayEvent::new(e_arr, txn, doc.clone()).into_py(py)
            }
            yrs::types::Event::Map(e_map) => YMapEvent::new(e_map, txn, doc.clone()).into_py(py),
            // TODO: check YXmlFragment Event
            yrs::types::Event::XmlFragment(e_xml) => {
                YXmlEvent::new(e_xml, txn, doc.clone()).into_py(py)
            }
            yrs::types::Event::XmlText(e_xml) => {
                YXmlTextEvent::new(e_xml, txn, doc.clone()).into_py(py)
            }
        });
        PyList::new(py, py_events).into()
    })
}
