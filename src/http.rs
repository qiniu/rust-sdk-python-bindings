use super::{
    exceptions::{
        QiniuBodySizeMissingError, QiniuInvalidHeaderValueError, QiniuInvalidHttpVersionError,
        QiniuInvalidMethodError, QiniuInvalidURLError,
    },
    utils::{parse_headers, parse_ip_addrs, parse_method, parse_uri, PythonIoBase},
};
use pyo3::{prelude::*, types::PyDict};
use qiniu_sdk::http::{header::ToStrError, Method, Uri};
use std::{borrow::Cow, collections::HashMap, convert::TryInto};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "http")?;
    m.add_class::<SyncHttpRequestBuilder>()?;
    m.add_class::<SyncHttpRequest>()?;
    m.add_class::<Version>()?;
    Ok(m)
}

#[pyclass]
struct SyncHttpRequestBuilder(qiniu_sdk::http::SyncRequestBuilder<'static>);

#[pymethods]
impl SyncHttpRequestBuilder {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::http::SyncRequestBuilder::new())
    }

    #[pyo3(text_signature = "($self, url)")]
    fn url(&mut self, url: &str) -> PyResult<()> {
        let url = parse_uri(url)?;
        self.0.url(url);
        Ok(())
    }

    #[pyo3(text_signature = "($self, method)")]
    fn method(&mut self, method: &str) -> PyResult<()> {
        let method = parse_method(method)?;
        self.0.method(method);
        Ok(())
    }

    #[pyo3(text_signature = "($self, version)")]
    fn version(&mut self, version: Version) -> PyResult<()> {
        self.0.version(version.try_into()?);
        Ok(())
    }

    #[pyo3(text_signature = "($self, headers)")]
    fn headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        let headers = parse_headers(headers)?;
        self.0.headers(headers);
        Ok(())
    }

    #[args(len = "None")]
    #[pyo3(text_signature = "($self, body, len)")]
    fn body(&mut self, body: PyObject, len: Option<u64>, py: Python<'_>) -> PyResult<()> {
        if let Ok(body) = body.extract::<String>(py) {
            self.0.body(qiniu_sdk::http::SyncRequestBody::from(body));
        } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
            self.0.body(qiniu_sdk::http::SyncRequestBody::from(body));
        } else if let Some(len) = len {
            self.0.body(qiniu_sdk::http::SyncRequestBody::from_reader(
                PythonIoBase::new(body),
                len,
            ));
        } else {
            return Err(QiniuBodySizeMissingError::new_err("`body` must be passed"));
        }
        Ok(())
    }

    #[pyo3(text_signature = "($self, user_agent)")]
    fn appended_user_agent(&mut self, user_agent: &str) {
        self.0.appended_user_agent(user_agent);
    }

    #[pyo3(text_signature = "($self, resolved_ip_addrs)")]
    fn resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
        let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
        self.0.resolved_ip_addrs(resolved_ip_addrs);
        Ok(())
    }

    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> SyncHttpRequest {
        SyncHttpRequest(self.0.build())
    }

    #[pyo3(text_signature = "($self)")]
    fn reset(&mut self) {
        self.0.reset();
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass]
struct SyncHttpRequest(qiniu_sdk::http::SyncRequest<'static>);

#[pymethods]
impl SyncHttpRequest {
    #[new]
    #[args(fields = "**")]
    fn new(fields: Option<&PyDict>, py: Python<'_>) -> PyResult<Self> {
        let mut builder = qiniu_sdk::http::SyncRequest::builder();
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields, py)?;
        }
        Ok(Self(builder.build()))
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> SyncHttpRequestBuilder {
        SyncHttpRequestBuilder(qiniu_sdk::http::SyncRequest::builder())
    }

    #[getter]
    fn get_url(&self) -> String {
        self.0.url().to_string()
    }

    #[setter]
    fn set_url(&mut self, url: &str) -> PyResult<()> {
        *self.0.url_mut() = url
            .parse::<Uri>()
            .map_err(|err| QiniuInvalidURLError::new_err(err.to_string()))?;
        Ok(())
    }

    #[getter]
    fn get_version(&self) -> PyResult<Version> {
        self.0.version().try_into()
    }

    #[setter]
    fn set_version(&mut self, version: Version) {
        *self.0.version_mut() = version.into();
    }

    #[getter]
    fn get_method(&self) -> String {
        self.0.method().to_string()
    }

    #[setter]
    fn set_method(&mut self, method: &str) -> PyResult<()> {
        *self.0.method_mut() = method
            .parse::<Method>()
            .map_err(|err| QiniuInvalidMethodError::new_err(err.to_string()))?;
        Ok(())
    }

    #[getter]
    fn get_headers(&self) -> PyResult<HashMap<String, String>> {
        self.0
            .headers()
            .iter()
            .map(|(name, value)| {
                value
                    .to_str()
                    .map(|value| (name.to_string(), value.to_string()))
            })
            .collect::<Result<_, ToStrError>>()
            .map_err(|err| QiniuInvalidHeaderValueError::new_err(err.to_string()))
    }

    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        *self.0.headers_mut() = parse_headers(headers)?;
        Ok(())
    }

    #[setter]
    fn set_body(&mut self, body: Vec<u8>) {
        *self.0.body_mut() = qiniu_sdk::http::SyncRequestBody::from(body);
    }

    #[getter]
    fn get_user_agent(&self) -> String {
        self.0.user_agent().to_string()
    }

    #[getter]
    fn get_appended_user_agent(&self) -> String {
        self.0.appended_user_agent().to_string()
    }

    #[setter]
    fn set_appended_user_agent(&mut self, appended_user_agent: &str) {
        *self.0.appended_user_agent_mut() = appended_user_agent.into();
    }

    #[getter]
    fn get_resolved_ip_addrs(&self) -> Option<Vec<String>> {
        self.0
            .resolved_ip_addrs()
            .map(|ip_addrs| ip_addrs.iter().map(|ip_addr| ip_addr.to_string()).collect())
    }

    #[setter]
    fn set_resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
        let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
        *self.0.resolved_ip_addrs_mut() = Some(Cow::Owned(resolved_ip_addrs));
        Ok(())
    }
}

impl SyncHttpRequest {
    fn set_builder_from_py_dict(
        builder: &mut qiniu_sdk::http::SyncRequestBuilder,
        fields: &PyDict,
        py: Python<'_>,
    ) -> PyResult<()> {
        if let Some(url) = fields.get_item("url") {
            if let Ok(url) = url.extract::<&str>() {
                let url = parse_uri(url)?;
                builder.url(url);
            }
        }
        if let Some(method) = fields.get_item("method") {
            if let Ok(method) = method.extract::<&str>() {
                let method = parse_method(method)?;
                builder.method(method);
            }
        }
        if let Some(version) = fields.get_item("version") {
            if let Ok(version) = version.extract::<Version>() {
                builder.version(version.try_into()?);
            }
        }
        if let Some(headers) = fields.get_item("headers") {
            if let Ok(headers) = headers.extract::<HashMap<String, String>>() {
                let headers = parse_headers(headers)?;
                builder.headers(headers);
            }
        }
        if let Some(appended_user_agent) = fields.get_item("appended_user_agent") {
            if let Ok(appended_user_agent) = appended_user_agent.extract::<&str>() {
                builder.appended_user_agent(appended_user_agent);
            }
        }
        if let Some(resolved_ip_addrs) = fields.get_item("resolved_ip_addrs") {
            if let Ok(resolved_ip_addrs) = resolved_ip_addrs.extract::<Vec<String>>() {
                let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
                builder.resolved_ip_addrs(resolved_ip_addrs);
            }
        }
        if let Some(body) = fields.get_item("body") {
            if let Ok(body) = body.extract::<String>() {
                builder.body(qiniu_sdk::http::SyncRequestBody::from(body));
            } else if let Ok(body) = body.extract::<Vec<u8>>() {
                builder.body(qiniu_sdk::http::SyncRequestBody::from(body));
            } else if let Some(body_len) = fields.get_item("body_len") {
                if let Ok(body_len) = body_len.extract::<u64>() {
                    builder.body(qiniu_sdk::http::SyncRequestBody::from_reader(
                        PythonIoBase::new(body.into_py(py)),
                        body_len,
                    ));
                } else {
                    return Err(QiniuBodySizeMissingError::new_err("`body` must be passed"));
                }
            } else {
                return Err(QiniuBodySizeMissingError::new_err("`body` must be passed"));
            }
        }
        Ok(())
    }
}

#[pyclass]
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
enum Version {
    HTTP_09 = 9,
    HTTP_10 = 10,
    HTTP_11 = 11,
    HTTP_2 = 20,
    HTTP_3 = 30,
}

impl TryFrom<qiniu_sdk::http::Version> for Version {
    type Error = PyErr;

    fn try_from(version: qiniu_sdk::http::Version) -> Result<Self, Self::Error> {
        match version {
            qiniu_sdk::http::Version::HTTP_09 => Ok(Version::HTTP_09),
            qiniu_sdk::http::Version::HTTP_10 => Ok(Version::HTTP_10),
            qiniu_sdk::http::Version::HTTP_11 => Ok(Version::HTTP_11),
            qiniu_sdk::http::Version::HTTP_2 => Ok(Version::HTTP_2),
            qiniu_sdk::http::Version::HTTP_3 => Ok(Version::HTTP_3),
            version => Err(QiniuInvalidHttpVersionError::new_err(format!(
                "Unknown HTTP version: {:?}",
                version
            ))),
        }
    }
}

impl From<Version> for qiniu_sdk::http::Version {
    fn from(version: Version) -> Self {
        match version {
            Version::HTTP_09 => qiniu_sdk::http::Version::HTTP_09,
            Version::HTTP_10 => qiniu_sdk::http::Version::HTTP_10,
            Version::HTTP_11 => qiniu_sdk::http::Version::HTTP_11,
            Version::HTTP_2 => qiniu_sdk::http::Version::HTTP_2,
            Version::HTTP_3 => qiniu_sdk::http::Version::HTTP_3,
        }
    }
}
