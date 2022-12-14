use super::region::ServiceName;
use crate::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuApiCallErrorInfo, QiniuAuthorizationError,
        QiniuBodySizeMissingError, QiniuEmptyChainedResolver, QiniuHeaderValueEncodingError,
        QiniuInvalidPrefixLengthError, QiniuIoError, QiniuIsahcError, QiniuJsonError,
        QiniuTrustDNSError,
    },
    http::{
        AsyncHttpRequest, AsyncHttpResponse, HttpCaller, HttpRequestParts, HttpResponseParts,
        HttpResponsePartsRef, Metrics, SyncHttpRequest, SyncHttpResponse, TransferProgressInfo,
        Version,
    },
    upload_token::UploadTokenProvider,
    utils::{
        convert_api_call_error, convert_headers_to_hashmap, convert_py_any_to_json_value,
        extract_async_multipart, extract_endpoints_provider, extract_ip_addrs_with_port,
        extract_sync_multipart, parse_domain_with_port, parse_header_name, parse_header_value,
        parse_headers, parse_ip_addr_with_port, parse_ip_addrs, parse_method, parse_mime,
        parse_query_pairs, PythonIoBase,
    },
};
use anyhow::Result as AnyResult;
use maybe_owned::MaybeOwned;
use num_integer::Integer;
use pyo3::{prelude::*, types::PyIterator};
use qiniu_sdk::prelude::AuthorizationProvider;
use std::{borrow::Cow, collections::HashMap, mem::transmute, path::PathBuf, time::Duration};

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
    m.add_class::<RequestRetrier>()?;
    m.add_class::<NeverRetrier>()?;
    m.add_class::<ErrorRetrier>()?;
    m.add_class::<LimitedRetrier>()?;
    m.add_class::<Backoff>()?;
    m.add_class::<FixedBackoff>()?;
    m.add_class::<RandomizedBackoff>()?;
    m.add_class::<ExponentialBackoff>()?;
    m.add_class::<LimitedBackoff>()?;
    m.add_class::<HttpClient>()?;
    m.add_class::<SimplifiedCallbackContext>()?;
    m.add_class::<CallbackContextMut>()?;
    m.add_class::<ExtendedCallbackContextRef>()?;
    m.add_class::<RequestBuilderPartsRef>()?;
    m.add_class::<JsonResponse>()?;

    Ok(())
}

/// ??????????????????
///
/// ???????????? `Authorization.upload_token(upload_token_provider)` ??? `Authorization.v1(credential_provider)` ??? `Authorization.v2(credential_provider)` ??? `Authorization.download(credential_provider)` ????????????????????????
#[pyclass]
#[derive(Clone)]
pub(crate) struct Authorization(qiniu_sdk::http_client::Authorization<'static>);

#[pymethods]
impl Authorization {
    /// ???????????????????????????????????????????????????????????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(upload_token_provider)")]
    fn upload_token(provider: UploadTokenProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::uptoken(provider))
    }

    /// ???????????????????????????????????????????????????????????? v1 ?????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn v1(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v1(provider))
    }

    /// ???????????????????????????????????????????????????????????? v2 ?????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn v2(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v2(provider))
    }

    /// ???????????????????????????????????????????????????????????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn download(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::download(provider))
    }

    /// ?????????????????????????????? HTTP ??????????????????
    #[pyo3(text_signature = "($self, request)")]
    fn sign(&self, request: PyRefMut<SyncHttpRequest>) -> PyResult<()> {
        SyncHttpRequest::with_request_from_ref_mut(request, |request| {
            self.0
                .sign(request)
                .map_err(QiniuAuthorizationError::from_err)
        })
    }

    /// ???????????????????????????????????? HTTP ??????????????????
    #[pyo3(text_signature = "($self, request)")]
    fn async_sign<'p>(&self, request: Py<AsyncHttpRequest>, py: Python<'p>) -> PyResult<&'p PyAny> {
        let auth = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(
            py,
            AsyncHttpRequest::with_request_from_ref_mut(request, move |request, agent| {
                Box::pin(async move {
                    if let Some(agent) = agent {
                        agent.run(auth.async_sign(request)).await?
                    } else {
                        auth.async_sign(request).await
                    }
                    .map_err(QiniuAuthorizationError::from_err)
                })
            }),
        )
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<qiniu_sdk::http_client::Authorization<'static>> for Authorization {
    fn from(authorization: qiniu_sdk::http_client::Authorization<'static>) -> Self {
        Self(authorization)
    }
}

impl From<Authorization> for qiniu_sdk::http_client::Authorization<'static> {
    fn from(authorization: Authorization) -> Self {
        authorization.0
    }
}

/// ??????????????????
///
/// ?????? `RetriedStatsInfo()` ????????????????????????
#[pyclass]
#[pyo3(text_signature = "()")]
#[derive(Clone)]
struct RetriedStatsInfo(qiniu_sdk::http_client::RetriedStatsInfo);

#[pymethods]
impl RetriedStatsInfo {
    #[new]
    fn new() -> Self {
        RetriedStatsInfo(Default::default())
    }

    /// ???????????????????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn increase_current_endpoint(&mut self) {
        self.0.increase_current_endpoint();
    }

    /// ????????????????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn increase_abandoned_endpoints(&mut self) {
        self.0.increase_abandoned_endpoints();
    }

    /// ???????????????????????? IP ???????????????
    #[pyo3(text_signature = "($self)")]
    fn increase_abandoned_ips_of_current_endpoint(&mut self) {
        self.0.increase_abandoned_ips_of_current_endpoint()
    }

    /// ???????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn switch_to_alternative_endpoints(&mut self) {
        self.0.switch_to_alternative_endpoints();
    }

    /// ??????????????????
    #[pyo3(text_signature = "($self)")]
    fn switch_endpoint(&mut self) {
        self.0.switch_endpoint();
    }

    /// ???????????? IP ??????
    #[pyo3(text_signature = "($self)")]
    fn switch_ips(&mut self) {
        self.0.switch_ips();
    }

    /// ???????????????????????????
    #[getter]
    fn get_retried_total(&self) -> usize {
        self.0.retried_total()
    }

    /// ???????????????????????????????????????
    #[getter]
    fn get_retried_on_current_endpoint(&self) -> usize {
        self.0.retried_on_current_endpoint()
    }

    /// ???????????? IP ?????????????????????
    #[getter]
    fn get_retried_on_current_ips(&self) -> usize {
        self.0.retried_on_current_ips()
    }

    /// ????????????????????????????????????
    #[getter]
    fn get_abandoned_endpoints(&self) -> usize {
        self.0.abandoned_endpoints()
    }

    /// ???????????????????????? IP ???????????????
    #[getter]
    fn get_abandoned_ips_of_current_endpoint(&self) -> usize {
        self.0.abandoned_ips_of_current_endpoint()
    }

    /// ????????????????????????????????????
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

/// ?????????????????????
///
/// ?????????
///
/// ????????????????????????????????????????????????????????????????????? `async` ??????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub(crate) struct Resolver(Box<dyn qiniu_sdk::http_client::Resolver>);

#[pymethods]
impl Resolver {
    /// ????????????
    #[pyo3(text_signature = "($self, domain, /, retried_stats_info = None)")]
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
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
            .into_ip_addrs()
            .into_iter()
            .map(|ip| ip.to_string())
            .collect();
        Ok(ips)
    }

    /// ??????????????????
    #[pyo3(text_signature = "($self, domain, /, retried_stats_info = None)")]
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
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
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

impl qiniu_sdk::http_client::Resolver for Resolver {
    fn resolve(
        &self,
        domain: &str,
        opts: qiniu_sdk::http_client::ResolveOptions<'_>,
    ) -> qiniu_sdk::http_client::ResolveResult {
        self.0.resolve(domain, opts)
    }

    fn async_resolve<'a>(
        &'a self,
        domain: &'a str,
        opts: qiniu_sdk::http_client::ResolveOptions<'a>,
    ) -> futures::future::BoxFuture<'a, qiniu_sdk::http_client::ResolveResult> {
        self.0.async_resolve(domain, opts)
    }
}

/// ?????????????????????
///
/// ?????? `libc` ??????????????????????????????
///
/// ?????? `SimpleResolver()` ???????????????????????????
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

/// ?????????????????????
///
/// ????????????????????????????????????????????????
///
/// ?????? `SimpleResolver(resolver)` ???????????????????????????
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
                resolver,
                Duration::from_millis(timeout_ms),
            ))),
        )
    }
}

/// ???????????????????????????
///
/// ????????????????????????????????????????????????????????????????????????
///
/// ?????? `ShuffledResolver(resolver)` ?????????????????????????????????
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
                resolver,
            ))),
        )
    }
}

/// ?????????????????????
///
/// ?????????????????????????????????????????????????????????????????????
///
/// ???????????? 120 ????????????????????? 120 ???
///
/// ?????? `CachedResolver(resolver, auto_persistent = None, cache_lifetime_secs = None, shrink_interval_secs = None)` ???????????????????????????
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

    /// ???????????????????????????????????????????????????
    ///
    /// ???????????????????????????????????????????????????
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

    /// ???????????????????????????
    ///
    /// ????????????????????????????????????
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
    ) -> qiniu_sdk::http_client::CachedResolverBuilder<Resolver> {
        let mut builder = qiniu_sdk::http_client::CachedResolverBuilder::new(resolver);
        if let Some(cache_lifetime) = cache_lifetime_secs {
            builder = builder.cache_lifetime(Duration::from_secs(cache_lifetime));
        }
        if let Some(shrink_interval) = shrink_interval_secs {
            builder = builder.shrink_interval(Duration::from_secs(shrink_interval));
        }
        builder
    }
}

/// ???????????????
///
/// ????????????????????????????????????????????????????????????????????????????????????
///
/// ?????? `ChainedResolver(resolvers)` ?????????????????????
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

/// Trust-DNS ???????????????
///
/// ?????? `TrustDnsResolver()` ?????? Trust-DNS ???????????????
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

/// ?????? IP ????????????
///
/// ?????????
///
/// ????????????????????????????????????????????????????????????????????????????????????????????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub(crate) struct Chooser(Box<dyn qiniu_sdk::http_client::Chooser>);

#[pymethods]
impl Chooser {
    /// ?????? IP ????????????
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
        let domain_with_port = domain_with_port.map(parse_domain_with_port).transpose()?;
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

    /// ???????????? IP ????????????
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
        let domain_with_port = domain_with_port.map(parse_domain_with_port).transpose()?;
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

    /// ??????????????? IP ?????????????????????
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
        let domain = domain.map(parse_domain_with_port).transpose()?;
        let error = error.map(PyErr::from);
        let error = error.as_ref().map(convert_api_call_error).transpose()?;
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

    /// ????????????????????? IP ?????????????????????
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
        let domain = domain.map(parse_domain_with_port).transpose()?;
        let error = error.map(PyErr::from);
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let error = error.as_ref().map(convert_api_call_error).transpose()?;
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

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
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
}

/// ???????????????
///
/// ?????????????????????????????????????????????????????????????????? IP ??????????????????
///
/// ?????? `DirectChooser()` ?????????????????????
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

/// IP ???????????????
///
/// ?????? IP ????????????????????????????????? API ????????????????????????????????? IP ????????????????????????
///
/// ?????? `IpChooser(block_duration_secs = None, shrink_interval_secs = None)` ?????? IP ???????????????
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

/// ???????????????
///
/// ??????????????????????????????????????? API ?????????????????????????????????????????? IP ????????????????????????
///
/// ?????? `SubnetChooser(block_duration_secs = None, shrink_interval_secs = None, ipv4_netmask_prefix_length = None, ipv6_netmask_prefix_length = None)` ?????????????????????
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
    ) -> PyResult<(Self, Chooser)> {
        let mut builder = qiniu_sdk::http_client::SubnetChooser::builder();
        if let Some(block_duration_secs) = block_duration_secs {
            builder.block_duration(Duration::from_secs(block_duration_secs));
        }
        if let Some(shrink_interval_secs) = shrink_interval_secs {
            builder.shrink_interval(Duration::from_secs(shrink_interval_secs));
        }
        if let Some(ipv4_netmask_prefix_length) = ipv4_netmask_prefix_length {
            builder
                .ipv4_netmask_prefix_length(ipv4_netmask_prefix_length)
                .map_err(QiniuInvalidPrefixLengthError::from_err)?;
        }
        if let Some(ipv6_netmask_prefix_length) = ipv6_netmask_prefix_length {
            builder
                .ipv6_netmask_prefix_length(ipv6_netmask_prefix_length)
                .map_err(QiniuInvalidPrefixLengthError::from_err)?;
        }
        Ok((Self, Chooser(Box::new(builder.build()))))
    }
}

/// ???????????????
///
/// ??????????????????????????????????????????????????????????????????
///
/// ?????? `ShuffledChooser(chooser)` ?????????????????????
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

/// ????????????????????????
///
/// ?????? [`Chooser`] ???????????????????????????????????? IP ??????????????????????????? HTTP ??????????????????????????????
/// ???????????? [`Chooser`] ????????????????????????????????????????????????????????? IP ????????? HTTP ???????????????????????????
///
/// ?????? `NeverEmptyHandedChooser(chooser, random_choose_fraction)` ??????????????????????????????
#[pyclass(extends = Chooser)]
#[pyo3(text_signature = "(chooser, random_choose_fraction)")]
#[derive(Clone)]
struct NeverEmptyHandedChooser;

#[pymethods]
impl NeverEmptyHandedChooser {
    #[new]
    fn new(chooser: Chooser, random_choose_fraction: &PyAny) -> PyResult<(Self, Chooser)> {
        let random_choose_ratio = convert_fraction(random_choose_fraction)?;
        Ok((
            Self,
            Chooser(Box::new(
                qiniu_sdk::http_client::NeverEmptyHandedChooser::new(chooser, random_choose_ratio),
            )),
        ))
    }
}

/// API ?????????
#[pyclass]
#[derive(Debug, Copy, Clone)]
pub(crate) enum Idempotent {
    /// ?????? HTTP ??????????????????
    ///
    /// ?????? <https://datatracker.ietf.org/doc/html/rfc7231#section-4.2.2>
    Default = 0,
    /// ????????????
    Always = 1,
    /// ?????????
    Never = 2,
}

#[pymethods]
impl Idempotent {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
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

/// ????????????
#[pyclass]
#[derive(Debug, Copy, Clone)]
enum RetryDecision {
    /// ????????????
    DontRetry = 0,

    /// ???????????????????????????
    TryNextServer = 1,

    /// ???????????????????????????
    TryAlternativeEndpoints = 2,

    /// ??????????????????
    RetryRequest = 3,

    /// ??????
    Throttled = 4,
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

/// ???????????????
///
/// ?????????
///
/// ?????? HTTP ????????????????????????????????????????????????????????????????????? [`RetryDecision`] ?????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub(crate) struct RequestRetrier(Box<dyn qiniu_sdk::http_client::RequestRetrier>);

#[pymethods]
impl RequestRetrier {
    /// ??????????????????
    #[pyo3(text_signature = "(request, error, /, idempotent = None, retried = None)")]
    #[args(idempotent = "None", retried = "None")]
    fn retry(
        &self,
        request: &mut HttpRequestParts,
        error: &QiniuApiCallError,
        idempotent: Option<Idempotent>,
        retried: Option<RetriedStatsInfo>,
    ) -> PyResult<RetryDecision> {
        let error = convert_api_call_error(&PyErr::from(error))?;
        let retried = retried.map(|r| r.0).unwrap_or_default();
        let mut builder =
            qiniu_sdk::http_client::RequestRetrierOptions::builder(error.as_ref(), &retried);
        if let Some(idempotent) = idempotent {
            builder.idempotent(idempotent.into());
        }
        let opts = builder.build();
        Ok(self.0.retry(&mut *request, opts).decision().into())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::http_client::RequestRetrier for RequestRetrier {
    fn retry(
        &self,
        request: &mut qiniu_sdk::http::RequestParts,
        opts: qiniu_sdk::http_client::RequestRetrierOptions<'_>,
    ) -> qiniu_sdk::http_client::RetryResult {
        self.0.retry(request, opts)
    }
}

/// ???????????????
///
/// ????????????????????????????????????
///
/// ?????? `NeverRetrier()` ?????????????????????
#[pyclass(extends = RequestRetrier)]
#[pyo3(text_signature = "()")]
#[derive(Copy, Clone)]
struct NeverRetrier;

#[pymethods]
impl NeverRetrier {
    #[new]
    fn new() -> (Self, RequestRetrier) {
        (
            Self,
            RequestRetrier(Box::new(qiniu_sdk::http_client::NeverRetrier)),
        )
    }
}

/// ???????????? API ????????????????????????????????????
///
/// ?????? `ErrorRetrier()` ??????????????????????????????
#[pyclass(extends = RequestRetrier)]
#[pyo3(text_signature = "()")]
#[derive(Copy, Clone)]
struct ErrorRetrier;

#[pymethods]
impl ErrorRetrier {
    #[new]
    fn new() -> (Self, RequestRetrier) {
        (
            Self,
            RequestRetrier(Box::new(qiniu_sdk::http_client::ErrorRetrier)),
        )
    }
}

/// ???????????????
///
/// ????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
///
/// ?????? `LimitedRetrier(retrier, retries)` ?????????????????????
#[pyclass(extends = RequestRetrier)]
#[pyo3(text_signature = "(retrier, retries)")]
#[derive(Copy, Clone)]
struct LimitedRetrier;

#[pymethods]
impl LimitedRetrier {
    #[new]
    fn new(retrier: RequestRetrier, retries: usize) -> (Self, RequestRetrier) {
        (
            Self,
            RequestRetrier(Box::new(qiniu_sdk::http_client::LimitedRetrier::new(
                retrier, retries,
            ))),
        )
    }

    /// ?????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(retrier, retries)")]
    fn limit_total(retrier: RequestRetrier, retries: usize, py: Python<'_>) -> PyResult<Py<Self>> {
        Py::new(
            py,
            (
                Self,
                RequestRetrier(Box::new(
                    qiniu_sdk::http_client::LimitedRetrier::limit_total(retrier, retries),
                )),
            ),
        )
    }
    /// ???????????????????????????????????????????????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(retrier, retries)")]
    fn limit_current_endpoint(
        retrier: RequestRetrier,
        retries: usize,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        Py::new(
            py,
            (
                Self,
                RequestRetrier(Box::new(
                    qiniu_sdk::http_client::LimitedRetrier::limit_current_endpoint(
                        retrier, retries,
                    ),
                )),
            ),
        )
    }
}

/// ????????????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub(crate) struct Backoff(Box<dyn qiniu_sdk::http_client::Backoff>);

#[pymethods]
impl Backoff {
    /// ??????????????????
    #[pyo3(text_signature = "(request, error, /, decision = None, retried = None)")]
    #[args(idempotent = "None", retried = "None")]
    fn time_ns(
        &self,
        request: &mut HttpRequestParts,
        error: &QiniuApiCallError,
        decision: Option<RetryDecision>,
        retried: Option<RetriedStatsInfo>,
    ) -> PyResult<u128> {
        let error = convert_api_call_error(&PyErr::from(error))?;
        let retried = retried.map(|r| r.0).unwrap_or_default();
        let mut builder = qiniu_sdk::http_client::BackoffOptions::builder(error.as_ref(), &retried);
        if let Some(decision) = decision {
            builder.retry_decision(decision.into());
        }
        let opts = builder.build();
        Ok(self.0.time(&mut *request, opts).duration().as_nanos())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::http_client::Backoff for Backoff {
    fn time(
        &self,
        request: &mut qiniu_sdk::http::RequestParts,
        opts: qiniu_sdk::http_client::BackoffOptions,
    ) -> qiniu_sdk::http_client::GotBackoffDuration {
        self.0.time(request, opts)
    }
}

/// ????????????????????????????????????
///
/// ?????? `FixedBackoff(delay_ns)` ??????????????????????????????????????????
#[pyclass(extends = Backoff)]
#[pyo3(text_signature = "(delay)")]
#[derive(Copy, Clone)]
struct FixedBackoff {
    delay_ns: u64,
}

#[pymethods]
impl FixedBackoff {
    #[new]
    fn new(delay_ns: u64) -> (Self, Backoff) {
        (
            Self { delay_ns },
            Backoff(Box::new(qiniu_sdk::http_client::FixedBackoff::new(
                Duration::from_nanos(delay_ns),
            ))),
        )
    }

    /// ??????????????????
    #[getter]
    fn get_delay(&self) -> u64 {
        self.delay_ns
    }
}

/// ???????????????????????????????????????
///
/// ?????? `ExponentialBackoff(base_number, base_delay_ns)` ?????????????????????????????????????????????
#[pyclass(extends = Backoff)]
#[pyo3(text_signature = "(base_number, base_delay)")]
#[derive(Copy, Clone)]
struct ExponentialBackoff {
    base_number: u32,
    base_delay_ns: u64,
}

#[pymethods]
impl ExponentialBackoff {
    #[new]
    fn new(base_number: u32, base_delay_ns: u64) -> (Self, Backoff) {
        (
            Self {
                base_number,
                base_delay_ns,
            },
            Backoff(Box::new(qiniu_sdk::http_client::ExponentialBackoff::new(
                base_number,
                Duration::from_nanos(base_delay_ns),
            ))),
        )
    }

    /// ????????????
    #[getter]
    fn get_base_number(&self) -> u32 {
        self.base_number
    }

    /// ????????????
    #[getter]
    fn get_base_delay(&self) -> u64 {
        self.base_delay_ns
    }
}

/// ??????????????????????????????????????????
///
/// ???????????????????????????????????????????????????????????????
///
/// ?????? `RandomizedBackoff(base_backoff, minification, magnification)` ????????????????????????????????????????????????
#[pyclass(extends = Backoff)]
#[pyo3(text_signature = "(base_backoff, minification, magnification)")]
#[derive(Clone)]
struct RandomizedBackoff {
    minification: PyObject,
    magnification: PyObject,
}

#[pymethods]
impl RandomizedBackoff {
    #[new]
    fn new(
        base_backoff: Backoff,
        minification: PyObject,
        magnification: PyObject,
        py: Python<'_>,
    ) -> PyResult<(Self, Backoff)> {
        let minification_ratio = convert_fraction(minification.as_ref(py))?;
        let magnification_ratio = convert_fraction(magnification.as_ref(py))?;
        Ok((
            Self {
                minification,
                magnification,
            },
            Backoff(Box::new(qiniu_sdk::http_client::RandomizedBackoff::new(
                base_backoff,
                minification_ratio,
                magnification_ratio,
            ))),
        ))
    }

    /// ????????????????????????
    #[getter]
    fn get_minification<'p>(&'p self, py: Python<'p>) -> &'p PyAny {
        self.minification.as_ref(py)
    }

    /// ????????????????????????
    #[getter]
    fn get_magnification<'p>(&'p self, py: Python<'p>) -> &'p PyAny {
        self.magnification.as_ref(py)
    }
}

/// ????????????????????????????????????
///
/// ?????? `LimitedBackoff(back_backoff, min_backoff_ns, max_backoff_ns)` ??????????????????????????????????????????
#[pyclass(extends = Backoff)]
#[pyo3(text_signature = "(back_backoff, min_backoff_ns, max_backoff_ns)")]
#[derive(Copy, Clone)]
struct LimitedBackoff {
    max_backoff_ns: u64,
    min_backoff_ns: u64,
}

#[pymethods]
impl LimitedBackoff {
    #[new]
    fn new(base_backoff: Backoff, min_backoff_ns: u64, max_backoff_ns: u64) -> (Self, Backoff) {
        (
            Self {
                max_backoff_ns,
                min_backoff_ns,
            },
            Backoff(Box::new(qiniu_sdk::http_client::LimitedBackoff::new(
                base_backoff,
                Duration::from_nanos(min_backoff_ns),
                Duration::from_nanos(max_backoff_ns),
            ))),
        )
    }

    /// ???????????????????????????
    #[getter]
    fn get_min_backoff(&self) -> u64 {
        self.min_backoff_ns
    }

    /// ???????????????????????????
    #[getter]
    fn get_max_backoff(&self) -> u64 {
        self.max_backoff_ns
    }
}

fn convert_fraction<'a, U: FromPyObject<'a> + Clone + Integer>(
    fraction: &'a PyAny,
) -> PyResult<qiniu_sdk::http_client::Ratio<U>> {
    let numerator = fraction.getattr("numerator")?.extract::<'a, U>()?;
    let denominator = fraction.getattr("denominator")?.extract::<'a, U>()?;
    let ratio = qiniu_sdk::http_client::Ratio::new(numerator, denominator);
    Ok(ratio)
}

/// HTTP ?????????
///
/// ???????????? HTTP ??????????????????
///
/// ?????? `HttpClient(http_caller = None, use_https = None, appended_user_agent = None, request_retrier = None, backoff = None, chooser = None, resolver = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)` ?????? HTTP ?????????
#[pyclass(subclass)]
#[pyo3(
    text_signature = "(/, http_caller = None, use_https = None, appended_user_agent = None, request_retrier = None, backoff = None, chooser = None, resolver = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)"
)]
#[derive(Clone)]
pub(crate) struct HttpClient(qiniu_sdk::http_client::HttpClient);

#[pymethods]
impl HttpClient {
    #[new]
    #[args(
        http_caller = "None",
        use_https = "None",
        appended_user_agent = "None",
        request_retrier = "None",
        backoff = "None",
        chooser = "None",
        resolver = "None",
        uploading_progress = "None",
        receive_response_status = "None",
        receive_response_header = "None",
        to_resolve_domain = "None",
        domain_resolved = "None",
        to_choose_ips = "None",
        ips_chosen = "None",
        before_request_signed = "None",
        after_request_signed = "None",
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        http_caller: Option<HttpCaller>,
        use_https: Option<bool>,
        appended_user_agent: Option<&str>,
        request_retrier: Option<RequestRetrier>,
        backoff: Option<Backoff>,
        chooser: Option<Chooser>,
        resolver: Option<Resolver>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
    ) -> PyResult<Self> {
        let mut builder = if let Some(http_caller) = http_caller {
            qiniu_sdk::http_client::HttpClient::builder(http_caller)
        } else {
            qiniu_sdk::http_client::HttpClient::build_isahc().map_err(QiniuIsahcError::from_err)?
        };

        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        if let Some(appended_user_agent) = appended_user_agent {
            builder.appended_user_agent(appended_user_agent);
        }
        if let Some(request_retrier) = request_retrier {
            builder.request_retrier(request_retrier);
        }
        if let Some(backoff) = backoff {
            builder.backoff(backoff);
        }
        if let Some(chooser) = chooser {
            builder.chooser(chooser);
        }
        if let Some(resolver) = resolver {
            builder.resolver(resolver);
        }
        if let Some(uploading_progress) = uploading_progress {
            builder.on_uploading_progress(on_uploading_progress(uploading_progress));
        }
        if let Some(receive_response_status) = receive_response_status {
            builder.on_receive_response_status(on_receive_response_status(receive_response_status));
        }
        if let Some(receive_response_header) = receive_response_header {
            builder.on_receive_response_header(on_receive_response_header(receive_response_header));
        }
        if let Some(to_resolve_domain) = to_resolve_domain {
            builder.on_to_resolve_domain(on_to_resolve_domain(to_resolve_domain));
        }
        if let Some(domain_resolved) = domain_resolved {
            builder.on_domain_resolved(on_domain_resolved(domain_resolved));
        }
        if let Some(to_choose_ips) = to_choose_ips {
            builder.on_to_choose_ips(on_to_choose_ips(to_choose_ips));
        }
        if let Some(ips_chosen) = ips_chosen {
            builder.on_ips_chosen(on_ips_chosen(ips_chosen));
        }
        if let Some(before_request_signed) = before_request_signed {
            builder.on_before_request_signed(on_request_signed(before_request_signed));
        }
        if let Some(after_request_signed) = after_request_signed {
            builder.on_after_request_signed(on_request_signed(after_request_signed));
        }
        if let Some(response_ok) = response_ok {
            builder.on_response(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            builder.on_error(on_error(response_error));
        }
        if let Some(before_backoff) = before_backoff {
            builder.on_before_backoff(on_backoff(before_backoff));
        }
        if let Some(after_backoff) = after_backoff {
            builder.on_after_backoff(on_backoff(after_backoff));
        }

        Ok(Self(builder.build()))
    }

    /// ??????????????? [`HttpCaller`] ??????
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn default_http_caller() -> HttpCaller {
        HttpCaller::new(qiniu_sdk::http_client::HttpClient::default_http_caller())
    }

    /// ??????????????? [`Resolver`] ??????
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn default_resolver() -> Resolver {
        Resolver(qiniu_sdk::http_client::HttpClient::default_resolver())
    }

    /// ??????????????? [`Chooser`] ??????
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn default_chooser() -> Chooser {
        Chooser(qiniu_sdk::http_client::HttpClient::default_chooser())
    }

    /// ??????????????? [`RequestRetrier`] ??????
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn default_retrier() -> RequestRetrier {
        RequestRetrier(qiniu_sdk::http_client::HttpClient::default_retrier())
    }

    /// ??????????????? [`Backoff`] ??????
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn default_backoff() -> Backoff {
        Backoff(qiniu_sdk::http_client::HttpClient::default_backoff())
    }

    /// ??????????????????
    #[pyo3(
        text_signature = "(method, endpoints, /, service_names = None, use_https = None, version = None, path = None, headers = None, accept_json = None, accept_application_octet_stream = None, query = None, query_pairs = None, appended_user_agent = None, authorization = None, idempotent = None, bytes = None, body = None, body_len = None, content_type = None, json = None, form = None, multipart = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)"
    )]
    #[args(
        service_names = "None",
        use_https = "None",
        version = "None",
        path = "None",
        headers = "None",
        accept_json = "None",
        accept_application_octet_stream = "None",
        query = "None",
        query_pairs = "None",
        appended_user_agent = "None",
        authorization = "None",
        idempotent = "None",
        bytes = "None",
        body = "None",
        body_len = "None",
        content_type = "None",
        json = "None",
        form = "None",
        multipart = "None",
        uploading_progress = "None",
        receive_response_status = "None",
        receive_response_header = "None",
        to_resolve_domain = "None",
        domain_resolved = "None",
        to_choose_ips = "None",
        ips_chosen = "None",
        before_request_signed = "None",
        after_request_signed = "None",
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn call(
        &self,
        method: String,
        endpoints: PyObject,
        service_names: Option<Vec<ServiceName>>,
        use_https: Option<bool>,
        version: Option<Version>,
        path: Option<String>,
        headers: Option<HashMap<String, String>>,
        accept_json: Option<bool>,
        accept_application_octet_stream: Option<bool>,
        query: Option<String>,
        query_pairs: Option<PyObject>,
        appended_user_agent: Option<String>,
        authorization: Option<Authorization>,
        idempotent: Option<Idempotent>,
        bytes: Option<Vec<u8>>,
        body: Option<PyObject>,
        body_len: Option<u64>,
        content_type: Option<String>,
        json: Option<PyObject>,
        form: Option<Vec<(String, Option<String>)>>,
        multipart: Option<HashMap<String, PyObject>>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<SyncHttpResponse>> {
        let (resp, parts) = self._call(
            method,
            endpoints,
            service_names,
            use_https,
            version,
            path,
            headers,
            accept_json,
            accept_application_octet_stream,
            query,
            query_pairs,
            appended_user_agent,
            authorization,
            idempotent,
            bytes,
            body,
            body_len,
            content_type,
            json,
            form,
            multipart,
            uploading_progress,
            receive_response_status,
            receive_response_header,
            to_resolve_domain,
            domain_resolved,
            to_choose_ips,
            ips_chosen,
            before_request_signed,
            after_request_signed,
            response_ok,
            response_error,
            before_backoff,
            after_backoff,
            py,
        )?;
        Py::new(py, (resp, parts))
    }

    /// ??????????????????
    #[pyo3(
        text_signature = "(method, endpoints, /, service_names = None, use_https = None, version = None, path = None, headers = None, accept_json = None, accept_application_octet_stream = None, query = None, query_pairs = None, appended_user_agent = None, authorization = None, idempotent = None, bytes = None, body = None, body_len = None, content_type = None, json = None, form = None, multipart = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)"
    )]
    #[args(
        service_names = "None",
        use_https = "None",
        version = "None",
        path = "None",
        headers = "None",
        accept_json = "None",
        accept_application_octet_stream = "None",
        query = "None",
        query_pairs = "None",
        appended_user_agent = "None",
        authorization = "None",
        idempotent = "None",
        bytes = "None",
        body = "None",
        body_len = "None",
        content_type = "None",
        json = "None",
        form = "None",
        multipart = "None",
        uploading_progress = "None",
        receive_response_status = "None",
        receive_response_header = "None",
        to_resolve_domain = "None",
        domain_resolved = "None",
        to_choose_ips = "None",
        ips_chosen = "None",
        before_request_signed = "None",
        after_request_signed = "None",
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn async_call<'p>(
        &self,
        method: String,
        endpoints: PyObject,
        service_names: Option<Vec<ServiceName>>,
        use_https: Option<bool>,
        version: Option<Version>,
        path: Option<String>,
        headers: Option<HashMap<String, String>>,
        accept_json: Option<bool>,
        accept_application_octet_stream: Option<bool>,
        query: Option<String>,
        query_pairs: Option<PyObject>,
        appended_user_agent: Option<String>,
        authorization: Option<Authorization>,
        idempotent: Option<Idempotent>,
        bytes: Option<Vec<u8>>,
        body: Option<PyObject>,
        body_len: Option<u64>,
        content_type: Option<String>,
        json: Option<PyObject>,
        form: Option<Vec<(String, Option<String>)>>,
        multipart: Option<HashMap<String, PyObject>>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let http_client = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let (resp, parts) = http_client
                ._async_call(
                    method,
                    endpoints,
                    service_names,
                    use_https,
                    version,
                    path,
                    headers,
                    accept_json,
                    accept_application_octet_stream,
                    query,
                    query_pairs,
                    appended_user_agent,
                    authorization,
                    idempotent,
                    bytes,
                    body,
                    body_len,
                    content_type,
                    json,
                    form,
                    multipart,
                    uploading_progress,
                    receive_response_status,
                    receive_response_header,
                    to_resolve_domain,
                    domain_resolved,
                    to_choose_ips,
                    ips_chosen,
                    before_request_signed,
                    after_request_signed,
                    response_ok,
                    response_error,
                    before_backoff,
                    after_backoff,
                )
                .await?;
            Python::with_gil(|py| Py::new(py, (resp, parts)))
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl HttpClient {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn _call(
        &self,
        method: String,
        endpoints: PyObject,
        service_names: Option<Vec<ServiceName>>,
        use_https: Option<bool>,
        version: Option<Version>,
        path: Option<String>,
        headers: Option<HashMap<String, String>>,
        accept_json: Option<bool>,
        accept_application_octet_stream: Option<bool>,
        query: Option<String>,
        query_pairs: Option<PyObject>,
        appended_user_agent: Option<String>,
        authorization: Option<Authorization>,
        idempotent: Option<Idempotent>,
        bytes: Option<Vec<u8>>,
        body: Option<PyObject>,
        body_len: Option<u64>,
        content_type: Option<String>,
        json: Option<PyObject>,
        form: Option<Vec<(String, Option<String>)>>,
        multipart: Option<HashMap<String, PyObject>>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<(SyncHttpResponse, HttpResponseParts)> {
        let service_names = service_names
            .unwrap_or_default()
            .into_iter()
            .map(qiniu_sdk::http_client::ServiceName::from)
            .collect::<Vec<_>>();
        let mut builder = self.0.new_request(
            parse_method(&method)?,
            &service_names,
            extract_endpoints_provider(endpoints.as_ref(py))?,
        );
        Self::set_request_builder(
            &mut builder,
            use_https,
            version,
            path,
            headers,
            accept_json,
            accept_application_octet_stream,
            query,
            query_pairs,
            appended_user_agent,
            authorization,
            idempotent,
            uploading_progress,
            receive_response_status,
            receive_response_header,
            to_resolve_domain,
            domain_resolved,
            to_choose_ips,
            ips_chosen,
            before_request_signed,
            after_request_signed,
            response_ok,
            response_error,
            before_backoff,
            after_backoff,
        )?;
        if let Some(bytes) = bytes {
            builder.bytes_as_body(
                bytes,
                content_type.as_ref().map(|s| parse_mime(s)).transpose()?,
            );
        } else if let Some(body) = body {
            if let Some(body_len) = body_len {
                builder.stream_as_body(
                    PythonIoBase::new(body),
                    body_len,
                    content_type.as_ref().map(|s| parse_mime(s)).transpose()?,
                );
            } else {
                return Err(QiniuBodySizeMissingError::new_err(
                    "`body_len` must be passed",
                ));
            }
        } else if let Some(json) = json {
            builder
                .json(convert_py_any_to_json_value(json)?)
                .map_err(QiniuJsonError::from_err)?;
        } else if let Some(form) = form {
            builder.post_form(form);
        } else if let Some(multipart) = multipart {
            builder
                .multipart(extract_sync_multipart(multipart)?)
                .map_err(QiniuIoError::from_err)?;
        }

        let response = py.allow_threads(|| {
            builder
                .call()
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
        })?;
        let (parts, body) = response.into_parts_and_body();
        Ok((SyncHttpResponse::from(body), HttpResponseParts::from(parts)))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn _async_call(
        &self,
        method: String,
        endpoints: PyObject,
        service_names: Option<Vec<ServiceName>>,
        use_https: Option<bool>,
        version: Option<Version>,
        path: Option<String>,
        headers: Option<HashMap<String, String>>,
        accept_json: Option<bool>,
        accept_application_octet_stream: Option<bool>,
        query: Option<String>,
        query_pairs: Option<PyObject>,
        appended_user_agent: Option<String>,
        authorization: Option<Authorization>,
        idempotent: Option<Idempotent>,
        bytes: Option<Vec<u8>>,
        body: Option<PyObject>,
        body_len: Option<u64>,
        content_type: Option<String>,
        json: Option<PyObject>,
        form: Option<Vec<(String, Option<String>)>>,
        multipart: Option<HashMap<String, PyObject>>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
    ) -> PyResult<(AsyncHttpResponse, HttpResponseParts)> {
        let mut local_agent = None;
        let service_names = service_names
            .unwrap_or_default()
            .into_iter()
            .map(qiniu_sdk::http_client::ServiceName::from)
            .collect::<Vec<_>>();
        let mut builder = self.0.new_async_request(
            parse_method(&method)?,
            &service_names,
            Python::with_gil(|py| extract_endpoints_provider(endpoints.as_ref(py)))?,
        );
        Self::set_request_builder(
            &mut builder,
            use_https,
            version,
            path,
            headers,
            accept_json,
            accept_application_octet_stream,
            query,
            query_pairs,
            appended_user_agent,
            authorization,
            idempotent,
            uploading_progress,
            receive_response_status,
            receive_response_header,
            to_resolve_domain,
            domain_resolved,
            to_choose_ips,
            ips_chosen,
            before_request_signed,
            after_request_signed,
            response_ok,
            response_error,
            before_backoff,
            after_backoff,
        )?;
        if let Some(bytes) = bytes {
            builder.bytes_as_body(
                bytes,
                content_type
                    .as_ref()
                    .map(|s| parse_mime(s.as_str()))
                    .transpose()?,
            );
        } else if let Some(body) = body {
            if let Some(body_len) = body_len {
                let (stream, agent) = PythonIoBase::new(body).into_async_read_with_local_agent();
                local_agent = Some(agent);
                builder.stream_as_body(
                    stream,
                    body_len,
                    content_type
                        .as_ref()
                        .map(|s| parse_mime(s.as_str()))
                        .transpose()?,
                );
            } else {
                return Err(QiniuBodySizeMissingError::new_err(
                    "`body_len` must be passed",
                ));
            }
        } else if let Some(json) = json {
            builder
                .json(convert_py_any_to_json_value(json)?)
                .map_err(QiniuJsonError::from_err)?;
        } else if let Some(form) = form {
            builder.post_form(form);
        } else if let Some(multipart) = multipart {
            builder
                .multipart(extract_async_multipart(multipart)?)
                .await
                .map_err(QiniuIoError::from_err)?;
        }

        let response = if let Some(mut local_agent) = local_agent {
            local_agent.run(builder.call()).await?
        } else {
            builder.call().await
        }
        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?;
        let (parts, body) = response.into_parts_and_body();
        Ok((
            AsyncHttpResponse::from(body),
            HttpResponseParts::from(parts),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn set_request_builder<B, E>(
        builder: &mut qiniu_sdk::http_client::RequestBuilder<'_, B, E>,
        use_https: Option<bool>,
        version: Option<Version>,
        path: Option<String>,
        headers: Option<HashMap<String, String>>,
        accept_json: Option<bool>,
        accept_application_octet_stream: Option<bool>,
        query: Option<String>,
        query_pairs: Option<PyObject>,
        appended_user_agent: Option<String>,
        authorization: Option<Authorization>,
        idempotent: Option<Idempotent>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
    ) -> PyResult<()> {
        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        if let Some(version) = version {
            builder.version(version.into());
        }
        if let Some(path) = path {
            builder.path(path);
        }
        if let Some(headers) = headers {
            builder.headers(Cow::Owned(parse_headers(headers)?));
        }
        if let Some(true) = accept_json {
            builder.accept_json();
        } else if let Some(true) = accept_application_octet_stream {
            builder.accept_application_octet_stream();
        }
        if let Some(query) = query {
            builder.query(query);
        }
        if let Some(query_pairs) = query_pairs {
            builder.query_pairs(parse_query_pairs(query_pairs)?);
        }
        if let Some(appended_user_agent) = appended_user_agent {
            builder.appended_user_agent(appended_user_agent);
        }
        if let Some(authorization) = authorization {
            builder.authorization(authorization.0);
        }
        if let Some(idempotent) = idempotent {
            builder.idempotent(idempotent.into());
        }
        if let Some(uploading_progress) = uploading_progress {
            builder.on_uploading_progress(on_uploading_progress(uploading_progress));
        }
        if let Some(receive_response_status) = receive_response_status {
            builder.on_receive_response_status(on_receive_response_status(receive_response_status));
        }
        if let Some(receive_response_header) = receive_response_header {
            builder.on_receive_response_header(on_receive_response_header(receive_response_header));
        }
        if let Some(to_resolve_domain) = to_resolve_domain {
            builder.on_to_resolve_domain(on_to_resolve_domain(to_resolve_domain));
        }
        if let Some(domain_resolved) = domain_resolved {
            builder.on_domain_resolved(on_domain_resolved(domain_resolved));
        }
        if let Some(to_choose_ips) = to_choose_ips {
            builder.on_to_choose_ips(on_to_choose_ips(to_choose_ips));
        }
        if let Some(ips_chosen) = ips_chosen {
            builder.on_ips_chosen(on_ips_chosen(ips_chosen));
        }
        if let Some(before_request_signed) = before_request_signed {
            builder.on_before_request_signed(on_request_signed(before_request_signed));
        }
        if let Some(after_request_signed) = after_request_signed {
            builder.on_after_request_signed(on_request_signed(after_request_signed));
        }
        if let Some(response_ok) = response_ok {
            builder.on_response(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            builder.on_error(on_error(response_error));
        }
        if let Some(before_backoff) = before_backoff {
            builder.on_before_backoff(on_backoff(before_backoff));
        }
        if let Some(after_backoff) = after_backoff {
            builder.on_after_backoff(on_backoff(after_backoff));
        }
        Ok(())
    }
}

impl From<HttpClient> for qiniu_sdk::http_client::HttpClient {
    fn from(client: HttpClient) -> Self {
        client.0
    }
}

impl From<qiniu_sdk::http_client::HttpClient> for HttpClient {
    fn from(client: qiniu_sdk::http_client::HttpClient) -> Self {
        Self(client)
    }
}

macro_rules! impl_callback_context {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// ???????????? HTTPS ??????
            #[getter]
            fn get_use_https(&self) -> bool {
                self.0.use_https()
            }

            /// ???????????? HTTP ??????
            #[getter]
            fn get_method(&self) -> String {
                self.0.method().to_string()
            }

            /// ???????????? HTTP ??????
            #[getter]
            fn get_version(&self) -> Version {
                self.0.version().into()
            }

            /// ??????????????????
            #[getter]
            fn get_path(&self) -> &str {
                self.0.path()
            }

            /// ????????????????????????
            #[getter]
            fn get_query(&self) -> &str {
                self.0.query()
            }

            /// ?????????????????????
            #[getter]
            fn get_query_pairs(&self) -> Vec<(&str, &str)> {
                self.0
                    .query_pairs()
                    .iter()
                    .map(|(key, value)| (key.as_ref(), value.as_ref()))
                    .collect()
            }

            /// ???????????? HTTP Headers
            #[getter]
            fn get_headers(&self) -> PyResult<HashMap<String, String>> {
                convert_headers_to_hashmap(self.0.headers())
            }

            /// ??????????????? UserAgent
            #[getter]
            fn get_appended_user_agent(&self) -> &str {
                self.0.appended_user_agent().as_str()
            }

            /// ????????????????????????
            #[getter]
            fn get_idempotent(&self) -> Idempotent {
                self.0.idempotent().into()
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

macro_rules! impl_callback_context_ext {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// ????????????????????????
            #[getter]
            fn get_timeout_ms(&self) -> Option<u128> {
                self.0
                    .extensions()
                    .get::<qiniu_sdk::isahc::TimeoutRequestExtension>()
                    .map(|ext| ext.get().as_millis())
            }

            /// ????????????????????????
            #[setter]
            fn set_timeout_ms(&mut self, timeout_ms: u64) {
                self.0
                    .extensions_mut()
                    .insert(qiniu_sdk::isahc::TimeoutRequestExtension::new(
                        Duration::from_millis(timeout_ms),
                    ));
            }

            /// ??????????????????????????????
            #[getter]
            fn get_connect_timeout_ms(&self) -> Option<u128> {
                self.0
                    .extensions()
                    .get::<qiniu_sdk::isahc::ConnectTimeoutRequestExtension>()
                    .map(|ext| ext.get().as_millis())
            }

            /// ??????????????????????????????
            #[setter]
            fn set_connect_timeout_ms(&mut self, timeout_ms: u64) {
                self.0.extensions_mut().insert(
                    qiniu_sdk::isahc::ConnectTimeoutRequestExtension::new(Duration::from_millis(
                        timeout_ms,
                    )),
                );
            }
        }
    };
}

/// ???????????????????????????
///
/// ??????????????????????????????????????????????????????????????????????????????????????????????????????????????????
///
/// ??????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
#[pyclass]
#[derive(Clone)]
struct SimplifiedCallbackContext(&'static dyn qiniu_sdk::http_client::SimplifiedCallbackContext);

impl SimplifiedCallbackContext {
    fn new(ctx: &dyn qiniu_sdk::http_client::SimplifiedCallbackContext) -> Self {
        #[allow(unsafe_code)]
        Self(unsafe { transmute(ctx) })
    }
}

impl_callback_context!(SimplifiedCallbackContext);

fn on_uploading_progress(
    callback: PyObject,
) -> impl Fn(
    &dyn qiniu_sdk::http_client::SimplifiedCallbackContext,
    qiniu_sdk::http::TransferProgressInfo<'_>,
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, progress| {
        Python::with_gil(|py| {
            callback.call1(
                py,
                (
                    SimplifiedCallbackContext::new(context),
                    TransferProgressInfo::new(progress.transferred_bytes(), progress.total_bytes()),
                ),
            )
        })?;
        Ok(())
    }
}

fn on_receive_response_status(
    callback: PyObject,
) -> impl Fn(
    &dyn qiniu_sdk::http_client::SimplifiedCallbackContext,
    qiniu_sdk::http::StatusCode,
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, status_code| {
        Python::with_gil(|py| {
            callback.call1(
                py,
                (
                    SimplifiedCallbackContext::new(context),
                    status_code.as_u16(),
                ),
            )
        })?;
        Ok(())
    }
}

fn on_receive_response_header(
    callback: PyObject,
) -> impl Fn(
    &dyn qiniu_sdk::http_client::SimplifiedCallbackContext,
    &qiniu_sdk::http::HeaderName,
    &qiniu_sdk::http::HeaderValue,
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, header_name, header_value| {
        Python::with_gil(|py| {
            callback.call1(
                py,
                (
                    SimplifiedCallbackContext::new(context),
                    header_name.as_str(),
                    header_value
                        .to_str()
                        .map_err(QiniuHeaderValueEncodingError::from_err)?,
                ),
            )
        })?;
        Ok(())
    }
}

/// ?????????????????????
///
/// ??????????????????????????????????????????????????????????????????????????????????????????????????????????????????
///
/// ??????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
#[pyclass]
pub(crate) struct CallbackContextMut(&'static mut dyn qiniu_sdk::http_client::CallbackContext);

impl CallbackContextMut {
    fn new(ctx: &mut dyn qiniu_sdk::http_client::CallbackContext) -> Self {
        #[allow(unsafe_code)]
        Self(unsafe { transmute(ctx) })
    }
}

impl_callback_context!(CallbackContextMut);
impl_callback_context_ext!(CallbackContextMut);

impl<'a> AsMut<dyn qiniu_sdk::http_client::CallbackContext + 'a> for CallbackContextMut {
    fn as_mut(&mut self) -> &mut (dyn qiniu_sdk::http_client::CallbackContext + 'a) {
        self.0
    }
}

fn on_to_resolve_domain(
    callback: PyObject,
) -> impl Fn(&mut dyn qiniu_sdk::http_client::CallbackContext, &str) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, domain| {
        Python::with_gil(|py| callback.call1(py, (CallbackContextMut::new(context), domain)))?;
        Ok(())
    }
}

fn on_domain_resolved(
    callback: PyObject,
) -> impl Fn(
    &mut dyn qiniu_sdk::http_client::CallbackContext,
    &str,
    &qiniu_sdk::http_client::ResolveAnswers,
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, domain, answers| {
        Python::with_gil(|py| {
            let ips = answers
                .ip_addrs()
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>();
            callback.call1(py, (CallbackContextMut::new(context), domain, ips))
        })?;
        Ok(())
    }
}

fn on_to_choose_ips(
    callback: PyObject,
) -> impl Fn(
    &mut dyn qiniu_sdk::http_client::CallbackContext,
    &[qiniu_sdk::http_client::IpAddrWithPort],
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, ips| {
        let ips = ips.iter().map(|ip| ip.to_string()).collect::<Vec<_>>();
        Python::with_gil(|py| callback.call1(py, (CallbackContextMut::new(context), ips)))?;
        Ok(())
    }
}

fn on_ips_chosen(
    callback: PyObject,
) -> impl Fn(
    &mut dyn qiniu_sdk::http_client::CallbackContext,
    &[qiniu_sdk::http_client::IpAddrWithPort],
    &[qiniu_sdk::http_client::IpAddrWithPort],
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, before, after| {
        let before = before.iter().map(|ip| ip.to_string()).collect::<Vec<_>>();
        let after = after.iter().map(|ip| ip.to_string()).collect::<Vec<_>>();
        Python::with_gil(|py| {
            callback.call1(py, (CallbackContextMut::new(context), before, after))
        })?;
        Ok(())
    }
}

/// ??????????????????????????????
///
/// ?????????????????????????????????????????????????????????????????????????????????????????????????????? UserAgent ?????????????????? IP ???????????????????????????????????????
///
/// ??????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
#[pyclass]
struct ExtendedCallbackContextRef(&'static mut dyn qiniu_sdk::http_client::ExtendedCallbackContext);

impl ExtendedCallbackContextRef {
    fn new(ctx: &mut dyn qiniu_sdk::http_client::ExtendedCallbackContext) -> Self {
        #[allow(unsafe_code)]
        Self(unsafe { transmute(ctx) })
    }
}

impl_callback_context!(ExtendedCallbackContextRef);
impl_callback_context_ext!(ExtendedCallbackContextRef);

#[pymethods]
impl ExtendedCallbackContextRef {
    /// ?????? HTTP ?????? URL
    #[getter]
    fn get_url(&self) -> String {
        self.0.url().to_string()
    }

    /// ???????????? HTTP ??????
    #[setter]
    fn set_url(&mut self, version: Version) {
        *self.0.version_mut() = version.into();
    }

    /// ???????????? HTTP Headers
    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        *self.0.headers_mut() = parse_headers(headers)?;
        Ok(())
    }

    /// ?????? UserAgent
    #[getter]
    fn get_user_agent(&self) -> String {
        self.0.user_agent().to_string()
    }

    /// ??????????????? UserAgent
    #[setter]
    fn set_appended_user_agent(&mut self, appended_user_agent: &str) {
        self.0.set_appended_user_agent(appended_user_agent.into());
    }

    /// ????????????????????? IP ????????????
    #[getter]
    fn get_resolved_ip_addrs(&self) -> Option<Vec<String>> {
        self.0
            .resolved_ip_addrs()
            .map(|ips| ips.iter().map(|ip| ip.to_string()).collect())
    }

    /// ????????????????????? IP ????????????
    #[setter]
    fn set_resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
        self.0
            .set_resolved_ip_addrs(parse_ip_addrs(resolved_ip_addrs)?);
        Ok(())
    }

    /// ????????????????????????
    #[getter]
    fn get_retried(&self) -> RetriedStatsInfo {
        RetriedStatsInfo(self.0.retried().to_owned())
    }
}

fn on_request_signed(
    callback: PyObject,
) -> impl Fn(&mut dyn qiniu_sdk::http_client::ExtendedCallbackContext) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context| {
        Python::with_gil(|py| callback.call1(py, (ExtendedCallbackContextRef::new(context),)))?;
        Ok(())
    }
}

fn on_response(
    callback: PyObject,
) -> impl Fn(
    &mut dyn qiniu_sdk::http_client::ExtendedCallbackContext,
    &qiniu_sdk::http::ResponseParts,
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, parts| {
        let parts = HttpResponsePartsRef::from(parts);
        Python::with_gil(|py| {
            callback.call1(py, (ExtendedCallbackContextRef::new(context), parts))
        })?;
        Ok(())
    }
}

fn on_error(
    callback: PyObject,
) -> impl Fn(
    &mut dyn qiniu_sdk::http_client::ExtendedCallbackContext,
    &mut qiniu_sdk::http_client::ResponseError,
) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, error| {
        #[allow(unsafe_code)]
        let error: &'static qiniu_sdk::http_client::ResponseError = unsafe { transmute(error) };
        let error = QiniuApiCallError::from_err(MaybeOwned::Borrowed(error));
        let error = convert_api_call_error(&error)?;
        Python::with_gil(|py| {
            callback.call1(py, (ExtendedCallbackContextRef::new(context), error))
        })?;
        Ok(())
    }
}

fn on_backoff(
    callback: PyObject,
) -> impl Fn(&mut dyn qiniu_sdk::http_client::ExtendedCallbackContext, Duration) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |context, duration| {
        Python::with_gil(|py| {
            callback.call1(
                py,
                (
                    ExtendedCallbackContextRef::new(context),
                    duration.as_nanos(),
                ),
            )
        })?;
        Ok(())
    }
}

/// HTTP ???????????????????????????
///
/// ?????? HTTP ?????????????????????????????????????????????????????????????????????
///
/// ??????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
#[pyclass]
pub(crate) struct RequestBuilderPartsRef(
    &'static mut qiniu_sdk::http_client::RequestBuilderParts<'static>,
);

impl RequestBuilderPartsRef {
    pub(crate) fn new(ctx: &mut qiniu_sdk::http_client::RequestBuilderParts<'_>) -> Self {
        #[allow(unsafe_code)]
        Self(unsafe { transmute(ctx) })
    }
}

#[pymethods]
impl RequestBuilderPartsRef {
    /// ?????????????????? HTTPS
    #[setter]
    fn set_use_https(&mut self, use_https: bool) {
        self.0.use_https(use_https);
    }

    /// ?????? HTTP ????????????
    #[setter]
    fn set_version(&mut self, version: Version) {
        self.0.version(version.into());
    }

    /// ?????? HTTP ????????????
    #[setter]
    fn set_path(&mut self, path: String) {
        self.0.path(path);
    }

    /// ?????? HTTP ?????????
    #[setter]
    fn set_headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        self.0.headers(Cow::Owned(parse_headers(headers)?));
        Ok(())
    }

    /// ?????? HTTP ?????????
    #[pyo3(text_signature = "($self, header_name, header_value)")]
    fn set_header(&mut self, header_name: &str, header_value: &str) -> PyResult<()> {
        self.0.set_header(
            parse_header_name(header_name)?,
            parse_header_value(header_value)?,
        );
        Ok(())
    }

    /// ?????? HTTP ??????????????? JSON ??????
    #[pyo3(text_signature = "($self)")]
    fn accept_json(&mut self) {
        self.0.accept_json();
    }

    /// ?????? HTTP ?????????????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn accept_application_octet_stream(&mut self) {
        self.0.accept_application_octet_stream();
    }

    /// ??????????????????
    #[setter]
    fn set_query(&mut self, query: String) {
        self.0.query(query);
    }

    /// ??????????????????
    #[setter]
    fn set_query_pairs(&mut self, query_pairs: PyObject) -> PyResult<()> {
        self.0.query_pairs(parse_query_pairs(query_pairs)?);
        Ok(())
    }

    /// ??????????????????
    #[pyo3(text_signature = "($self, query_pair_key, query_pair_value)")]
    fn append_query_pair(&mut self, query_pair_key: String, query_pair_value: String) {
        self.0.append_query_pair(query_pair_key, query_pair_value);
    }

    /// ?????? UserAgent
    #[setter]
    fn set_appended_user_agent(&mut self, user_agent: String) {
        self.0.appended_user_agent(user_agent);
    }

    /// ??????????????????
    #[setter]
    fn set_authorization(&mut self, authorization: Authorization) {
        self.0.authorization(authorization.into());
    }

    /// ?????????????????????
    #[setter]
    fn set_idempotent(&mut self, idempotent: Idempotent) {
        self.0.idempotent(idempotent.into());
    }

    /// ????????????????????????
    #[setter]
    fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.0
            .add_extension(qiniu_sdk::isahc::TimeoutRequestExtension::new(
                Duration::from_millis(timeout_ms),
            ));
    }

    /// ??????????????????????????????
    #[setter]
    fn set_connect_timeout_ms(&mut self, timeout_ms: u64) {
        self.0
            .add_extension(qiniu_sdk::isahc::ConnectTimeoutRequestExtension::new(
                Duration::from_millis(timeout_ms),
            ));
    }

    /// ??????????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_uploading_progress(&mut self, callback: PyObject) {
        self.0
            .on_uploading_progress(on_uploading_progress(callback));
    }

    /// ?????????????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_receive_response_status(&mut self, callback: PyObject) {
        self.0
            .on_receive_response_status(on_receive_response_status(callback));
    }

    /// ???????????? HTTP ???????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_receive_response_header(&mut self, callback: PyObject) {
        self.0
            .on_receive_response_header(on_receive_response_header(callback));
    }

    /// ?????????????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_to_resolve_domain(&mut self, callback: PyObject) {
        self.0.on_to_resolve_domain(on_to_resolve_domain(callback));
    }

    /// ????????????????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_domain_resolved(&mut self, callback: PyObject) {
        self.0.on_domain_resolved(on_domain_resolved(callback));
    }

    /// ?????? IP ???????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_to_choose_ips(&mut self, callback: PyObject) {
        self.0.on_to_choose_ips(on_to_choose_ips(callback));
    }

    /// ?????? IP ??????????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_ips_chosen(&mut self, callback: PyObject) {
        self.0.on_ips_chosen(on_ips_chosen(callback));
    }

    /// ?????? HTTP ???????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_before_request_signed(&mut self, callback: PyObject) {
        self.0.on_before_request_signed(on_request_signed(callback));
    }

    /// ?????? HTTP ?????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_after_request_signed(&mut self, callback: PyObject) {
        self.0.on_after_request_signed(on_request_signed(callback));
    }

    /// ??????????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_response(&mut self, callback: PyObject) {
        self.0.on_response(on_response(callback));
    }

    /// ???????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_before_backoff(&mut self, callback: PyObject) {
        self.0.on_before_backoff(on_backoff(callback));
    }

    /// ???????????????????????????
    #[pyo3(text_signature = "($self, callback)")]
    fn on_after_backoff(&mut self, callback: PyObject) {
        self.0.on_after_backoff(on_backoff(callback));
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// JSON API ??????
///
/// ?????? JSON API ??????????????????
///
/// ??????????????????????????????????????? API ???????????? `call()` ????????????
#[pyclass(extends = HttpResponseParts)]
pub(crate) struct JsonResponse(PyObject);

#[pymethods]
impl JsonResponse {
    /// ?????? JSON ?????????
    #[getter]
    fn get_body<'p>(&'p self, py: Python<'p>) -> &'p PyAny {
        self.0.as_ref(py)
    }

    fn __len__(&self, py: Python<'_>) -> PyResult<usize> {
        self.0.as_ref(py).len()
    }

    fn __contains__(&self, value: PyObject, py: Python<'_>) -> PyResult<bool> {
        self.0.as_ref(py).contains(value)
    }

    fn __getitem__<'p>(&'p self, key: PyObject, py: Python<'p>) -> PyResult<&'p PyAny> {
        self.0.as_ref(py).get_item(key)
    }

    fn __setitem__(&self, key: PyObject, value: PyObject, py: Python<'_>) -> PyResult<()> {
        self.0.as_ref(py).set_item(key, value)
    }

    fn __delitem__(&self, key: PyObject, py: Python<'_>) -> PyResult<()> {
        self.0.as_ref(py).del_item(key)
    }

    fn __iter__(&mut self, py: Python<'_>) -> PyResult<Py<PyIterator>> {
        Ok(self.0.as_ref(py).iter()?.into_py(py))
    }
}

impl From<PyObject> for JsonResponse {
    fn from(obj: PyObject) -> Self {
        Self(obj)
    }
}
