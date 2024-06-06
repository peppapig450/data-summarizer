use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

// Convert PyDict to a serde_json::Value
fn pydict_to_value(py_dict: &PyDict) -> Result<Value, PyErr> {
    let mut map = Map::new();
    for (key, value) in py_dict {
        let key: String = key.extract()?;
        let value = py_to_value(value)?;
        map.insert(key, value);
    }
    Ok(Value::Object(map))
}

// Convert PyValue to a serde_json::Value
fn py_to_value(py_value: &PyAny) -> Result<Value, PyErr> {
    if let Ok(py_dict) = py_value.downcast::<PyDict>() {
        pydict_to_value(py_dict)
    } else if let Ok(py_str) = py_value.extract::<String>() {
        Ok(Value::String(py_str))
    } else if let Ok(py_int) = py_value.extract::<i64>() {
        Ok(Value::Number(serde_json::Number::from(py_int)))
    } else if let Ok(py_float) = py_value.extract::<f64>() {
        Ok(Value::Number(
            serde_json::Number::from_f64(py_float).unwrap(),
        ))
    } else if let Ok(py_bool) = py_value.extract::<bool>() {
        Ok(Value::Bool(py_bool))
    } else if py_value.is_none() {
        Ok(Value::Null)
    } else {
        Err(pyo3::exceptions::PyValueError::new_err("Unsupported type."))
    }
}

// Recursively summarize a json structure
fn summarize_value(
    value: &Value,
    depth: usize,
    type_counts: &mut HashMap<String, usize>,
    nested_dicts: &mut usize,
    nested_lists_with_dicts: &mut usize,
) -> (usize, HashSet<String>) {
    let mut size = 0;
    let mut keys = HashSet::new();

    match value {
        Value::Object(map) => {
            size += map.len();
            *type_counts.entry("Object".to_string()).or_insert(0) += 1;
            *nested_dicts += 1;
            for (key, val) in map {
                keys.insert(key.clone());
                let (sub_size, sub_keys) = summarize_value(
                    val,
                    depth + 1,
                    type_counts,
                    nested_dicts,
                    nested_lists_with_dicts,
                );
                size += sub_size;
                keys.extend(sub_keys);
            }
        }
        Value::Array(arr) => {
            *type_counts.entry("Array".to_string()).or_insert(0) += 1;
            for val in arr {
                if let Value::Object(_) = val {
                    *nested_lists_with_dicts += 1;
                }
                let (sub_size, sub_keys) = summarize_value(
                    val,
                    depth + 1,
                    type_counts,
                    nested_dicts,
                    nested_lists_with_dicts,
                );
                size += sub_size;
                keys.extend(sub_keys);
            }
        }
        Value::String(_) => {
            *type_counts.entry("String".to_string()).or_insert(0) += 1;
            size += 1;
        }
        Value::Number(_) => {
            *type_counts.entry("Number".to_string()).or_insert(0) += 1;
            size += 1;
        }
        Value::Bool(_) => {
            *type_counts.entry("Boolean".to_string()).or_insert(0) += 1;
            size += 1;
        }
        Value::Null => {
            *type_counts.entry("Null".to_string()).or_insert(0) += 1;
            size += 1;
        }
    }
    (size, keys)
}

#[pyfunction]
fn summarize_large_json(
    py_dict: &PyDict,
) -> PyResult<(usize, Vec<String>, usize, HashMap<String, usize>, usize)> {
    // Convert PyDict to a HashMap<String, Value>
    let json_value = pydict_to_value(py_dict)?;

    let mut type_counts = HashMap::new();
    let mut nested_dicts = 0;
    let mut nested_lists_with_dicts = 0;
    let (size, keys) = summarize_value(
        &json_value,
        0,
        &mut type_counts,
        &mut nested_dicts,
        &mut nested_lists_with_dicts,
    );
    Ok((
        size,
        keys.into_iter().collect(),
        nested_dicts,
        type_counts,
        nested_lists_with_dicts,
    ))
}

/// A Python module implemented in Rust.
#[pymodule]
fn data_summarizer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(summarize_large_json, m)?)?;
    Ok(())
}
