use std::collections::BTreeMap;

use convex::ConvexError;
use pyo3::{
    exceptions::PyException,
    types::{
        PyAnyMethods,
        PyBool,
        PyBytes,
        PyDict,
        PyDictMethods,
        PyFloat,
        PyInt,
        PyList,
        PyListMethods,
        PyString,
    },
    Borrowed,
    PyAny,
    PyObject,
    PyResult,
    Python,
};

// TODO using an enum would be cleaner here
pub fn value_to_py_wrapped(py: Python<'_>, v: convex::Value) -> PyObject {
    let py_dict = PyDict::new(py);
    py_dict
        .set_item("type", PyString::new(py, "value"))
        .unwrap();
    py_dict.set_item("value", value_to_py(py, v)).unwrap();
    py_dict.into()
}

pub fn convex_error_to_py_wrapped(py: Python<'_>, err: ConvexError) -> PyObject {
    let py_dict = PyDict::new(py);
    py_dict
        .set_item("type", PyString::new(py, "convexerror"))
        .unwrap();
    py_dict.set_item("message", err.message).unwrap();
    py_dict.set_item("data", value_to_py(py, err.data)).unwrap();
    py_dict.into()
}

pub fn value_to_py(py: Python<'_>, v: convex::Value) -> PyObject {
    match v {
        convex::Value::Null => py.None(),
        convex::Value::Int64(val) => {
            let int64_module = py
                .import("_convex.int64")
                .expect("Couldn't import _convex.int64");
            let int_64_class = int64_module
                .getattr("ConvexInt64")
                .expect("Couldn't import ConvexInt64 from _convex.int64");
            let obj: PyObject = int_64_class
                .call((val,), None)
                .unwrap_or_else(|_| panic!("Couldn't construct ConvexInt64() from {:?}", val))
                .into();
            obj
        },

        convex::Value::Float64(val) => PyFloat::new(py, val).into(),
        convex::Value::Boolean(val) => PyBool::new(py, val).as_any().clone().unbind(),
        convex::Value::String(val) => PyString::new(py, &val).into(),
        convex::Value::Bytes(val) => PyBytes::new(py, &val).into(),
        convex::Value::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                py_list.append(value_to_py(py, item)).unwrap();
            }
            py_list.into()
        },
        convex::Value::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (key, value) in obj {
                py_dict.set_item(key, value_to_py(py, value)).unwrap();
            }
            py_dict.into()
        },
    }
}

// TODO Implement all or most of the coercions from the Python client.
/// Translate a Python value to Rust, doing isinstance coersion (e.g. subclasses
/// of list will be interpreted as lists) but not other conversions (e.g. tuple
/// to list).
pub fn py_to_value(py_val: Borrowed<'_, '_, PyAny>) -> PyResult<convex::Value> {
    let py = py_val.py();
    let int64_module = py.import("_convex.int64")?;
    let int_64_class = int64_module.getattr("ConvexInt64")?;

    // check boolean first, since it's a subclass of int
    if py_val.is_instance_of::<PyBool>() {
        let val: bool = py_val.extract::<bool>()?;
        return Ok(convex::Value::Boolean(val));
    }
    if py_val.is_instance_of::<PyInt>() {
        // Note conversion from int to float
        let val: f64 = py_val.extract()?;
        return Ok(convex::Value::Float64(val));
    }
    if py_val.is_instance_of::<PyFloat>() {
        let val: f64 = py_val.extract::<f64>()?;
        return Ok(convex::Value::Float64(val));
    }
    if py_val.is_instance(&int_64_class)? {
        let value = py_val.getattr("value")?;
        let val: i64 = value.extract()?;
        return Ok(convex::Value::Int64(val));
    }
    if py_val.is_instance_of::<PyString>() {
        let val: String = py_val.extract::<String>()?;
        return Ok(convex::Value::String(val));
    }
    if py_val.is_instance_of::<PyBytes>() {
        let val: Vec<u8> = py_val.extract::<Vec<u8>>()?;
        return Ok(convex::Value::Bytes(val));
    }
    if py_val.is_instance_of::<PyList>() {
        let py_list = py_val.downcast::<PyList>()?;
        let mut vec: Vec<convex::Value> = Vec::new();
        for item in py_list {
            let inner_value: convex::Value = py_to_value(item.as_borrowed())?;
            vec.push(inner_value);
        }
        return Ok(convex::Value::Array(vec));
    }
    if py_val.is_instance_of::<PyDict>() {
        let py_dict = py_val.downcast::<PyDict>()?;
        let mut map: BTreeMap<String, convex::Value> = BTreeMap::new();
        for (key, value) in py_dict.iter() {
            let inner_value: convex::Value = py_to_value(value.as_borrowed())?;
            let inner_key: convex::Value = py_to_value(key.as_borrowed())?;
            match inner_key {
                convex::Value::String(s) => map.insert(s, inner_value),
                _ => {
                    return Err(PyException::new_err(format!(
                        "Bad key for Convex object: {:?}",
                        key
                    )))
                },
            };
        }
        return Ok(convex::Value::Object(map));
    }
    if py_val.is_none() {
        return Ok(convex::Value::Null);
    }

    Err(PyException::new_err(format!(
        "Failed to serialize to Convex value {:?} of type {:?}",
        py_val,
        py_val.get_type()
    )))
}
