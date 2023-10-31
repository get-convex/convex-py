use std::{
    collections::hash_map::DefaultHasher,
    hash::{
        Hash,
        Hasher,
    },
    sync::Arc,
};

use convex::{
    FunctionResult,
    SubscriberId,
};
use futures::StreamExt;
use parking_lot::Mutex;
use pyo3::{
    self,
    exceptions::{
        PyAssertionError,
        PyNotImplementedError,
        PyStopAsyncIteration,
        PyStopIteration,
    },
    prelude::*,
    pyclass::CompareOp,
    types::PyDict,
    IntoPy,
};
use tokio::time::{
    sleep,
    Duration,
};

use crate::query_result::value_to_py;

#[pyclass]
pub struct PyQuerySubscription {
    inner: Arc<Mutex<Option<convex::QuerySubscription>>>,
    pub rt_handle: Option<tokio::runtime::Handle>,
}

impl From<convex::QuerySubscription> for PyQuerySubscription {
    fn from(query_sub: convex::QuerySubscription) -> Self {
        PyQuerySubscription {
            inner: Arc::new(Mutex::new(Some(query_sub))),
            rt_handle: None,
        }
    }
}

#[pyclass]
pub struct PySubscriberId {
    inner: convex::SubscriberId,
}

impl From<convex::SubscriberId> for PySubscriberId {
    fn from(sub_id: convex::SubscriberId) -> Self {
        PySubscriberId { inner: sub_id }
    }
}

#[pymethods]
impl PySubscriberId {
    fn __str__(slf: PyRef<'_, Self>) -> String {
        let sub_id = &slf.inner;
        format!("{sub_id:#?}")
    }

    fn __repr__(slf: PyRef<'_, Self>) -> String {
        let sub_id = &slf.inner;
        format!("{sub_id:#?}")
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self.inner == other.inner),
            CompareOp::Ne => Ok(self.inner != other.inner),
            _ => Err(PyNotImplementedError::new_err(
                "Can't compare SubscriberIds in the requested way.",
            )),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

async fn check_python_signals_periodically() -> PyResult<()> {
    loop {
        sleep(Duration::from_secs(1)).await;
        Python::with_gil(|py| py.check_signals())?;
    }
}

#[pymethods]
impl PyQuerySubscription {
    fn exists(&self, py: Python) -> Py<PyAny> {
        let exists = self.inner.lock().is_some();
        exists.into_py(py)
    }

    #[getter]
    fn id(&self, py: Python) -> PyObject {
        let query_sub = self.inner.clone();
        let query_sub_inner = query_sub.lock().take().unwrap();
        let sub_id: SubscriberId = *query_sub_inner.id();
        let _ = query_sub.lock().insert(query_sub_inner);
        let py_sub_id: PySubscriberId = sub_id.into();
        py_sub_id.into_py(py)
    }

    // Drops the inner subscription object, which causes a
    // downstream unsubscription event.
    fn unsubscribe(&self) {
        self.inner.lock().take();
    }

    fn next(&mut self, py: Python) -> PyResult<PyObject> {
        let query_sub = self.inner.clone();
        let res = self.rt_handle.as_mut().unwrap().block_on(async {
            tokio::select!(
                res1 = async move {
                    let query_sub_inner = query_sub.lock().take();
                    if query_sub_inner.is_none() {
                        return Err(PyStopIteration::new_err("Stream requires reset"));
                    }
                    let mut query_sub_inner = query_sub_inner.unwrap();
                    let res = query_sub_inner.next().await;
                    let _ = query_sub.lock().insert(query_sub_inner);
                    Ok(res)} => res1,
                res2 = check_python_signals_periodically() => Err(res2.err().unwrap())
            )
        })?;
        match res.unwrap() {
            FunctionResult::Value(v) => Ok(value_to_py(py, v)),
            FunctionResult::ErrorMessage(e) => Err(PyErr::new::<PyAssertionError, _>(e)),
        }
    }

    fn anext(slf: PyRefMut<Self>) -> PyResult<Option<PyObject>> {
        let query_sub = slf.inner.clone();
        let fut = pyo3_asyncio::tokio::future_into_py(slf.py(), async move {
            let query_sub_inner = query_sub.lock().take();
            if query_sub_inner.is_none() {
                return Err(PyStopAsyncIteration::new_err("Stream requires reset"));
            }
            let mut query_sub_inner = query_sub_inner.unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);
            Python::with_gil(|py| match res.unwrap() {
                FunctionResult::Value(v) => Ok(value_to_py(py, v)),
                FunctionResult::ErrorMessage(e) => Ok(value_to_py(py, convex::Value::String(e))),
            })
        })?;
        Ok(Some(fut.into()))
    }
}

#[pyclass]
pub struct PyQuerySetSubscription {
    inner: Arc<Mutex<Option<convex::QuerySetSubscription>>>,
    pub rt_handle: Option<tokio::runtime::Handle>,
}

impl From<convex::QuerySetSubscription> for PyQuerySetSubscription {
    fn from(query_set_sub: convex::QuerySetSubscription) -> Self {
        PyQuerySetSubscription {
            inner: Arc::new(Mutex::new(Some(query_set_sub))),
            rt_handle: None,
        }
    }
}

#[pymethods]
impl PyQuerySetSubscription {
    fn exists(&self, py: Python) -> Py<PyAny> {
        let exists = self.inner.lock().is_some();
        exists.into_py(py)
    }

    fn next(&mut self, py: Python) -> PyResult<PyObject> {
        let query_sub = self.inner.clone();
        let res = self.rt_handle.as_mut().unwrap().block_on(async {
            tokio::select!(
                res1 = async move {
                    let query_sub_inner = query_sub.lock().take();
                    if query_sub_inner.is_none() {
                        return Err(PyStopIteration::new_err("Stream requires reset"));
                    }
                    let mut query_sub_inner = query_sub_inner.unwrap();
                    let res = query_sub_inner.next().await;
                    let _ = query_sub.lock().insert(query_sub_inner);
                    Ok(res)} => res1,
                res2 = check_python_signals_periodically() => Err(res2.err().unwrap())
            )
        })?;
        let query_results = res.unwrap();
        let py_dict = PyDict::new(py);
        for (sub_id, function_result) in query_results.iter() {
            if function_result.is_none() {
                continue;
            }
            let py_sub_id: PySubscriberId = (*sub_id).into();

            let sub_value: PyObject = match function_result.unwrap() {
                FunctionResult::Value(v) => value_to_py(py, v.clone()),
                FunctionResult::ErrorMessage(e) => {
                    value_to_py(py, convex::Value::String(e.clone()))
                },
            };
            py_dict.set_item(py_sub_id.into_py(py), sub_value).unwrap();
        }
        Ok(py_dict.into_py(py))
    }

    fn anext(slf: PyRefMut<Self>) -> PyResult<Option<PyObject>> {
        let query_sub = slf.inner.clone();
        let fut: &PyAny = pyo3_asyncio::tokio::future_into_py(slf.py(), async move {
            let query_sub_inner = query_sub.lock().take();
            if query_sub_inner.is_none() {
                return Err(PyStopAsyncIteration::new_err("Stream requires reset"));
            }
            let mut query_sub_inner = query_sub_inner.unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);

            Python::with_gil(|py| -> PyResult<PyObject> {
                let query_results = res.unwrap();
                let py_dict = PyDict::new(py);
                for (sub_id, function_result) in query_results.iter() {
                    if function_result.is_none() {
                        continue;
                    }
                    let py_sub_id: PySubscriberId = (*sub_id).into();
                    let sub_value: PyObject = match function_result.unwrap() {
                        FunctionResult::Value(v) => value_to_py(py, v.clone()),
                        FunctionResult::ErrorMessage(e) => {
                            value_to_py(py, convex::Value::String(e.clone()))
                        },
                    };
                    py_dict.set_item(py_sub_id.into_py(py), sub_value).unwrap();
                }
                Ok(py_dict.into())
            })
        })?;

        Ok(Some(fut.into()))
    }
}
