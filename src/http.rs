use super::{
    exceptions::{
        QiniuBodySizeMissingError, QiniuDataLockedError, QiniuHttpCallError,
        QiniuInvalidHeaderValueError, QiniuInvalidHttpVersionError, QiniuInvalidIpAddrError,
        QiniuInvalidMethodError, QiniuInvalidURLError, QiniuIsahcError,
    },
    utils::{
        convert_headers_to_hashmap, extract_async_request_body, extract_async_response_body,
        extract_sync_request_body, extract_sync_response_body, parse_headers, parse_ip_addr,
        parse_ip_addrs, parse_method, parse_port, parse_status_code, parse_uri, PythonIoBase,
    },
};
use futures::lock::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
use futures::AsyncReadExt;
use pyo3::{
    exceptions::{PyIOError, PyNotImplementedError},
    prelude::*,
    types::{PyBytes, PyTuple},
};
use qiniu_sdk::http::{Method, Uri};
use std::{
    borrow::Cow,
    collections::HashMap,
    convert::TryInto,
    io::Read,
    net::IpAddr,
    num::NonZeroU16,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "http")?;
    m.add_class::<HttpCaller>()?;
    m.add_class::<IsahcHttpCaller>()?;
    m.add_class::<SyncHttpRequestBuilder>()?;
    m.add_class::<SyncHttpRequest>()?;
    m.add_class::<AsyncHttpRequestBuilder>()?;
    m.add_class::<AsyncHttpRequest>()?;
    m.add_class::<Version>()?;
    m.add_class::<Metrics>()?;
    m.add_class::<ResponseParts>()?;
    m.add_class::<SyncHttpResponse>()?;
    m.add_class::<AsyncHttpResponse>()?;
    Ok(m)
}

/// HTTP 请求处理接口
///
/// 实现该接口，即可处理所有七牛 SDK 发送的 HTTP 请求
#[pyclass(subclass)]
pub(super) struct HttpCaller(Arc<dyn qiniu_sdk::http::HttpCaller>);

#[pymethods]
impl HttpCaller {
    /// 阻塞发送 HTTP 请求
    #[pyo3(text_signature = "($self, request)")]
    fn call(
        &self,
        request: &mut SyncHttpRequest,
        py: Python<'_>,
    ) -> PyResult<Py<SyncHttpResponse>> {
        let response = py.allow_threads(|| {
            self.0
                .call(&mut request.0)
                .map_err(|err| QiniuHttpCallError::new_err(err.to_string()))
        })?;
        let (parts, body) = response.into_parts_and_body();
        Py::new(py, (SyncHttpResponse(body), ResponseParts(parts)))
    }

    /// 异步发送 HTTP 请求
    #[pyo3(text_signature = "($self, request)")]
    fn async_call<'p>(&self, request: AsyncHttpRequest, py: Python<'p>) -> PyResult<&'p PyAny> {
        let http_caller = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let response = http_caller
                .async_call(&mut *request.0.lock().await)
                .await
                .map_err(|err| QiniuHttpCallError::new_err(err.to_string()))?;
            let (parts, body) = response.into_parts_and_body();
            Python::with_gil(|py| {
                Py::new(
                    py,
                    (
                        AsyncHttpResponse(Arc::new(AsyncMutex::new(body))),
                        ResponseParts(parts),
                    ),
                )
            })
        })
    }
}

/// 七牛 Isahc HTTP 客户端实现
///
/// 基于 Isahc 库提供 HTTP 客户端接口实现
#[pyclass(extends = HttpCaller)]
#[pyo3(text_signature = "()")]
struct IsahcHttpCaller;

#[pymethods]
impl IsahcHttpCaller {
    #[new]
    fn new() -> PyResult<(Self, HttpCaller)> {
        Ok((
            IsahcHttpCaller,
            HttpCaller(Arc::new(
                qiniu_sdk::isahc::Client::default_client()
                    .map_err(|err| QiniuIsahcError::new_err(err.to_string()))?,
            )),
        ))
    }
}

macro_rules! impl_http_request_builder {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// 设置请求 URL
            #[pyo3(text_signature = "($self, url)")]
            fn url(&mut self, url: &str) -> PyResult<()> {
                let url = parse_uri(url)?;
                self.0.url(url);
                Ok(())
            }

            /// 设置请求 HTTP 方法
            #[pyo3(text_signature = "($self, method)")]
            fn method(&mut self, method: &str) -> PyResult<()> {
                let method = parse_method(method)?;
                self.0.method(method);
                Ok(())
            }

            /// 设置请求 HTTP 版本
            #[pyo3(text_signature = "($self, version)")]
            fn version(&mut self, version: Version) -> PyResult<()> {
                self.0.version(version.try_into()?);
                Ok(())
            }

            /// 设置请求 HTTP Headers
            #[pyo3(text_signature = "($self, headers)")]
            fn headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
                let headers = parse_headers(headers)?;
                self.0.headers(headers);
                Ok(())
            }

            /// 设置用户代理
            #[pyo3(text_signature = "($self, user_agent)")]
            fn appended_user_agent(&mut self, user_agent: &str) {
                self.0.appended_user_agent(user_agent);
            }

            /// 设置预解析的服务器套接字地址
            #[pyo3(text_signature = "($self, resolved_ip_addrs)")]
            fn resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
                let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
                self.0.resolved_ip_addrs(resolved_ip_addrs);
                Ok(())
            }

            /// 获取上传进度回调
            #[pyo3(text_signature = "($self, callback)")]
            fn on_uploading_progress(&mut self, callback: PyObject) -> PyResult<()> {
                self.0
                    .on_uploading_progress(on_uploading_progress(callback));
                Ok(())
            }

            /// 设置接受到响应状态回调
            #[pyo3(text_signature = "($self, callback)")]
            fn on_receive_response_status(&mut self, callback: PyObject) -> PyResult<()> {
                self.0
                    .on_receive_response_status(on_receive_response_status(callback));
                Ok(())
            }

            /// 设置接受到响应 Header 回调
            #[pyo3(text_signature = "($self, callback)")]
            fn on_receive_response_header(&mut self, callback: PyObject) -> PyResult<()> {
                self.0
                    .on_receive_response_header(on_receive_response_header(callback));
                Ok(())
            }

            /// 重置 HTTP 请求构建器
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
    };
}

/// 数据传输进度信息
#[pyclass]
#[pyo3(text_signature = "(transferred_bytes, total_bytes)")]
#[derive(Clone, Copy, Debug)]
struct TransferProgressInfo {
    transferred_bytes: u64,
    total_bytes: u64,
}

#[pymethods]
impl TransferProgressInfo {
    #[new]
    fn new(transferred_bytes: u64, total_bytes: u64) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
        }
    }

    /// 获取已经传输的数据量
    ///
    /// 单位为字节
    #[getter]
    fn get_transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    /// 获取总共需要传输的数据量
    ///
    /// 单位为字节
    #[getter]
    fn get_total_bytes(&self) -> u64 {
        self.total_bytes
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ToPyObject for TransferProgressInfo {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.to_owned().into_py(py)
    }
}

/// 阻塞 HTTP 请求构建器
#[pyclass]
#[pyo3(text_signature = "()")]
struct SyncHttpRequestBuilder(qiniu_sdk::http::SyncRequestBuilder<'static>);
impl_http_request_builder!(SyncHttpRequestBuilder);

#[pymethods]
impl SyncHttpRequestBuilder {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::http::SyncRequestBuilder::new())
    }

    /// 设置请求 HTTP 请求体
    #[args(len = "None")]
    #[pyo3(text_signature = "($self, body, len = None)")]
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

    /// 构建 HTTP 请求，同时构建器被重置
    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> SyncHttpRequest {
        SyncHttpRequest(self.0.build())
    }
}

/// 异步 HTTP 请求构建器
#[pyclass]
#[pyo3(text_signature = "()")]
struct AsyncHttpRequestBuilder(qiniu_sdk::http::AsyncRequestBuilder<'static>);
impl_http_request_builder!(AsyncHttpRequestBuilder);

#[pymethods]
impl AsyncHttpRequestBuilder {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::http::AsyncRequestBuilder::new())
    }

    /// 设置请求 HTTP 请求体
    #[args(len = "None")]
    #[pyo3(text_signature = "($self, body, len = None)")]
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

    /// 构建 HTTP 请求，同时构建器被重置
    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> AsyncHttpRequest {
        AsyncHttpRequest(Arc::new(AsyncMutex::new(self.0.build())))
    }
}

/// 阻塞 HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[pyclass]
#[pyo3(
    text_signature = "(/, url = None, method = None, headers = None, body = None, body_len = None, appended_user_agent = None, resolved_ip_addrs = None, uploading_progress = None, receive_response_status = None, receive_response_header = None)"
)]
pub(super) struct SyncHttpRequest(qiniu_sdk::http::SyncRequest<'static>);

#[pymethods]
impl SyncHttpRequest {
    #[new]
    #[args(
        url = "None",
        method = "None",
        version = "None",
        headers = "None",
        appended_user_agent = "None",
        resolved_ip_addrs = "None",
        body = "None",
        body_len = "None",
        uploading_progress = "None",
        receive_response_status = "None",
        receive_response_header = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        url: Option<&str>,
        method: Option<&str>,
        version: Option<Version>,
        headers: Option<HashMap<String, String>>,
        appended_user_agent: Option<&str>,
        resolved_ip_addrs: Option<Vec<String>>,
        body: Option<PyObject>,
        body_len: Option<u64>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::http::SyncRequest::builder();
        if let Some(url) = url {
            builder.url(parse_uri(url)?);
        }
        if let Some(method) = method {
            builder.method(parse_method(method)?);
        }
        if let Some(version) = version {
            builder.version(version.into());
        }
        if let Some(headers) = headers {
            builder.headers(parse_headers(headers)?);
        }
        if let Some(appended_user_agent) = appended_user_agent {
            builder.appended_user_agent(appended_user_agent);
        }
        if let Some(resolved_ip_addrs) = resolved_ip_addrs {
            builder.resolved_ip_addrs(parse_ip_addrs(resolved_ip_addrs)?);
        }
        if let Some(body) = body {
            builder.body(extract_sync_request_body(body, body_len, py)?);
        }
        if let Some(callback) = uploading_progress {
            builder.on_uploading_progress(on_uploading_progress(callback));
        }
        if let Some(callback) = receive_response_status {
            builder.on_receive_response_status(on_receive_response_status(callback));
        }
        if let Some(callback) = receive_response_header {
            builder.on_receive_response_header(on_receive_response_header(callback));
        }
        Ok(Self(builder.build()))
    }

    /// 创建 HTTP 请求构建器
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> SyncHttpRequestBuilder {
        SyncHttpRequestBuilder(qiniu_sdk::http::SyncRequest::builder())
    }

    /// 获取 HTTP 请求 URL
    #[getter]
    fn get_url(&self) -> String {
        self.0.url().to_string()
    }

    /// 设置 HTTP 请求 URL
    #[setter]
    fn set_url(&mut self, url: &str) -> PyResult<()> {
        *self.0.url_mut() = url
            .parse::<Uri>()
            .map_err(|err| QiniuInvalidURLError::new_err(err.to_string()))?;
        Ok(())
    }

    /// 获取请求 HTTP 版本
    #[getter]
    fn get_version(&self) -> PyResult<Version> {
        self.0.version().try_into()
    }

    /// 设置请求 HTTP 版本
    #[setter]
    fn set_version(&mut self, version: Version) {
        *self.0.version_mut() = version.into();
    }

    /// 获取请求 HTTP 方法
    #[getter]
    fn get_method(&self) -> String {
        self.0.method().to_string()
    }

    /// 设置请求 HTTP 方法
    #[setter]
    fn set_method(&mut self, method: &str) -> PyResult<()> {
        *self.0.method_mut() = method
            .parse::<Method>()
            .map_err(|err| QiniuInvalidMethodError::new_err(err.to_string()))?;
        Ok(())
    }

    /// 获取请求 HTTP Headers
    #[getter]
    fn get_headers(&self) -> PyResult<HashMap<String, String>> {
        convert_headers_to_hashmap(self.0.headers())
    }

    /// 设置请求 HTTP Headers
    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        *self.0.headers_mut() = parse_headers(headers)?;
        Ok(())
    }

    /// 获取用户代理
    #[getter]
    fn get_user_agent(&self) -> String {
        self.0.user_agent().to_string()
    }

    /// 获取追加的用户代理
    #[getter]
    fn get_appended_user_agent(&self) -> String {
        self.0.appended_user_agent().to_string()
    }

    /// 设置追加的用户代理
    #[setter]
    fn set_appended_user_agent(&mut self, appended_user_agent: &str) {
        *self.0.appended_user_agent_mut() = appended_user_agent.into();
    }

    /// 获取预解析的服务器套接字地址
    #[getter]
    fn get_resolved_ip_addrs(&self) -> Option<Vec<String>> {
        self.0
            .resolved_ip_addrs()
            .map(|ip_addrs| ip_addrs.iter().map(|ip_addr| ip_addr.to_string()).collect())
    }

    /// 设置预解析的服务器套接字地址
    #[setter]
    fn set_resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
        let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
        *self.0.resolved_ip_addrs_mut() = Some(Cow::Owned(resolved_ip_addrs));
        Ok(())
    }

    /// 设置请求体
    #[setter]
    fn set_body(&mut self, body: Vec<u8>) {
        *self.0.body_mut() = qiniu_sdk::http::SyncRequestBody::from(body);
    }

    /// 设置上传进度回调
    #[setter]
    fn set_uploading_progress(&mut self, callback: PyObject) -> PyResult<()> {
        *self.0.on_uploading_progress_mut() = Some(on_uploading_progress(callback));
        Ok(())
    }

    /// 设置接受到响应状态回调
    #[setter]
    fn set_receive_response_status(&mut self, callback: PyObject) -> PyResult<()> {
        *self.0.on_receive_response_status_mut() = Some(on_receive_response_status(callback));
        Ok(())
    }

    /// 设置接受到响应 Header 回调
    #[setter]
    fn set_receive_response_header(&mut self, callback: PyObject) -> PyResult<()> {
        *self.0.on_receive_response_header_mut() = Some(on_receive_response_header(callback));
        Ok(())
    }
}

impl Deref for SyncHttpRequest {
    type Target = qiniu_sdk::http::SyncRequest<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SyncHttpRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 异步 HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[pyclass]
#[derive(Clone)]
#[pyo3(
    text_signature = "(/, url = None, method = None, headers = None, body = None, body_len = None, appended_user_agent = None, resolved_ip_addrs = None)"
)]
pub(super) struct AsyncHttpRequest(Arc<AsyncMutex<qiniu_sdk::http::AsyncRequest<'static>>>);

#[pymethods]
impl AsyncHttpRequest {
    #[new]
    #[args(
        url = "None",
        method = "None",
        version = "None",
        headers = "None",
        appended_user_agent = "None",
        resolved_ip_addrs = "None",
        body = "None",
        body_len = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        url: Option<&str>,
        method: Option<&str>,
        version: Option<Version>,
        headers: Option<HashMap<String, String>>,
        appended_user_agent: Option<&str>,
        resolved_ip_addrs: Option<Vec<String>>,
        body: Option<PyObject>,
        body_len: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::http::AsyncRequest::builder();
        if let Some(url) = url {
            builder.url(parse_uri(url)?);
        }
        if let Some(method) = method {
            builder.method(parse_method(method)?);
        }
        if let Some(version) = version {
            builder.version(version.into());
        }
        if let Some(headers) = headers {
            builder.headers(parse_headers(headers)?);
        }
        if let Some(appended_user_agent) = appended_user_agent {
            builder.appended_user_agent(appended_user_agent);
        }
        if let Some(resolved_ip_addrs) = resolved_ip_addrs {
            builder.resolved_ip_addrs(parse_ip_addrs(resolved_ip_addrs)?);
        }
        if let Some(body) = body {
            builder.body(extract_async_request_body(body, body_len, py)?);
        }
        Ok(Self(Arc::new(AsyncMutex::new(builder.build()))))
    }

    /// 创建 HTTP 请求构建器
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> AsyncHttpRequestBuilder {
        AsyncHttpRequestBuilder(qiniu_sdk::http::AsyncRequest::builder())
    }

    /// 获取 HTTP 请求 URL
    #[getter]
    fn get_url(&self) -> PyResult<String> {
        Ok(self.lock()?.url().to_string())
    }

    /// 设置 HTTP 请求 URL
    #[setter]
    fn set_url(&mut self, url: &str) -> PyResult<()> {
        *self.lock()?.url_mut() = url
            .parse::<Uri>()
            .map_err(|err| QiniuInvalidURLError::new_err(err.to_string()))?;
        Ok(())
    }

    /// 获取请求 HTTP 版本
    #[getter]
    fn get_version(&mut self) -> PyResult<Version> {
        self.lock()?.version().try_into()
    }

    /// 设置请求 HTTP 版本
    #[setter]
    fn set_version(&mut self, version: Version) -> PyResult<()> {
        *self.lock()?.version_mut() = version.into();
        Ok(())
    }

    /// 获取请求 HTTP 方法
    #[getter]
    fn get_method(&mut self) -> PyResult<String> {
        Ok(self.lock()?.method().to_string())
    }

    /// 设置请求 HTTP 方法
    #[setter]
    fn set_method(&mut self, method: &str) -> PyResult<()> {
        *self.lock()?.method_mut() = method
            .parse::<Method>()
            .map_err(|err| QiniuInvalidMethodError::new_err(err.to_string()))?;
        Ok(())
    }

    /// 获取请求 HTTP Headers
    #[getter]
    fn get_headers(&mut self) -> PyResult<HashMap<String, String>> {
        convert_headers_to_hashmap(self.lock()?.headers())
    }

    /// 设置请求 HTTP Headers
    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        *self.lock()?.headers_mut() = parse_headers(headers)?;
        Ok(())
    }

    /// 获取用户代理
    #[getter]
    fn get_user_agent(&mut self) -> PyResult<String> {
        Ok(self.lock()?.user_agent().to_string())
    }

    /// 获取追加的用户代理
    #[getter]
    fn get_appended_user_agent(&mut self) -> PyResult<String> {
        Ok(self.lock()?.appended_user_agent().to_string())
    }

    /// 设置追加的用户代理
    #[setter]
    fn set_appended_user_agent(&mut self, appended_user_agent: &str) -> PyResult<()> {
        *self.lock()?.appended_user_agent_mut() = appended_user_agent.into();
        Ok(())
    }

    /// 获取预解析的服务器套接字地址
    #[getter]
    fn get_resolved_ip_addrs(&mut self) -> PyResult<Option<Vec<String>>> {
        Ok(self
            .lock()?
            .resolved_ip_addrs()
            .map(|ip_addrs| ip_addrs.iter().map(|ip_addr| ip_addr.to_string()).collect()))
    }

    /// 设置预解析的服务器套接字地址
    #[setter]
    fn set_resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
        let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
        *self.lock()?.resolved_ip_addrs_mut() = Some(Cow::Owned(resolved_ip_addrs));
        Ok(())
    }

    /// 设置请求体
    #[setter]
    fn set_body(&mut self, body: Vec<u8>) -> PyResult<()> {
        *self.lock()?.body_mut() = qiniu_sdk::http::AsyncRequestBody::from(body);
        Ok(())
    }

    /// 设置上传进度回调
    #[setter]
    fn set_uploading_progress(&mut self, callback: PyObject) -> PyResult<()> {
        *self.lock()?.on_uploading_progress_mut() = Some(on_uploading_progress(callback));
        Ok(())
    }

    /// 设置接受到响应状态回调
    #[setter]
    fn set_receive_response_status(&mut self, callback: PyObject) -> PyResult<()> {
        *self.lock()?.on_receive_response_status_mut() = Some(on_receive_response_status(callback));
        Ok(())
    }

    /// 设置接受到响应 Header 回调
    #[setter]
    fn set_receive_response_header(&mut self, callback: PyObject) -> PyResult<()> {
        *self.lock()?.on_receive_response_header_mut() = Some(on_receive_response_header(callback));
        Ok(())
    }
}

impl AsyncHttpRequest {
    pub(super) fn lock(
        &self,
    ) -> PyResult<AsyncMutexGuard<'_, qiniu_sdk::http::AsyncRequest<'static>>> {
        self.0.try_lock().map_or_else(
            || Err(QiniuDataLockedError::new_err("AsyncHttpRequest is locked")),
            Ok,
        )
    }
}

/// HTTP 版本
#[pyclass]
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub(super) enum Version {
    /// HTTP 0.9
    HTTP_09 = 9,
    /// HTTP 1.0
    HTTP_10 = 10,
    /// HTTP 1.1
    HTTP_11 = 11,
    /// HTTP 2
    HTTP_2 = 20,
    /// HTTP 3
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

/// HTTP 响应的指标信息
#[pyclass]
#[derive(Clone)]
#[pyo3(
    text_signature = "(/, total_duration_ns = None, name_lookup_duration_ns = None, connect_duration_ns = None, secure_connect_duration_ns = None, redirect_duration_ns = None, transfer_duration_ns = None)"
)]
pub(super) struct Metrics(qiniu_sdk::http::Metrics);

#[pymethods]
impl Metrics {
    #[new]
    #[args(
        total_duration_ns = "None",
        name_lookup_duration_ns = "None",
        connect_duration_ns = "None",
        secure_connect_duration_ns = "None",
        redirect_duration_ns = "None",
        transfer_duration_ns = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        total_duration_ns: Option<u64>,
        name_lookup_duration_ns: Option<u64>,
        connect_duration_ns: Option<u64>,
        secure_connect_duration_ns: Option<u64>,
        redirect_duration_ns: Option<u64>,
        transfer_duration_ns: Option<u64>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::http::MetricsBuilder::default();
        if let Some(duration) = total_duration_ns {
            builder.total_duration(Duration::from_nanos(duration));
        }
        if let Some(duration) = name_lookup_duration_ns {
            builder.name_lookup_duration(Duration::from_nanos(duration));
        }
        if let Some(duration) = connect_duration_ns {
            builder.connect_duration(Duration::from_nanos(duration));
        }
        if let Some(duration) = secure_connect_duration_ns {
            builder.secure_connect_duration(Duration::from_nanos(duration));
        }
        if let Some(duration) = redirect_duration_ns {
            builder.redirect_duration(Duration::from_nanos(duration));
        }
        if let Some(duration) = transfer_duration_ns {
            builder.transfer_duration(Duration::from_nanos(duration));
        }
        Ok(Self(builder.build()))
    }

    /// 获取总体请求耗时
    #[getter]
    fn get_total_duration(&self) -> Option<u128> {
        self.0.total_duration().map(|duration| duration.as_nanos())
    }

    /// 设置总体请求耗时
    #[setter]
    fn set_total_duration(&mut self, duration_ns: u64) {
        *self.0.total_duration_mut() = Some(Duration::from_nanos(duration_ns));
    }

    /// 获取域名查询的耗时
    #[getter]
    fn get_name_lookup_duration(&self) -> Option<u128> {
        self.0
            .name_lookup_duration()
            .map(|duration| duration.as_nanos())
    }

    /// 设置域名查询的耗时
    #[setter]
    fn set_name_lookup_duration(&mut self, duration_ns: u64) {
        *self.0.name_lookup_duration_mut() = Some(Duration::from_nanos(duration_ns));
    }

    /// 获取建立连接的耗时
    #[getter]
    fn get_connect_duration(&self) -> Option<u128> {
        self.0
            .connect_duration()
            .map(|duration| duration.as_nanos())
    }

    /// 设置建立连接的耗时
    #[setter]
    fn set_connect_duration(&mut self, duration_ns: u64) {
        *self.0.connect_duration_mut() = Some(Duration::from_nanos(duration_ns));
    }

    /// 获取建立安全连接的耗时
    #[getter]
    fn get_secure_connect_duration(&self) -> Option<u128> {
        self.0
            .secure_connect_duration()
            .map(|duration| duration.as_nanos())
    }

    /// 设置建立安全连接的耗时
    #[setter]
    fn set_secure_connect_duration(&mut self, duration_ns: u64) {
        *self.0.secure_connect_duration_mut() = Some(Duration::from_nanos(duration_ns));
    }

    /// 获取重定向的耗时
    #[getter]
    fn get_redirect_duration(&self) -> Option<u128> {
        self.0
            .redirect_duration()
            .map(|duration| duration.as_nanos())
    }

    /// 设置重定向的耗时
    #[setter]
    fn set_redirect_duration(&mut self, duration_ns: u64) {
        *self.0.redirect_duration_mut() = Some(Duration::from_nanos(duration_ns));
    }

    /// 获取请求和响应数据传输的耗时
    #[getter]
    fn get_transfer_duration(&self) -> Option<u128> {
        self.0
            .transfer_duration()
            .map(|duration| duration.as_nanos())
    }

    /// 设置请求和响应数据传输的耗时
    #[setter]
    fn set_transfer_duration(&mut self, duration_ns: u64) {
        *self.0.transfer_duration_mut() = Some(Duration::from_nanos(duration_ns));
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
    /// 获取 HTTP 状态码
    #[getter]
    fn get_status_code(&self) -> u16 {
        self.0.status_code().as_u16()
    }

    /// 设置 HTTP 状态码
    #[setter]
    fn set_status_code(&mut self, status_code: u16) -> PyResult<()> {
        *self.0.status_code_mut() = parse_status_code(status_code)?;
        Ok(())
    }

    /// 获取 HTTP Headers
    #[getter]
    fn get_headers(&self) -> PyResult<HashMap<String, String>> {
        convert_headers_to_hashmap(self.0.headers())
    }

    /// 设置 HTTP Headers
    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        *self.0.headers_mut() = parse_headers(headers)?;
        Ok(())
    }

    /// 获取 HTTP 版本
    #[getter]
    fn get_version(&self) -> PyResult<Version> {
        self.0
            .version()
            .try_into()
            .map_err(|err: PyErr| QiniuInvalidHttpVersionError::new_err(err.to_string()))
    }

    /// 设置 HTTP 版本
    #[setter]
    fn set_version(&mut self, version: Version) {
        *self.0.version_mut() = version.into();
    }

    /// 获取 HTTP 服务器 IP 地址
    #[getter]
    fn get_server_ip(&self) -> Option<String> {
        self.0.server_ip().map(|ip| ip.to_string())
    }

    /// 设置 HTTP 服务器 IP 地址
    #[setter]
    fn set_server_ip(&mut self, server_ip: String) -> PyResult<()> {
        *self.0.server_ip_mut() = server_ip
            .parse::<IpAddr>()
            .map(Some)
            .map_err(|err| QiniuInvalidIpAddrError::new_err(err.to_string()))?;
        Ok(())
    }

    /// 获取 HTTP 服务器端口号
    #[getter]
    fn get_server_port(&self) -> Option<u16> {
        self.0.server_port().map(|ip| ip.get())
    }

    /// 设置 HTTP 服务器端口号
    #[setter]
    fn set_server_port(&mut self, server_port: u16) {
        *self.0.server_port_mut() = NonZeroU16::new(server_port);
    }

    /// 获取 HTTP 响应的指标信息
    #[getter]
    fn get_metrics(&self) -> Option<Metrics> {
        self.0.metrics().cloned().map(Metrics)
    }

    /// 设置 HTTP 响应的指标信息
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

macro_rules! impl_response_body {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            #[getter]
            pub fn get_closed(&self) -> bool {
                false
            }

            #[pyo3(text_signature = "($self)")]
            pub fn close(&self) -> PyResult<()> {
                Err(PyNotImplementedError::new_err("close"))
            }

            #[pyo3(text_signature = "($self)")]
            pub fn fileno(&self) -> PyResult<u32> {
                Err(PyNotImplementedError::new_err("fileno"))
            }

            #[pyo3(text_signature = "($self)")]
            pub fn flush(&self) -> PyResult<()> {
                Err(PyNotImplementedError::new_err("flush"))
            }

            #[pyo3(text_signature = "($self)")]
            pub fn isatty(&self) -> PyResult<bool> {
                Ok(false)
            }

            #[pyo3(text_signature = "($self)")]
            pub fn readable(&self) -> PyResult<bool> {
                Ok(true)
            }

            #[pyo3(text_signature = "($self, offset, whence = 0)")]
            #[args(whence = "0")]
            pub fn seek(&self, offset: i64, whence: i64) -> PyResult<bool> {
                let _offset = offset;
                let _whence = whence;
                Err(PyNotImplementedError::new_err("seek"))
            }

            #[pyo3(text_signature = "($self)")]
            pub fn seekable(&self) -> PyResult<bool> {
                Ok(false)
            }

            #[pyo3(text_signature = "($self)")]
            pub fn tell(&self) -> PyResult<bool> {
                Err(PyNotImplementedError::new_err("tell"))
            }

            #[pyo3(text_signature = "($self, size = None)")]
            #[args(size = "None")]
            pub fn truncate(&self, size: Option<u64>) -> PyResult<()> {
                let _size = size;
                Err(PyNotImplementedError::new_err("truncate"))
            }

            #[pyo3(text_signature = "($self)")]
            pub fn writable(&self) -> PyResult<bool> {
                Ok(false)
            }

            #[pyo3(text_signature = "($self, lines)")]
            pub fn writelines(&self, lines: Vec<String>) -> PyResult<()> {
                drop(lines);
                Err(PyNotImplementedError::new_err("writelines"))
            }
        }
    };
}

/// 阻塞 HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[pyclass(extends = ResponseParts)]
#[pyo3(
    text_signature = "(/, status_code = None, headers = None, version = None, server_ip = None, server_port = None, body = None, metrics = None)"
)]
struct SyncHttpResponse(qiniu_sdk::http::SyncResponseBody);

#[pymethods]
impl SyncHttpResponse {
    #[new]
    #[args(
        status_code = "None",
        headers = "None",
        version = "None",
        server_ip = "None",
        server_port = "None",
        body = "None",
        metrics = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status_code: Option<u16>,
        headers: Option<HashMap<String, String>>,
        version: Option<Version>,
        server_ip: Option<&str>,
        server_port: Option<u16>,
        body: Option<PyObject>,
        metrics: Option<Metrics>,
        py: Python<'_>,
    ) -> PyResult<(SyncHttpResponse, ResponseParts)> {
        let mut builder = qiniu_sdk::http::Response::builder();
        if let Some(status_code) = status_code {
            builder.status_code(parse_status_code(status_code)?);
        }
        if let Some(headers) = headers {
            builder.headers(parse_headers(headers)?);
        }
        if let Some(version) = version {
            builder.version(version.into());
        }
        if let Some(server_ip) = server_ip {
            builder.server_ip(parse_ip_addr(server_ip)?);
        }
        if let Some(server_port) = server_port {
            builder.server_port(parse_port(server_port)?);
        }
        if let Some(body) = body {
            builder.body(extract_sync_response_body(body, py));
        }
        if let Some(metrics) = metrics {
            builder.metrics(metrics.0);
        }
        let (parts, body) = builder.build().into_parts_and_body();
        Ok((Self(body), ResponseParts(parts)))
    }

    /// 读取响应体数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    pub fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyBytes> {
        let mut buf = Vec::new();
        if let Ok(size) = u64::try_from(size) {
            buf.reserve(size as usize);
            (&mut self.0)
                .take(size)
                .read_to_end(&mut buf)
                .map_err(PyIOError::new_err)?;
        } else {
            self.0.read_to_end(&mut buf).map_err(PyIOError::new_err)?;
        }
        Ok(PyBytes::new(py, &buf))
    }

    /// 读取所有响应体数据
    #[pyo3(text_signature = "($self)")]
    pub fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        self.read(-1, py)
    }

    #[pyo3(text_signature = "($self, b)")]
    pub fn write(&mut self, b: PyObject) -> PyResult<u64> {
        drop(b);
        Err(PyNotImplementedError::new_err("write"))
    }
}

impl_response_body!(SyncHttpResponse);

/// 异步 HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[pyclass(extends = ResponseParts)]
#[pyo3(
    text_signature = "(/, status_code = None, headers = None, version = None, server_ip = None, server_port = None, body = None, metrics = None)"
)]
struct AsyncHttpResponse(Arc<AsyncMutex<qiniu_sdk::http::AsyncResponseBody>>);

#[pymethods]
impl AsyncHttpResponse {
    #[new]
    #[args(
        status_code = "None",
        headers = "None",
        version = "None",
        server_ip = "None",
        server_port = "None",
        body = "None",
        metrics = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status_code: Option<u16>,
        headers: Option<HashMap<String, String>>,
        version: Option<Version>,
        server_ip: Option<&str>,
        server_port: Option<u16>,
        body: Option<PyObject>,
        metrics: Option<Metrics>,
        py: Python<'_>,
    ) -> PyResult<(AsyncHttpResponse, ResponseParts)> {
        let mut builder = qiniu_sdk::http::Response::builder();
        if let Some(status_code) = status_code {
            builder.status_code(parse_status_code(status_code)?);
        }
        if let Some(headers) = headers {
            builder.headers(parse_headers(headers)?);
        }
        if let Some(version) = version {
            builder.version(version.into());
        }
        if let Some(server_ip) = server_ip {
            builder.server_ip(parse_ip_addr(server_ip)?);
        }
        if let Some(server_port) = server_port {
            builder.server_port(parse_port(server_port)?);
        }
        if let Some(body) = body {
            builder.body(extract_async_response_body(body, py));
        }
        if let Some(metrics) = metrics {
            builder.metrics(metrics.0);
        }
        let (parts, body) = builder.build().into_parts_and_body();
        Ok((Self(Arc::new(AsyncMutex::new(body))), ResponseParts(parts)))
    }

    /// 异步读取响应体数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    pub fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyAny> {
        let reader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut reader = reader.lock().await;
            let mut buf = Vec::new();
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut *reader).take(size).read_to_end(&mut buf).await
            } else {
                reader.read_to_end(&mut buf).await
            }
            .map_err(PyIOError::new_err)?;
            Python::with_gil(|py| Ok(PyBytes::new(py, &buf).to_object(py)))
        })
    }

    /// 异步所有读取响应体数据
    #[pyo3(text_signature = "($self)")]
    pub fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        self.read(-1, py)
    }

    #[pyo3(text_signature = "($self, b)")]
    pub fn write(&mut self, b: PyObject) -> PyResult<u64> {
        drop(b);
        Err(PyNotImplementedError::new_err("write"))
    }
}

impl_response_body!(AsyncHttpResponse);

fn on_uploading_progress(callback: PyObject) -> qiniu_sdk::http::OnProgressCallback<'static> {
    qiniu_sdk::http::OnProgressCallback::new(move |progress| {
        Python::with_gil(|py| {
            let args = PyTuple::new(
                py,
                [TransferProgressInfo {
                    transferred_bytes: progress.transferred_bytes(),
                    total_bytes: progress.total_bytes(),
                }],
            );
            callback.call1(py, args)
        })?;
        Ok(())
    })
}

fn on_receive_response_status(
    callback: PyObject,
) -> qiniu_sdk::http::OnStatusCodeCallback<'static> {
    qiniu_sdk::http::OnStatusCodeCallback::new(move |status_code| {
        Python::with_gil(|py| {
            let args = PyTuple::new(py, [status_code.as_u16()]);
            callback.call1(py, args)
        })?;
        Ok(())
    })
}

fn on_receive_response_header(callback: PyObject) -> qiniu_sdk::http::OnHeaderCallback<'static> {
    qiniu_sdk::http::OnHeaderCallback::new(move |header_name, header_value| {
        Python::with_gil(|py| {
            let args = PyTuple::new(
                py,
                [
                    header_name.as_str(),
                    header_value
                        .to_str()
                        .map_err(|err| QiniuInvalidHeaderValueError::new_err(err.to_string()))?,
                ],
            );
            callback.call1(py, args)
        })?;
        Ok(())
    })
}
