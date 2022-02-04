use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
mod shared_types;
mod type_conversions;
mod y_array;
mod y_doc;
mod y_map;
mod y_text;
mod y_transaction;
mod y_xml;
use crate::y_doc::*;

/// Python bindings for Y.rs
#[pymodule]
pub fn y_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<y_doc::YDoc>()?;
    m.add_class::<y_text::YText>()?;
    m.add_class::<y_array::YArray>()?;
    m.add_class::<y_map::YMap>()?;
    m.add_class::<y_xml::YXmlText>()?;
    m.add_class::<y_xml::YXmlElement>()?;
    m.add_wrapped(wrap_pyfunction!(encode_state_vector))?;
    m.add_wrapped(wrap_pyfunction!(encode_state_as_update))?;
    m.add_wrapped(wrap_pyfunction!(apply_update))?;
    Ok(())
}
