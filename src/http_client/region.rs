use crate::{
    exceptions::{
        QiniuInvalidDomainWithPortError, QiniuInvalidEndpointError, QiniuInvalidIpAddrWithPortError,
    },
    utils::extract_endpoints,
};
use pyo3::{prelude::*, pyclass::CompareOp};
use qiniu_sdk::http_client::{
    DomainWithPortParseError, EndpointParseError, IpAddrWithPortParseError,
};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<DomainWithPort>()?;
    m.add_class::<IpAddrWithPort>()?;
    m.add_class::<Endpoint>()?;
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

/// 七牛存储区域
///
/// 提供七牛不同服务的终端地址列表
#[pyclass]
#[pyo3(text_signature = "(region_id, **opts)")]
#[derive(Clone)]
struct Region(qiniu_sdk::http_client::Region);

#[pymethods]
impl Region {
    #[new]
    #[args(opts = "**")]
    fn new(region_id: String, opts: Option<PyObject>, py: Python<'_>) -> PyResult<Self> {
        let mut builder = qiniu_sdk::http_client::Region::builder(region_id);
        if let Some(opts) = opts {
            if let Ok(s3_region_id) = opts
                .as_ref(py)
                .get_item("s3_region_id")?
                .extract::<String>()
            {
                builder.s3_region_id(s3_region_id);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("up_preferred_endpoints") {
                builder.add_up_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("up_alternative_endpoints") {
                builder.add_up_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("io_preferred_endpoints") {
                builder.add_io_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("io_alternative_endpoints") {
                builder.add_io_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("uc_preferred_endpoints") {
                builder.add_uc_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("uc_alternative_endpoints") {
                builder.add_uc_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("rs_preferred_endpoints") {
                builder.add_rs_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("rs_alternative_endpoints") {
                builder.add_rs_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("rsf_preferred_endpoints") {
                builder.add_rsf_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("rsf_alternative_endpoints") {
                builder.add_rsf_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("s3_preferred_endpoints") {
                builder.add_s3_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("s3_alternative_endpoints") {
                builder.add_s3_alternative_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("api_preferred_endpoints") {
                builder.add_api_preferred_endpoints(extract_endpoints(endpoints)?);
            }
            if let Ok(endpoints) = opts.as_ref(py).get_item("api_alternative_endpoints") {
                builder.add_api_alternative_endpoints(extract_endpoints(endpoints)?);
            }
        }
        Ok(Self(builder.build()))
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

    /// 获取上传服务主要终端列表
    #[getter]
    fn get_up_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.up_preferred_endpoints())
    }

    /// 获取上传服务备选终端列表
    #[getter]
    fn get_up_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.up_alternative_endpoints())
    }

    /// 获取下载服务主要终端列表
    #[getter]
    fn get_io_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.io_preferred_endpoints())
    }

    /// 获取下载服务备选终端列表
    #[getter]
    fn get_io_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.io_alternative_endpoints())
    }

    /// 获取元数据管理服务主要终端列表
    #[getter]
    fn get_rs_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.rs_preferred_endpoints())
    }

    /// 获取元数据管理服务备选终端列表
    #[getter]
    fn get_rs_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.rs_alternative_endpoints())
    }

    /// 获取元数据列举服务主要终端列表
    #[getter]
    fn get_rsf_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.rsf_preferred_endpoints())
    }

    /// 获取元数据列举服务备选终端列表
    #[getter]
    fn get_rsf_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.rsf_alternative_endpoints())
    }

    /// 获取 API 入口服务主要终端列表
    #[getter]
    fn get_api_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.api_preferred_endpoints())
    }

    /// 获取 API 入口服务备选终端列表
    #[getter]
    fn get_api_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.api_alternative_endpoints())
    }

    /// 获取 S3 入口服务主要终端列表
    #[getter]
    fn get_s3_preferred_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.s3_preferred_endpoints())
    }

    /// 获取 S3 入口服务备选终端列表
    #[getter]
    fn get_s3_alternative_endpoints(&self) -> Vec<Endpoint> {
        encapsulate_endpoints(self.0.s3_alternative_endpoints())
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

fn encapsulate_endpoints(endpoints: &[qiniu_sdk::http_client::Endpoint]) -> Vec<Endpoint> {
    endpoints.iter().cloned().map(Endpoint).collect()
}
