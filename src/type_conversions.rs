use lib0::any::Any;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types as pytypes;
use pyo3::types::PyList;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::ops::Deref;
use yrs::block::{ItemContent, Prelim};
use yrs::types::Events;
use yrs::types::{Attrs, Branch, BranchPtr, Change, Delta, EntryChange, Value};
use yrs::{Array, Map, Text, Transaction};

use crate::shared_types::CompatiblePyType;
use crate::shared_types::{SharedType, YPyType};
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

impl<T: ToPython> ToPython for Vec<T> {
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

#[repr(transparent)]
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

#[repr(transparent)]
pub(crate) struct PyObjectWrapper(pub PyObject);

impl From<PyObject> for PyObjectWrapper {
    fn from(value: PyObject) -> Self {
        PyObjectWrapper(value)
    }
}

impl Deref for PyObjectWrapper {
    type Target = PyObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Prelim for PyObjectWrapper {
    fn into_content(self, txn: &mut Transaction) -> (ItemContent, Option<Self>) {
        Python::with_gil(|py| {
            let valid_type: CompatiblePyType = self.0.extract(py).unwrap_or_else(|err| {
                err.restore(py);
                CompatiblePyType::None
            });
            let (item_content, py_any) = valid_type.into_content(txn);
            let wrapper: Option<Self> = py_any.map(|py_type| PyObjectWrapper(py_type.into()));
            (item_content, wrapper)
        })
    }

    fn integrate(self, txn: &mut Transaction, inner_ref: BranchPtr) {
        Python::with_gil(|py| {
            let valid_type: CompatiblePyType = self.0.extract(py).unwrap_or_else(|err| {
                err.restore(py);
                CompatiblePyType::None
            });
            valid_type.integrate(txn, inner_ref);
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
    fn into_content(self, _txn: &mut Transaction) -> (ItemContent, Option<Self>) {
        let content = match self.clone() {
            CompatiblePyType::YType(y_type) if y_type.is_prelim() => {
                let branch = Branch::new(y_type.type_ref(), None);
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

    fn integrate(self, txn: &mut Transaction, inner_ref: BranchPtr) {
        match self {
            CompatiblePyType::YType(y_type) if y_type.is_prelim() => {
                match y_type {
                    YPyType::Text(v) => {
                        let text = Text::from(inner_ref);
                        let mut y_text = v.borrow_mut();

                        if let SharedType::Prelim(v) = y_text.0.to_owned() {
                            text.push(txn, v.as_str());
                        }
                        y_text.0 = SharedType::Integrated(text.clone());
                    }
                    YPyType::Array(v) => {
                        let array = Array::from(inner_ref);
                        let mut y_array = v.borrow_mut();
                        if let SharedType::Prelim(items) = y_array.0.to_owned() {
                            let len = array.len();
                            YArray::insert_multiple_at(&array, txn, len, items).unwrap();
                        }
                        y_array.0 = SharedType::Integrated(array.clone());
                    }
                    YPyType::Map(v) => {
                        let map = Map::from(inner_ref);
                        let mut y_map = v.borrow_mut();
                        Python::with_gil(|py| {
                            if let SharedType::Prelim(ref entries) = y_map.0 {
                                for (k, v) in entries {
                                    let x: CompatiblePyType = v.extract(py).unwrap_or_else(|err| {
                                        err.restore(py);
                                        CompatiblePyType::None
                                    });
                                    map.insert(txn, k.to_owned(), x);
                                }
                            }
                        });
                        
                        y_map.0 = SharedType::Integrated(map.clone());
                    }
                    YPyType::XmlElement(_) | YPyType::XmlText(_) => unreachable!("As defined in Shared::is_prelim(), neither XML type can ever exist outside a YDoc"),
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
            Err(pyo3::exceptions::PyValueError::new_err(
                format!("Could not extract a Ypy type from this object: {value}")
            ))
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
