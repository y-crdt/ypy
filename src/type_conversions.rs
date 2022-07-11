use lib0::any::Any;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types as pytypes;
use pyo3::types::PyList;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Deref;
use yrs::block::{ItemContent, Prelim};
use yrs::types::Events;
use yrs::types::{Attrs, Branch, BranchPtr, Change, Delta, EntryChange, Value};
use yrs::{Array, Map, Text, Transaction};

use crate::shared_types::{Shared, SharedType};
use crate::y_array::YArray;
use crate::y_array::YArrayEvent;
use crate::y_map::YMap;
use crate::y_map::YMapEvent;
use crate::y_text::YText;
use crate::y_text::YTextEvent;
use crate::y_xml::YXmlEvent;
use crate::y_xml::YXmlTextEvent;
use crate::y_xml::{YXmlElement, YXmlText};

create_exception!(y_py, MultipleIntegrationError, PyException, "A Ypy data type instance cannot be integrated into multiple YDocs or the same YDoc multiple times");

pub trait ToPython {
    fn into_py(self, py: Python) -> PyObject;
}

impl ToPython for Vec<Any> {
    fn into_py(self, py: Python) -> PyObject {
        let elements = self.into_iter().map(|v| v.into_py(py));
        let arr: PyObject = pyo3::types::PyList::new(py, elements).into();
        return arr;
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

impl ToPython for Delta {
    fn into_py(self, py: Python) -> PyObject {
        let result = pytypes::PyDict::new(py);
        match self {
            Delta::Inserted(value, attrs) => {
                let value = value.clone().into_py(py);
                result.set_item("insert", value).unwrap();

                if let Some(attrs) = attrs {
                    let attrs = attrs_into_py(attrs.deref());
                    result.set_item("attributes", attrs).unwrap();
                }
            }
            Delta::Retain(len, attrs) => {
                result.set_item("retain", len).unwrap();

                if let Some(attrs) = attrs {
                    let attrs = attrs_into_py(attrs.deref());
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

fn attrs_into_py(attrs: &Attrs) -> PyObject {
    Python::with_gil(|py| {
        let o = pytypes::PyDict::new(py);
        for (key, value) in attrs.iter() {
            let key = key.as_ref();
            let value = Value::Any(value.clone()).into_py(py);
            o.set_item(key, value).unwrap();
        }
        o.into()
    })
}

impl ToPython for &Change {
    fn into_py(self, py: Python) -> PyObject {
        let result = pytypes::PyDict::new(py);
        match self {
            Change::Added(values) => {
                let values: Vec<PyObject> =
                    values.into_iter().map(|v| v.clone().into_py(py)).collect();
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

struct EntryChangeWrapper<'a>(&'a EntryChange);

impl<'a> IntoPy<PyObject> for EntryChangeWrapper<'a> {
    fn into_py(self, py: Python) -> PyObject {
        let result = pytypes::PyDict::new(py);
        let action = "action";
        match self.0 {
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

pub(crate) struct PyObjectWrapper(pub PyObject);

impl Prelim for PyObjectWrapper {
    fn into_content(self, _txn: &mut Transaction) -> (ItemContent, Option<Self>) {
        let content = match py_into_any(self.0.clone()) {
            Ok(any) => ItemContent::Any(vec![any]),
            Err(err) => {
                if Python::with_gil(|py| err.is_instance_of::<MultipleIntegrationError>(py)) {
                    let shared = Shared::try_from(self.0.clone()).unwrap();
                    if shared.is_prelim() {
                        let branch = Branch::new(shared.type_ref(), None);
                        ItemContent::Type(branch)
                    } else {
                        Python::with_gil(|py| err.restore(py));
                        ItemContent::Any(vec![])
                    }
                } else {
                    Python::with_gil(|py| err.restore(py));
                    ItemContent::Any(vec![])
                }
            }
        };

        let this = if let ItemContent::Type(_) = &content {
            Some(self)
        } else {
            None
        };

        (content, this)
    }

    fn integrate(self, txn: &mut Transaction, inner_ref: BranchPtr) {
        if let Ok(shared) = Shared::try_from(self.0) {
            if shared.is_prelim() {
                Python::with_gil(|py| {
                    match shared {
                    Shared::Text(v) => {
                        let text = Text::from(inner_ref);
                        let mut y_text = v.borrow_mut(py);

                        if let SharedType::Prelim(v) = y_text.0.to_owned() {
                            text.push(txn, v.as_str());
                        }
                        y_text.0 = SharedType::Integrated(text.clone());
                    }
                    Shared::Array(v) => {
                        let array = Array::from(inner_ref);
                        let mut y_array = v.borrow_mut(py);
                        if let SharedType::Prelim(items) = y_array.0.to_owned() {
                            let len = array.len();
                            YArray::insert_multiple_at(&array, txn, len, items);
                        }
                        y_array.0 = SharedType::Integrated(array.clone());
                    }
                    Shared::Map(v) => {
                        let map = Map::from(inner_ref);
                        let mut y_map = v.borrow_mut(py);
                        if let SharedType::Prelim(entries) = y_map.0.to_owned() {
                            for (k, v) in entries {
                                map.insert(txn, k, PyValueWrapper(v));
                            }
                        }
                        y_map.0 = SharedType::Integrated(map.clone());
                    }
                    Shared::XmlElement(_) | Shared::XmlText(_) => unreachable!("As defined in Shared::is_prelim(), neither XML type can ever exist outside a YDoc"),
                }
                })
            }
        }
    }
}

const MAX_JS_NUMBER: i64 = 2_i64.pow(53) - 1;
/// Converts a Python object into an integrated Any type.
pub(crate) fn py_into_any(v: PyObject) -> PyResult<Any> {
    Python::with_gil(|py| {
        let v = v.as_ref(py);

        if let Ok(b) = v.downcast::<pytypes::PyBool>() {
            Ok(Any::Bool(b.extract().unwrap()))
        } else if let Ok(l) = v.downcast::<pytypes::PyInt>() {
            let num: i64 = l.extract().unwrap();
            if num > MAX_JS_NUMBER {
                Ok(Any::BigInt(num))
            } else {
                Ok(Any::Number(num as f64))
            }
        } else if v.is_none() {
            Ok(Any::Null)
        } else if let Ok(f) = v.downcast::<pytypes::PyFloat>() {
            Ok(Any::Number(f.extract().unwrap()))
        } else if let Ok(s) = v.downcast::<pytypes::PyString>() {
            let string: String = s.extract().unwrap();
            Ok(Any::String(string.into_boxed_str()))
        } else if let Ok(list) = v.downcast::<pytypes::PyList>() {
            let result: PyResult<Vec<Any>> = list
                .into_iter()
                .map(|py_val| py_into_any(py_val.into()))
                .collect();
            result.map(|res| Any::Array(res.into_boxed_slice()))
        } else if let Ok(dict) = v.downcast::<pytypes::PyDict>() {
            if let Ok(shared) = Shared::extract(v) {
                Err(MultipleIntegrationError::new_err(format!(
                    "Cannot integrate a nested Ypy object because is already integrated into a YDoc: {shared}"
                )))
            } else {
                let result: PyResult<HashMap<String, Any>> = dict
                    .iter()
                    .map(|(k, v)| {
                        let key: String = k.extract()?;
                        let value = py_into_any(v.into())?;
                        Ok((key, value))
                    })
                    .collect();
                result.map(|res| Any::Map(Box::new(res)))
            }
        } else if let Ok(v) = Shared::try_from(PyObject::from(v)) {
            Err(MultipleIntegrationError::new_err(format!(
                "Cannot integrate a nested Ypy object because is already integrated into a YDoc: {v}"
            )))
        } else {
            Err(PyTypeError::new_err(format!(
                "Cannot integrate this type into a YDoc: {v}"
            )))
        }
    })
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

impl ToPython for Value {
    fn into_py(self, py: Python) -> pyo3::PyObject {
        match self {
            Value::Any(v) => v.into_py(py),
            Value::YText(v) => YText::from(v).into_py(py),
            Value::YArray(v) => YArray::from(v).into_py(py),
            Value::YMap(v) => YMap::from(v).into_py(py),
            Value::YXmlElement(v) => YXmlElement(v).into_py(py),
            Value::YXmlText(v) => YXmlText(v).into_py(py),
        }
    }
}

pub(crate) fn events_into_py(txn: &Transaction, events: &Events) -> PyObject {
    Python::with_gil(|py| {
        let py_events = events.iter().map(|event| match event {
            yrs::types::Event::Text(e_txt) => YTextEvent::new(e_txt, txn).into_py(py),
            yrs::types::Event::Array(e_arr) => YArrayEvent::new(e_arr, txn).into_py(py),
            yrs::types::Event::Map(e_map) => YMapEvent::new(e_map, txn).into_py(py),
            yrs::types::Event::XmlElement(e_xml) => YXmlEvent::new(e_xml, txn).into_py(py),
            yrs::types::Event::XmlText(e_xml) => YXmlTextEvent::new(e_xml, txn).into_py(py),
        });
        PyList::new(py, py_events).into()
    })
}

pub struct PyValueWrapper(pub PyObject);

impl Prelim for PyValueWrapper {
    fn into_content(self, _txn: &mut Transaction) -> (ItemContent, Option<Self>) {
        let content = match py_into_any(self.0.clone()) {
            Ok(any) => ItemContent::Any(vec![any]),
            Err(err) => {
                if Python::with_gil(|py| err.is_instance_of::<MultipleIntegrationError>(py)) {
                    let shared = Shared::try_from(self.0.clone()).unwrap();
                    if shared.is_prelim() {
                        let branch = Branch::new(shared.type_ref(), None);
                        ItemContent::Type(branch)
                    } else {
                        Python::with_gil(|py| err.restore(py));
                        ItemContent::Any(vec![])
                    }
                } else {
                    Python::with_gil(|py| err.restore(py));
                    ItemContent::Any(vec![])
                }
            }
        };

        let this = if let ItemContent::Type(_) = &content {
            Some(self)
        } else {
            None
        };

        (content, this)
    }

    fn integrate(self, txn: &mut Transaction, inner_ref: BranchPtr) {
        if let Ok(shared) = Shared::try_from(self.0) {
            if shared.is_prelim() {
                Python::with_gil(|py| {
                    match shared {
                    Shared::Text(v) => {
                        let text = Text::from(inner_ref);
                        let mut y_text = v.borrow_mut(py);

                        if let SharedType::Prelim(v) = y_text.0.to_owned() {
                            text.push(txn, v.as_str());
                        }
                        y_text.0 = SharedType::Integrated(text.clone());
                    }
                    Shared::Array(v) => {
                        let array = Array::from(inner_ref);
                        let mut y_array = v.borrow_mut(py);
                        if let SharedType::Prelim(items) = y_array.0.to_owned() {
                            let len = array.len();
                            YArray::insert_multiple_at(&array, txn, len, items);
                        }
                        y_array.0 = SharedType::Integrated(array.clone());
                    }
                    Shared::Map(v) => {
                        let map = Map::from(inner_ref);
                        let mut y_map = v.borrow_mut(py);

                        if let SharedType::Prelim(entries) = y_map.0.to_owned() {
                            for (k, v) in entries {
                                map.insert(txn, k, PyValueWrapper(v));
                            }
                        }
                        y_map.0 = SharedType::Integrated(map.clone());
                    }
                    Shared::XmlElement(_) | Shared::XmlText(_) => unreachable!("As defined in Shared::is_prelim(), neither XML type can ever exist outside a YDoc"),
                }
                })
            }
        }
    }
}
