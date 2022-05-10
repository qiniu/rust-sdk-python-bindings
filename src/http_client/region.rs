use crate::{
    exceptions::{
        QiniuApiCallError, QiniuInvalidDomainWithPortError, QiniuInvalidEndpointError,
        QiniuInvalidIpAddrWithPortError, QiniuInvalidServiceNameError,
    },
    utils::{extract_endpoints, extract_service_names},
};
use pyo3::{prelude::*, pyclass::CompareOp, types::PyDict};
use qiniu_sdk::http_client::{
    DomainWithPortParseError, EndpointParseError, EndpointsGetOptions, IpAddrWithPortParseError,
};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<DomainWithPort>()?;
    m.add_class::<IpAddrWithPort>()?;
    m.add_class::<Endpoint>()?;
    m.add_class::<Endpoints>()?;
    m.add_class::<Region>()?;
    Ok(())
}

/// 域名和端口号
///
/// 用来表示一个七牛服务器的地址，端口号是可选的，如果不提供，则根据传输协议判定默认的端口号。
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
            format!("{}:{}", domain, port).parse()
        } else {
            domain.parse()
        }
        .map_err(|err: DomainWithPortParseError| {
            QiniuInvalidDomainWithPortError::new_err(err.to_string())
        })?;
        Ok(Self(host))
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
        .map_err(|err: IpAddrWithPortParseError| {
            QiniuInvalidIpAddrWithPortError::new_err(err.to_string())
        })?;
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
        .map_err(|err: EndpointParseError| QiniuInvalidEndpointError::new_err(err.to_string()))?;
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

impl Endpoint {
    pub(crate) fn into_inner(self) -> qiniu_sdk::http_client::Endpoint {
        self.0
    }
}

#[pyclass]
#[derive(Clone, Copy)]
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

impl TryFrom<qiniu_sdk::http_client::ServiceName> for ServiceName {
    type Error = PyErr;

    fn try_from(svc: qiniu_sdk::http_client::ServiceName) -> Result<Self, Self::Error> {
        match svc {
            qiniu_sdk::http_client::ServiceName::Up => Ok(ServiceName::Up),
            qiniu_sdk::http_client::ServiceName::Io => Ok(ServiceName::Io),
            qiniu_sdk::http_client::ServiceName::Uc => Ok(ServiceName::Uc),
            qiniu_sdk::http_client::ServiceName::Rs => Ok(ServiceName::Rs),
            qiniu_sdk::http_client::ServiceName::Rsf => Ok(ServiceName::Rsf),
            qiniu_sdk::http_client::ServiceName::Api => Ok(ServiceName::Api),
            qiniu_sdk::http_client::ServiceName::S3 => Ok(ServiceName::S3),
            _ => Err(QiniuInvalidServiceNameError::new_err(format!(
                "Unrecognized ServiceName {:?}",
                svc
            ))),
        }
    }
}

/// 终端地址列表获取接口
///
/// 同时提供阻塞获取接口和异步获取接口，异步获取接口则需要启用 `async` 功能
#[pyclass(subclass)]
#[derive(Clone)]
struct EndpointsProvider(Box<dyn qiniu_sdk::http_client::EndpointsProvider>);

#[pymethods]
impl EndpointsProvider {
    #[args(opts = "**")]
    fn get_endpoints(&self, opts: Option<&PyDict>, py: Python<'_>) -> PyResult<Py<Endpoints>> {
        let mut service_names = Vec::new();
        let opts = Self::extend_endpoints_get_options(opts, &mut service_names)?;
        let endpoints = py
            .allow_threads(|| self.0.get_endpoints(opts))
            .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
            .into_owned();
        Self::make_initializer(endpoints, py)
    }

    #[args(opts = "**")]
    fn async_get_endpoints<'p>(
        &self,
        opts: Option<Py<PyDict>>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut service_names = Vec::new();
            let opts = Python::with_gil(|py| {
                Self::extend_endpoints_get_options(
                    opts.as_ref().map(|opts| opts.as_ref(py)),
                    &mut service_names,
                )
            })?;
            let endpoints = provider
                .async_get_endpoints(opts)
                .await
                .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
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

impl EndpointsProvider {
    fn extend_endpoints_get_options<'a>(
        opts: Option<&PyDict>,
        service_names: &'a mut Vec<qiniu_sdk::http_client::ServiceName>,
    ) -> PyResult<EndpointsGetOptions<'a>> {
        if let Some(opts) = opts {
            if let Some(svcs) = opts.get_item("service_names") {
                service_names.extend(extract_service_names(svcs)?);
            }
        }
        let opts = EndpointsGetOptions::builder()
            .service_names(service_names)
            .build();
        Ok(opts)
    }

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
#[pyclass(extends = EndpointsProvider)]
#[pyo3(text_signature = "(preferred_endpoints, alternative_endpoints = None)")]
#[derive(Clone)]
struct Endpoints(qiniu_sdk::http_client::Endpoints);

#[pymethods]
impl Endpoints {
    #[new]
    #[args(alternative_endpoints = "None")]
    fn new(
        preferred_endpoints: &PyAny,
        alternative_endpoints: Option<&PyAny>,
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

/// 区域信息获取接口
///
/// 可以获取一个区域也可以获取多个区域
///
/// 同时提供阻塞获取接口和异步获取接口，异步获取接口则需要启用 `async` 功能
#[pyclass(subclass)]
#[derive(Clone)]
struct RegionsProvider(Box<dyn qiniu_sdk::http_client::RegionsProvider>);

#[pymethods]
impl RegionsProvider {
    #[pyo3(text_signature = "()")]
    fn get(&self, py: Python<'_>) -> PyResult<Py<Region>> {
        let region = py
            .allow_threads(|| self.0.get(Default::default()))
            .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
            .into_region();
        Self::make_initializer(region, py)
    }

    #[pyo3(text_signature = "()")]
    fn get_all(&self, py: Python<'_>) -> PyResult<Vec<Py<Region>>> {
        let regions = py
            .allow_threads(|| self.0.get_all(Default::default()))
            .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
            .into_regions()
            .into_iter()
            .map(|region| Self::make_initializer(region, py))
            .collect::<PyResult<Vec<Py<Region>>>>()?;
        Ok(regions)
    }

    #[pyo3(text_signature = "()")]
    fn async_get<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let region = provider
                .async_get(Default::default())
                .await
                .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
                .into_region();
            Python::with_gil(|py| Self::make_initializer(region, py))
        })
    }

    #[pyo3(text_signature = "()")]
    fn async_get_all<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let regions = provider
                .async_get_all(Default::default())
                .await
                .map_err(|err| QiniuApiCallError::new_err(err.to_string()))?
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

/// 七牛存储区域
///
/// 提供七牛不同服务的终端地址列表
#[pyclass(extends = RegionsProvider)]
#[pyo3(text_signature = "(region_id, **opts)")]
#[derive(Clone)]
struct Region(qiniu_sdk::http_client::Region);

#[pymethods]
impl Region {
    #[new]
    #[args(opts = "**")]
    fn new(region_id: String, opts: Option<&PyDict>) -> PyResult<(Self, RegionsProvider)> {
        let mut builder = qiniu_sdk::http_client::Region::builder(region_id);
        if let Some(opts) = opts {
            if let Some(s3_region_id) = opts.get_item("s3_region_id") {
                builder.s3_region_id(s3_region_id.extract::<String>()?);
            }
            if let Some(endpoints) = opts.get_item("up_preferred_endpoints") {
                builder.add_up_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("up_alternative_endpoints") {
                builder.add_up_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("io_preferred_endpoints") {
                builder.add_io_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("io_alternative_endpoints") {
                builder.add_io_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("uc_preferred_endpoints") {
                builder.add_uc_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("uc_alternative_endpoints") {
                builder.add_uc_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("rs_preferred_endpoints") {
                builder.add_rs_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("rs_alternative_endpoints") {
                builder.add_rs_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("rsf_preferred_endpoints") {
                builder.add_rsf_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("rsf_alternative_endpoints") {
                builder.add_rsf_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("s3_preferred_endpoints") {
                builder.add_s3_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("s3_alternative_endpoints") {
                builder.add_s3_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("api_preferred_endpoints") {
                builder.add_api_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Some(endpoints) = opts.get_item("api_alternative_endpoints") {
                builder.add_api_alternative_endpoints(extract_endpoints(endpoints)?);
            }
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
