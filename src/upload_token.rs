use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyDict, PyString},
};
use qiniu_sdk::upload_token::UploadPolicyBuilder;
use std::time::{Duration, SystemTime};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload_token")?;
    m.add_class::<UploadPolicy>()?;
    Ok(m)
}

#[pyclass]
struct UploadPolicy(qiniu_sdk::upload_token::UploadPolicy);

#[pymethods]
impl UploadPolicy {
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, lifetime, **fields)")]
    fn new_for_bucket(
        bucket: &str,
        upload_token_lifetime: u64,
        fields: Option<&PyDict>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::upload_token::UploadPolicy::new_for_bucket(
            bucket,
            Duration::from_secs(upload_token_lifetime),
        );
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields)?;
        }
        Ok(UploadPolicy(builder.build()))
    }

    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, object, lifetime, **fields)")]
    fn new_for_object(
        bucket: &str,
        object: &str,
        upload_token_lifetime: u64,
        fields: Option<&PyDict>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::upload_token::UploadPolicy::new_for_object(
            bucket,
            object,
            Duration::from_secs(upload_token_lifetime),
        );
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields)?;
        }
        Ok(UploadPolicy(builder.build()))
    }

    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, prefix, lifetime, **fields)")]
    fn new_for_objects_with_prefix(
        bucket: &str,
        prefix: &str,
        upload_token_lifetime: u64,
        fields: Option<&PyDict>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::upload_token::UploadPolicy::new_for_objects_with_prefix(
            bucket,
            prefix,
            Duration::from_secs(upload_token_lifetime),
        );
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields)?;
        }
        Ok(UploadPolicy(builder.build()))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let policy = qiniu_sdk::upload_token::UploadPolicy::from_json(json)
            .map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(UploadPolicy(policy))
    }

    #[pyo3(text_signature = "($self)")]
    fn bucket<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.bucket().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn key<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.key().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn use_prefixal_object_key(&self) -> bool {
        self.0.use_prefixal_object_key()
    }

    #[pyo3(text_signature = "($self)")]
    fn is_insert_only(&self) -> bool {
        self.0.is_insert_only()
    }

    #[pyo3(text_signature = "($self)")]
    fn mime_detection_enabled(&self) -> bool {
        self.0.mime_detection_enabled()
    }

    #[pyo3(text_signature = "($self)")]
    fn token_deadline(&self) -> PyResult<Option<u64>> {
        self.0
            .token_deadline()
            .map(|deadline| {
                deadline
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
            })
            .map_or(Ok(None), |v| v.map(Some))
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    #[pyo3(text_signature = "($self)")]
    fn return_url<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.return_url().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn return_body<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.return_body().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn callback_urls<'p>(&self, py: Python<'p>) -> Option<Vec<&'p PyString>> {
        self.0
            .callback_urls()
            .map(|url| url.map(|s| PyString::new(py, s)).collect())
    }

    #[pyo3(text_signature = "($self)")]
    fn callback_host<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.callback_host().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn callback_body<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.callback_body().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn callback_body_type<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.callback_body_type().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn save_key<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.save_key().map(|s| PyString::new(py, s))
    }

    #[pyo3(text_signature = "($self)")]
    fn is_save_key_forced(&self) -> bool {
        self.0.is_save_key_forced()
    }

    #[pyo3(text_signature = "($self)")]
    fn maximum_file_size(&self) -> Option<u64> {
        self.0.file_size_limitation().1
    }

    #[pyo3(text_signature = "($self)")]
    fn minimum_file_size(&self) -> Option<u64> {
        self.0.file_size_limitation().0
    }

    #[pyo3(text_signature = "($self)")]
    fn mime_types<'p>(&self, py: Python<'p>) -> Option<Vec<&'p PyString>> {
        self.0
            .mime_types()
            .map(|mime_type| mime_type.map(|s| PyString::new(py, s)).collect())
    }

    #[pyo3(text_signature = "($self)")]
    fn file_type(&self) -> Option<u8> {
        self.0.file_type().map(|ft| ft.into())
    }

    #[pyo3(text_signature = "($self)")]
    fn object_lifetime(&self) -> Option<u64> {
        self.0.object_lifetime().map(|dur| dur.as_secs())
    }

    #[pyo3(text_signature = "($self)")]
    fn as_json(&self) -> String {
        self.0.as_json()
    }

    #[pyo3(text_signature = "($self, key)")]
    fn get(&self, key: &str, py: Python<'_>) -> PyResult<Option<PyObject>> {
        self.0
            .get(key)
            .map(|v| convert_json_value_to_py_object(v, py))
            .map_or(Ok(None), |v| v.map(Some))
    }

    #[pyo3(text_signature = "($self)")]
    fn keys<'p>(&self, py: Python<'p>) -> Vec<&'p PyString> {
        self.0.keys().map(|s| PyString::new(py, s)).collect()
    }

    #[pyo3(text_signature = "($self)")]
    fn values(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.0
            .values()
            .map(|v| convert_json_value_to_py_object(v, py))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl UploadPolicy {
    fn set_builder_from_py_dict(
        builder: &mut UploadPolicyBuilder,
        fields: &PyDict,
    ) -> PyResult<()> {
        for key in fields.keys().iter() {
            if let Some(value) = fields.get_item(key) {
                builder.set(
                    key.extract::<String>()?,
                    convert_py_any_to_json_value(value)?,
                );
            }
        }
        Ok(())
    }
}

fn convert_py_any_to_json_value(any: &PyAny) -> PyResult<serde_json::Value> {
    // TODO: extract all possible values
    if let Ok(value) = any.extract::<String>() {
        Ok(serde_json::Value::from(value))
    } else if let Ok(value) = any.extract::<bool>() {
        Ok(serde_json::Value::from(value))
    } else if let Ok(value) = any.extract::<u64>() {
        Ok(serde_json::Value::from(value))
    } else if let Ok(value) = any.extract::<i64>() {
        Ok(serde_json::Value::from(value))
    } else if let Ok(value) = any.extract::<f64>() {
        Ok(serde_json::Value::from(value))
    } else {
        Err(PyValueError::new_err(format!(
            "Unsupported type: {:?}",
            any
        )))
    }
}

fn convert_json_value_to_py_object(
    value: &serde_json::Value,
    py: Python<'_>,
) -> PyResult<PyObject> {
    // TODO: convert all possible values
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Number(n) => {
            if let Some(n) = n.as_u64() {
                Ok(n.to_object(py))
            } else if let Some(n) = n.as_i64() {
                Ok(n.to_object(py))
            } else if let Some(n) = n.as_f64() {
                Ok(n.to_object(py))
            } else {
                Err(PyValueError::new_err(format!(
                    "Unsupported number type: {:?}",
                    n
                )))
            }
        }
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        v => Err(PyValueError::new_err(format!("Unsupported type: {:?}", v))),
    }
}
