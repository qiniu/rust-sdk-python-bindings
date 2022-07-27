use crate::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuEmptyRegionsProvider, QiniuInvalidEndpointError,
        QiniuInvalidIpAddrWithPortError,
    },
    utils::{extract_endpoints, parse_domain_with_port},
};
use futures::future::BoxFuture;
use maybe_owned::MaybeOwned;
use pyo3::{prelude::*, pyclass::CompareOp};
use qiniu_sdk::http_client::EndpointsGetOptions;
use std::{borrow::Cow, path::PathBuf, time::Duration};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<DomainWithPort>()?;
    m.add_class::<IpAddrWithPort>()?;
    m.add_class::<Endpoint>()?;
    m.add_class::<ServiceName>()?;
    m.add_class::<Endpoints>()?;
    m.add_class::<EndpointsProvider>()?;
    m.add_class::<Region>()?;
    m.add_class::<RegionsProvider>()?;
    m.add_class::<AllRegionsProvider>()?;
    m.add_class::<BucketRegionsQueryer>()?;
    m.add_class::<BucketDomainsQueryer>()?;

    Ok(())
}

/// 域名和端口号
///
/// 用来表示一个七牛服务器的地址，端口号是可选的，如果不提供，则根据传输协议判定默认的端口号。
///
/// 通过 `DomainWithPort(domain, port = None)` 创建域名和端口号
#[pyclass]
#[pyo3(text_signature = "(domain, port = None)")]
#[derive(Clone)]
struct DomainWithPort(qiniu_sdk::http_client::DomainWithPort);

#[pymethods]
impl DomainWithPort {
    #[new]
    #[args(port = "None")]
    fn new(domain: String, port: Option<u16>) -> PyResult<Self> {
        let host = if let Some(port) = port {
            format!("{}:{}", domain, port)
        } else {
            domain
        };
        Ok(Self(parse_domain_with_port(&host)?))
    }

    /// 获取域名
    #[getter]
    fn get_domain(&self) -> &str {
        self.0.domain()
    }

    /// 获取端口
    #[getter]
    fn get_port(&self) -> Option<u16> {
        self.0.port().map(|port| port.get())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == other.0).to_object(py),
            _ => py.NotImplemented(),
        }
    }
}

/// IP 地址和端口号
///
/// 用来表示一个七牛服务器的地址，端口号是可选的，如果不提供，则根据传输协议判定默认的端口号。
///
/// 通过 `IpAddrWithPort(ip, port = None)` 创建域名和端口号
#[pyclass]
#[pyo3(text_signature = "(ip, port = None)")]
#[derive(Clone)]
struct IpAddrWithPort(qiniu_sdk::http_client::IpAddrWithPort);

#[pymethods]
impl IpAddrWithPort {
    #[new]
    #[args(port = "None")]
    fn new(ip_addr: String, port: Option<u16>) -> PyResult<Self> {
        let host = if let Some(port) = port {
            format!("{}:{}", ip_addr, port).parse()
        } else {
            ip_addr.parse()
        }
        .map_err(QiniuInvalidIpAddrWithPortError::from_err)?;
        Ok(Self(host))
    }

    /// 获取 IP 地址
    #[getter]
    fn get_ip_addr(&self) -> String {
        self.0.ip_addr().to_string()
    }

    /// 获取端口
    #[getter]
    fn get_port(&self) -> Option<u16> {
        self.0.port().map(|port| port.get())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == other.0).to_object(py),
            _ => py.NotImplemented(),
        }
    }
}

/// 终端地址
///
/// 用来表示一个域名和端口号，或 IP 地址和端口号。
///
/// 通过 `Endpoint(domain_or_ip_addr, port = None)` 创建域名和端口号
#[pyclass]
#[pyo3(text_signature = "(domain_or_ip_addr, port = None)")]
#[derive(Clone)]
pub(crate) struct Endpoint(qiniu_sdk::http_client::Endpoint);

#[pymethods]
impl Endpoint {
    #[new]
    #[args(port = "None")]
    fn new(domain_or_ip_addr: String, port: Option<u16>) -> PyResult<Self> {
        let host = if let Some(port) = port {
            format!("{}:{}", domain_or_ip_addr, port).parse()
        } else {
            domain_or_ip_addr.parse()
        }
        .map_err(QiniuInvalidEndpointError::from_err)?;
        Ok(Self(host))
    }

    /// 获取域名
    #[getter]
    fn get_domain(&self) -> Option<&str> {
        self.0.domain()
    }

    /// 获取 IP 地址
    #[getter]
    fn get_ip_addr(&self) -> Option<String> {
        self.0.ip_addr().map(|ip| ip.to_string())
    }

    /// 获取端口
    #[getter]
    fn get_port(&self) -> Option<u16> {
        self.0.port().map(|port| port.get())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == other.0).to_object(py),
            _ => py.NotImplemented(),
        }
    }
}

impl From<Endpoint> for qiniu_sdk::http_client::Endpoint {
    fn from(e: Endpoint) -> Self {
        e.0
    }
}

/// 七牛服务名称
#[pyclass]
#[derive(Clone, Copy, Debug)]
pub(crate) enum ServiceName {
    /// 上传服务
    Up = 0,

    /// 下载服务
    Io = 1,

    /// 存储空间管理服务
    Uc = 2,

    /// 元数据管理服务
    Rs = 3,

    /// 元数据列举服务
    Rsf = 4,

    /// API 入口服务
    Api = 5,

    /// S3 入口服务
    S3 = 6,
}

#[pymethods]
impl ServiceName {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<ServiceName> for qiniu_sdk::http_client::ServiceName {
    fn from(svc: ServiceName) -> Self {
        match svc {
            ServiceName::Up => qiniu_sdk::http_client::ServiceName::Up,
            ServiceName::Io => qiniu_sdk::http_client::ServiceName::Io,
            ServiceName::Uc => qiniu_sdk::http_client::ServiceName::Uc,
            ServiceName::Rs => qiniu_sdk::http_client::ServiceName::Rs,
            ServiceName::Rsf => qiniu_sdk::http_client::ServiceName::Rsf,
            ServiceName::Api => qiniu_sdk::http_client::ServiceName::Api,
            ServiceName::S3 => qiniu_sdk::http_client::ServiceName::S3,
        }
    }
}

impl From<qiniu_sdk::http_client::ServiceName> for ServiceName {
    fn from(svc: qiniu_sdk::http_client::ServiceName) -> Self {
        match svc {
            qiniu_sdk::http_client::ServiceName::Up => ServiceName::Up,
            qiniu_sdk::http_client::ServiceName::Io => ServiceName::Io,
            qiniu_sdk::http_client::ServiceName::Uc => ServiceName::Uc,
            qiniu_sdk::http_client::ServiceName::Rs => ServiceName::Rs,
            qiniu_sdk::http_client::ServiceName::Rsf => ServiceName::Rsf,
            qiniu_sdk::http_client::ServiceName::Api => ServiceName::Api,
            qiniu_sdk::http_client::ServiceName::S3 => ServiceName::S3,
            _ => unreachable!("Unrecognized ServiceName {:?}", svc),
        }
    }
}

/// 终端地址列表获取接口
///
/// 同时提供阻塞获取接口和异步获取接口，异步获取接口则需要启用 `async` 功能
///
/// 通过 `EndpointsProvider(regions_provider)` 创建终端地址列表获取接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(regions_provider)")]
pub(crate) struct EndpointsProvider(Box<dyn qiniu_sdk::http_client::EndpointsProvider>);

#[pymethods]
impl EndpointsProvider {
    #[new]
    fn new(regions_provider: RegionsProvider) -> Self {
        Self(Box::new(
            qiniu_sdk::http_client::RegionsProviderEndpoints::new(regions_provider.0),
        ))
    }

    /// 获取终端地址列表
    #[pyo3(text_signature = "($self, /, service_names = None)")]
    fn get(
        &self,
        service_names: Option<Vec<ServiceName>>,
        py: Python<'_>,
    ) -> PyResult<Py<Endpoints>> {
        let service_names = service_names
            .unwrap_or_default()
            .into_iter()
            .map(|svc| svc.into())
            .collect::<Vec<_>>();
        let opts = EndpointsGetOptions::builder()
            .service_names(&service_names)
            .build();
        let endpoints = py
            .allow_threads(|| self.0.get_endpoints(opts))
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
            .into_owned();
        Self::make_initializer(endpoints, py)
    }

    /// 异步获取终端地址列表
    #[pyo3(text_signature = "($self, /, service_names = None)")]
    fn async_get<'p>(
        &self,
        service_names: Option<Vec<ServiceName>>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let service_names = service_names
                .unwrap_or_default()
                .into_iter()
                .map(|svc| svc.into())
                .collect::<Vec<_>>();
            let opts = EndpointsGetOptions::builder()
                .service_names(&service_names)
                .build();
            let endpoints = provider
                .async_get_endpoints(opts)
                .await
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
                .into_owned();
            Python::with_gil(|py| Self::make_initializer(endpoints, py))
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::http_client::EndpointsProvider for EndpointsProvider {
    fn get_endpoints<'e>(
        &'e self,
        options: qiniu_sdk::http_client::EndpointsGetOptions<'_>,
    ) -> qiniu_sdk::http_client::ApiResult<Cow<'e, qiniu_sdk::http_client::Endpoints>> {
        self.0.get_endpoints(options)
    }

    fn async_get_endpoints<'a>(
        &'a self,
        options: qiniu_sdk::http_client::EndpointsGetOptions<'a>,
    ) -> BoxFuture<'a, qiniu_sdk::http_client::ApiResult<Cow<'a, qiniu_sdk::http_client::Endpoints>>>
    {
        self.0.async_get_endpoints(options)
    }
}

impl EndpointsProvider {
    fn make_initializer(
        endpoint: qiniu_sdk::http_client::Endpoints,
        py: Python<'_>,
    ) -> PyResult<Py<Endpoints>> {
        Py::new(
            py,
            (
                Endpoints(endpoint.to_owned()),
                EndpointsProvider(Box::new(endpoint)),
            ),
        )
    }
}

/// 终端地址列表
///
/// 存储一个七牛服务的多个终端地址，包含主要地址列表和备选地址列表
///
/// 通过 `Endpoints(preferred_endpoints, alternative_endpoints = None)` 创建终端地址列表
#[pyclass(extends = EndpointsProvider)]
#[pyo3(text_signature = "(preferred_endpoints, alternative_endpoints = None)")]
#[derive(Clone)]
pub(crate) struct Endpoints(qiniu_sdk::http_client::Endpoints);

#[pymethods]
impl Endpoints {
    #[new]
    #[args(alternative_endpoints = "None")]
    fn new(
        preferred_endpoints: Vec<&PyAny>,
        alternative_endpoints: Option<Vec<&PyAny>>,
    ) -> PyResult<(Self, EndpointsProvider)> {
        let mut builder = qiniu_sdk::http_client::EndpointsBuilder::default();
        builder.add_preferred_endpoints(extract_endpoints(preferred_endpoints)?);
        if let Some(alternative_endpoints) = alternative_endpoints {
            builder.add_alternative_endpoints(extract_endpoints(alternative_endpoints)?);
        }
        let endpoints = builder.build();
        Ok((
            Self(endpoints.to_owned()),
            EndpointsProvider(Box::new(endpoints)),
        ))
    }

    /// 返回主要终端地址列表
    #[getter]
    fn get_preferred(&self) -> Vec<Endpoint> {
        self.0.preferred().iter().cloned().map(Endpoint).collect()
    }

    /// 返回备选终端地址列表
    #[getter]
    fn get_alternative(&self) -> Vec<Endpoint> {
        self.0.alternative().iter().cloned().map(Endpoint).collect()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == other.0).to_object(py),
            _ => py.NotImplemented(),
        }
    }
}

impl From<Endpoints> for qiniu_sdk::http_client::Endpoints {
    fn from(endpoints: Endpoints) -> Self {
        endpoints.0
    }
}

impl From<qiniu_sdk::http_client::Endpoints> for Endpoints {
    fn from(endpoints: qiniu_sdk::http_client::Endpoints) -> Self {
        Self(endpoints)
    }
}

/// 区域信息获取接口
///
/// 可以获取一个区域也可以获取多个区域
///
/// 同时提供阻塞获取接口和异步获取接口，异步获取接口则需要启用 `async` 功能
///
/// 通过 `RegionsProvider(regions)` 创建区域信息获取接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(regions)")]
pub(crate) struct RegionsProvider(Box<dyn qiniu_sdk::http_client::RegionsProvider>);

#[pymethods]
impl RegionsProvider {
    #[new]
    fn new(regions: Vec<Region>) -> PyResult<Self> {
        let mut iter = regions.into_iter();
        if let Some(region) = iter.next() {
            let mut provider = qiniu_sdk::http_client::StaticRegionsProvider::new(region.0);
            provider.extend(iter.map(|r| r.0));
            Ok(Self(Box::new(provider)))
        } else {
            Err(QiniuEmptyRegionsProvider::new_err("regions is empty"))
        }
    }

    /// 获取七牛区域信息
    #[pyo3(text_signature = "($self)")]
    fn get(&self, py: Python<'_>) -> PyResult<Py<Region>> {
        let region = py
            .allow_threads(|| self.0.get(Default::default()))
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
            .into_region();
        Self::make_initializer(region, py)
    }

    /// 获取多个七牛区域信息
    #[pyo3(text_signature = "($self)")]
    fn get_all(&self, py: Python<'_>) -> PyResult<Vec<Py<Region>>> {
        let regions = py
            .allow_threads(|| self.0.get_all(Default::default()))
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
            .into_regions()
            .into_iter()
            .map(|region| Self::make_initializer(region, py))
            .collect::<PyResult<Vec<Py<Region>>>>()?;
        Ok(regions)
    }

    /// 异步获取七牛区域信息
    #[pyo3(text_signature = "($self)")]
    fn async_get<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let region = provider
                .async_get(Default::default())
                .await
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
                .into_region();
            Python::with_gil(|py| Self::make_initializer(region, py))
        })
    }

    /// 异步获取多个七牛区域信息
    #[pyo3(text_signature = "($self)")]
    fn async_get_all<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let regions = provider
                .async_get_all(Default::default())
                .await
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))?
                .into_regions()
                .into_iter()
                .map(|region| Python::with_gil(|py| Self::make_initializer(region, py)))
                .collect::<PyResult<Vec<Py<Region>>>>()?;
            Ok(regions)
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::http_client::RegionsProvider for RegionsProvider {
    fn get(
        &self,
        opts: qiniu_sdk::http_client::RegionsGetOptions,
    ) -> qiniu_sdk::http_client::ApiResult<qiniu_sdk::http_client::GotRegion> {
        self.0.get(opts)
    }

    fn get_all(
        &self,
        opts: qiniu_sdk::http_client::RegionsGetOptions,
    ) -> qiniu_sdk::http_client::ApiResult<qiniu_sdk::http_client::GotRegions> {
        self.0.get_all(opts)
    }

    fn async_get(
        &self,
        opts: qiniu_sdk::http_client::RegionsGetOptions,
    ) -> BoxFuture<'_, qiniu_sdk::http_client::ApiResult<qiniu_sdk::http_client::GotRegion>> {
        self.0.async_get(opts)
    }

    fn async_get_all(
        &self,
        opts: qiniu_sdk::http_client::RegionsGetOptions,
    ) -> BoxFuture<'_, qiniu_sdk::http_client::ApiResult<qiniu_sdk::http_client::GotRegions>> {
        self.0.async_get_all(opts)
    }
}

impl RegionsProvider {
    fn make_initializer(
        region: qiniu_sdk::http_client::Region,
        py: Python<'_>,
    ) -> PyResult<Py<Region>> {
        Py::new(
            py,
            (Region(region.to_owned()), RegionsProvider(Box::new(region))),
        )
    }
}

impl From<Box<dyn qiniu_sdk::http_client::RegionsProvider>> for RegionsProvider {
    fn from(provider: Box<dyn qiniu_sdk::http_client::RegionsProvider>) -> Self {
        RegionsProvider(provider)
    }
}

impl From<RegionsProvider> for Box<dyn qiniu_sdk::http_client::RegionsProvider> {
    fn from(provider: RegionsProvider) -> Self {
        provider.0
    }
}

/// 七牛存储区域
///
/// 提供七牛不同服务的终端地址列表
///
/// 通过 `Region(region_id, s3_region_id = None, up_preferred_endpoints = None, up_alternative_endpoints = None, io_preferred_endpoints = None, io_alternative_endpoints = None, uc_preferred_endpoints = None, uc_preferred_endpoints = None, rs_preferred_endpoints = None, rs_alternative_endpoints = None, rsf_preferred_endpoints = None, rsf_alternative_endpoints = None, s3_preferred_endpoints = None, s3_alternative_endpoints = None, api_preferred_endpoints = None, api_alternative_endpoints = None)` 创建七牛存储区域
#[pyclass(extends = RegionsProvider)]
#[pyo3(
    text_signature = "(region_id, /, s3_region_id = None, up_preferred_endpoints = None, up_alternative_endpoints = None, io_preferred_endpoints = None, io_alternative_endpoints = None, uc_preferred_endpoints = None, uc_preferred_endpoints = None, rs_preferred_endpoints = None, rs_alternative_endpoints = None, rsf_preferred_endpoints = None, rsf_alternative_endpoints = None, s3_preferred_endpoints = None, s3_alternative_endpoints = None, api_preferred_endpoints = None, api_alternative_endpoints = None)"
)]
#[derive(Clone)]
struct Region(qiniu_sdk::http_client::Region);

#[pymethods]
impl Region {
    #[new]
    #[args(
        s3_region_id = "None",
        up_preferred_endpoints = "None",
        up_alternative_endpoints = "None",
        io_preferred_endpoints = "None",
        io_alternative_endpoints = "None",
        uc_preferred_endpoints = "None",
        uc_preferred_endpoints = "None",
        rs_preferred_endpoints = "None",
        rs_alternative_endpoints = "None",
        rsf_preferred_endpoints = "None",
        rsf_alternative_endpoints = "None",
        s3_preferred_endpoints = "None",
        s3_alternative_endpoints = "None",
        api_preferred_endpoints = "None",
        api_alternative_endpoints = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        region_id: String,
        s3_region_id: Option<String>,
        up_preferred_endpoints: Option<Vec<&PyAny>>,
        up_alternative_endpoints: Option<Vec<&PyAny>>,
        io_preferred_endpoints: Option<Vec<&PyAny>>,
        io_alternative_endpoints: Option<Vec<&PyAny>>,
        uc_preferred_endpoints: Option<Vec<&PyAny>>,
        uc_alternative_endpoints: Option<Vec<&PyAny>>,
        rs_preferred_endpoints: Option<Vec<&PyAny>>,
        rs_alternative_endpoints: Option<Vec<&PyAny>>,
        rsf_preferred_endpoints: Option<Vec<&PyAny>>,
        rsf_alternative_endpoints: Option<Vec<&PyAny>>,
        s3_preferred_endpoints: Option<Vec<&PyAny>>,
        s3_alternative_endpoints: Option<Vec<&PyAny>>,
        api_preferred_endpoints: Option<Vec<&PyAny>>,
        api_alternative_endpoints: Option<Vec<&PyAny>>,
    ) -> PyResult<(Self, RegionsProvider)> {
        let mut builder = qiniu_sdk::http_client::Region::builder(region_id);
        if let Some(s3_region_id) = s3_region_id {
            builder.s3_region_id(s3_region_id);
        }
        if let Some(endpoints) = up_preferred_endpoints {
            builder.add_up_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = up_alternative_endpoints {
            builder.add_up_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = io_preferred_endpoints {
            builder.add_io_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = io_alternative_endpoints {
            builder.add_io_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = uc_preferred_endpoints {
            builder.add_uc_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = uc_alternative_endpoints {
            builder.add_uc_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = rs_preferred_endpoints {
            builder.add_rs_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = rs_alternative_endpoints {
            builder.add_rs_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = rsf_preferred_endpoints {
            builder.add_rsf_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = rsf_alternative_endpoints {
            builder.add_rsf_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = s3_preferred_endpoints {
            builder.add_s3_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = s3_alternative_endpoints {
            builder.add_s3_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = api_preferred_endpoints {
            builder.add_api_preferred_endpoints(extract_endpoints(endpoints)?);
        }
        if let Some(endpoints) = api_alternative_endpoints {
            builder.add_api_alternative_endpoints(extract_endpoints(endpoints)?);
        }
        let region = builder.build();
        Ok((Self(region.to_owned()), RegionsProvider(Box::new(region))))
    }

    /// 获取区域 ID
    #[getter]
    fn get_region_id(&self) -> &str {
        self.0.region_id()
    }

    /// 获取 S3 区域 ID
    #[getter]
    fn get_s3_region_id(&self) -> &str {
        self.0.s3_region_id()
    }

    /// 获取上传服务终端列表
    #[getter]
    fn get_up(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.up())
    }

    /// 获取上传服务主要终端列表
    #[getter]
    fn get_up_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.up_preferred_endpoints())
    }

    /// 获取上传服务备选终端列表
    #[getter]
    fn get_up_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.up_alternative_endpoints())
    }

    /// 获取下载服务终端列表
    #[getter]
    fn get_io(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.io())
    }

    /// 获取下载服务主要终端列表
    #[getter]
    fn get_io_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.io_preferred_endpoints())
    }

    /// 获取下载服务备选终端列表
    #[getter]
    fn get_io_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.io_alternative_endpoints())
    }

    /// 获取存储空间管理服务终端列表
    #[getter]
    fn get_uc(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.uc())
    }

    /// 获取存储空间管理服务主要终端列表
    #[getter]
    fn get_uc_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.uc_preferred_endpoints())
    }

    /// 获取存储空间管理服务备选终端列表
    #[getter]
    fn get_uc_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.uc_alternative_endpoints())
    }

    /// 获取元数据管理服务终端列表
    #[getter]
    fn get_rs(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.rs())
    }

    /// 获取元数据管理服务主要终端列表
    #[getter]
    fn get_rs_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.rs_preferred_endpoints())
    }

    /// 获取元数据管理服务备选终端列表
    #[getter]
    fn get_rs_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.rs_alternative_endpoints())
    }

    /// 获取元数据列举服务终端列表
    #[getter]
    fn get_rsf(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.rsf())
    }

    /// 获取元数据列举服务主要终端列表
    #[getter]
    fn get_rsf_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.rsf_preferred_endpoints())
    }

    /// 获取元数据列举服务备选终端列表
    #[getter]
    fn get_rsf_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.rsf_alternative_endpoints())
    }

    /// 获取 API 入口服务终端列表
    #[getter]
    fn get_api(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.api())
    }

    /// 获取 API 入口服务主要终端列表
    #[getter]
    fn get_api_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.api_preferred_endpoints())
    }

    /// 获取 API 入口服务备选终端列表
    #[getter]
    fn get_api_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.api_alternative_endpoints())
    }

    /// 获取 S3 入口服务终端列表
    #[getter]
    fn get_s3(&self) -> PyResult<Py<Endpoints>> {
        encapsulate_endpoints(self.0.s3())
    }

    /// 获取 S3 入口服务主要终端列表
    #[getter]
    fn get_s3_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.s3_preferred_endpoints())
    }

    /// 获取 S3 入口服务备选终端列表
    #[getter]
    fn get_s3_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoint_vec(self.0.s3_alternative_endpoints())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.0 == other.0).to_object(py),
            _ => py.NotImplemented(),
        }
    }
}

/// 七牛所有区域信息查询器
///
/// 通过 `AllRegionsProvider(credential_provider, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)` 创建七牛所有区域信息查询器
#[pyclass(extends = RegionsProvider)]
#[pyo3(
    text_signature = "(credential_provider, /, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
)]
#[derive(Clone)]
struct AllRegionsProvider;

#[pymethods]
impl AllRegionsProvider {
    #[new]
    #[args(
        auto_persistent = "true",
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        credential_provider: CredentialProvider,
        auto_persistent: bool,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> (Self, RegionsProvider) {
        let builder = Self::new_builder(
            credential_provider,
            use_https,
            uc_endpoints,
            cache_lifetime_secs,
            shrink_interval_secs,
        );
        (
            Self,
            RegionsProvider(Box::new(
                builder.default_load_or_create_from(auto_persistent),
            )),
        )
    }

    /// 从文件系统加载或构建七牛所有区域信息查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    #[staticmethod]
    #[pyo3(
        text_signature = "(credential_provider, path, /, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[args(
        auto_persistent = "true",
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn load_or_create_from(
        credential_provider: CredentialProvider,
        path: PathBuf,
        auto_persistent: bool,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        let builder = Self::new_builder(
            credential_provider,
            use_https,
            uc_endpoints,
            cache_lifetime_secs,
            shrink_interval_secs,
        );
        Py::new(
            py,
            (
                Self,
                RegionsProvider(Box::new(builder.load_or_create_from(path, auto_persistent))),
            ),
        )
    }

    /// 构建七牛所有区域信息查询器
    ///
    /// 不启用文件系统持久化缓存
    #[staticmethod]
    #[pyo3(
        text_signature = "(credential_provider, /, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[args(
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn in_memory(
        credential_provider: CredentialProvider,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Py<Self>> {
        let builder = Self::new_builder(
            credential_provider,
            use_https,
            uc_endpoints,
            cache_lifetime_secs,
            shrink_interval_secs,
        );
        Py::new(py, (Self, RegionsProvider(Box::new(builder.in_memory()))))
    }
}

impl AllRegionsProvider {
    fn new_builder(
        credential_provider: CredentialProvider,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> qiniu_sdk::http_client::AllRegionsProviderBuilder {
        let mut builder = qiniu_sdk::http_client::AllRegionsProvider::builder(credential_provider);
        builder = builder.use_https(use_https);
        if let Some(uc_endpoints) = uc_endpoints {
            builder = builder.uc_endpoints(uc_endpoints.0);
        }
        if let Some(cache_lifetime_secs) = cache_lifetime_secs {
            builder = builder.cache_lifetime(Duration::from_secs(cache_lifetime_secs));
        }
        if let Some(shrink_interval_secs) = shrink_interval_secs {
            builder = builder.shrink_interval(Duration::from_secs(shrink_interval_secs));
        }
        builder
    }
}

/// 存储空间相关区域查询构建器
///
/// 通过 `BucketRegionsQueryer(auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)` 创建存储空间相关区域查询构建器
#[pyclass]
#[pyo3(
    text_signature = "(/, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
)]
#[derive(Clone)]
pub(crate) struct BucketRegionsQueryer(qiniu_sdk::http_client::BucketRegionsQueryer);

#[pymethods]
impl BucketRegionsQueryer {
    #[new]
    #[args(
        auto_persistent = "true",
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        auto_persistent: bool,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> Self {
        Self(
            Self::make_queryer_builder(
                use_https,
                uc_endpoints,
                cache_lifetime_secs,
                shrink_interval_secs,
            )
            .default_load_or_create_from(auto_persistent),
        )
    }

    /// 从文件系统加载或构建存储空间相关区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    #[staticmethod]
    #[args(
        auto_persistent = "true",
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[pyo3(
        text_signature = "(path, /, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn load_or_create_from(
        path: PathBuf,
        auto_persistent: bool,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> Self {
        Self(
            Self::make_queryer_builder(
                use_https,
                uc_endpoints,
                cache_lifetime_secs,
                shrink_interval_secs,
            )
            .load_or_create_from(path, auto_persistent),
        )
    }

    /// 构建存储空间相关区域查询器
    ///
    /// 不启用文件系统持久化缓存
    #[staticmethod]
    #[args(
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[pyo3(
        text_signature = "(/, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn in_memory(
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> Self {
        Self(
            Self::make_queryer_builder(
                use_https,
                uc_endpoints,
                cache_lifetime_secs,
                shrink_interval_secs,
            )
            .in_memory(),
        )
    }

    /// 查询存储空间相关区域
    #[pyo3(text_signature = "($self, access_key, bucket_name)")]
    fn query(&self, access_key: &str, bucket_name: &str) -> RegionsProvider {
        RegionsProvider(Box::new(self.0.query(access_key, bucket_name)))
    }
}

impl BucketRegionsQueryer {
    fn make_queryer_builder(
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> qiniu_sdk::http_client::BucketRegionsQueryerBuilder {
        let mut builder = qiniu_sdk::http_client::BucketRegionsQueryer::builder();
        builder.use_https(use_https);
        if let Some(uc_endpoints) = uc_endpoints {
            builder.uc_endpoints(uc_endpoints.0);
        }
        if let Some(cache_lifetime_secs) = cache_lifetime_secs {
            builder.cache_lifetime(Duration::from_secs(cache_lifetime_secs));
        }
        if let Some(shrink_interval_secs) = shrink_interval_secs {
            builder.shrink_interval(Duration::from_secs(shrink_interval_secs));
        }
        builder
    }
}

impl From<BucketRegionsQueryer> for qiniu_sdk::http_client::BucketRegionsQueryer {
    fn from(queryer: BucketRegionsQueryer) -> Self {
        queryer.0
    }
}

impl From<qiniu_sdk::http_client::BucketRegionsQueryer> for BucketRegionsQueryer {
    fn from(queryer: qiniu_sdk::http_client::BucketRegionsQueryer) -> Self {
        Self(queryer)
    }
}

/// 存储空间绑定域名查询器
///
/// 查询该存储空间绑定的域名。
///
/// 通过 `BucketDomainsQueryer(auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)` 创建存储空间绑定域名查询器
#[pyclass]
#[pyo3(
    text_signature = "(/, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
)]
#[derive(Clone)]
struct BucketDomainsQueryer(qiniu_sdk::http_client::BucketDomainsQueryer);

#[pymethods]
impl BucketDomainsQueryer {
    #[new]
    #[args(
        auto_persistent = "true",
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        auto_persistent: bool,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> Self {
        Self(
            Self::make_queryer_builder(
                use_https,
                uc_endpoints,
                cache_lifetime_secs,
                shrink_interval_secs,
            )
            .default_load_or_create_from(auto_persistent),
        )
    }

    /// 从文件系统加载或构建存储空间绑定域名查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    #[staticmethod]
    #[args(
        auto_persistent = "true",
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[pyo3(
        text_signature = "(path, /, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn load_or_create_from(
        path: PathBuf,
        auto_persistent: bool,
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> Self {
        Self(
            Self::make_queryer_builder(
                use_https,
                uc_endpoints,
                cache_lifetime_secs,
                shrink_interval_secs,
            )
            .load_or_create_from(path, auto_persistent),
        )
    }

    /// 构建存储空间绑定域名查询器
    ///
    /// 不启用文件系统持久化缓存
    #[staticmethod]
    #[args(
        use_https = "true",
        uc_endpoints = "None",
        cache_lifetime_secs = "None",
        shrink_interval_secs = "None"
    )]
    #[pyo3(
        text_signature = "(/, auto_persistent = True, use_https = True, uc_endpoints = None, cache_lifetime_secs = None, shrink_interval_secs = None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn in_memory(
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> Self {
        Self(
            Self::make_queryer_builder(
                use_https,
                uc_endpoints,
                cache_lifetime_secs,
                shrink_interval_secs,
            )
            .in_memory(),
        )
    }

    /// 查询存储空间相关域名
    #[pyo3(text_signature = "($self, access_key, bucket_name)")]
    fn query(&self, credential: CredentialProvider, bucket_name: &str) -> EndpointsProvider {
        EndpointsProvider(Box::new(self.0.query(credential, bucket_name)))
    }
}

impl BucketDomainsQueryer {
    fn make_queryer_builder(
        use_https: bool,
        uc_endpoints: Option<Endpoints>,
        cache_lifetime_secs: Option<u64>,
        shrink_interval_secs: Option<u64>,
    ) -> qiniu_sdk::http_client::BucketDomainsQueryerBuilder {
        let mut builder = qiniu_sdk::http_client::BucketDomainsQueryer::builder();
        builder.use_https(use_https);
        if let Some(uc_endpoints) = uc_endpoints {
            builder.uc_endpoints(uc_endpoints.0);
        }
        if let Some(cache_lifetime_secs) = cache_lifetime_secs {
            builder.cache_lifetime(Duration::from_secs(cache_lifetime_secs));
        }
        if let Some(shrink_interval_secs) = shrink_interval_secs {
            builder.shrink_interval(Duration::from_secs(shrink_interval_secs));
        }
        builder
    }
}

fn encapsulate_endpoint_vec(endpoints: &[qiniu_sdk::http_client::Endpoint]) -> Vec<Endpoint> {
    endpoints.iter().cloned().map(Endpoint).collect()
}

fn encapsulate_endpoints(endpoints: &qiniu_sdk::http_client::Endpoints) -> PyResult<Py<Endpoints>> {
    Python::with_gil(|py| {
        Py::new(
            py,
            (
                Endpoints(endpoints.to_owned()),
                EndpointsProvider(Box::new(endpoints.to_owned())),
            ),
        )
    })
}
