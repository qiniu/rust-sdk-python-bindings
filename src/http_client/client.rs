use crate::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuApiCallErrorInfo, QiniuAuthorizationError,
        QiniuEmptyChainedResolver, QiniuTrustDNSError,
    },
    http::{AsyncHttpRequest, Metrics, SyncHttpRequest},
    upload_token::UploadTokenProvider,
    utils::{extract_ip_addrs_with_port, parse_domain_with_port, parse_ip_addr_with_port},
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
    m.add_class::<Chooser>()?;
    m.add_class::<DirectChooser>()?;
    m.add_class::<IpChooser>()?;
    m.add_class::<SubnetChooser>()?;
    m.add_class::<ShuffledChooser>()?;
    m.add_class::<NeverEmptyHandedChooser>()?;
    m.add_class::<Idempotent>()?;
    m.add_class::<RetryDecision>()?;

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
            .map_err(QiniuAuthorizationError::from_err)
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
                .map_err(QiniuAuthorizationError::from_err)
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

impl AsRef<qiniu_sdk::http_client::RetriedStatsInfo> for RetriedStatsInfo {
    fn as_ref(&self) -> &qiniu_sdk::http_client::RetriedStatsInfo {
        &self.0
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
            .map_err(QiniuApiCallError::from_err)?
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
                .map_err(QiniuApiCallError::from_err)?
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
    text_signature = "(resolver, /, auto_persistent = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
)]
#[derive(Clone, Copy)]
struct CachedResolver;

#[pymethods]
impl CachedResolver {
    #[new]
    #[args(
        auto_persistent = "true",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        resolver: Resolver,
        auto_persistent: bool,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> (Self, Resolver) {
        (
            Self,
            Resolver(Box::new(
                Self::new_builder(resolver, cache_lifetime_secs, shrink_interval_secs)
                    .default_load_or_create_from(auto_persistent),
            )),
        )
    }

    #[staticmethod]
    #[args(
        auto_persistent = "true",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[pyo3(
        text_signature = "(resolver, path, /, auto_persistent = True, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn load_or_create_from(
        resolver: Resolver,
        path: PathBuf,
        auto_persistent: bool,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        Py::new(
            py,
            (
                Self,
                Resolver(Box::new(
                    Self::new_builder(resolver, cache_lifetime_secs, shrink_interval_secs)
                        .load_or_create_from(path, auto_persistent),
                )),
            ),
        )
    }

    #[staticmethod]
    #[args(cache_lifetime_secs = "None", shrink_interval_secs = "None")]
    #[pyo3(
        text_signature = "(resolver, /, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn in_memory(
        resolver: Resolver,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        Py::new(
            py,
            (
                Self,
                Resolver(Box::new(
                    Self::new_builder(resolver, cache_lifetime_secs, shrink_interval_secs)
                        .in_memory(),
                )),
            ),
        )
    }
}

impl CachedResolver {
    fn new_builder(
        resolver: Resolver,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> qiniu_sdk::http_client::CachedResolverBuilder<Box<dyn qiniu_sdk::http_client::Resolver>>
    {
        let mut builder = qiniu_sdk::http_client::CachedResolverBuilder::new(resolver.0);
        if let Some(cache_lifetime) = cache_lifetime_secs {
            builder = builder.cache_lifetime(Duration::from_secs(cache_lifetime));
        }
        if let Some(shrink_interval) = shrink_interval_secs {
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
                .map_err(QiniuTrustDNSError::from_err)?,
            )),
        ))
    }
}

/// 选择 IP 地址接口
///
/// 还提供了对选择结果的反馈接口，用以修正自身选择逻辑，优化选择结果
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct Chooser(Box<dyn qiniu_sdk::http_client::Chooser>);

#[pymethods]
impl Chooser {
    /// 选择 IP 地址列表
    #[pyo3(text_signature = "(ips, /, domain_with_port = None)")]
    #[args(domain_with_port = "None")]
    fn choose(
        &self,
        ips: Vec<&str>,
        domain_with_port: Option<&str>,
        py: Python<'_>,
    ) -> PyResult<Vec<String>> {
        let ips = ips
            .into_iter()
            .map(parse_ip_addr_with_port)
            .collect::<PyResult<Vec<_>>>()?;
        let domain_with_port = domain_with_port
            .map(parse_domain_with_port)
            .map_or(Ok(None), |v| v.map(Some))?;
        let mut builder = qiniu_sdk::http_client::ChooseOptions::builder();
        if let Some(domain_with_port) = &domain_with_port {
            builder.domain(domain_with_port);
        }
        Ok(py.allow_threads(|| {
            self.0
                .choose(&ips, builder.build())
                .into_iter()
                .map(|ip| ip.to_string())
                .collect()
        }))
    }

    /// 异步选择 IP 地址列表
    #[pyo3(text_signature = "(ips, /, domain_with_port = None)")]
    #[args(domain_with_port = "None")]
    fn async_choose<'p>(
        &self,
        ips: Vec<String>,
        domain_with_port: Option<&str>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let chooser = self.0.to_owned();
        let ips = ips
            .iter()
            .map(|s| parse_ip_addr_with_port(s.as_str()))
            .collect::<PyResult<Vec<_>>>()?;
        let domain_with_port = domain_with_port
            .map(parse_domain_with_port)
            .map_or(Ok(None), |v| v.map(Some))?;
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut builder = qiniu_sdk::http_client::ChooseOptions::builder();
            if let Some(domain_with_port) = &domain_with_port {
                builder.domain(domain_with_port);
            }
            Ok(chooser
                .async_choose(&ips, builder.build())
                .await
                .into_iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>())
        })
    }

    /// 反馈选择的 IP 地址列表的结果
    #[pyo3(
        text_signature = "(ips, /, domain = None, retried = None, metrics = None, error = None)"
    )]
    #[args(domain = "None", retried = "None", metrics = "None", error = "None")]
    fn feedback(
        &self,
        ips: Vec<&str>,
        domain: Option<&str>,
        retried: Option<RetriedStatsInfo>,
        metrics: Option<Metrics>,
        error: Option<&QiniuApiCallError>,
        py: Python<'_>,
    ) -> PyResult<()> {
        let ips = extract_ip_addrs_with_port(&ips)?;
        let domain = domain
            .map(parse_domain_with_port)
            .map_or(Ok(None), |v| v.map(Some))?;
        let error = error.map(PyErr::from);
        let error = error
            .as_ref()
            .map(Self::convert_api_call_error)
            .map_or(Ok(None), |v| v.map(Some))?;
        let feedback = Self::make_feedback(
            &ips,
            domain.as_ref(),
            retried.as_ref(),
            metrics.as_ref(),
            error.as_ref(),
        )?;
        py.allow_threads(|| self.0.feedback(feedback));
        Ok(())
    }

    /// 异步反馈选择的 IP 地址列表的结果
    #[pyo3(
        text_signature = "(ips, /, domain = None, retried = None, metrics = None, error = None)"
    )]
    #[args(domain = "None", retried = "None", metrics = "None", error = "None")]
    fn async_feedback<'p>(
        &self,
        ips: Vec<&str>,
        domain: Option<&str>,
        retried: Option<RetriedStatsInfo>,
        metrics: Option<Metrics>,
        error: Option<&QiniuApiCallError>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let chooser = self.0.to_owned();
        let ips = extract_ip_addrs_with_port(&ips)?;
        let domain = domain
            .map(parse_domain_with_port)
            .map_or(Ok(None), |v| v.map(Some))?;
        let error = error.map(PyErr::from);
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let error = error
                .as_ref()
                .map(Self::convert_api_call_error)
                .map_or(Ok(None), |v| v.map(Some))?;
            chooser
                .async_feedback(Self::make_feedback(
                    &ips,
                    domain.as_ref(),
                    retried.as_ref(),
                    metrics.as_ref(),
                    error.as_ref(),
                )?)
                .await;
            Ok(())
        })
    }
}

impl qiniu_sdk::http_client::Chooser for Chooser {
    fn choose(
        &self,
        ips: &[qiniu_sdk::http_client::IpAddrWithPort],
        opts: qiniu_sdk::http_client::ChooseOptions,
    ) -> qiniu_sdk::http_client::ChosenResults {
        self.0.choose(ips, opts)
    }

    fn feedback(&self, feedback: qiniu_sdk::http_client::ChooserFeedback) {
        self.0.feedback(feedback)
    }

    fn async_choose<'a>(
        &'a self,
        ips: &'a [qiniu_sdk::http_client::IpAddrWithPort],
        opts: qiniu_sdk::http_client::ChooseOptions<'a>,
    ) -> futures::future::BoxFuture<'a, qiniu_sdk::http_client::ChosenResults> {
        self.0.async_choose(ips, opts)
    }

    fn async_feedback<'a>(
        &'a self,
        feedback: qiniu_sdk::http_client::ChooserFeedback<'a>,
    ) -> futures::future::BoxFuture<'a, ()> {
        self.0.async_feedback(feedback)
    }
}

impl Chooser {
    fn make_feedback<'a>(
        ips: &'a [qiniu_sdk::http_client::IpAddrWithPort],
        domain: Option<&'a qiniu_sdk::http_client::DomainWithPort>,
        retried: Option<&'a RetriedStatsInfo>,
        metrics: Option<&'a Metrics>,
        error: Option<&'a QiniuApiCallErrorInfo>,
    ) -> PyResult<qiniu_sdk::http_client::ChooserFeedback<'a>> {
        let mut builder = qiniu_sdk::http_client::ChooserFeedback::builder(ips);
        if let Some(domain) = domain {
            builder.domain(domain);
        }
        if let Some(retried) = retried {
            builder.retried(retried.as_ref());
        }
        if let Some(metrics) = metrics {
            builder.metrics(metrics.as_ref());
        }
        if let Some(error) = error {
            builder.error(error.as_ref());
        }
        Ok(builder.build())
    }

    fn convert_api_call_error(error: &PyErr) -> PyResult<QiniuApiCallErrorInfo> {
        Python::with_gil(|py| error.value(py).getattr("args")?.get_item(0i32)?.extract())
    }
}

/// 直接选择器
///
/// 不做任何筛选，也不接受任何反馈，直接将给出的 IP 地址列表返回
#[pyclass(extends = Chooser)]
#[pyo3(text_signature = "()")]
#[derive(Clone)]
struct DirectChooser;

#[pymethods]
impl DirectChooser {
    #[new]
    fn new() -> (Self, Chooser) {
        (
            Self,
            Chooser(Box::new(qiniu_sdk::http_client::DirectChooser)),
        )
    }
}

/// IP 地址选择器
///
/// 包含 IP 地址黑名单，一旦被反馈 API 调用失败，则将所有相关 IP 地址冻结一段时间
#[pyclass(extends = Chooser)]
#[pyo3(text_signature = "(/, block_duration_secs = None, shrink_interval_secs = None)")]
#[derive(Clone)]
struct IpChooser;

#[pymethods]
impl IpChooser {
    #[new]
    #[args(block_duration_secs = "None", shrink_interval_secs = "None")]
    fn new(block_duration_secs: Option<u64>, shrink_interval_secs: Option<u64>) -> (Self, Chooser) {
        let mut builder = qiniu_sdk::http_client::IpChooser::builder();
        if let Some(block_duration_secs) = block_duration_secs {
            builder.block_duration(Duration::from_secs(block_duration_secs));
        }
        if let Some(shrink_interval_secs) = shrink_interval_secs {
            builder.shrink_interval(Duration::from_secs(shrink_interval_secs));
        }
        (Self, Chooser(Box::new(builder.build())))
    }
}

/// 子网选择器
///
/// 包含子网黑名单，一旦被反馈 API 调用失败，则将所有相关子网内 IP 地址冻结一段时间
#[pyclass(extends = Chooser)]
#[pyo3(
    text_signature = "(/, block_duration_secs = None, shrink_interval_secs = None, ipv4_netmask_prefix_length = None, ipv6_netmask_prefix_length = None)"
)]
#[derive(Clone)]
struct SubnetChooser;

#[pymethods]
impl SubnetChooser {
    #[new]
    #[args(
        block_duration_secs = "None",
        shrink_interval_secs = "None",
        ipv4_netmask_prefix_length = "None",
        ipv6_netmask_prefix_length = "None"
    )]
    fn new(
        block_duration_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
        ipv4_netmask_prefix_length: Option<u8>,
        ipv6_netmask_prefix_length: Option<u8>,
    ) -> (Self, Chooser) {
        let mut builder = qiniu_sdk::http_client::SubnetChooser::builder();
        if let Some(block_duration_secs) = block_duration_secs {
            builder.block_duration(Duration::from_secs(block_duration_secs));
        }
        if let Some(shrink_interval_secs) = shrink_interval_secs {
            builder.shrink_interval(Duration::from_secs(shrink_interval_secs));
        }
        if let Some(ipv4_netmask_prefix_length) = ipv4_netmask_prefix_length {
            builder.ipv4_netmask_prefix_length(ipv4_netmask_prefix_length);
        }
        if let Some(ipv6_netmask_prefix_length) = ipv6_netmask_prefix_length {
            builder.ipv6_netmask_prefix_length(ipv6_netmask_prefix_length);
        }
        (Self, Chooser(Box::new(builder.build())))
    }
}

/// 随机选择器
///
/// 基于一个选择器实例，但将其返回的选择结果打乱
#[pyclass(extends = Chooser)]
#[pyo3(text_signature = "(chooser)")]
#[derive(Clone)]
struct ShuffledChooser;

#[pymethods]
impl ShuffledChooser {
    #[new]
    fn new(chooser: Chooser) -> (Self, Chooser) {
        (
            Self,
            Chooser(Box::new(qiniu_sdk::http_client::ShuffledChooser::new(
                chooser,
            ))),
        )
    }
}

/// 永不空手的选择器
///
/// 确保 [`Chooser`] 实例不会因为所有可选择的 IP 地址都被屏蔽而导致 HTTP 客户端直接返回错误，
/// 在内置的 [`Chooser`] 没有返回结果时，将会随机返回一定比例的 IP 地址供 HTTP 客户端做一轮尝试。
#[pyclass(extends = Chooser)]
#[pyo3(text_signature = "(chooser, random_choose_fraction)")]
#[derive(Clone)]
struct NeverEmptyHandedChooser;

#[pymethods]
impl NeverEmptyHandedChooser {
    #[new]
    fn new(chooser: Chooser, random_choose_fraction: &PyAny) -> PyResult<(Self, Chooser)> {
        let numerator = random_choose_fraction
            .getattr("numerator")?
            .extract::<usize>()?;
        let denominator = random_choose_fraction
            .getattr("denominator")?
            .extract::<usize>()?;
        let random_choose_ratio = qiniu_sdk::http_client::Ratio::new(numerator, denominator);
        Ok((
            Self,
            Chooser(Box::new(
                qiniu_sdk::http_client::NeverEmptyHandedChooser::new(chooser, random_choose_ratio),
            )),
        ))
    }
}

#[pyclass]
#[derive(Debug, Copy, Clone)]
enum Idempotent {
    /// 根据 HTTP 方法自动判定
    ///
    /// 参考 <https://datatracker.ietf.org/doc/html/rfc7231#section-4.2.2>
    Default = 0,
    /// 总是幂等
    Always = 1,
    /// 不幂等
    Never = 2,
}

impl From<Idempotent> for qiniu_sdk::http_client::Idempotent {
    fn from(idempotent: Idempotent) -> Self {
        match idempotent {
            Idempotent::Default => qiniu_sdk::http_client::Idempotent::Default,
            Idempotent::Always => qiniu_sdk::http_client::Idempotent::Always,
            Idempotent::Never => qiniu_sdk::http_client::Idempotent::Never,
        }
    }
}

impl From<qiniu_sdk::http_client::Idempotent> for Idempotent {
    fn from(idempotent: qiniu_sdk::http_client::Idempotent) -> Self {
        match idempotent {
            qiniu_sdk::http_client::Idempotent::Default => Idempotent::Default,
            qiniu_sdk::http_client::Idempotent::Always => Idempotent::Always,
            qiniu_sdk::http_client::Idempotent::Never => Idempotent::Never,
            _ => {
                unreachable!("Unrecognized idempotent {:?}", idempotent)
            }
        }
    }
}

#[pyclass]
#[derive(Debug, Copy, Clone)]
enum RetryDecision {
    /// 不再重试
    DontRetry = 0,

    /// 切换到下一个服务器
    TryNextServer = 1,

    /// 切换到备选终端地址
    TryAlternativeEndpoints = 2,

    /// 重试当前请求
    RetryRequest = 3,

    /// 节流
    Throttled = 4,
}

impl From<RetryDecision> for qiniu_sdk::http_client::RetryDecision {
    fn from(decision: RetryDecision) -> Self {
        match decision {
            RetryDecision::DontRetry => qiniu_sdk::http_client::RetryDecision::DontRetry,
            RetryDecision::TryNextServer => qiniu_sdk::http_client::RetryDecision::TryNextServer,
            RetryDecision::TryAlternativeEndpoints => {
                qiniu_sdk::http_client::RetryDecision::TryAlternativeEndpoints
            }
            RetryDecision::RetryRequest => qiniu_sdk::http_client::RetryDecision::RetryRequest,
            RetryDecision::Throttled => qiniu_sdk::http_client::RetryDecision::Throttled,
        }
    }
}

impl From<qiniu_sdk::http_client::RetryDecision> for RetryDecision {
    fn from(decision: qiniu_sdk::http_client::RetryDecision) -> Self {
        match decision {
            qiniu_sdk::http_client::RetryDecision::DontRetry => RetryDecision::DontRetry,
            qiniu_sdk::http_client::RetryDecision::TryNextServer => RetryDecision::TryNextServer,
            qiniu_sdk::http_client::RetryDecision::TryAlternativeEndpoints => {
                RetryDecision::TryAlternativeEndpoints
            }
            qiniu_sdk::http_client::RetryDecision::RetryRequest => RetryDecision::RetryRequest,
            qiniu_sdk::http_client::RetryDecision::Throttled => RetryDecision::Throttled,
            _ => {
                unreachable!("Unrecognized decision {:?}", decision)
            }
        }
    }
}

// /// 请求重试器
// ///
// /// 根据 HTTP 客户端返回的错误，决定是否重试请求，重试决定由 [`RetryDecision`] 定义。
// #[pyclass(subclass)]
// #[derive(Clone, Debug)]
// struct RequestRetrier(Box<dyn qiniu_sdk::http_client::RequestRetrier>);

// #[pymethods]
// impl RequestRetrier {
//     /// 作出重试决定
//     #[pyo3(text_signature = "(request, response_error, /, idempotent = None, retried = None)")]
//     #[args(domain_with_port = "None")]
//     fn retry(
//         &self,
//         ips: Vec<&str>,
//         domain_with_port: Option<&str>,
//         py: Python<'_>,
//     ) -> PyResult<RetryDecision> {
//         todo!()
//     }
// }
