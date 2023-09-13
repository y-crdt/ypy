use crate::{y_array::YArray, y_map::YMap, y_text::YText};
use pyo3::exceptions::PyException;
use pyo3::types::PyBytes;
use pyo3::{create_exception, prelude::*};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::{Encode, Encoder};
use yrs::{
    updates::{decoder::DecoderV1, encoder::EncoderV1},
    StateVector, Transaction, Update,
};

create_exception!(
    y_py,
    EncodingException,
    PyException,
    "Occurs due to issues in the encoding/decoding process of y_py updates."
);

/// A transaction that serves as a proxy to document block store. Ypy shared data types execute
/// their operations in a context of a given transaction. Each document can have only one active
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
#[pyclass(unsendable)]
pub struct YTransaction {
    pub inner: Transaction,
    pub cached_before_state: Option<PyObject>,
}

impl Deref for YTransaction {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for YTransaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl YTransaction {
    pub fn new(txn: Transaction) -> Self {
        YTransaction {
            inner: txn,
            cached_before_state: None,
        }
    }
}

#[pymethods]
impl YTransaction {
    #[getter]
    pub fn before_state(&mut self) -> PyObject {
        if self.cached_before_state.is_none() {
            let before_state = Python::with_gil(|py| {
                let state_map: HashMap<u64, u32> =
                    self.before_state.iter().map(|(x, y)| (*x, *y)).collect();
                state_map.into_py(py)
            });
            self.cached_before_state = Some(before_state);
        }
        return self.cached_before_state.as_ref().unwrap().clone();
    }

    /// Returns a `YText` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YText` instance.
    pub fn get_text(&mut self, name: &str) -> YText {
        self.deref_mut().get_text(name).into()
    }

    /// Returns a `YArray` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YArray` instance.
    pub fn get_array(&mut self, name: &str) -> YArray {
        self.deref_mut().get_array(name).into()
    }

    /// Returns a `YMap` shared data type, that's accessible for subsequent accesses using given
    /// `name`.
    ///
    /// If there was no instance with this name before, it will be created and then returned.
    ///
    /// If there was an instance with this name, but it was of different type, it will be projected
    /// onto `YMap` instance.
    pub fn get_map(&mut self, name: &str) -> YMap {
        self.deref_mut().get_map(name).into()
    }

    /// Triggers a post-update series of operations without `free`ing the transaction. This includes
    /// compaction and optimization of internal representation of updates, triggering events etc.
    /// Ypy transactions are auto-committed when they are `free`d.
    pub fn commit(&mut self) {
        self.deref_mut().commit()
    }

    /// Encodes a state vector of a given transaction document into its binary representation using
    /// lib0 v1 encoding. State vector is a compact representation of updates performed on a given
    /// document and can be used by `encode_state_as_update` on remote peer to generate a delta
    /// update payload to synchronize changes between peers.
    ///
    /// Example:
    ///
    /// ```python
    /// from y_py import YDoc
    ///
    /// # document on machine A
    /// local_doc = YDoc()
    /// local_txn = local_doc.begin_transaction()
    ///
    /// # document on machine B
    /// remote_doc = YDoc()
    /// remote_txn = local_doc.begin_transaction()
    ///
    /// try:
    ///     local_sv = local_txn.state_vector_v1()
    ///     remote_delta = remote_txn.diff_v1(local_sv)
    ///     local_txn.applyV1(remote_delta)
    /// finally:
    ///     del local_txn
    ///     del remote_txn
    ///
    /// ```
    pub fn state_vector_v1(&self) -> PyObject {
        let sv = self.state_vector();
        let payload = sv.encode_v1();
        Python::with_gil(|py| PyBytes::new(py, &payload).into())
    }

    /// Encodes all updates that have happened since a given version `vector` into a compact delta
    /// representation using lib0 v1 encoding. If `vector` parameter has not been provided, generated
    /// delta payload will contain all changes of a current Ypy document, working effectively as
    /// its state snapshot.
    ///
    /// Example:
    ///
    /// ```python
    /// from y_py import YDoc
    ///
    /// # document on machine A
    /// local_doc = YDoc()
    /// local_txn = local_doc.begin_transaction()
    ///
    /// # document on machine B
    /// remote_doc = YDoc()
    /// remote_txn = local_doc.begin_transaction()
    ///
    /// try:
    ///     local_sv = local_txn.state_vector_v1()
    ///     remote_delta = remote_txn.diff_v1(local_sv)
    ///     local_txn.applyV1(remote_delta)
    /// finally:
    ///     del local_txn
    ///     del remote_txn
    /// ```
    pub fn diff_v1(&self, vector: Option<Vec<u8>>) -> PyResult<PyObject> {
        let mut encoder = EncoderV1::new();
        let sv = if let Some(vector) = vector {
            StateVector::decode_v1(vector.to_vec().as_slice())
                .map_err(|e| EncodingException::new_err(e.to_string()))?
        } else {
            StateVector::default()
        };
        self.encode_diff(&sv, &mut encoder);
        let bytes: PyObject = Python::with_gil(|py| PyBytes::new(py, &encoder.to_vec()).into());
        Ok(bytes)
    }

    /// Applies delta update generated by the remote document replica to a current transaction's
    /// document. This method assumes that a payload maintains lib0 v1 encoding format.
    ///
    /// Example:
    ///
    /// ```python
    /// from y_py import YDoc
    ///
    /// # document on machine A
    /// local_doc = YDoc()
    /// local_txn = local_doc.begin_transaction()
    ///
    /// # document on machine B
    /// remote_doc = YDoc()
    /// remote_txn = local_doc.begin_transaction()
    ///
    /// try:
    ///     local_sv = local_txn.state_vector_v1()
    ///     remote_delta = remote_txn.diff_v1(local_sv)
    ///     local_txn.applyV1(remote_delta)
    /// finally:
    ///     del local_txn
    ///     del remote_txn
    /// ```
    pub fn apply_v1(&mut self, diff: Vec<u8>) -> PyResult<()> {
        let diff: Vec<u8> = diff.to_vec();
        let mut decoder = DecoderV1::from(diff.as_slice());
        let update =
            Update::decode(&mut decoder).map_err(|e| EncodingException::new_err(e.to_string()))?;
        self.apply_update(update);
        Ok(())
    }

    /// Allows YTransaction to be used with a Python context block.
    ///
    /// Example
    /// ```python
    /// from y_py import YDoc
    ///
    /// doc = YDoc()
    ///
    /// with doc.begin_transaction() as txn:
    ///     # Perform updates within this block
    ///
    /// ```
    fn __enter__<'p>(slf: PyRef<'p, Self>, _py: Python<'p>) -> PyResult<PyRef<'p, Self>> {
        Ok(slf)
    }

    /// Allows YTransaction to be used with a Python context block.
    /// Commits the results when the `with` context closes.
    ///
    /// Example
    /// ```python
    /// from y_py import YDoc
    ///
    /// doc = YDoc()
    ///
    /// with doc.begin_transaction() as txn:
    ///     # Updates
    /// # Commit is called here when the context exits
    ///
    /// ```
    fn __exit__<'p>(
        &'p mut self,
        exception_type: Option<&'p PyAny>,
        _exception_value: Option<&'p PyAny>,
        _traceback: Option<&'p PyAny>,
    ) -> PyResult<bool> {
        self.commit();
        Ok(exception_type.is_none())
    }
}
