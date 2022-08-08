use super::{
    credential::CredentialProvider,
    exceptions::{QiniuApiCallError, QiniuDownloadError, QiniuEmptyEndpoints},
    http::HttpResponsePartsMut,
    http_client::{CallbackContextMut, EndpointsProvider, HttpClient, RequestBuilderPartsRef},
    utils::{convert_api_call_error, extract_endpoints, parse_headers, PythonIoBase},
};
use anyhow::Result as AnyResult;
use futures::{lock::Mutex as AsyncMutex, AsyncReadExt};
use maybe_owned::MaybeOwned;
use pyo3::{exceptions::PyIOError, prelude::*, types::PyBytes};
use std::{
    collections::HashMap, io::Read, mem::transmute, num::NonZeroU64, sync::Arc, time::Duration,
};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "download")?;
    m.add_class::<RetryDecision>()?;
    m.add_class::<RetriedStatsInfo>()?;
    m.add_class::<DownloadRetrier>()?;
    m.add_class::<NeverRetrier>()?;
    m.add_class::<ErrorRetrier>()?;
    m.add_class::<DownloadUrlsGenerator>()?;
    m.add_class::<UrlsSigner>()?;
    m.add_class::<StaticDomainsUrlsGenerator>()?;
    m.add_class::<EndpointsUrlGenerator>()?;
    m.add_class::<DownloadManager>()?;
    m.add_class::<DownloadingObjectReader>()?;
    m.add_class::<AsyncDownloadingObjectReader>()?;
    m.add_class::<DownloadingProgressInfo>()?;
    Ok(m)
}

/// 重试决定
#[pyclass]
#[derive(Copy, Clone, Debug)]
enum RetryDecision {
    /// 不再重试
    DontRetry = 0,

    /// 切换到下一个服务器
    TryNextServer = 1,

    /// 重试当前请求
    RetryRequest = 2,
}

#[pymethods]
impl RetryDecision {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<qiniu_sdk::download::RetryDecision> for RetryDecision {
    fn from(decision: qiniu_sdk::download::RetryDecision) -> Self {
        match decision {
            qiniu_sdk::download::RetryDecision::DontRetry => RetryDecision::DontRetry,
            qiniu_sdk::download::RetryDecision::TryNextServer => RetryDecision::TryNextServer,
            qiniu_sdk::download::RetryDecision::RetryRequest => RetryDecision::RetryRequest,
            _ => panic!("Unrecognized retry decision"),
        }
    }
}

impl From<RetryDecision> for qiniu_sdk::download::RetryDecision {
    fn from(decision: RetryDecision) -> Self {
        match decision {
            RetryDecision::DontRetry => qiniu_sdk::download::RetryDecision::DontRetry,
            RetryDecision::TryNextServer => qiniu_sdk::download::RetryDecision::TryNextServer,
            RetryDecision::RetryRequest => qiniu_sdk::download::RetryDecision::RetryRequest,
        }
    }
}

/// 重试统计信息
///
/// 通过 `RetriedStatsInfo()` 创建重试统计信息
#[pyclass]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "()")]
struct RetriedStatsInfo(qiniu_sdk::download::RetriedStatsInfo);

#[pymethods]
impl RetriedStatsInfo {
    #[new]
    /// 创建重试统计信息
    fn new() -> Self {
        Self(Default::default())
    }

    /// 提升当前终端地址的重试次数
    #[pyo3(text_signature = "($self)")]
    fn increase(&mut self) {
        self.0.increase()
    }

    /// 切换终端地址
    #[pyo3(text_signature = "($self)")]
    fn switch_endpoint(&mut self) {
        self.0.switch_endpoint()
    }

    /// 获取总共重试的次数
    #[getter]
    fn get_retried_total(&self) -> usize {
        self.0.retried_total()
    }

    /// 获取当前终端地址的重试次数
    #[getter]
    fn get_retried_on_current_endpoint(&self) -> usize {
        self.0.retried_on_current_endpoint()
    }

    /// 获取放弃的终端地址的数量
    #[getter]
    fn get_abandoned_endpoints(&self) -> usize {
        self.0.abandoned_endpoints()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 下载重试器
///
/// 抽象类
///
/// 根据 HTTP 客户端返回的错误，决定是否重试请求，重试决定由 `RetryDecision` 定义。
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct DownloadRetrier(Box<dyn qiniu_sdk::download::DownloadRetrier>);

#[pymethods]
impl DownloadRetrier {
    #[pyo3(text_signature = "($self, request, response_error, retried)")]
    fn retry(
        &self,
        request: &mut CallbackContextMut,
        response_error: &QiniuApiCallError,
        retried: &RetriedStatsInfo,
    ) -> PyResult<RetryDecision> {
        let error = convert_api_call_error(&PyErr::from(response_error))?;
        let decision = self
            .0
            .retry(
                request.as_mut(),
                qiniu_sdk::download::DownloadRetrierOptions::new(error.as_ref(), &retried.0),
            )
            .decision();
        Ok(decision.into())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::download::DownloadRetrier for DownloadRetrier {
    fn retry(
        &self,
        request: &mut dyn qiniu_sdk::http_client::CallbackContext,
        opts: qiniu_sdk::download::DownloadRetrierOptions<'_>,
    ) -> qiniu_sdk::download::RetryResult {
        self.0.retry(request, opts)
    }
}

/// 永不重试器
///
/// 总是返回不再重试的重试器
///
/// 通过 `NeverRetrier()` 创建永不重试器
#[pyclass(extends = DownloadRetrier)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct NeverRetrier;

#[pymethods]
impl NeverRetrier {
    /// 创建永不重试器
    #[new]
    fn new() -> (Self, DownloadRetrier) {
        (
            Self,
            DownloadRetrier(Box::new(qiniu_sdk::download::NeverRetrier)),
        )
    }
}

/// 根据七牛 API 返回的状态码作出重试决定
///
/// 通过 `ErrorRetrier()` 创建错误重试器
#[pyclass(extends = DownloadRetrier)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct ErrorRetrier;

#[pymethods]
impl ErrorRetrier {
    /// 创建错误重试器
    #[new]
    fn new() -> (Self, DownloadRetrier) {
        (
            Self,
            DownloadRetrier(Box::new(qiniu_sdk::download::ErrorRetrier)),
        )
    }
}

/// 生成下载 URL 列表的接口
///
/// 抽象类
///
/// 同时提供阻塞接口和异步接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct DownloadUrlsGenerator(Box<dyn qiniu_sdk::download::DownloadUrlsGenerator>);

#[pymethods]
impl DownloadUrlsGenerator {
    /// 生成下载 URL 列表
    #[pyo3(text_signature = "($self, object_name, /, ttl_secs=None)")]
    #[args(ttl_secs = "None")]
    fn generate(&self, object_name: &str, ttl_secs: Option<u64>) -> PyResult<Vec<String>> {
        let mut builder = qiniu_sdk::download::GeneratorOptions::builder();
        if let Some(ttl_secs) = ttl_secs {
            builder.ttl(Duration::from_secs(ttl_secs));
        }
        self.0
            .generate(object_name, builder.build())
            .map(|urls| urls.iter().map(|url| url.to_string()).collect())
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
    }

    /// 异步生成下载 URL 列表
    #[pyo3(text_signature = "($self, object_name, /, ttl_secs=None)")]
    #[args(ttl_secs = "None")]
    fn async_generate<'p>(
        &'p self,
        object_name: String,
        ttl_secs: Option<u64>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let generator = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut builder = qiniu_sdk::download::GeneratorOptions::builder();
            if let Some(ttl_secs) = ttl_secs {
                builder.ttl(Duration::from_secs(ttl_secs));
            }
            generator
                .async_generate(&object_name, builder.build())
                .await
                .map(|urls| urls.iter().map(|url| url.to_string()).collect::<Vec<_>>())
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
        })
    }
}

impl qiniu_sdk::download::DownloadUrlsGenerator for DownloadUrlsGenerator {
    fn generate(
        &self,
        object_name: &str,
        options: qiniu_sdk::download::GeneratorOptions<'_>,
    ) -> qiniu_sdk::http_client::ApiResult<Vec<qiniu_sdk::http::Uri>> {
        self.0.generate(object_name, options)
    }

    fn async_generate<'a>(
        &'a self,
        object_name: &'a str,
        options: qiniu_sdk::download::GeneratorOptions<'a>,
    ) -> futures::future::BoxFuture<'a, qiniu_sdk::http_client::ApiResult<Vec<qiniu_sdk::http::Uri>>>
    {
        self.0.async_generate(object_name, options)
    }
}

/// URL 列表签名器
///
/// 通过 `UrlsSigner(credential, generator)` 创建 URL 列表签名器
#[pyclass(extends = DownloadUrlsGenerator)]
#[derive(Debug, Clone)]
#[pyo3(text_signature = "(credential, generator)")]
struct UrlsSigner;

#[pymethods]
impl UrlsSigner {
    #[new]
    fn new(
        credential: CredentialProvider,
        generator: DownloadUrlsGenerator,
    ) -> (Self, DownloadUrlsGenerator) {
        (
            Self,
            DownloadUrlsGenerator(Box::new(qiniu_sdk::download::UrlsSigner::new(
                credential, generator,
            ))),
        )
    }
}

/// 静态公开空间域名下载 URL 列表生成器
///
/// 通过 `StaticDomainsUrlsGenerator(endpoints, use_https=None)` 创建静态公开空间域名下载 URL 列表生成器
#[derive(Debug, Clone)]
#[pyclass(extends = DownloadUrlsGenerator)]
#[pyo3(text_signature = "(endpoints, /, use_https=None)")]
struct StaticDomainsUrlsGenerator;

#[pymethods]
impl StaticDomainsUrlsGenerator {
    #[new]
    #[args(use_https = "None")]
    fn new(
        endpoints: Vec<&PyAny>,
        use_https: Option<bool>,
    ) -> PyResult<(Self, DownloadUrlsGenerator)> {
        let endpoints = extract_endpoints(endpoints)?;
        let mut iter = endpoints.into_iter();
        let mut builder = if let Some(endpoint) = iter.next() {
            qiniu_sdk::download::StaticDomainsUrlsGenerator::builder(endpoint)
        } else {
            return Err(QiniuEmptyEndpoints::new_err("empty endpoints"));
        };
        for endpoint in iter {
            builder.add_domain(endpoint);
        }
        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        Ok((Self, DownloadUrlsGenerator(Box::new(builder.build()))))
    }
}

/// 终端地址下载 URL 列表生成器
///
/// 通过 `EndpointsUrlGenerator(endpoints, use_https=None)` 创建终端地址下载 URL 列表生成器
#[derive(Debug, Clone)]
#[pyclass(extends = DownloadUrlsGenerator)]
#[pyo3(text_signature = "(endpoints, /, use_https=None)")]
struct EndpointsUrlGenerator;

#[pymethods]
impl EndpointsUrlGenerator {
    #[new]
    #[args(use_https = "None")]
    fn new(endpoints: EndpointsProvider, use_https: Option<bool>) -> (Self, DownloadUrlsGenerator) {
        let mut builder = qiniu_sdk::download::EndpointsUrlGenerator::builder(endpoints);
        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        (Self, DownloadUrlsGenerator(Box::new(builder.build())))
    }
}

/// 下载管理器
///
/// 通过 `DownloadManager(urls_generator, use_https = None, http_client = None)` 创建下载管理器
#[pyclass]
#[derive(Debug, Clone)]
#[pyo3(text_signature = "(urls_generator, /, use_https = None, http_client = None)")]
struct DownloadManager(qiniu_sdk::download::DownloadManager);

#[pymethods]
impl DownloadManager {
    /// 创建下载管理器
    #[new]
    #[args(use_https = "None", http_client = "None")]
    fn new(
        urls_generator: DownloadUrlsGenerator,
        use_https: Option<bool>,
        http_client: Option<HttpClient>,
    ) -> Self {
        let mut builder = qiniu_sdk::download::DownloadManager::builder(urls_generator);
        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        if let Some(http_client) = http_client {
            builder.http_client(http_client.into());
        }
        Self(builder.build())
    }

    /// 获取下载内容阅读器
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, object_name, /, range_from=None, range_to=None, retrier=None, headers=None, before_request=None, download_progress=None, response_ok=None, response_error=None)"
    )]
    #[args(
        range_from = "None",
        range_to = "None",
        retrier = "None",
        headers = "None",
        before_request = "None",
        download_progress = "None",
        response_ok = "None",
        response_error = "None"
    )]
    fn reader(
        &self,
        object_name: &str,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
    ) -> PyResult<DownloadingObjectReader> {
        let object = self.make_download_object(
            object_name,
            range_from,
            range_to,
            retrier,
            headers,
            before_request,
            download_progress,
            response_ok,
            response_error,
        )?;
        Ok(DownloadingObjectReader(object.into_read()))
    }

    /// 将下载的对象内容写入指定的文件系统路径
    ///
    /// 需要注意，如果文件已经存在，则会覆盖该文件，如果文件不存在，则会创建该文件。
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, object_name, to_path, /, range_from=None, range_to=None, retrier=None, headers=None, before_request=None, download_progress=None, response_ok=None, response_error=None)"
    )]
    #[args(
        range_from = "None",
        range_to = "None",
        retrier = "None",
        headers = "None",
        before_request = "None",
        download_progress = "None",
        response_ok = "None",
        response_error = "None"
    )]
    fn download_to_path(
        &self,
        object_name: &str,
        to_path: &str,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
    ) -> PyResult<()> {
        let object = self.make_download_object(
            object_name,
            range_from,
            range_to,
            retrier,
            headers,
            before_request,
            download_progress,
            response_ok,
            response_error,
        )?;
        object
            .to_path(to_path)
            .map_err(QiniuDownloadError::from_err)
    }

    /// 将下载的对象内容写入指定的输出流
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, object_name, to_object, /, range_from=None, range_to=None, retrier=None, headers=None, before_request=None, download_progress=None, response_ok=None, response_error=None)"
    )]
    #[args(
        range_from = "None",
        range_to = "None",
        retrier = "None",
        headers = "None",
        before_request = "None",
        download_progress = "None",
        response_ok = "None",
        response_error = "None"
    )]
    fn download_to_writer(
        &self,
        object_name: &str,
        to_object: PyObject,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
    ) -> PyResult<()> {
        let object = self.make_download_object(
            object_name,
            range_from,
            range_to,
            retrier,
            headers,
            before_request,
            download_progress,
            response_ok,
            response_error,
        )?;
        object
            .to_writer(&mut PythonIoBase::new(to_object))
            .map_err(QiniuDownloadError::from_err)
    }

    /// 异步获取下载内容阅读器
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, object_name, /, range_from=None, range_to=None, retrier=None, headers=None, before_request=None, download_progress=None, response_ok=None, response_error=None)"
    )]
    #[args(
        range_from = "None",
        range_to = "None",
        retrier = "None",
        headers = "None",
        before_request = "None",
        download_progress = "None",
        response_ok = "None",
        response_error = "None"
    )]
    fn async_reader(
        &self,
        object_name: &str,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
    ) -> PyResult<AsyncDownloadingObjectReader> {
        let object = self.make_download_object(
            object_name,
            range_from,
            range_to,
            retrier,
            headers,
            before_request,
            download_progress,
            response_ok,
            response_error,
        )?;
        Ok(AsyncDownloadingObjectReader(Arc::new(AsyncMutex::new(
            object.into_async_read(),
        ))))
    }

    /// 将下载的对象内容异步写入指定的文件系统路径
    ///
    /// 需要注意，如果文件已经存在，则会覆盖该文件，如果文件不存在，则会创建该文件。
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, object_name, to_path, /, range_from=None, range_to=None, retrier=None, headers=None, before_request=None, download_progress=None, response_ok=None, response_error=None)"
    )]
    #[args(
        range_from = "None",
        range_to = "None",
        retrier = "None",
        headers = "None",
        before_request = "None",
        download_progress = "None",
        response_ok = "None",
        response_error = "None"
    )]
    fn async_download_to_path<'p>(
        &'p self,
        object_name: &str,
        to_path: String,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let object = self.make_download_object(
            object_name,
            range_from,
            range_to,
            retrier,
            headers,
            before_request,
            download_progress,
            response_ok,
            response_error,
        )?;
        pyo3_asyncio::async_std::future_into_py(py, async move {
            object
                .async_to_path(to_path)
                .await
                .map_err(QiniuDownloadError::from_err)
        })
    }

    /// 将下载的对象内容写入指定的输出流
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, object_name, to_object, /, range_from=None, range_to=None, retrier=None, headers=None, before_request=None, download_progress=None, response_ok=None, response_error=None)"
    )]
    #[args(
        range_from = "None",
        range_to = "None",
        retrier = "None",
        headers = "None",
        before_request = "None",
        download_progress = "None",
        response_ok = "None",
        response_error = "None"
    )]
    fn download_to_async_writer<'p>(
        &'p self,
        object_name: &str,
        to_object: PyObject,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let object = self.make_download_object(
            object_name,
            range_from,
            range_to,
            retrier,
            headers,
            before_request,
            download_progress,
            response_ok,
            response_error,
        )?;
        pyo3_asyncio::async_std::future_into_py(py, async move {
            object
                .to_async_writer(&mut PythonIoBase::new(to_object).into_async_write())
                .await
                .map_err(QiniuDownloadError::from_err)
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 下载阅读器
///
/// 通过 `download_manager.read()` 创建下载阅读器
#[pyclass]
#[derive(Debug)]
struct DownloadingObjectReader(qiniu_sdk::download::DownloadingObjectReader);

#[pymethods]
impl DownloadingObjectReader {
    /// 读取下载的数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyBytes> {
        let mut buf = Vec::new();
        py.allow_threads(|| {
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut self.0).take(size).read_to_end(&mut buf)
            } else {
                self.0.read_to_end(&mut buf)
            }
            .map_err(PyIOError::new_err)
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// 读取所有下载的数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        self.read(-1, py)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 异步下载阅读器
///
/// 通过 `download_manager.async_reader()` 创建下载阅读器
#[pyclass]
#[derive(Debug)]
struct AsyncDownloadingObjectReader(
    Arc<AsyncMutex<qiniu_sdk::download::AsyncDownloadingObjectReader>>,
);

#[pymethods]
impl AsyncDownloadingObjectReader {
    /// 异步读取下载的数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyAny> {
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

    /// 异步所有读取下载的数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        self.read(-1, py)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl DownloadManager {
    #[allow(clippy::too_many_arguments)]
    fn make_download_object(
        &self,
        object_name: &str,
        range_from: Option<u64>,
        range_to: Option<u64>,
        retrier: Option<DownloadRetrier>,
        headers: Option<HashMap<String, String>>,
        before_request: Option<PyObject>,
        download_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
    ) -> PyResult<qiniu_sdk::download::DownloadingObject> {
        let mut object = self
            .0
            .download(object_name)
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?;
        if let Some(range_from) = range_from {
            if let Some(range_from) = NonZeroU64::new(range_from) {
                object = object.range_from(range_from);
            }
        }
        if let Some(range_to) = range_to {
            if let Some(range_to) = NonZeroU64::new(range_to) {
                object = object.range_to(range_to);
            }
        }
        if let Some(retrier) = retrier {
            object = object.retrier(retrier);
        }
        if let Some(headers) = headers {
            object = object.headers(parse_headers(headers)?);
        }
        if let Some(before_request) = before_request {
            object = object.on_before_request(on_before_request(before_request));
        }
        if let Some(download_progress) = download_progress {
            object = object.on_download_progress(on_download_progress(download_progress));
        }
        if let Some(response_ok) = response_ok {
            object = object.on_response_ok(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            object = object.on_response_error(on_error(response_error));
        }
        Ok(object)
    }
}

/// 下载传度信息
///
/// 通过 `DownloadingProgressInfo(transferred_bytes, total_bytes)` 创建下载传度信息
#[pyclass]
#[pyo3(text_signature = "(transferred_bytes, /, total_bytes = None)")]
#[derive(Clone, Copy, Debug)]
pub(super) struct DownloadingProgressInfo {
    transferred_bytes: u64,
    total_bytes: Option<u64>,
}

#[pymethods]
impl DownloadingProgressInfo {
    #[new]
    #[args(total_bytes = "None")]
    pub(super) fn new(transferred_bytes: u64, total_bytes: Option<u64>) -> Self {
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
    fn get_total_bytes(&self) -> Option<u64> {
        self.total_bytes
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ToPyObject for DownloadingProgressInfo {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.to_owned().into_py(py)
    }
}

fn on_before_request(
    callback: PyObject,
) -> impl Fn(&mut qiniu_sdk::http_client::RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'static
{
    move |parts| {
        Python::with_gil(|py| callback.call1(py, (RequestBuilderPartsRef::new(parts),)))?;
        Ok(())
    }
}

fn on_download_progress(
    callback: PyObject,
) -> impl Fn(qiniu_sdk::download::DownloadingProgressInfo) -> AnyResult<()> + Send + Sync + 'static
{
    move |progress| {
        Python::with_gil(|py| {
            callback.call1(
                py,
                (DownloadingProgressInfo::new(
                    progress.transferred_bytes(),
                    progress.total_bytes(),
                ),),
            )
        })?;
        Ok(())
    }
}

fn on_response(
    callback: PyObject,
) -> impl Fn(&mut qiniu_sdk::http::ResponseParts) -> AnyResult<()> + Send + Sync + 'static {
    move |parts| {
        let parts = HttpResponsePartsMut::from(parts);
        Python::with_gil(|py| callback.call1(py, (parts,)))?;
        Ok(())
    }
}

fn on_error(
    callback: PyObject,
) -> impl Fn(&qiniu_sdk::http_client::ResponseError) -> AnyResult<()> + Send + Sync + 'static {
    move |error| {
        #[allow(unsafe_code)]
        let error: &'static qiniu_sdk::http_client::ResponseError = unsafe { transmute(error) };
        let error = QiniuApiCallError::from_err(MaybeOwned::Borrowed(error));
        let error = convert_api_call_error(&error)?;
        Python::with_gil(|py| callback.call1(py, (error,)))?;
        Ok(())
    }
}
