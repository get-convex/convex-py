use std::sync::Arc;

use convex::{
    FunctionResult,
    SubscriberId,
};
use futures::StreamExt;
use parking_lot::Mutex;
use pyo3::{
    self,
    exceptions::PyStopAsyncIteration,
    prelude::*,
    pyclass,
    types::PyDict,
    IntoPy,
};

use crate::query_result::value_to_py;

#[pyclass]
pub struct PyQuerySubscription {
    inner: Arc<Mutex<Option<convex::QuerySubscription>>>,
}

impl From<convex::QuerySubscription> for PyQuerySubscription {
    fn from(query_sub: convex::QuerySubscription) -> Self {
        PyQuerySubscription {
            inner: Arc::new(Mutex::new(Some(query_sub))),
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
}

#[pymethods]
impl PyQuerySubscription {
    #[getter]
    fn id(&self, py: Python) -> PyObject {
        let query_sub = self.inner.clone();
        let query_sub_inner = query_sub.lock().take().unwrap();
        let sub_id: SubscriberId = *query_sub_inner.id();
        let _ = query_sub.lock().insert(query_sub_inner);
        let py_sub_id: PySubscriberId = sub_id.into();
        py_sub_id.into_py(py)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python) -> Option<PyObject> {
        let query_sub = self.inner.clone();

        let val = pyo3_asyncio::tokio::run(py, async move {
            let mut query_sub_inner = query_sub.lock().take().unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);
            Ok(res)
        })
        .ok()
        .unwrap();
        match val {
            Some(v) => match v {
                FunctionResult::Value(v) => Some(value_to_py(py, v)),
                FunctionResult::ErrorMessage(e) => Some(value_to_py(py, convex::Value::String(e))),
            },
            None => None,
        }
    }

    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__(slf: PyRefMut<Self>) -> PyResult<Option<PyObject>> {
        let query_sub = slf.inner.clone();
        let fut = pyo3_asyncio::tokio::future_into_py(slf.py(), async move {
            let mut query_sub_inner = query_sub.lock().take().unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);
            Python::with_gil(|py| match res {
                Some(v) => match v {
                    FunctionResult::Value(v) => Ok(value_to_py(py, v)),
                    FunctionResult::ErrorMessage(e) => {
                        Ok(value_to_py(py, convex::Value::String(e)))
                    },
                },
                None => Err(PyStopAsyncIteration::new_err("Stream exhausted")),
            })
        })?;
        Ok(Some(fut.into()))
    }
}

#[pyclass]
pub struct PyQuerySetSubscription {
    inner: Arc<Mutex<Option<convex::QuerySetSubscription>>>,
}

impl From<convex::QuerySetSubscription> for PyQuerySetSubscription {
    fn from(query_set_sub: convex::QuerySetSubscription) -> Self {
        PyQuerySetSubscription {
            inner: Arc::new(Mutex::new(Some(query_set_sub))),
        }
    }
}

#[pymethods]
impl PyQuerySetSubscription {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python) -> Option<PyObject> {
        let query_sub = self.inner.clone();

        let val = pyo3_asyncio::tokio::run(py, async move {
            let mut query_sub_inner = query_sub.lock().take().unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);
            Ok(res)
        })
        .ok()
        .unwrap();

        match val {
            Some(query_results) => {
                // QueryResults should get serialized to a PyDict (sub_id -> Value)
                let py_dict = PyDict::new(py);
                for (sub_id, func_res) in query_results.iter() {
                    let py_sub_id: PySubscriberId = (*sub_id).into();
                    if func_res.is_none() {
                        continue;
                    }
                    let sub_value: PyObject = match func_res.expect("Expect a result") {
                        FunctionResult::Value(v) => value_to_py(py, v.clone()),
                        FunctionResult::ErrorMessage(e) => {
                            value_to_py(py, convex::Value::String(e.clone()))
                        },
                    };
                    py_dict.set_item(py_sub_id.into_py(py), sub_value).unwrap();
                }
                Some(py_dict.into())
            },
            None => None,
        }
    }

    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__(slf: PyRefMut<Self>) -> PyResult<Option<PyObject>> {
        let query_sub = slf.inner.clone();
        let fut = pyo3_asyncio::tokio::future_into_py(slf.py(), async move {
            let mut query_sub_inner = query_sub.lock().take().unwrap();
            let res = query_sub_inner.next().await;
            let _ = query_sub.lock().insert(query_sub_inner);

            Python::with_gil(|py| -> PyResult<PyObject> {
                match res {
                    Some(query_results) => {
                        let py_dict = PyDict::new(py);
                        for (sub_id, func_res) in query_results.iter() {
                            let py_sub_id: PySubscriberId = (*sub_id).into();
                            if func_res.is_none() {
                                continue;
                            }
                            let sub_value: PyObject = match func_res.expect("Expect a result") {
                                FunctionResult::Value(v) => value_to_py(py, v.clone()),
                                FunctionResult::ErrorMessage(e) => {
                                    value_to_py(py, convex::Value::String(e.clone()))
                                },
                            };
                            py_dict.set_item(py_sub_id.into_py(py), sub_value).unwrap();
                        }
                        Ok(py_dict.into())
                    },
                    None => Err(PyStopAsyncIteration::new_err("Stream exhausted")),
                }
            })
        })?;

        Ok(Some(fut.into()))
    }
}
