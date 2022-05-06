use super::{
    credential::CredentialProvider,
    exceptions::{
        QiniuBase64Error, QiniuCallbackError, QiniuIoError, QiniuJsonError, QiniuTimeError,
        QiniuUnknownError, QiniuUnsupportedTypeError, QiniuUploadTokenFormatError,
    },
};
use pyo3::{
    prelude::*,
    types::{PyDict, PyString, PyTuple},
};
use qiniu_sdk::{
    prelude::UploadTokenProviderExt,
    upload_token::{FileType, ParseError, ToStringError},
};
use std::{
    mem::take,
    time::{Duration, SystemTime},
};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload_token")?;
    m.add_class::<UploadPolicy>()?;
    m.add_class::<UploadPolicyBuilder>()?;
    m.add_class::<UploadTokenProvider>()?;
    m.add_class::<GetAccessKeyOptions>()?;
    m.add_class::<GetPolicyOptions>()?;
    m.add_class::<ToStringOptions>()?;
    m.add_class::<StaticUploadTokenProvider>()?;
    m.add_class::<FromUploadPolicy>()?;
    m.add_class::<BucketUploadTokenProvider>()?;
    m.add_class::<ObjectUploadTokenProvider>()?;
    Ok(m)
}

#[pyclass]
#[derive(Clone)]
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
            .map_err(|err| QiniuJsonError::new_err(err.to_string()))?;
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
            .map_err(|err| QiniuTimeError::new_err(err.to_string()))
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

    #[pyo3(text_signature = "($self)")]
    fn to_upload_token_provider(&self, credential: CredentialProvider) -> UploadTokenProvider {
        UploadTokenProvider(Box::new(
            self.to_owned()
                .0
                .into_dynamic_upload_token_provider(credential.into_inner()),
        ))
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
        Err(QiniuUnsupportedTypeError::new_err(format!(
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
                Err(QiniuUnsupportedTypeError::new_err(format!(
                    "Unsupported number type: {:?}",
                    n
                )))
            }
        }
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        v => Err(QiniuUnsupportedTypeError::new_err(format!(
            "Unsupported type: {:?}",
            v
        ))),
    }
}

#[pyclass]
#[derive(Clone)]
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

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ToPyObject for UploadPolicyBuilder {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.to_owned().into_py(py)
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

#[pyclass(subclass)]
#[derive(Clone)]
struct UploadTokenProvider(Box<dyn qiniu_sdk::upload_token::UploadTokenProvider>);

#[pymethods]
impl UploadTokenProvider {
    #[args(opts = "GetAccessKeyOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn access_key(&self, opts: GetAccessKeyOptions) -> PyResult<String> {
        Ok(self
            .0
            .access_key(opts.0)
            .map_err(convert_parse_error_to_py_err)?
            .into_access_key()
            .to_string())
    }

    #[args(opts = "GetPolicyOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn policy(&self, opts: GetPolicyOptions) -> PyResult<UploadPolicy> {
        Ok(UploadPolicy(
            self.0
                .policy(opts.0)
                .map_err(convert_parse_error_to_py_err)?
                .into_upload_policy(),
        ))
    }

    #[args(opts = "ToStringOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn to_token_string(&self, opts: ToStringOptions) -> PyResult<String> {
        Ok(self
            .0
            .to_token_string(opts.0)
            .map_err(convert_to_string_error_to_py_err)?
            .into_owned())
    }

    #[args(opts = "GetPolicyOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn bucket_name(&self, opts: GetPolicyOptions) -> PyResult<String> {
        Ok(self
            .0
            .bucket_name(opts.0)
            .map_err(convert_parse_error_to_py_err)?
            .to_string())
    }

    #[args(opts = "GetAccessKeyOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn async_access_key<'p>(
        &self,
        opts: GetAccessKeyOptions,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(provider
                .async_access_key(opts.0)
                .await
                .map_err(convert_parse_error_to_py_err)?
                .into_access_key()
                .to_string())
        })
    }

    #[args(opts = "GetPolicyOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn async_policy<'p>(&self, opts: GetPolicyOptions, py: Python<'p>) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(UploadPolicy(
                provider
                    .async_policy(opts.0)
                    .await
                    .map_err(convert_parse_error_to_py_err)?
                    .into_upload_policy(),
            ))
        })
    }

    #[args(opts = "ToStringOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn async_to_token_string<'p>(
        &self,
        opts: ToStringOptions,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(provider
                .async_to_token_string(opts.0)
                .await
                .map_err(convert_to_string_error_to_py_err)?
                .into_owned())
        })
    }

    #[args(opts = "GetPolicyOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn async_bucket_name<'p>(&self, opts: GetPolicyOptions, py: Python<'p>) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(provider
                .async_bucket_name(opts.0)
                .await
                .map_err(convert_parse_error_to_py_err)?
                .to_string())
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_token_string(Default::default())
    }
}

fn convert_parse_error_to_py_err(err: ParseError) -> PyErr {
    match err {
        ParseError::CredentialGetError(err) => QiniuIoError::new_err(err),
        ParseError::InvalidUploadTokenFormat => {
            QiniuUploadTokenFormatError::new_err(err.to_string())
        }
        ParseError::Base64DecodeError(err) => QiniuBase64Error::new_err(err.to_string()),
        ParseError::JsonDecodeError(err) => QiniuJsonError::new_err(err.to_string()),
        ParseError::CallbackError(err) => QiniuCallbackError::new_err(err.to_string()),
        err => QiniuUnknownError::new_err(err.to_string()),
    }
}

fn convert_to_string_error_to_py_err(err: ToStringError) -> PyErr {
    match err {
        ToStringError::CredentialGetError(err) => QiniuIoError::new_err(err),
        ToStringError::CallbackError(err) => QiniuCallbackError::new_err(err.to_string()),
        err => QiniuUnknownError::new_err(err.to_string()),
    }
}

#[pyclass(extends = UploadTokenProvider)]
struct StaticUploadTokenProvider;

#[pymethods]
impl StaticUploadTokenProvider {
    #[new]
    fn new(upload_token: &str) -> (Self, UploadTokenProvider) {
        (
            Self,
            UploadTokenProvider(Box::new(
                qiniu_sdk::upload_token::StaticUploadTokenProvider::new(upload_token),
            )),
        )
    }
}

#[pyclass(extends = UploadTokenProvider)]
struct FromUploadPolicy;

#[pymethods]
impl FromUploadPolicy {
    #[new]
    fn new(
        upload_policy: UploadPolicy,
        credential: CredentialProvider,
    ) -> (Self, UploadTokenProvider) {
        (
            Self,
            UploadTokenProvider(Box::new(qiniu_sdk::upload_token::FromUploadPolicy::new(
                upload_policy.0,
                credential.into_inner(),
            ))),
        )
    }
}

#[pyclass(extends = UploadTokenProvider)]
struct BucketUploadTokenProvider;

#[pymethods]
impl BucketUploadTokenProvider {
    #[new]
    #[args(opts = "**")]
    fn new(
        bucket: &str,
        upload_token_lifetime: u64,
        credential: CredentialProvider,
        opts: Option<&PyDict>,
    ) -> (Self, UploadTokenProvider) {
        let mut builder = qiniu_sdk::upload_token::BucketUploadTokenProvider::builder(
            bucket,
            Duration::from_secs(upload_token_lifetime),
            credential.into_inner(),
        );
        if let Some(opts) = opts {
            if let Some(any) = opts.get_item("on_policy_generated") {
                builder = set_on_policy_generated_to_builder(builder, any);
            }
        }
        let provider = builder.build();
        return (Self, UploadTokenProvider(Box::new(provider)));

        fn set_on_policy_generated_to_builder<'a, C: Clone + 'a>(
            mut builder: qiniu_sdk::upload_token::BucketUploadTokenProviderBuilder<'a, C>,
            any: &PyAny,
        ) -> qiniu_sdk::upload_token::BucketUploadTokenProviderBuilder<'a, C> {
            if any.is_callable() {
                builder = Python::with_gil(|py| {
                    let obj = any.to_object(py);
                    builder.on_policy_generated(move |upload_policy_builder| {
                        let builder = UploadPolicyBuilder(take(upload_policy_builder));
                        let builder = Python::with_gil(|py| {
                            obj.call1(py, PyTuple::new(py, [builder]))
                                .and_then(|retval| retval.extract::<UploadPolicyBuilder>(py))
                        })?;
                        *upload_policy_builder = builder.0;
                        Ok(())
                    })
                });
            }
            builder
        }
    }
}

#[pyclass(extends = UploadTokenProvider)]
struct ObjectUploadTokenProvider;

#[pymethods]
impl ObjectUploadTokenProvider {
    #[new]
    #[args(opts = "**")]
    fn new(
        bucket: &str,
        object: &str,
        upload_token_lifetime: u64,
        credential: CredentialProvider,
        opts: Option<&PyDict>,
    ) -> (Self, UploadTokenProvider) {
        let mut builder = qiniu_sdk::upload_token::ObjectUploadTokenProvider::builder(
            bucket,
            object,
            Duration::from_secs(upload_token_lifetime),
            credential.into_inner(),
        );
        if let Some(opts) = opts {
            if let Some(any) = opts.get_item("on_policy_generated") {
                builder = set_on_policy_generated_to_builder(builder, any);
            }
        }
        let provider = builder.build();
        return (Self, UploadTokenProvider(Box::new(provider)));

        fn set_on_policy_generated_to_builder<'a, C: Clone + 'a>(
            mut builder: qiniu_sdk::upload_token::ObjectUploadTokenProviderBuilder<'a, C>,
            any: &PyAny,
        ) -> qiniu_sdk::upload_token::ObjectUploadTokenProviderBuilder<'a, C> {
            if any.is_callable() {
                builder = Python::with_gil(|py| {
                    let obj = any.to_object(py);
                    builder.on_policy_generated(move |upload_policy_builder| {
                        let builder = UploadPolicyBuilder(take(upload_policy_builder));
                        let builder = Python::with_gil(|py| {
                            obj.call1(py, PyTuple::new(py, [builder]))
                                .and_then(|retval| retval.extract::<UploadPolicyBuilder>(py))
                        })?;
                        *upload_policy_builder = builder.0;
                        Ok(())
                    })
                });
            }
            builder
        }
    }
}

#[pyclass]
#[derive(Default, Copy, Clone)]
struct GetAccessKeyOptions(qiniu_sdk::upload_token::GetAccessKeyOptions);

#[pymethods]
impl GetAccessKeyOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass]
#[derive(Default, Copy, Clone)]
struct GetPolicyOptions(qiniu_sdk::upload_token::GetPolicyOptions);

#[pymethods]
impl GetPolicyOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass]
#[derive(Default, Copy, Clone)]
struct ToStringOptions(qiniu_sdk::upload_token::ToStringOptions);

#[pymethods]
impl ToStringOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
