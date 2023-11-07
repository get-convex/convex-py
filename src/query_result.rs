use pyo3::{
    exceptions::PyException,
    types::{
        PyBool,
        PyBytes,
        PyDict,
        PyFloat,
        PyInt,
        PyList,
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
    if py_val.is_none() {
        return Ok(convex::Value::Null);
    }

    Err(PyException::new_err("Failed to serialize to Convex value"))
}
