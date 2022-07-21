use super::{
    credential::CredentialProvider,
    exceptions::{QiniuApiCallError, QiniuEmptyEndpoints},
    http_client::{CallbackContextMut, EndpointsProvider},
    utils::{convert_api_call_error, extract_endpoints},
};
use maybe_owned::MaybeOwned;
use pyo3::prelude::*;
use std::time::Duration;

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

/// 永不重试器
///
/// 总是返回不再重试的重试器
#[pyclass(extends = DownloadRetrier)]
#[derive(Copy, Clone, Debug)]
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
#[pyclass(extends = DownloadRetrier)]
#[derive(Copy, Clone, Debug)]
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
/// 同时提供阻塞接口和异步接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct DownloadUrlsGenerator(Box<dyn qiniu_sdk::download::DownloadUrlsGenerator>);

#[pymethods]
impl DownloadUrlsGenerator {
    /// 生成下载 URL 列表
    #[pyo3(text_signature = "($self, object_name, ttl_secs=None)")]
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
    #[pyo3(text_signature = "($self, object_name, ttl_secs=None)")]
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
#[derive(Debug, Clone)]
#[pyclass(extends = DownloadUrlsGenerator)]
#[pyo3(text_signature = "(endpoints, use_https=None)")]
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
#[derive(Debug, Clone)]
#[pyclass(extends = DownloadUrlsGenerator)]
#[pyo3(text_signature = "(endpoints, use_https=None)")]
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
