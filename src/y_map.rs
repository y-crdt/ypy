use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

use std::mem::ManuallyDrop;
use std::ops::DerefMut;
use yrs::types::map::{MapEvent, MapIter};
use yrs::types::DeepObservable;
use yrs::{Map, SubscriptionId, Transaction};

use crate::json_builder::JsonBuilder;
use crate::shared_types::{
    DeepSubscription, DefaultPyErr, PreliminaryObservationException, ShallowSubscription,
    SharedType, SubId,
};
use crate::type_conversions::{events_into_py, PyObjectWrapper, ToPython};
use crate::y_transaction::YTransaction;

/// Collection used to store key-value entries in an unordered manner. Keys are always represented
/// as UTF-8 strings. Values can be any value type supported by Yrs: JSON-like primitives as well as
/// shared data types.
///
/// In terms of conflict resolution, [Map] uses logical last-write-wins principle, meaning the past
/// updates are automatically overridden and discarded by newer ones, while concurrent updates made
/// by different peers are resolved into a single value using document id seniority to establish
/// order.
#[pyclass(unsendable)]
pub struct YMap(pub SharedType<Map, HashMap<String, PyObject>>);

impl From<Map> for YMap {
    fn from(v: Map) -> Self {
        YMap(SharedType::new(v))
    }
}

#[pymethods]
impl YMap {
    /// Creates a new preliminary instance of a `YMap` shared data type, with its state
    /// initialized to provided parameter.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[new]
    pub fn new(dict: &PyDict) -> PyResult<Self> {
        let mut map: HashMap<String, PyObject> = HashMap::new();
        for (k, v) in dict.iter() {
            let k = k.downcast::<pyo3::types::PyString>()?.to_string();
            let v: PyObject = v.into();
            map.insert(k, v);
        }
        Ok(YMap(SharedType::Prelim(map)))
    }

    /// Returns true if this is a preliminary instance of `YMap`.
    ///
    /// Preliminary instances can be nested into other shared data types such as `YArray` and `YMap`.
    /// Once a preliminary instance has been inserted this way, it becomes integrated into Ypy
    /// document store and cannot be nested again: attempt to do so will result in an exception.
    #[getter]
    pub fn prelim(&self) -> bool {
        match &self.0 {
            SharedType::Prelim(_) => true,
            _ => false,
        }
    }

    /// Returns a number of entries stored within this instance of `YMap`.
    pub fn __len__(&self) -> usize {
        match &self.0 {
            SharedType::Integrated(v) => v.len() as usize,
            SharedType::Prelim(v) => v.len() as usize,
        }
    }

    pub fn __str__(&self) -> String {
        Python::with_gil(|py| match &self.0 {
            SharedType::Integrated(y_map) => y_map.to_json().into_py(py).to_string(),
            SharedType::Prelim(contents) => contents.clone().into_py(py).to_string(),
        })
    }

    pub fn __dict__(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| match &self.0 {
            SharedType::Integrated(v) => Ok(v.to_json().into_py(py)),
            SharedType::Prelim(v) => {
                let dict = PyDict::new(py);
                for (k, v) in v.iter() {
                    dict.set_item(k, v)?;
                }
                Ok(dict.into())
            }
        })
    }

    pub fn __repr__(&self) -> String {
        format!("YMap({})", self.__str__())
    }

    /// Converts contents of this `YMap` instance into a JSON representation.
    pub fn to_json(&self) -> PyResult<String> {
        let mut json_builder = JsonBuilder::new();
        match &self.0 {
            SharedType::Integrated(dict) => json_builder.append_json(&dict.to_json())?,
            SharedType::Prelim(dict) => json_builder.append_json(dict)?,
        }
        Ok(json_builder.into())
    }

    /// Sets a given `key`-`value` entry within this instance of `YMap`. If another entry was
    /// already stored under given `key`, it will be overridden with new `value`.
    pub fn set(&mut self, txn: &mut YTransaction, key: &str, value: PyObject) {
        match &mut self.0 {
            SharedType::Integrated(v) => {
                v.insert(txn, key.to_string(), PyObjectWrapper(value));
            }
            SharedType::Prelim(v) => {
                v.insert(key.to_string(), value);
            }
        }
    }
    /// Updates `YMap` with the key value pairs in the `items` object.
    pub fn update(&mut self, txn: &mut YTransaction, items: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            // Handle collection types
            if let Ok(dict) = items.extract::<HashMap<String, PyObject>>(py) {
                dict.into_iter().for_each(|(k, v)| self.set(txn, &k, v));
                return Ok(());
            }
            // Handle iterable of tuples
            match items.as_ref(py).iter() {
                Ok(iterable) => {
                    for value in iterable {
                        match value {
                            Ok(kv_pair) => {
                                if let Ok((key, value)) = kv_pair.extract::<(String, PyObject)>() {
                                    self.set(txn, &key, value);
                                } else {
                                    return Err(PyTypeError::new_err(format!("Update items should be formatted as (str, value) tuples, found: {}", kv_pair)));
                                }
                            }
                            Err(err) => return Err(err),
                        }
                    }
                    Ok(())
                }
                Err(err) => Err(err),
            }
        })
    }

    /// Removes an entry identified by a given `key` from this instance of `YMap`, if such exists.
    pub fn pop(
        &mut self,
        txn: &mut YTransaction,
        key: &str,
        fallback: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let popped = match &mut self.0 {
            SharedType::Integrated(v) => v
                .remove(txn, key)
                .map(|v| Python::with_gil(|py| v.into_py(py))),
            SharedType::Prelim(v) => v.remove(key),
        };
        if let Some(value) = popped {
            Ok(value)
        } else if let Some(fallback) = fallback {
            Ok(fallback)
        } else {
            Err(PyKeyError::new_err(format!("{key}")))
        }
    }

    /// Retrieves an item from the map. If the item isn't found, the fallback value is returned.
    pub fn get(&self, key: &str, fallback: Option<PyObject>) -> PyObject {
        self.__getitem__(key)
            .ok()
            .unwrap_or_else(|| fallback.unwrap_or_else(|| Python::with_gil(|py| py.None())))
    }

    /// Returns value of an entry stored under given `key` within this instance of `YMap`,
    /// or `undefined` if no such entry existed.
    pub fn __getitem__(&self, key: &str) -> PyResult<PyObject> {
        let entry = match &self.0 {
            SharedType::Integrated(y_map) => y_map
                .get(key)
                .map(|value| Python::with_gil(|py| value.into_py(py))),
            SharedType::Prelim(hash_map) => hash_map.get(key).map(|value| value.clone()),
        };

        entry.ok_or_else(|| PyKeyError::new_err(format!("{key}")))
    }

    /// Returns an item view that can be used to traverse over all entries stored within this
    /// instance of `YMap`. Order of entry is not specified.
    ///
    /// Example:
    ///
    /// ```python
    /// from y_py import YDoc
    ///
    /// # document on machine A
    /// doc = YDoc()
    /// map = doc.get_map('name')
    /// with doc.begin_transaction() as txn:
    ///     map.set(txn, 'key1', 'value1')
    ///     map.set(txn, 'key2', true)
    ///     for (key, value) in map.entries(txn)):
    ///         print(key, value)
    /// ```
    pub fn items(&self) -> ItemView {
        ItemView(&self.0)
    }

    pub fn keys(&self) -> KeyView {
        let inner: *const _ = &self.0;
        KeyView(inner)
    }

    pub fn __iter__(&self) -> KeyIterator {
        let inner: *const _ = &self.0;
        KeyIterator(YMapIterator::from(inner))
    }

    pub fn values(&self) -> ValueView {
        let inner: *const _ = &self.0;
        ValueView(inner)
    }

    pub fn observe(&mut self, f: PyObject) -> PyResult<ShallowSubscription> {
        match &mut self.0 {
            SharedType::Integrated(v) => {
                let sub_id: SubscriptionId = v
                    .observe(move |txn, e| {
                        Python::with_gil(|py| {
                            let e = YMapEvent::new(e, txn);
                            if let Err(err) = f.call1(py, (e,)) {
                                err.restore(py)
                            }
                        })
                    })
                    .into();
                Ok(ShallowSubscription(sub_id))
            }
            SharedType::Prelim(_) => Err(PreliminaryObservationException::default_message()),
        }
    }

    pub fn observe_deep(&mut self, f: PyObject) -> PyResult<DeepSubscription> {
        match &mut self.0 {
            SharedType::Integrated(map) => {
                let sub: SubscriptionId = map
                    .observe_deep(move |txn, events| {
                        Python::with_gil(|py| {
                            let events = events_into_py(txn, events);
                            if let Err(err) = f.call1(py, (events,)) {
                                err.restore(py)
                            }
                        })
                    })
                    .into();
                Ok(DeepSubscription(sub))
            }
            SharedType::Prelim(_) => Err(PreliminaryObservationException::default_message()),
        }
    }
    /// Cancels the observer callback associated with the `subscripton_id`.
    pub fn unobserve(&mut self, subscription_id: SubId) -> PyResult<()> {
        match &mut self.0 {
            SharedType::Integrated(map) => Ok(match subscription_id {
                SubId::Shallow(ShallowSubscription(id)) => map.unobserve(id),
                SubId::Deep(DeepSubscription(id)) => map.unobserve_deep(id),
            }),
            SharedType::Prelim(_) => Err(PreliminaryObservationException::default_message()),
        }
    }
}

#[pyclass(unsendable)]
pub struct ItemView(*const SharedType<Map, HashMap<String, PyObject>>);

#[pymethods]
impl ItemView {
    fn __iter__(slf: PyRef<Self>) -> YMapIterator {
        YMapIterator::from(slf.0)
    }

    fn __len__(&self) -> usize {
        unsafe {
            match &*self.0 {
                SharedType::Integrated(map) => map.len() as usize,
                SharedType::Prelim(map) => map.len(),
            }
        }
    }

    fn __str__(&self) -> String {
        let vals: String = YMapIterator::from(self.0)
            .map(|(key, val)| format!("({key}, {val})"))
            .collect::<Vec<String>>()
            .join(", ");
        format!("{{{vals}}}")
    }

    fn __repr__(&self) -> String {
        let data = self.__str__();
        format!("ItemView({data})")
    }

    fn __contains__(&self, el: PyObject) -> bool {
        let kv: Result<(String, PyObject), _> = Python::with_gil(|py| el.extract(py));
        kv.ok()
            .and_then(|(key, value)| unsafe {
                match &*self.0 {
                    SharedType::Integrated(map) if map.contains(&key) => map.get(&key).map(|v| {
                        Python::with_gil(|py| v.into_py(py).as_ref(py).eq(value)).unwrap_or(false)
                    }),
                    SharedType::Prelim(map) if map.contains_key(&key) => map
                        .get(&key)
                        .map(|v| Python::with_gil(|py| v.as_ref(py).eq(value).unwrap_or(false))),
                    _ => None,
                }
            })
            .unwrap_or(false)
    }
}

#[pyclass(unsendable)]
pub struct KeyView(*const SharedType<Map, HashMap<String, PyObject>>);

#[pymethods]
impl KeyView {
    fn __iter__(slf: PyRef<Self>) -> KeyIterator {
        KeyIterator(YMapIterator::from(slf.0))
    }

    fn __len__(&self) -> usize {
        unsafe {
            match &*self.0 {
                SharedType::Integrated(map) => map.len() as usize,
                SharedType::Prelim(map) => map.len(),
            }
        }
    }

    fn __str__(&self) -> String {
        let vals: String = YMapIterator::from(self.0)
            .map(|(key, _)| key)
            .collect::<Vec<String>>()
            .join(", ");
        format!("{{{vals}}}")
    }

    fn __repr__(&self) -> String {
        let data = self.__str__();
        format!("KeyView({data})")
    }

    fn __contains__(&self, el: PyObject) -> bool {
        let key: Result<String, _> = Python::with_gil(|py| el.extract(py));
        key.ok()
            .map(|key| unsafe {
                match &*self.0 {
                    SharedType::Integrated(map) => map.contains(&key),
                    SharedType::Prelim(map) => map.contains_key(&key),
                }
            })
            .unwrap_or(false)
    }
}

#[pyclass(unsendable)]
pub struct ValueView(*const SharedType<Map, HashMap<String, PyObject>>);

#[pymethods]
impl ValueView {
    fn __iter__(slf: PyRef<Self>) -> ValueIterator {
        ValueIterator(YMapIterator::from(slf.0))
    }

    fn __len__(&self) -> usize {
        unsafe {
            match &*self.0 {
                SharedType::Integrated(map) => map.len() as usize,
                SharedType::Prelim(map) => map.len(),
            }
        }
    }

    fn __str__(&self) -> String {
        let vals: String = YMapIterator::from(self.0)
            .map(|(_, v)| v.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        format!("{{{vals}}}")
    }

    fn __repr__(&self) -> String {
        let data = self.__str__();
        format!("ValueView({data})")
    }
}

pub enum InnerYMapIterator {
    Integrated(MapIter<'static>),
    Prelim(std::collections::hash_map::Iter<'static, String, PyObject>),
}

#[pyclass(unsendable)]
pub struct YMapIterator(ManuallyDrop<InnerYMapIterator>);

impl Drop for YMapIterator {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.0) }
    }
}

impl From<*const SharedType<Map, HashMap<String, PyObject>>> for YMapIterator {
    fn from(inner_map_ptr: *const SharedType<Map, HashMap<String, PyObject>>) -> Self {
        unsafe {
            match &*inner_map_ptr {
                SharedType::Integrated(val) => {
                    let this: *const Map = val;
                    let shared_iter = InnerYMapIterator::Integrated((*this).iter());
                    YMapIterator(ManuallyDrop::new(shared_iter))
                }
                SharedType::Prelim(val) => {
                    let this: *const HashMap<String, PyObject> = val;
                    let shared_iter = InnerYMapIterator::Prelim((*this).iter());
                    YMapIterator(ManuallyDrop::new(shared_iter))
                }
            }
        }
    }
}

impl Iterator for YMapIterator {
    type Item = (String, PyObject);

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.deref_mut() {
            InnerYMapIterator::Integrated(iter) => {
                Python::with_gil(|py| iter.next().map(|(k, v)| (k.to_string(), v.into_py(py))))
            }
            InnerYMapIterator::Prelim(iter) => iter.next().map(|(k, v)| (k.clone(), v.clone())),
        }
    }
}

#[pymethods]
impl YMapIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<(String, PyObject)> {
        slf.next()
    }
}

#[pyclass(unsendable)]
pub struct KeyIterator(YMapIterator);

#[pymethods]
impl KeyIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        slf.0.next().map(|(k, _)| k)
    }
}

#[pyclass(unsendable)]
pub struct ValueIterator(YMapIterator);

#[pymethods]
impl ValueIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<PyObject> {
        slf.0.next().map(|(_, v)| v)
    }
}

/// Event generated by `YMap.observe` method. Emitted during transaction commit phase.
#[pyclass(unsendable)]
pub struct YMapEvent {
    inner: *const MapEvent,
    txn: *const Transaction,
    target: Option<PyObject>,
    keys: Option<PyObject>,
}

impl YMapEvent {
    pub fn new(event: &MapEvent, txn: &Transaction) -> Self {
        let inner = event as *const MapEvent;
        let txn = txn as *const Transaction;
        YMapEvent {
            inner,
            txn,
            target: None,
            keys: None,
        }
    }

    fn inner(&self) -> &MapEvent {
        unsafe { self.inner.as_ref().unwrap() }
    }

    fn txn(&self) -> &Transaction {
        unsafe { self.txn.as_ref().unwrap() }
    }
}

#[pymethods]
impl YMapEvent {
    /// Returns a current shared type instance, that current event changes refer to.
    #[getter]
    pub fn target(&mut self) -> PyObject {
        if let Some(target) = self.target.as_ref() {
            target.clone()
        } else {
            let target: PyObject =
                Python::with_gil(|py| YMap::from(self.inner().target().clone()).into_py(py));
            self.target = Some(target.clone());
            target
        }
    }

    pub fn __repr__(&mut self) -> String {
        let target = self.target();
        let keys = self.keys();
        let path = self.path();
        format!("YMapEvent(target={target}, keys={keys}, path={path})")
    }

    /// Returns an array of keys and indexes creating a path from root type down to current instance
    /// of shared type (accessible via `target` getter).
    pub fn path(&self) -> PyObject {
        Python::with_gil(|py| self.inner().path().into_py(py))
    }

    /// Returns a list of key-value changes made over corresponding `YMap` collection within
    /// bounds of current transaction. These changes follow a format:
    ///
    /// - { action: 'add'|'update'|'delete', oldValue: any|undefined, newValue: any|undefined }
    #[getter]
    pub fn keys(&mut self) -> PyObject {
        if let Some(keys) = &self.keys {
            keys.clone()
        } else {
            let keys: PyObject = Python::with_gil(|py| {
                let keys = self.inner().keys(self.txn());
                let result = PyDict::new(py);
                for (key, value) in keys.iter() {
                    let key = &**key;
                    result.set_item(key, value.into_py(py)).unwrap();
                }
                result.into()
            });

            self.keys = Some(keys.clone());
            keys
        }
    }
}
