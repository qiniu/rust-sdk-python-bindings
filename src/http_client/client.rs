use crate::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuAuthorizationError, QiniuEmptyChainedResolver, QiniuTrustDNSError,
    },
    http::{AsyncHttpRequest, SyncHttpRequest},
    upload_token::UploadTokenProvider,
};
use pyo3::prelude::*;
use qiniu_sdk::prelude::AuthorizationProvider;
use std::{path::PathBuf, time::Duration};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Authorization>()?;
    m.add_class::<RetriedStatsInfo>()?;
    m.add_class::<Resolver>()?;
    m.add_class::<SimpleResolver>()?;
    m.add_class::<TimeoutResolver>()?;
    m.add_class::<ShuffledResolver>()?;
    m.add_class::<CachedResolver>()?;
    m.add_class::<ChainedResolver>()?;
    m.add_class::<TrustDnsResolver>()?;

    Ok(())
}

/// 七牛鉴权签名
#[pyclass]
struct Authorization(qiniu_sdk::http_client::Authorization<'static>);

#[pymethods]
impl Authorization {
    /// 根据上传凭证获取接口创建一个上传凭证签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(upload_token_provider)")]
    fn upload_token(provider: UploadTokenProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::uptoken(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v1 签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn v1(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v1(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v2 签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn v2(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v2(provider))
    }

    /// 根据认证信息获取接口创建一个下载凭证签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn download(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::download(provider))
    }

    #[pyo3(text_signature = "($self, request)")]
    fn sign(&self, request: &mut SyncHttpRequest) -> PyResult<()> {
        self.0
            .sign(request)
            .map_err(|err| QiniuAuthorizationError::new_err(err.to_string()))
    }

    #[pyo3(text_signature = "($self, request)")]
    fn async_sign<'p>(
        &self,
        request: &mut AsyncHttpRequest,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let auth = self.0.to_owned();
        let request = request.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            auth.async_sign(&mut *request.lock()?)
                .await
                .map_err(|err| QiniuAuthorizationError::new_err(err.to_string()))
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 重试统计信息
#[pyclass]
#[derive(Clone)]
struct RetriedStatsInfo(qiniu_sdk::http_client::RetriedStatsInfo);

#[pymethods]
impl RetriedStatsInfo {
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

    /// 获取当前 IP 地址的重试次数
    #[getter]
    fn get_retried_on_current_ips(&self) -> usize {
        self.0.retried_on_current_ips()
    }

    /// 获取放弃的终端地址的数量
    #[getter]
    fn get_abandoned_endpoints(&self) -> usize {
        self.0.abandoned_endpoints()
    }

    /// 获取放弃的终端的 IP 地址的数量
    #[getter]
    fn get_abandoned_ips_of_current_endpoint(&self) -> usize {
        self.0.abandoned_ips_of_current_endpoint()
    }

    /// 是否切换到了备选终端地址
    #[getter]
    fn get_switched_to_alternative_endpoints(&self) -> bool {
        self.0.switched_to_alternative_endpoints()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}

/// 域名解析的接口
///
/// 同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能
#[pyclass(subclass)]
#[derive(Clone)]
struct Resolver(Box<dyn qiniu_sdk::http_client::Resolver>);

#[pymethods]
impl Resolver {
    /// 解析域名
    #[pyo3(text_signature = "domain, /, retried_stats_info = None")]
    #[args(retried_stats_info = "None")]
    fn resolve(
        &self,
        domain: &str,
        retried_stats_info: Option<&RetriedStatsInfo>,
        py: Python<'_>,
    ) -> PyResult<Vec<String>> {
        let mut builder = qiniu_sdk::http_client::ResolveOptions::builder();
        if let Some(retried_stats_info) = retried_stats_info {
            builder.retried(&retried_stats_info.0);
        }
        let ips = py
            .allow_threads(|| self.0.resolve(domain, builder.build()))
            .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
            .into_ip_addrs()
            .into_iter()
            .map(|ip| ip.to_string())
            .collect();
        Ok(ips)
    }

    /// 异步解析域名
    #[pyo3(text_signature = "domain, /, retried_stats_info = None")]
    #[args(retried_stats_info = "None")]
    fn async_resolve<'p>(
        &self,
        domain: String,
        retried_stats_info: Option<RetriedStatsInfo>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let resolver = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let retried_stats_info = retried_stats_info.map(|info| info.0);
            let mut builder = qiniu_sdk::http_client::ResolveOptions::builder();
            if let Some(retried_stats_info) = &retried_stats_info {
                builder.retried(retried_stats_info);
            }
            let ips = resolver
                .resolve(&domain, builder.build())
                .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
                .into_ip_addrs()
                .into_iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>();
            Ok(ips)
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 简单域名解析器
///
/// 基于 libc 库的域名解析接口实现
#[pyclass(extends = Resolver)]
#[pyo3(text_signature = "()")]
#[derive(Clone)]
struct SimpleResolver;

#[pymethods]
impl SimpleResolver {
    #[new]
    fn new() -> (Self, Resolver) {
        (
            Self,
            Resolver(Box::new(qiniu_sdk::http_client::SimpleResolver)),
        )
    }
}

/// 超时域名解析器
///
/// 为一个域名解析器实例提供超时功能
///
/// 默认超时时间为 5 秒
#[pyclass(extends = Resolver)]
#[pyo3(text_signature = "(resolver, timeout)")]
#[derive(Clone, Copy)]
struct TimeoutResolver;

#[pymethods]
impl TimeoutResolver {
    #[new]
    fn new(resolver: Resolver, timeout_ms: u64) -> (Self, Resolver) {
        (
            Self,
            Resolver(Box::new(qiniu_sdk::http_client::TimeoutResolver::new(
                resolver.0,
                Duration::from_millis(timeout_ms),
            ))),
        )
    }
}

/// 域名解析随机混淆器
///
/// 基于一个域名解析器实例，但将其返回的解析结果打乱
#[pyclass(extends = Resolver)]
#[pyo3(text_signature = "(resolver)")]
#[derive(Clone, Copy)]
struct ShuffledResolver;

#[pymethods]
impl ShuffledResolver {
    #[new]
    fn new(resolver: Resolver) -> (Self, Resolver) {
        (
            Self,
            Resolver(Box::new(qiniu_sdk::http_client::ShuffledResolver::new(
                resolver.0,
            ))),
        )
    }
}

/// 域名解析缓存器
///
/// 为一个域名解析器实例提供内存和文件系统缓存功能
///
/// 默认缓存 120 秒，清理间隔为 120 秒
#[pyclass(extends = Resolver)]
#[pyo3(
    text_signature = "(resolver, /, auto_persistent = None, cache_lifetime = None, shrink_interval = None)"
)]
#[derive(Clone, Copy)]
struct CachedResolver;

#[pymethods]
impl CachedResolver {
    #[new]
    #[args(
        auto_persistent = "true",
        cache_lifetime = "None",
        shrink_interval = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        resolver: Resolver,
        auto_persistent: bool,
        cache_lifetime: Option<u64>,
        shrink_interval: Option<u64>,
    ) -> (Self, Resolver) {
        (
            Self,
            Resolver(Box::new(
                Self::new_builder(resolver, cache_lifetime, shrink_interval)
                    .default_load_or_create_from(auto_persistent),
            )),
        )
    }

    #[staticmethod]
    #[args(
        auto_persistent = "true",
        cache_lifetime = "None",
        shrink_interval = "None"
    )]
    #[pyo3(
        text_signature = "(resolver, path, /, auto_persistent = True, cache_lifetime = None, shrink_interval = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn load_or_create_from(
        resolver: Resolver,
        path: PathBuf,
        auto_persistent: bool,
        cache_lifetime: Option<u64>,
        shrink_interval: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        Py::new(
            py,
            (
                Self,
                Resolver(Box::new(
                    Self::new_builder(resolver, cache_lifetime, shrink_interval)
                        .load_or_create_from(path, auto_persistent),
                )),
            ),
        )
    }

    #[staticmethod]
    #[args(cache_lifetime = "None", shrink_interval = "None")]
    #[pyo3(text_signature = "(resolver, /, cache_lifetime = None, shrink_interval = None)")]
    #[allow(clippy::too_many_arguments)]
    fn in_memory(
        resolver: Resolver,
        cache_lifetime: Option<u64>,
        shrink_interval: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        Py::new(
            py,
            (
                Self,
                Resolver(Box::new(
                    Self::new_builder(resolver, cache_lifetime, shrink_interval).in_memory(),
                )),
            ),
        )
    }
}

impl CachedResolver {
    fn new_builder(
        resolver: Resolver,
        cache_lifetime: Option<u64>,
        shrink_interval: Option<u64>,
    ) -> qiniu_sdk::http_client::CachedResolverBuilder<Box<dyn qiniu_sdk::http_client::Resolver>>
    {
        let mut builder = qiniu_sdk::http_client::CachedResolverBuilder::new(resolver.0);
        if let Some(cache_lifetime) = cache_lifetime {
            builder = builder.cache_lifetime(Duration::from_secs(cache_lifetime));
        }
        if let Some(shrink_interval) = shrink_interval {
            builder = builder.shrink_interval(Duration::from_secs(shrink_interval));
        }
        builder
    }
}

/// 域名解析串
///
/// 将多个域名解析器串联起来，遍历并找寻第一个可用的解析结果
#[pyclass(extends = Resolver)]
#[pyo3(text_signature = "(resolvers)")]
#[derive(Clone, Copy)]
struct ChainedResolver;

#[pymethods]
impl ChainedResolver {
    #[new]
    fn new(resolvers: Vec<Resolver>) -> PyResult<(Self, Resolver)> {
        let mut iter = resolvers.into_iter().map(|r| r.0);
        if let Some(first) = iter.next() {
            let mut builder = qiniu_sdk::http_client::ChainedResolver::builder(first);
            builder.extend(iter);
            Ok((Self, Resolver(Box::new(builder.build()))))
        } else {
            Err(QiniuEmptyChainedResolver::new_err("empty resolvers"))
        }
    }
}

/// Trust-DNS 域名解析器
#[pyclass(extends = Resolver)]
#[pyo3(text_signature = "()")]
#[derive(Clone, Copy)]
struct TrustDnsResolver;

#[pymethods]
impl TrustDnsResolver {
    #[new]
    fn new() -> PyResult<(Self, Resolver)> {
        Ok((
            Self,
            Resolver(Box::new(
                async_std::task::block_on(async {
                    qiniu_sdk::http_client::TrustDnsResolver::from_system_conf().await
                })
                .map_err(|err| QiniuTrustDNSError::new_err(err.to_string()))?,
            )),
        ))
    }
}
