use std::collections::{
    BTreeMap,
    BTreeSet,
};

use pyo3::{
    exceptions::PyException,
    types::{
        PyBool,
        PyBytes,
        PyDict,
        PyFloat,
        PyInt,
        PyList,
        PySet,
        PyString,
    },
    IntoPy,
    PyAny,
    PyObject,
    PyResult,
    Python,
};

pub fn value_to_py(py: Python<'_>, v: convex::Value) -> PyObject {
    match v {
        convex::Value::Null => py.None(),
        convex::Value::Int64(val) => val.into_py(py),
        convex::Value::Float64(val) => PyFloat::new(py, val).into(),
        convex::Value::Boolean(val) => PyBool::new(py, val).into(),
        convex::Value::String(val) => PyString::new(py, &val).into(),
        convex::Value::Bytes(val) => PyBytes::new(py, &val).into(),
        convex::Value::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                py_list.append(value_to_py(py, item)).unwrap();
            }
            py_list.into()
        },
        convex::Value::Set(set) => {
            let py_set = pyo3::types::PySet::empty(py).unwrap();
            for item in set {
                py_set.add(value_to_py(py, item)).unwrap();
            }
            py_set.into()
        },
        convex::Value::Map(map) => {
            let py_dict = PyDict::new(py);
            for (key, value) in map {
                py_dict
                    .set_item(value_to_py(py, key), value_to_py(py, value))
                    .unwrap();
            }
            py_dict.into()
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

pub fn py_to_value(py_val: &PyAny) -> PyResult<convex::Value> {
    if py_val.is_instance_of::<PyInt>() {
        let val: i64 = py_val.extract::<i64>()?;
        return Ok(convex::Value::Int64(val));
    }
    if py_val.is_instance_of::<PyFloat>() {
        let val: f64 = py_val.extract::<f64>()?;
        return Ok(convex::Value::Float64(val));
    }
    if py_val.is_instance_of::<PyBool>() {
        let val: bool = py_val.extract::<bool>()?;
        return Ok(convex::Value::Boolean(val));
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
            let inner_value: convex::Value = py_to_value(item)?;
            vec.push(inner_value);
        }
        return Ok(convex::Value::Array(vec));
    }
    if py_val.is_instance_of::<PySet>() {
        let py_set = py_val.downcast::<PySet>()?;
        let mut set: BTreeSet<convex::Value> = BTreeSet::new();
        for item in py_set {
            let inner_value: convex::Value = py_to_value(item)?;
            set.insert(inner_value);
        }
        return Ok(convex::Value::Set(set));
    }
    if py_val.is_instance_of::<PyDict>() {
        let py_map = py_val.downcast::<PyDict>()?;
        let mut map: BTreeMap<convex::Value, convex::Value> = BTreeMap::new();
        for (key, value) in py_map {
            let inner_key: convex::Value = py_to_value(key)?;
            let inner_value: convex::Value = py_to_value(value)?;
            map.insert(inner_key, inner_value);
        }
        return Ok(convex::Value::Map(map));
    }
    if py_val.is_none() {
        return Ok(convex::Value::Null);
    }

    Err(PyException::new_err("Failed to serialize to Convex value"))
}
