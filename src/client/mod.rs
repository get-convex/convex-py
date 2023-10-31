use std::{
    collections::BTreeMap,
    ops::Deref,
};

use convex::{
    ConvexClient,
    FunctionResult,
    Value,
};
use pyo3::{
    exceptions::PyException,
    prelude::*,
    pyclass,
    types::{
        PyDict,
        PyString,
    },
};
use tokio::{
    runtime,
    time::{
        sleep,
        Duration,
    },
};

use crate::{
    query_result::{
        py_to_value,
        value_to_py,
    },
    subscription::{
        PyQuerySetSubscription,
        PyQuerySubscription,
    },
};

struct BTreeMapWrapper<String, Value>(BTreeMap<String, Value>);
impl From<&PyDict> for BTreeMapWrapper<String, Value> {
    fn from(d: &PyDict) -> Self {
        let map = d
            .iter()
            .filter_map(|(key, value)| {
                let k: Result<&pyo3::types::PyString, _> = key.extract();
                let v: Result<Value, _> = py_to_value(value);
                if let (Ok(k), Ok(v)) = (k, v) {
                    Some((k.to_string(), v))
                } else {
                    None
                }
            })
            .collect();

        BTreeMapWrapper(map)
    }
}

impl<K, V> Deref for BTreeMapWrapper<K, V> {
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn check_python_signals_periodically() -> PyResult<()> {
    loop {
        sleep(Duration::from_secs(1)).await;
        Python::with_gil(|py| py.check_signals())?;
    }
}
/// An asynchronous client to interact with a specific project to perform
/// queries/mutations/actions and manage query subscriptions.
#[pyclass]
pub struct PyConvexClient {
    rt: tokio::runtime::Runtime,
    client: ConvexClient,
}

#[pymethods]
impl PyConvexClient {
    #[new]
    fn py_new(deployment_url: &PyString) -> PyResult<Self> {
        let dep = deployment_url.to_str()?;
        // The ConvexClient is instantiated in the context of a tokio Runtime, and
        // needs to run its worker in the background so that it can constantly
        // listen for new messages from the server. Here, we choose to build a
        // multi-thread scheduler to make that possible.
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .unwrap();

        // Block on the async function using the Tokio runtime.
        let instance = rt.block_on(ConvexClient::new(dep));
        match instance {
            Ok(instance) => Ok(PyConvexClient {
                rt,
                client: instance,
            }),
            Err(e) => Err(PyException::new_err(format!(
                "{}: {}",
                "Failed to create PyConvexClient",
                &e.to_string()
            ))),
        }
    }

    /// Creates a single subscription to a query, with optional args.
    pub fn subscribe(
        &mut self,
        py: Python<'_>,
        name: &PyString,
        args: Option<&PyDict>,
    ) -> PyResult<PyQuerySubscription> {
        let name: &str = name.to_str()?;
        let args: BTreeMapWrapper<String, Value> = args.unwrap_or(PyDict::new(py)).into();
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.subscribe(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });
        match res {
            Ok(res) => {
                let mut py_res: PyQuerySubscription = res.into();
                py_res.rt_handle = Some(self.rt.handle().clone());
                Ok(py_res)
            },
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }

    /// Make a oneshot request to a query `name` with `args`.
    ///
    /// Returns a `convex::Value` representing the result of the query.
    pub fn query(
        &mut self,
        py: Python<'_>,
        name: &PyString,
        args: Option<&PyDict>,
    ) -> PyResult<PyObject> {
        let name: &str = name.to_str()?;
        let args: BTreeMapWrapper<String, Value> = args.unwrap_or(PyDict::new(py)).into();
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.query(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });

        match res {
            Ok(res) => match res {
                FunctionResult::Value(v) => Ok(value_to_py(py, v)),
                FunctionResult::ErrorMessage(e) => Ok(value_to_py(py, convex::Value::String(e))),
            },
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }

    /// Perform a mutation `name` with `args` and return a future
    /// containing the return value of the mutation once it completes.
    pub fn mutation(
        &mut self,
        py: Python<'_>,
        name: &PyString,
        args: Option<&PyDict>,
    ) -> PyResult<PyObject> {
        let name: &str = name.to_str()?;
        let args: BTreeMapWrapper<String, Value> = args.unwrap_or(PyDict::new(py)).into();
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.mutation(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });

        match res {
            Ok(res) => match res {
                FunctionResult::Value(v) => Ok(value_to_py(py, v)),
                FunctionResult::ErrorMessage(e) => Ok(value_to_py(py, convex::Value::String(e))),
            },
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }

    /// Perform an action `name` with `args` and return a future
    /// containing the return value of the action once it completes.
    pub fn action(
        &mut self,
        py: Python<'_>,
        name: &PyString,
        args: Option<&PyDict>,
    ) -> PyResult<PyObject> {
        let name: &str = name.to_str()?;
        let args: BTreeMapWrapper<String, Value> = args.unwrap_or(PyDict::new(py)).into();
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.action(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });

        match res {
            Ok(res) => match res {
                FunctionResult::Value(v) => Ok(value_to_py(py, v)),
                FunctionResult::ErrorMessage(e) => Ok(value_to_py(py, convex::Value::String(e))),
            },
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }

    /// Get a consistent view of the results of every query the client is
    /// currently subscribed to. This set changes over time as subscriptions
    /// are added and dropped.
    pub fn watch_all(&mut self, _py: Python<'_>) -> PyQuerySetSubscription {
        let mut py_res: PyQuerySetSubscription = self.client.watch_all().into();
        py_res.rt_handle = Some(self.rt.handle().clone());
        py_res
    }

    /// Set auth for use when calling Convex functions.
    ///
    /// Set it with a token that you get from your auth provider via their login
    /// flow. If `None` is passed as the token, then auth is unset (logging
    /// out).
    pub fn set_auth(&mut self, token: Option<&PyString>) {
        let token = token.map(|t| t.to_string());
        self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.set_auth(token) => res1,
                _ = check_python_signals_periodically() => panic!()
            )
        });
    }
}

#[pymodule]
fn py_client(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyConvexClient>()?;
    m.add_class::<PyQuerySubscription>()?;
    m.add_class::<PyQuerySetSubscription>()?;

    Ok(())
}
