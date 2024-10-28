use std::{
    collections::BTreeMap,
    io::{
        self,
        Write,
    },
    ops::Deref,
};

use convex::{
    ConvexClient,
    ConvexClientBuilder,
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
use tracing::{
    field::{
        Field,
        Visit,
    },
    subscriber::set_global_default,
    Event,
    Level,
    Subscriber,
};
use tracing_subscriber::{
    layer::Context,
    prelude::__tracing_subscriber_SubscriberExt,
    Layer,
    Registry,
};

use crate::{
    query_result::{
        convex_error_to_py_wrapped,
        py_to_value,
        value_to_py,
        value_to_py_wrapped,
    },
    subscription::{
        PyQuerySetSubscription,
        PyQuerySubscription,
    },
};

// TODO can this be removed?
struct BTreeMapWrapper<String, Value>(BTreeMap<String, Value>);
impl BTreeMapWrapper<String, Value> {
    fn from(py: Python<'_>, d: &PyDict) -> Self {
        let map = d
            .iter()
            .filter_map(|(key, value)| {
                let k: Result<&pyo3::types::PyString, _> = key.extract();
                let v: Result<Value, _> = py_to_value(py, value);
                if let (Ok(k), Ok(v)) = (k, v) {
                    Some((k.to_string(), v))
                } else {
                    // TODO this seems wrong, shouldn't we raise here?
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

impl PyConvexClient {
    fn function_result_to_py_result(
        &mut self,
        py: Python<'_>,
        result: FunctionResult,
    ) -> PyResult<PyObject> {
        match result {
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
}

#[pymethods]
impl PyConvexClient {
    /// Note that the WebSocket is not connected yet and therefore the
    /// connection url is not validated to be accepting connections.
    #[new]
    fn py_new(deployment_url: &PyString, version: &PyString) -> PyResult<Self> {
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
        let client_id = format!("python-{}", version.to_str()?);
        let instance = rt.block_on(
            ConvexClientBuilder::new(dep)
                .with_client_id(&client_id)
                .build(),
        );
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
        let args: BTreeMapWrapper<String, Value> =
            BTreeMapWrapper::from(py, args.unwrap_or(PyDict::new(py)));
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
        let args: BTreeMapWrapper<String, Value> =
            BTreeMapWrapper::from(py, args.unwrap_or(PyDict::new(py)));
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.query(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });

        match res {
            Ok(res) => self.function_result_to_py_result(py, res),
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
        let args: BTreeMapWrapper<String, Value> =
            BTreeMapWrapper::from(py, args.unwrap_or(PyDict::new(py)));
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.mutation(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });

        match res {
            Ok(res) => self.function_result_to_py_result(py, res),
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
        let args: BTreeMapWrapper<String, Value> =
            BTreeMapWrapper::from(py, args.unwrap_or(PyDict::new(py)));
        let args: BTreeMap<String, Value> = args.deref().clone();

        let res = self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.action(name, args) => res1,
                res2 = check_python_signals_periodically() => Err(res2.expect_err("Panic!").into())
            )
        });

        match res {
            Ok(res) => self.function_result_to_py_result(py, res),
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

    /// Set auth which allows access to system resources.
    ///
    /// Set it with a deploy key obtained from the convex dashboard of a
    /// deployment you control. This auth cannot be unset.
    pub fn set_admin_auth(&mut self, token: &PyString) {
        let token = token.to_string();
        self.rt.block_on(async {
            tokio::select!(
                res1 = self.client.set_admin_auth(token, None) => res1,
                _ = check_python_signals_periodically() => panic!()
            )
        });
    }
}

struct UDFLogVisitor {
    fields: BTreeMap<String, String>,
}

impl UDFLogVisitor {
    fn new() -> Self {
        UDFLogVisitor {
            fields: BTreeMap::new(),
        }
    }
}

// Extracts a BTreeMap from our log line
impl Visit for UDFLogVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        self.fields.insert(field.name().to_string(), s);
    }
}

struct ConvexLoggingLayer;

impl<S: Subscriber> Layer<S> for ConvexLoggingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = UDFLogVisitor::new();
        event.record(&mut visitor);
        let mut log_writer = io::stdout();
        if let Some(message) = visitor.fields.get("message") {
            writeln!(log_writer, "{}", message).unwrap();
        }
    }
}

#[pyfunction]
fn init_logging() {
    let subscriber = Registry::default().with(ConvexLoggingLayer.with_filter(
        tracing_subscriber::filter::Targets::new().with_target("convex_logs", Level::DEBUG),
    ));

    set_global_default(subscriber).expect("Failed to set up custom logging subscriber");
}

// Exposed for testing
#[pyfunction]
fn py_to_rust_to_py(py: Python<'_>, py_val: &PyAny) -> PyResult<PyObject> {
    // this is just a map
    match py_to_value(py, py_val) {
        Ok(val) => Ok(value_to_py(py, val)),
        Err(err) => Err(err),
    }
}

#[pymodule]
#[pyo3(name = "_convex")]
fn _convex(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyConvexClient>()?;
    m.add_class::<PyQuerySubscription>()?;
    m.add_class::<PyQuerySetSubscription>()?;
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_function(wrap_pyfunction!(py_to_rust_to_py, m)?)?;
    Ok(())
}
