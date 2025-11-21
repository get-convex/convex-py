use std::{
    collections::BTreeMap,
    future::Future,
    io::{
        self,
        Write,
    },
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
    types::PyDict,
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

/// A wrapper type that can accept a Python `Dict[str, CoercibleToConvexValue]`
#[derive(Default)]
pub struct FunctionArgsWrapper(BTreeMap<String, Value>);
impl<'a, 'py> FromPyObject<'a, 'py> for FunctionArgsWrapper {
    type Error = PyErr;

    // const INPUT_TYPE: &str = "dict[str, CoercibleToConvexValue]";

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        let map = obj
            .cast::<PyDict>()?
            .iter()
            .map(|(key, value)| {
                let k = key.extract::<String>()?;
                let v = py_to_value(value.as_borrowed())?;
                Ok((k, v))
            })
            .collect::<PyResult<_>>()?;

        Ok(FunctionArgsWrapper(map))
    }
}

async fn check_python_signals_periodically() -> PyErr {
    loop {
        sleep(Duration::from_secs(1)).await;
        if let Err(e) = Python::attach(|py| py.check_signals()) {
            return e;
        }
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
    ) -> PyResult<Py<PyAny>> {
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

    fn block_on_and_check_signals<'a, T, E: ToString, F: Future<Output = Result<T, E>>>(
        &'a mut self,
        f: impl FnOnce(&'a mut ConvexClient) -> F,
    ) -> PyResult<T> {
        self.rt.block_on(async {
            tokio::select!(
                res1 = f(&mut self.client) => res1.map_err(|e| PyException::new_err(e.to_string())),
                res2 = check_python_signals_periodically() => Err(res2),
            )
        })
    }
}

#[pymethods]
impl PyConvexClient {
    /// Note that the WebSocket is not connected yet and therefore the
    /// connection url is not validated to be accepting connections.
    #[new]
    fn py_new(deployment_url: &str, version: &str) -> PyResult<Self> {
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
        let client_id = format!("python-{version}");
        let instance = rt.block_on(
            ConvexClientBuilder::new(deployment_url)
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
    #[pyo3(signature = (name, args=None))]
    pub fn subscribe(
        &mut self,
        name: &str,
        args: Option<FunctionArgsWrapper>,
    ) -> PyResult<PyQuerySubscription> {
        let args: BTreeMap<String, Value> = args.unwrap_or_default().0;
        let res = self.block_on_and_check_signals(|client| client.subscribe(name, args))?;
        Ok(PyQuerySubscription::new(res, self.rt.handle().clone()))
    }

    /// Make a oneshot request to a query `name` with `args`.
    ///
    /// Returns a `convex::Value` representing the result of the query.
    #[pyo3(signature = (name, args=None))]
    pub fn query(
        &mut self,
        py: Python<'_>,
        name: &str,
        args: Option<FunctionArgsWrapper>,
    ) -> PyResult<Py<PyAny>> {
        let args: BTreeMap<String, Value> = args.unwrap_or_default().0;
        let res = self.block_on_and_check_signals(|client| client.query(name, args))?;
        self.function_result_to_py_result(py, res)
    }

    /// Perform a mutation `name` with `args` and return a future
    /// containing the return value of the mutation once it completes.
    #[pyo3(signature = (name, args=None))]
    pub fn mutation(
        &mut self,
        py: Python<'_>,
        name: &str,
        args: Option<FunctionArgsWrapper>,
    ) -> PyResult<Py<PyAny>> {
        let args: BTreeMap<String, Value> = args.unwrap_or_default().0;
        let res = self.block_on_and_check_signals(|client| client.mutation(name, args))?;
        self.function_result_to_py_result(py, res)
    }

    /// Perform an action `name` with `args` and return a future
    /// containing the return value of the action once it completes.
    #[pyo3(signature = (name, args=None))]
    pub fn action(
        &mut self,
        py: Python<'_>,
        name: &str,
        args: Option<FunctionArgsWrapper>,
    ) -> PyResult<Py<PyAny>> {
        let args: BTreeMap<String, Value> = args.unwrap_or_default().0;
        let res = self.block_on_and_check_signals(|client| client.action(name, args))?;
        self.function_result_to_py_result(py, res)
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
    #[pyo3(signature = (token=None))]
    pub fn set_auth(&mut self, token: Option<String>) -> PyResult<()> {
        self.rt.block_on(async {
            tokio::select!(
                () = self.client.set_auth(token) => Ok(()),
                err = check_python_signals_periodically() => Err(err),
            )
        })
    }

    /// Set auth which allows access to system resources.
    ///
    /// Set it with a deploy key obtained from the convex dashboard of a
    /// deployment you control. This auth cannot be unset.
    pub fn set_admin_auth(&mut self, token: String) -> PyResult<()> {
        self.rt.block_on(async {
            tokio::select!(
                () = self.client.set_admin_auth(token, None) => Ok(()),
                err = check_python_signals_periodically() => Err(err),
            )
        })
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
        let s = format!("{value:?}");
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
            writeln!(log_writer, "{message}").unwrap();
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
fn py_to_rust_to_py(py: Python<'_>, py_val: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    // this is just a map
    match py_to_value(py_val.as_borrowed()) {
        Ok(val) => Ok(value_to_py(py, val)),
        Err(err) => Err(err),
    }
}

#[pymodule]
#[pyo3(name = "_convex")]
fn _convex(_py: Python, m: Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConvexClient>()?;
    m.add_class::<PyQuerySubscription>()?;
    m.add_class::<PyQuerySetSubscription>()?;
    m.add_function(wrap_pyfunction!(init_logging, &m)?)?;
    m.add_function(wrap_pyfunction!(py_to_rust_to_py, &m)?)?;
    Ok(())
}
