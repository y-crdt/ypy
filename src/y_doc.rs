use crate::y_array::YArray;
use crate::y_map::YMap;
use crate::y_text::YText;
use crate::y_transaction::YTransaction;
use crate::y_xml::YXmlElement;
use crate::y_xml::YXmlText;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::types::PyTuple;
use yrs::updates::encoder::Encode;
use yrs::AfterTransactionEvent as YrsAfterTransactionEvent;
use yrs::Doc;
use yrs::OffsetKind;
use yrs::Options;
use yrs::SubscriptionId;
use yrs::Transaction;

/// A Ypy document type. Documents are most important units of collaborative resources management.
/// All shared collections live within a scope of their corresponding documents. All updates are
/// generated on per document basis (rather than individual shared type). All operations on shared
/// collections happen via [YTransaction], which lifetime is also bound to a document.
///
/// Document manages so called root types, which are top-level shared types definitions (as opposed
/// to recursively nested types).
///
/// A basic workflow sample:
///
/// ```python
/// from y_py import YDoc
///
/// doc = YDoc()
/// with doc.begin_transaction() as txn:
///     text = txn.get_text('name')
///     text.push(txn, 'hello world')
///     output = text.to_string(txn)
///     print(output)
/// ```
#[pyclass(unsendable, subclass)]
pub struct YDoc(pub Doc);

#[pymethods]
impl YDoc {
    /// Creates a new Ypy document. If `client_id` parameter was passed it will be used as this
    /// document globally unique identifier (it's up to caller to ensure that requirement).
    /// Otherwise it will be assigned a randomly generated number.
    #[new]
    pub fn new(
        client_id: Option<u64>,
        offset_kind: Option<String>,
        skip_gc: Option<bool>,
    ) -> PyResult<Self> {
        let mut options = Options::default();
        if let Some(client_id) = client_id {
            options.client_id = client_id;
        }

        if let Some(raw_offset) = offset_kind {
            let clean_offset = raw_offset.to_lowercase().replace("-", "");
            let offset = match clean_offset.as_str() {
                "utf8" => Ok(OffsetKind::Bytes),
                "utf16" => Ok(OffsetKind::Utf16),
                "utf32" => Ok(OffsetKind::Utf32),
                _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "'{}' is not a valid offset kind (utf8, utf16, or utf32).",
                    clean_offset
                ))),
            }?;
            options.offset_kind = offset;
        }

        if let Some(skip_gc) = skip_gc {
            options.skip_gc = skip_gc;
        }

        Ok(YDoc(Doc::with_options(options)))
    }

    /// Gets globally unique identifier of this `YDoc` instance.
    #[getter]
    pub fn client_id(&self) -> u64 {
        self.0.client_id as u64
    }

    /// Returns a new transaction for this document. Ypy shared data types execute their
    /// operations in a context of a given transaction. Each document can have only one active
    /// transaction at the time - subsequent attempts will cause exception to be thrown.
    ///
    /// Transactions started with `doc.begin_transaction` can be released by deleting the transaction object
    /// method.
    ///
    /// Example:
    ///
    /// ```python
    /// from y_py import YDoc
    /// doc = YDoc()
    /// text = doc.get_text('name')
    /// with doc.begin_transaction() as txn:
    ///     text.insert(txn, 0, 'hello world')
    /// ```
    pub fn begin_transaction(&self) -> YTransaction {
        YTransaction::new(self.0.transact())
    }

    pub fn transact(&mut self, callback: PyObject) -> PyResult<PyObject> {
        let txn = self.begin_transaction();
        Python::with_gil(|py| {
            let args = PyTuple::new(py, std::iter::once(txn.into_py(py)));
            callback.call(py, args, None)
        })
    }

    /// Returns a `YMap` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YMap` instance.
    pub fn get_map(&mut self, name: &str) -> YMap {
        self.begin_transaction().get_map(name)
    }

    /// Returns a `YXmlElement` shared data type, that's accessible for subsequent accesses using
    /// given `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YXmlElement` instance.
    pub fn get_xml_element(&mut self, name: &str) -> YXmlElement {
        YXmlElement(self.begin_transaction().get_xml_element(name))
    }

    /// Returns a `YXmlText` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YXmlText` instance.
    pub fn get_xml_text(&mut self, name: &str) -> YXmlText {
        YXmlText(self.begin_transaction().get_xml_text(name))
    }

    /// Returns a `YArray` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YArray` instance.
    pub fn get_array(&mut self, name: &str) -> YArray {
        self.begin_transaction().get_array(name)
    }

    /// Returns a `YText` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YText` instance.
    pub fn get_text(&mut self, name: &str) -> YText {
        self.begin_transaction().get_text(name)
    }

    /// Subscribes a callback to a `YDoc` lifecycle event.
    pub fn observe_after_transaction(&mut self, callback: PyObject) -> SubscriptionId {
        self.0
            .observe_transaction_cleanup(move |txn, event| {
                Python::with_gil(|py| {
                    let event = AfterTransactionEvent::new(event, txn);
                    if let Err(err) = callback.call1(py, (event,)) {
                        err.restore(py)
                    }
                })
            })
            .into()
    }
}

/// Encodes a state vector of a given Ypy document into its binary representation using lib0 v1
/// encoding. State vector is a compact representation of updates performed on a given document and
/// can be used by `encode_state_as_update` on remote peer to generate a delta update payload to
/// synchronize changes between peers.
///
/// Example:
///
/// ```python
/// from y_py import YDoc, encode_state_vector, encode_state_as_update, apply_update from y_py
///
/// # document on machine A
/// local_doc = YDoc()
/// local_sv = encode_state_vector(local_doc)
///
/// # document on machine B
/// remote_doc = YDoc()
/// remote_delta = encode_state_as_update(remote_doc, local_sv)
///
/// apply_update(local_doc, remote_delta)
/// ```
#[pyfunction]
pub fn encode_state_vector(doc: &mut YDoc) -> PyObject {
    doc.begin_transaction().state_vector_v1()
}

/// Encodes all updates that have happened since a given version `vector` into a compact delta
/// representation using lib0 v1 encoding. If `vector` parameter has not been provided, generated
/// delta payload will contain all changes of a current Ypy document, working effectively as its
/// state snapshot.
///
/// Example:
///
/// ```python
/// from y_py import YDoc, encode_state_vector, encode_state_as_update, apply_update
///
/// # document on machine A
/// local_doc = YDoc()
/// local_sv = encode_state_vector(local_doc)
///
/// # document on machine B
/// remote_doc = YDoc()
/// remote_delta = encode_state_as_update(remote_doc, local_sv)
///
/// apply_update(local_doc, remote_delta)
/// ```
#[pyfunction]
pub fn encode_state_as_update(doc: &YDoc, vector: Option<Vec<u8>>) -> PyResult<PyObject> {
    doc.begin_transaction().diff_v1(vector)
}

/// Applies delta update generated by the remote document replica to a current document. This
/// method assumes that a payload maintains lib0 v1 encoding format.
///
/// Example:
///
/// ```python
/// from y_py import YDoc, encode_state_vector, encode_state_as_update, apply_update
///
/// # document on machine A
/// local_doc = YDoc()
/// local_sv = encode_state_vector(local_doc)
///
/// # document on machine B
/// remote_doc = YDoc()
/// remote_delta = encode_state_as_update(remote_doc, local_sv)
///
/// apply_update(local_doc, remote_delta)
/// ```
#[pyfunction]
pub fn apply_update(doc: &mut YDoc, diff: Vec<u8>) -> PyResult<()> {
    doc.begin_transaction().apply_v1(diff)?;
    Ok(())
}

#[pyclass(unsendable)]
pub struct AfterTransactionEvent {
    inner: *const YrsAfterTransactionEvent,
    txn: *const Transaction,
    before_state: Option<PyObject>,
    after_state: Option<PyObject>,
    delete_set: Option<PyObject>,
}

impl AfterTransactionEvent {
    fn new(event: &YrsAfterTransactionEvent, txn: &Transaction) -> Self {
        let inner = event as *const YrsAfterTransactionEvent;
        let txn = txn as *const Transaction;
        AfterTransactionEvent {
            inner,
            txn,
            before_state: None,
            after_state: None,
            delete_set: None,
        }
    }

    fn inner(&self) -> &YrsAfterTransactionEvent {
        unsafe { self.inner.as_ref().unwrap() }
    }

    fn txn(&self) -> &Transaction {
        unsafe { self.txn.as_ref().unwrap() }
    }
}

#[pymethods]
impl AfterTransactionEvent {
    /// Returns a current shared type instance, that current event changes refer to.
    #[getter]
    pub fn before_state(&mut self) -> PyObject {
        if let Some(before_state) = self.before_state.as_ref() {
            before_state.clone()
        } else {
            let before_state = self.inner().before_state.encode_v1();
            let before_state: PyObject =
                Python::with_gil(|py| PyBytes::new(py, &before_state).into());
            self.before_state = Some(before_state.clone());
            before_state
        }
    }

    #[getter]
    pub fn after_state(&mut self) -> PyObject {
        if let Some(after_state) = self.after_state.as_ref() {
            after_state.clone()
        } else {
            let after_state = self.inner().after_state.encode_v1();
            let after_state: PyObject =
                Python::with_gil(|py| PyBytes::new(py, &after_state).into());
            self.after_state = Some(after_state.clone());
            after_state
        }
    }

    #[getter]
    pub fn delete_set(&mut self) -> PyObject {
        if let Some(delete_set) = self.delete_set.as_ref() {
            delete_set.clone()
        } else {
            let delete_set = self.inner().delete_set.encode_v1();
            let delete_set: PyObject = Python::with_gil(|py| PyBytes::new(py, &delete_set).into());
            self.delete_set = Some(delete_set.clone());
            delete_set
        }
    }

    pub fn get_update(&self) -> PyObject {
        let update = self.txn().encode_update_v1();
        Python::with_gil(|py| PyBytes::new(py, &update).into())
    }
}
