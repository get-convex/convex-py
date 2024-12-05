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
    exceptions::{
        PyException,
        PyNotImplementedError,
        PyStopAsyncIteration,
        PyStopIteration,
    },
    prelude::*,
    pyclass::CompareOp,
    types::PyDict,
};
use tokio::time::{
    sleep,
    Duration,
};

use crate::query_result::{
    convex_error_to_py_wrapped,
    value_to_py,
    value_to_py_wrapped,
};

#[pyclass(frozen)]
pub struct PyQuerySubscription {
    // TODO document here why this needs to be an Arc<Mutex<Option<Sub>>>
    inner: Arc<Mutex<Option<convex::QuerySubscription>>>,
    pub rt_handle: tokio::runtime::Handle,
}

impl PyQuerySubscription {
    pub fn new(query_sub: convex::QuerySubscription, rt_handle: tokio::runtime::Handle) -> Self {
        PyQuerySubscription {
            inner: Arc::new(Mutex::new(Some(query_sub))),
            rt_handle,
        }
    }
}

#[pyclass(frozen)]
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
    fn __str__(&self) -> String {
        format!("{:#?}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("{:#?}", self.inner)
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

async fn check_python_signals_periodically() -> PyErr {
    loop {
        sleep(Duration::from_secs(1)).await;
        if let Err(e) = Python::with_gil(|py| py.check_signals()) {
            return e;
        }
    }
}

#[pymethods]
impl PyQuerySubscription {
    fn exists(&self) -> bool {
        self.inner.lock().is_some()
    }

    #[getter]
    fn id(&self) -> PySubscriberId {
        let query_sub = self.inner.clone();
        let query_sub_inner = query_sub.lock().take().unwrap();
        let sub_id: SubscriberId = *query_sub_inner.id();
        let _ = query_sub.lock().insert(query_sub_inner);
        PySubscriberId::from(sub_id)
    }

    // Drops the inner subscription object, which causes a
    // downstream unsubscription event.
    fn unsubscribe(&self) {
        self.inner.lock().take();
    }

    fn next(&self, py: Python) -> PyResult<PyObject> {
        let query_sub = self.inner.clone();
        let res = self.rt_handle.block_on(async {
            tokio::select!(
                res1 = async move {
                    let query_sub_inner = query_sub.lock().take();
                    if query_sub_inner.is_none() {
                        return Err(PyStopIteration::new_err("Stream requires reset"));
                    }
                    let mut query_sub_inner = query_sub_inner.unwrap();
                    let res = query_sub_inner.next().await;
                    let _ = query_sub.lock().insert(query_sub_inner);
                    Ok(res)
                } => res1,
                res2 = check_python_signals_periodically() => Err(res2)
            )
        })?;
        match res.unwrap() {
            FunctionResult::Value(v) => Ok(value_to_py_wrapped(py, v)),
            FunctionResult::ErrorMessage(e) => Err(PyException::new_err(e)),
            FunctionResult::ConvexError(v) => {
                // pyo3 can't defined new custom exceptions when using the common abi
                // `features = ["abi3"]` https://github.com/PyO3/pyo3/issues/1344
                // so we define this error in Python. So just return a wrapped one.
                Ok(convex_error_to_py_wrapped(py, v))
            },
        }
    }

    fn anext(&self, py: Python<'_>) -> PyResult<PyObject> {
        let query_sub = self.inner.clone();
        let fut = pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let query_sub_inner = query_sub.lock().take();
            if query_sub_inner.is_none() {
                return Err(PyStopAsyncIteration::new_err("Stream requires reset"));
            }
            let mut query_sub_inner = query_sub_inner.unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);
            Python::with_gil(|py| match res.unwrap() {
                FunctionResult::Value(v) => Ok(value_to_py_wrapped(py, v)),
                FunctionResult::ErrorMessage(e) => Err(PyException::new_err(e)),
                FunctionResult::ConvexError(v) => {
                    // pyo3 can't defined new custom exceptions when using the common abi
                    // `features = ["abi3"]` https://github.com/PyO3/pyo3/issues/1344
                    // so we define this error in Python. So just return a wrapped one.
                    Ok(convex_error_to_py_wrapped(py, v))
                },
            })
        })?;
        Ok(fut.unbind())
    }
}

#[pyclass(frozen)]
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
    fn exists(&self) -> bool {
        self.inner.lock().is_some()
    }

    fn next(&self, py: Python) -> PyResult<PyObject> {
        let query_sub = self.inner.clone();
        let res = self.rt_handle.as_ref().unwrap().block_on(async {
            tokio::select!(
                res1 = async move {
                    let query_sub_inner = query_sub.lock().take();
                    if query_sub_inner.is_none() {
                        return Err(PyStopIteration::new_err("Stream requires reset"));
                    }
                    let mut query_sub_inner = query_sub_inner.unwrap();
                    let res = query_sub_inner.next().await;
                    let _ = query_sub.lock().insert(query_sub_inner);
                    Ok(res)
                } => res1,
                res2 = check_python_signals_periodically() => Err(res2)
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
                FunctionResult::Value(v) => value_to_py_wrapped(py, v.clone()),
                FunctionResult::ErrorMessage(e) => {
                    // TODO this is wrong!
                    value_to_py(py, convex::Value::String(e.clone()))
                },
                FunctionResult::ConvexError(v) => {
                    // pyo3 can't defined new custom exceptions when using the common abi
                    // `features = ["abi3"]` https://github.com/PyO3/pyo3/issues/1344
                    // so we define this error in Python. So just return a wrapped one.
                    convex_error_to_py_wrapped(py, v.clone())
                        .into_pyobject(py)?
                        .unbind()
                },
            };
            py_dict
                .set_item(py_sub_id.into_pyobject(py)?, sub_value)
                .unwrap();
        }
        Ok(py_dict.into_any().unbind())
    }

    fn anext(&self, py: Python<'_>) -> PyResult<PyObject> {
        let query_sub = self.inner.clone();
        let fut = pyo3_async_runtimes::tokio::future_into_py(py, async move {
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
                        // TODO: this conflates errors with genuine values
                        FunctionResult::ErrorMessage(e) => {
                            value_to_py(py, convex::Value::String(e.to_string()))
                        },
                        FunctionResult::ConvexError(e) => {
                            let e = e.clone();
                            (
                                value_to_py(py, convex::Value::String(e.message)),
                                value_to_py(py, e.data),
                            )
                                .into_pyobject(py)?
                                .into_any()
                                .unbind()
                        },
                    };
                    py_dict.set_item(py_sub_id, sub_value).unwrap();
                }
                Ok(py_dict.into())
            })
        })?;
        Ok(fut.unbind())
    }
}
