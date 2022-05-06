use super::{
    exceptions::{
        QiniuBodySizeMissingError, QiniuInvalidHttpVersionError, QiniuInvalidIpAddrError,
        QiniuInvalidMethodError, QiniuInvalidStatusCodeError, QiniuInvalidURLError,
    },
    utils::{
        convert_headers_to_hashmap, parse_headers, parse_ip_addrs, parse_method, parse_port,
        parse_uri, PythonIoBase,
    },
};
use pyo3::{prelude::*, types::PyDict};
use qiniu_sdk::http::{Method, StatusCode, Uri};
use std::{
    borrow::Cow, collections::HashMap, convert::TryInto, net::IpAddr, num::NonZeroU16,
    time::Duration,
};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "http")?;
    m.add_class::<SyncHttpRequestBuilder>()?;
    m.add_class::<SyncHttpRequest>()?;
    m.add_class::<AsyncHttpRequestBuilder>()?;
    m.add_class::<AsyncHttpRequest>()?;
    m.add_class::<Version>()?;
    Ok(m)
}

macro_rules! impl_http_request_builder {
    ($name:ident) => {
        #[pymethods]
        impl $name {
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
            fn reset(&mut self) {
                self.0.reset();
            }

            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }

            // TODO: ADD `on_uploading_progress`, `on_receive_response_status`, `on_receive_response_header`
        }
    };
}

#[pyclass]
struct SyncHttpRequestBuilder(qiniu_sdk::http::SyncRequestBuilder<'static>);
impl_http_request_builder!(SyncHttpRequestBuilder);

#[pymethods]
impl SyncHttpRequestBuilder {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::http::SyncRequestBuilder::new())
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

    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> SyncHttpRequest {
        SyncHttpRequest(self.0.build())
    }
}

#[pyclass]
struct AsyncHttpRequestBuilder(qiniu_sdk::http::AsyncRequestBuilder<'static>);
impl_http_request_builder!(AsyncHttpRequestBuilder);

#[pymethods]
impl AsyncHttpRequestBuilder {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::http::AsyncRequestBuilder::new())
    }

    #[args(len = "None")]
    #[pyo3(text_signature = "($self, body, len)")]
    fn body(&mut self, body: PyObject, len: Option<u64>, py: Python<'_>) -> PyResult<()> {
        if let Ok(body) = body.extract::<String>(py) {
            self.0.body(qiniu_sdk::http::AsyncRequestBody::from(body));
        } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
            self.0.body(qiniu_sdk::http::AsyncRequestBody::from(body));
        } else if let Some(len) = len {
            self.0.body(qiniu_sdk::http::AsyncRequestBody::from_reader(
                PythonIoBase::new(body).into_async_read(),
                len,
            ));
        } else {
            return Err(QiniuBodySizeMissingError::new_err("`body` must be passed"));
        }
        Ok(())
    }

    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> AsyncHttpRequest {
        AsyncHttpRequest(self.0.build())
    }
}

macro_rules! impl_http_request {
    ($name:ident) => {
        #[pymethods]
        impl $name {
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
                convert_headers_to_hashmap(self.0.headers())
            }

            #[setter]
            fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
                *self.0.headers_mut() = parse_headers(headers)?;
                Ok(())
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

            // TODO: ADD `on_uploading_progress`, `on_receive_response_status`, `on_receive_response_header`
        }
    };
}

#[pyclass]
struct SyncHttpRequest(qiniu_sdk::http::SyncRequest<'static>);
impl_http_request!(SyncHttpRequest);

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

    #[setter]
    fn set_body(&mut self, body: Vec<u8>) {
        *self.0.body_mut() = qiniu_sdk::http::SyncRequestBody::from(body);
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
struct AsyncHttpRequest(qiniu_sdk::http::AsyncRequest<'static>);
impl_http_request!(AsyncHttpRequest);

#[pymethods]
impl AsyncHttpRequest {
    #[new]
    #[args(fields = "**")]
    fn new(fields: Option<&PyDict>, py: Python<'_>) -> PyResult<Self> {
        let mut builder = qiniu_sdk::http::AsyncRequest::builder();
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields, py)?;
        }
        Ok(Self(builder.build()))
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> AsyncHttpRequestBuilder {
        AsyncHttpRequestBuilder(qiniu_sdk::http::AsyncRequest::builder())
    }

    #[setter]
    fn set_body(&mut self, body: Vec<u8>) {
        *self.0.body_mut() = qiniu_sdk::http::AsyncRequestBody::from(body);
    }
}

impl AsyncHttpRequest {
    fn set_builder_from_py_dict(
        builder: &mut qiniu_sdk::http::AsyncRequestBuilder,
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
                builder.body(qiniu_sdk::http::AsyncRequestBody::from(body));
            } else if let Ok(body) = body.extract::<Vec<u8>>() {
                builder.body(qiniu_sdk::http::AsyncRequestBody::from(body));
            } else if let Some(body_len) = fields.get_item("body_len") {
                if let Ok(body_len) = body_len.extract::<u64>() {
                    builder.body(qiniu_sdk::http::AsyncRequestBody::from_reader(
                        PythonIoBase::new(body.into_py(py)).into_async_read(),
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

#[pyclass]
#[derive(Clone)]
struct Metrics(qiniu_sdk::http::Metrics);

#[pymethods]
impl Metrics {
    #[new]
    #[args(opts = "**")]
    fn new(opts: Option<PyObject>, py: Python<'_>) -> Self {
        let mut builder = qiniu_sdk::http::MetricsBuilder::default();
        if let Some(opts) = opts {
            if let Some(duration) = parse_duration(opts.as_ref(py), "total_duration") {
                builder.total_duration(duration);
            }
            if let Some(duration) = parse_duration(opts.as_ref(py), "name_lookup_duration") {
                builder.name_lookup_duration(duration);
            }
            if let Some(duration) = parse_duration(opts.as_ref(py), "connect_duration") {
                builder.connect_duration(duration);
            }
            if let Some(duration) = parse_duration(opts.as_ref(py), "secure_connect_duration") {
                builder.secure_connect_duration(duration);
            }
            if let Some(duration) = parse_duration(opts.as_ref(py), "redirect_duration") {
                builder.redirect_duration(duration);
            }
            if let Some(duration) = parse_duration(opts.as_ref(py), "transfer_duration") {
                builder.transfer_duration(duration);
            }
        }
        return Self(builder.build());

        fn parse_duration(opts: &PyAny, item_name: &str) -> Option<Duration> {
            if let Ok(duration) = opts.get_item(item_name) {
                if let Ok(micros) = duration.extract::<u64>() {
                    return Some(Duration::from_micros(micros));
                }
            }
            None
        }
    }

    #[getter]
    fn get_total_duration(&self) -> Option<u128> {
        self.0.total_duration().map(|duration| duration.as_micros())
    }

    #[setter]
    fn set_total_duration(&mut self, duration: u64) {
        *self.0.total_duration_mut() = Some(Duration::from_micros(duration));
    }

    #[getter]
    fn get_name_lookup_duration(&self) -> Option<u128> {
        self.0
            .name_lookup_duration()
            .map(|duration| duration.as_micros())
    }

    #[setter]
    fn set_name_lookup_duration(&mut self, duration: u64) {
        *self.0.name_lookup_duration_mut() = Some(Duration::from_micros(duration));
    }

    #[getter]
    fn get_connect_duration(&self) -> Option<u128> {
        self.0
            .connect_duration()
            .map(|duration| duration.as_micros())
    }

    #[setter]
    fn set_connect_duration(&mut self, duration: u64) {
        *self.0.connect_duration_mut() = Some(Duration::from_micros(duration));
    }

    #[getter]
    fn get_secure_connect_duration(&self) -> Option<u128> {
        self.0
            .secure_connect_duration()
            .map(|duration| duration.as_micros())
    }

    #[setter]
    fn set_secure_connect_duration(&mut self, duration: u64) {
        *self.0.secure_connect_duration_mut() = Some(Duration::from_micros(duration));
    }

    #[getter]
    fn get_redirect_duration(&self) -> Option<u128> {
        self.0
            .redirect_duration()
            .map(|duration| duration.as_micros())
    }

    #[setter]
    fn set_redirect_duration(&mut self, duration: u64) {
        *self.0.redirect_duration_mut() = Some(Duration::from_micros(duration));
    }

    #[getter]
    fn get_transfer_duration(&self) -> Option<u128> {
        self.0
            .transfer_duration()
            .map(|duration| duration.as_micros())
    }

    #[setter]
    fn set_transfer_duration(&mut self, duration: u64) {
        *self.0.transfer_duration_mut() = Some(Duration::from_micros(duration));
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass(subclass)]
struct ResponseParts(qiniu_sdk::http::ResponseParts);

#[pymethods]
impl ResponseParts {
    #[getter]
    fn get_status_code(&self) -> u16 {
        self.0.status_code().as_u16()
    }

    #[setter]
    fn set_status_code(&mut self, status_code: u16) -> PyResult<()> {
        *self.0.status_code_mut() = StatusCode::from_u16(status_code)
            .map_err(|err| QiniuInvalidStatusCodeError::new_err(err.to_string()))?;
        Ok(())
    }

    #[getter]
    fn get_headers(&self) -> PyResult<HashMap<String, String>> {
        convert_headers_to_hashmap(self.0.headers())
    }

    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        *self.0.headers_mut() = parse_headers(headers)?;
        Ok(())
    }

    #[getter]
    fn get_version(&self) -> PyResult<Version> {
        self.0
            .version()
            .try_into()
            .map_err(|err: PyErr| QiniuInvalidHttpVersionError::new_err(err.to_string()))
    }

    #[setter]
    fn set_version(&mut self, version: Version) {
        *self.0.version_mut() = version.into();
    }

    #[getter]
    fn get_server_ip(&self) -> Option<String> {
        self.0.server_ip().map(|ip| ip.to_string())
    }

    #[setter]
    fn set_server_ip(&mut self, server_ip: String) -> PyResult<()> {
        *self.0.server_ip_mut() = server_ip
            .parse::<IpAddr>()
            .map(Some)
            .map_err(|err| QiniuInvalidIpAddrError::new_err(err.to_string()))?;
        Ok(())
    }

    #[getter]
    fn get_server_port(&self) -> Option<u16> {
        self.0.server_port().map(|ip| ip.get())
    }

    #[setter]
    fn set_server_port(&mut self, server_port: u16) {
        *self.0.server_port_mut() = NonZeroU16::new(server_port);
    }

    #[getter]
    fn get_metrics(&self) -> Option<Metrics> {
        self.0.metrics().cloned().map(Metrics)
    }

    #[setter]
    fn set_metrics(&mut self, metrics: Metrics) {
        *self.0.metrics_mut() = Some(metrics.0);
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass(extends = ResponseParts)]
struct SyncHttpResponse(qiniu_sdk::http::SyncResponseBody);

#[pymethods]
impl SyncHttpResponse {
    #[new]
    #[args(opts = "**")]
    pub fn new(
        opts: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<(SyncHttpResponse, ResponseParts)> {
        let mut builder = qiniu_sdk::http::Response::builder();
        if let Some(opts) = opts {
            if let Ok(status_code) = opts.as_ref(py).get_item("status_code") {
                if let Ok(status_code) = status_code.extract::<u16>() {
                    builder
                        .status_code(StatusCode::from_u16(status_code).map_err(|err| {
                            QiniuInvalidStatusCodeError::new_err(err.to_string())
                        })?);
                }
            }
            if let Ok(headers) = opts.as_ref(py).get_item("headers") {
                if let Ok(headers) = headers.extract::<HashMap<String, String>>() {
                    let headers = parse_headers(headers)?;
                    builder.headers(headers);
                }
            }
            if let Ok(version) = opts.as_ref(py).get_item("version") {
                if let Ok(version) = version.extract::<Version>() {
                    builder.version(version.into());
                }
            }
            if let Ok(server_ip) = opts.as_ref(py).get_item("server_ip") {
                if let Ok(server_ip) = server_ip.extract::<String>() {
                    builder.server_ip(
                        server_ip
                            .parse::<IpAddr>()
                            .map_err(|err| QiniuInvalidIpAddrError::new_err(err.to_string()))?,
                    );
                }
            }
            if let Ok(server_port) = opts.as_ref(py).get_item("server_port") {
                if let Ok(server_port) = server_port.extract::<u16>() {
                    let server_port = parse_port(server_port)?;
                    builder.server_port(server_port);
                }
            }
            if let Ok(body) = opts.as_ref(py).get_item("body") {
                if let Ok(body) = body.extract::<String>() {
                    builder.body(qiniu_sdk::http::SyncResponseBody::from_bytes(
                        body.into_bytes(),
                    ));
                } else if let Ok(body) = body.extract::<Vec<u8>>() {
                    builder.body(qiniu_sdk::http::SyncResponseBody::from_bytes(body));
                } else {
                    builder.body(qiniu_sdk::http::SyncResponseBody::from_reader(
                        PythonIoBase::new(body.to_object(py)),
                    ));
                }
            }
            if let Ok(metrics) = opts.as_ref(py).get_item("metrics") {
                if let Ok(metrics) = metrics.extract::<Metrics>() {
                    builder.metrics(metrics.0);
                }
            }
        }
        let (parts, body) = builder.build().into_parts_and_body();
        Ok((Self(body), ResponseParts(parts)))
    }
}

#[pyclass(extends = ResponseParts)]
struct AsyncHttpResponse(qiniu_sdk::http::AsyncResponseBody);

#[pymethods]
impl AsyncHttpResponse {
    #[new]
    #[args(opts = "**")]
    pub fn new(
        opts: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<(AsyncHttpResponse, ResponseParts)> {
        let mut builder = qiniu_sdk::http::Response::builder();
        if let Some(opts) = opts {
            if let Ok(status_code) = opts.as_ref(py).get_item("status_code") {
                if let Ok(status_code) = status_code.extract::<u16>() {
                    builder
                        .status_code(StatusCode::from_u16(status_code).map_err(|err| {
                            QiniuInvalidStatusCodeError::new_err(err.to_string())
                        })?);
                }
            }
            if let Ok(headers) = opts.as_ref(py).get_item("headers") {
                if let Ok(headers) = headers.extract::<HashMap<String, String>>() {
                    let headers = parse_headers(headers)?;
                    builder.headers(headers);
                }
            }
            if let Ok(version) = opts.as_ref(py).get_item("version") {
                if let Ok(version) = version.extract::<Version>() {
                    builder.version(version.into());
                }
            }
            if let Ok(server_ip) = opts.as_ref(py).get_item("server_ip") {
                if let Ok(server_ip) = server_ip.extract::<String>() {
                    builder.server_ip(
                        server_ip
                            .parse::<IpAddr>()
                            .map_err(|err| QiniuInvalidIpAddrError::new_err(err.to_string()))?,
                    );
                }
            }
            if let Ok(server_port) = opts.as_ref(py).get_item("server_port") {
                if let Ok(server_port) = server_port.extract::<u16>() {
                    let server_port = parse_port(server_port)?;
                    builder.server_port(server_port);
                }
            }
            if let Ok(body) = opts.as_ref(py).get_item("body") {
                if let Ok(body) = body.extract::<String>() {
                    builder.body(qiniu_sdk::http::AsyncResponseBody::from_bytes(
                        body.into_bytes(),
                    ));
                } else if let Ok(body) = body.extract::<Vec<u8>>() {
                    builder.body(qiniu_sdk::http::AsyncResponseBody::from_bytes(body));
                } else {
                    builder.body(qiniu_sdk::http::AsyncResponseBody::from_reader(
                        PythonIoBase::new(body.to_object(py)).into_async_read(),
                    ));
                }
            }
            if let Ok(metrics) = opts.as_ref(py).get_item("metrics") {
                if let Ok(metrics) = metrics.extract::<Metrics>() {
                    builder.metrics(metrics.0);
                }
            }
        }
        let (parts, body) = builder.build().into_parts_and_body();
        Ok((Self(body), ResponseParts(parts)))
    }
}
