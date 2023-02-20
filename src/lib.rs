use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
mod json_builder;
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
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Data Types
    m.add_class::<y_doc::YDoc>()?;
    m.add_class::<y_transaction::YTransaction>()?;
    m.add_class::<y_text::YText>()?;
    m.add_class::<y_array::YArray>()?;
    m.add_class::<y_map::YMap>()?;
    m.add_class::<y_xml::YXmlText>()?;
    m.add_class::<y_xml::YXmlElement>()?;
    // Events
    m.add_class::<y_text::YTextEvent>()?;
    m.add_class::<y_array::YArrayEvent>()?;
    m.add_class::<y_map::YMapEvent>()?;
    m.add_class::<y_xml::YXmlTextEvent>()?;
    m.add_class::<y_xml::YXmlEvent>()?;
    m.add_class::<y_doc::AfterTransactionEvent>()?;
    // Functions
    m.add_wrapped(wrap_pyfunction!(encode_state_vector))?;
    m.add_wrapped(wrap_pyfunction!(encode_state_as_update))?;
    m.add_wrapped(wrap_pyfunction!(apply_update))?;
    Ok(())
}
