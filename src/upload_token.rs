use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyDict, PyString},
};
use qiniu_sdk::upload_token::FileType;
use std::time::{Duration, SystemTime};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload_token")?;
    m.add_class::<UploadPolicy>()?;
    m.add_class::<UploadPolicyBuilder>()?;
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
    ) -> PyResult<UploadPolicyBuilder> {
        UploadPolicyBuilder::new_for_bucket(bucket, upload_token_lifetime, fields)
    }

    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, object, lifetime, **fields)")]
    fn new_for_object(
        bucket: &str,
        object: &str,
        upload_token_lifetime: u64,
        fields: Option<&PyDict>,
    ) -> PyResult<UploadPolicyBuilder> {
        UploadPolicyBuilder::new_for_object(bucket, object, upload_token_lifetime, fields)
    }

    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, prefix, lifetime, **fields)")]
    fn new_for_objects_with_prefix(
        bucket: &str,
        prefix: &str,
        upload_token_lifetime: u64,
        fields: Option<&PyDict>,
    ) -> PyResult<UploadPolicyBuilder> {
        UploadPolicyBuilder::new_for_objects_with_prefix(
            bucket,
            prefix,
            upload_token_lifetime,
            fields,
        )
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

#[pyclass]
struct UploadPolicyBuilder(qiniu_sdk::upload_token::UploadPolicyBuilder);

#[pymethods]
impl UploadPolicyBuilder {
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
        Ok(Self(builder))
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
        Ok(Self(builder))
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
        Ok(Self(builder))
    }

    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> UploadPolicy {
        UploadPolicy(self.0.build())
    }

    #[pyo3(text_signature = "($self, lifetime)")]
    fn token_lifetime(&mut self, lifetime: u64) {
        self.0.token_lifetime(Duration::from_secs(lifetime));
    }

    #[pyo3(text_signature = "($self, deadline)")]
    fn token_deadline(&mut self, deadline: u64) {
        self.0
            .token_deadline(SystemTime::UNIX_EPOCH + Duration::from_secs(deadline));
    }

    #[pyo3(text_signature = "($self)")]
    fn insert_only(&mut self) {
        self.0.insert_only();
    }

    #[pyo3(text_signature = "($self)")]
    fn enable_mime_detection(&mut self) {
        self.0.enable_mime_detection();
    }

    #[pyo3(text_signature = "($self)")]
    fn disable_mime_detection(&mut self) {
        self.0.disable_mime_detection();
    }

    #[pyo3(text_signature = "($self, file_type)")]
    fn file_type(&mut self, file_type: u8) {
        self.0.file_type(FileType::from(file_type));
    }

    #[pyo3(text_signature = "($self, url)")]
    fn return_url(&mut self, url: &str) {
        self.0.return_url(url);
    }

    #[pyo3(text_signature = "($self, body)")]
    fn return_body(&mut self, body: &str) {
        self.0.return_body(body);
    }

    #[args(host = "\"\"", body = "\"\"", body_type = "\"\"")]
    #[pyo3(text_signature = "($self, urls, host, body, body_type)")]
    fn callback(&mut self, urls: Vec<String>, host: &str, body: &str, body_type: &str) {
        self.0.callback(urls, host, body, body_type);
    }

    #[args(force = "false")]
    #[pyo3(text_signature = "($self, save_as, force)")]
    fn save_as(&mut self, save_as: &str, force: bool) {
        self.0.save_as(save_as, force);
    }

    #[args(min = "None", max = "None")]
    #[pyo3(text_signature = "($self, min, max)")]
    fn file_size_limitation(&mut self, min: Option<u64>, max: Option<u64>) {
        match (min, max) {
            (Some(min), Some(max)) => {
                self.0.file_size_limitation(min..=max);
            }
            (Some(min), None) => {
                self.0.file_size_limitation(min..);
            }
            (None, Some(max)) => {
                self.0.file_size_limitation(..=max);
            }
            _ => {}
        }
    }

    #[args(force = "false")]
    #[pyo3(text_signature = "($self, content_types)")]
    fn mime_types(&mut self, content_types: Vec<String>) {
        self.0.mime_types(content_types);
    }

    #[pyo3(text_signature = "($self, lifetime)")]
    fn object_lifetime(&mut self, lifetime: u64) {
        self.0.object_lifetime(Duration::from_secs(lifetime));
    }
}

impl UploadPolicyBuilder {
    fn set_builder_from_py_dict(
        builder: &mut qiniu_sdk::upload_token::UploadPolicyBuilder,
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
