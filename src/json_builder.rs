use std::{collections::HashMap, convert::TryFrom};

use lib0::any::Any;
use pyo3::{exceptions::PyTypeError, PyErr, PyObject, PyResult, Python};

use crate::shared_types::{CompatiblePyType, YPyType};

#[derive(Clone, Debug)]
pub(crate) struct JsonBuilder(String);

impl JsonBuilder {
    pub fn new() -> Self {
        JsonBuilder(String::new())
    }

    pub fn append_json<T: JsonBuildable>(&mut self, buildable: &T) -> Result<(), T::JsonError> {
        let buffer = &mut self.0;
        buildable.build_json(buffer)
    }
}

impl From<JsonBuilder> for String {
    fn from(json_builder: JsonBuilder) -> Self {
        json_builder.0
    }
}

pub(crate) trait JsonBuildable {
    type JsonError;
    fn build_json(&self, buffer: &mut String) -> Result<(), Self::JsonError>;
}

impl<'a> JsonBuildable for CompatiblePyType<'a> {
    type JsonError = PyErr;

    fn build_json(&self, buffer: &mut String) -> Result<(), Self::JsonError> {
        match self {
            CompatiblePyType::Bool(b) => {
                let t: bool = b.extract().unwrap();
                buffer.push_str(if t { "true" } else { "false" });
            }
            CompatiblePyType::Int(i) => buffer.push_str(&i.to_string()),
            CompatiblePyType::Float(f) => buffer.push_str(&f.to_string()),
            CompatiblePyType::String(s) => {
                let string: String = s.extract().unwrap();
                buffer.reserve(string.len() + 2);
                buffer.push_str("\"");
                buffer.push_str(&string);
                buffer.push_str("\"");
            }
            CompatiblePyType::List(list) => {
                buffer.push_str("[");
                let length = list.len();
                for (i, element) in list.iter().enumerate() {
                    CompatiblePyType::try_from(element)?.build_json(buffer)?;
                    if i + 1 < length {
                        buffer.push_str(",");
                    }
                }

                buffer.push_str("]");
            }
            CompatiblePyType::Dict(dict) => {
                buffer.push_str("{");
                let length = dict.len();
                for (i, (k, v)) in dict.iter().enumerate() {
                    CompatiblePyType::try_from(k)?.build_json(buffer)?;
                    buffer.push_str(":");
                    CompatiblePyType::try_from(v)?.build_json(buffer)?;
                    if i + 1 < length {
                        buffer.push_str(",");
                    }
                }
                buffer.push_str("}");
            }
            CompatiblePyType::YType(y_type) => y_type.build_json(buffer)?,
            CompatiblePyType::None => buffer.push_str("null"),
        }

        Ok(())
    }
}

impl<'a> JsonBuildable for YPyType<'a> {
    type JsonError = PyErr;

    fn build_json(&self, buffer: &mut String) -> Result<(), Self::JsonError> {
        let json = match self {
            YPyType::Text(text) => Ok(text.borrow().to_json()),
            YPyType::Array(array) => array.borrow().to_json(),
            YPyType::Map(map) => map.borrow().to_json(),
            xml => Err(PyTypeError::new_err(format!(
                "XML elements cannot be converted to a JSON format: {xml}"
            ))),
        };
        buffer.push_str(&json?);
        Ok(())
    }
}

impl JsonBuildable for Any {
    type JsonError = PyErr;
    fn build_json(&self, buffer: &mut String) -> Result<(), Self::JsonError> {
        self.to_json(buffer);
        Ok(())
    }
}

impl JsonBuildable for HashMap<String, PyObject> {
    type JsonError = PyErr;

    fn build_json(&self, buffer: &mut String) -> Result<(), Self::JsonError> {
        buffer.push_str("{");
        let res: PyResult<()> = Python::with_gil(|py| {
            for (i, (k, py_obj)) in self.iter().enumerate() {
                let value: CompatiblePyType = py_obj.extract(py)?;
                if i != 0 {
                    buffer.push_str(",");
                }
                buffer.push_str(k);
                buffer.push_str(":");
                value.build_json(buffer)?;
            }
            Ok(())
        });
        res?;

        buffer.push_str("}");
        Ok(())
    }
}

impl JsonBuildable for Vec<PyObject> {
    type JsonError = PyErr;

    fn build_json(&self, buffer: &mut String) -> Result<(), Self::JsonError> {
        buffer.push_str("[");
        let res: PyResult<()> = Python::with_gil(|py| {
            self.iter().enumerate().try_for_each(|(i, object)| {
                let py_type: CompatiblePyType = object.extract(py)?;
                if i != 0 {
                    buffer.push_str(",");
                }
                py_type.build_json(buffer)?;
                Ok(())
            })
        });
        res?;
        buffer.push_str("]");
        Ok(())
    }
}
