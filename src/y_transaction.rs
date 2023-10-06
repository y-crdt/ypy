use pyo3::exceptions::{PyAssertionError, PyException};
use pyo3::types::PyBytes;
use pyo3::{create_exception, prelude::*};
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::{Encode, Encoder};
use yrs::{
    updates::{decoder::DecoderV1, encoder::EncoderV1},
    StateVector, Update,
};
use yrs::{ReadTxn, TransactionMut};

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
pub struct YTransactionInner {
    pub inner: ManuallyDrop<TransactionMut<'static>>,
    pub cached_before_state: Option<PyObject>,
    pub committed: bool,
}

impl ReadTxn for YTransactionInner {
    fn store(&self) -> &yrs::Store {
        self.deref().store()
    }
}

impl Deref for YTransactionInner {
    type Target = TransactionMut<'static>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for YTransactionInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Drop for YTransactionInner {
    fn drop(&mut self) {
        if !self.committed {
            self.commit();
        }
    }
}

impl YTransactionInner {
    pub fn new(txn: TransactionMut<'static>) -> Self {
        YTransactionInner {
            inner: ManuallyDrop::new(txn),
            cached_before_state: None,
            committed: false,
        }
    }
}

impl YTransactionInner {
    pub fn before_state(&mut self) -> PyObject {
        if self.cached_before_state.is_none() {
            let before_state = Python::with_gil(|py| {
                let txn = (*self).deref();
                let state_map: HashMap<u64, u32> =
                    txn.before_state().iter().map(|(x, y)| (*x, *y)).collect();
                state_map.into_py(py)
            });
            self.cached_before_state = Some(before_state);
        }
        return self.cached_before_state.as_ref().unwrap().clone();
    }

    /// Triggers a post-update series of operations without `free`ing the transaction. This includes
    /// compaction and optimization of internal representation of updates, triggering events etc.
    /// Ypy transactions are auto-committed when they are `free`d.
    pub fn commit(&mut self) {
        if !self.committed {
            self.deref_mut().commit();
            self.committed = true;
            unsafe { ManuallyDrop::drop(&mut self.inner) }
        } else {
            panic!("Transaction already committed!");
        }
    }
}

#[pyclass(unsendable)]
pub struct YTransaction {
    inner: Rc<RefCell<YTransactionInner>>,
    committed: bool,
}

impl YTransaction {
    pub fn new(txn: Rc<RefCell<YTransactionInner>>) -> Self {
        YTransaction {
            inner: txn.clone(),
            committed: txn.borrow().committed,
        }
    }
    pub fn get_inner(&self) -> Rc<RefCell<YTransactionInner>> {
        self.inner.clone()
    }

    fn raise_alread_committed(&self) -> PyErr {
        PyAssertionError::new_err("Transaction already committed!")
    }

    pub fn transact<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&mut YTransactionInner) -> R,
    {
        let inner = self.get_inner();
        let mut txn = inner.borrow_mut();
        if txn.committed {
            Err(self.raise_alread_committed())
        } else {
            Ok(f(&mut txn))
        }
    }
}

#[pymethods]
impl YTransaction {
    #[getter]
    pub fn before_state(&mut self) -> PyObject {
        self.get_inner().borrow_mut().before_state()
    }

    pub fn commit(&mut self) -> PyResult<()> {
        if !self.committed {
            self.get_inner().borrow_mut().commit();
            self.committed = true;
            Ok(())
        } else {
            Err(self.raise_alread_committed())
        }
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
        let sv = self.get_inner().borrow().state_vector();
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
        self.get_inner().borrow_mut().encode_diff(&sv, &mut encoder);
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
        self.get_inner().borrow_mut().apply_update(update);
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
        self.commit()?;
        Ok(exception_type.is_none())
    }
}
