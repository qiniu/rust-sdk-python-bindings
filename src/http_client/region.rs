use crate::exceptions::{
    QiniuInvalidDomainWithPortError, QiniuInvalidEndpointError, QiniuInvalidIpAddrWithPortError,
};
use pyo3::prelude::*;
use qiniu_sdk::http_client::{
    DomainWithPortParseError, EndpointParseError, IpAddrWithPortParseError,
};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<DomainWithPort>()?;
    m.add_class::<IpAddrWithPort>()?;
    m.add_class::<Endpoint>()?;
    Ok(())
}

/// 域名和端口号
///
/// 用来表示一个七牛服务器的地址，端口号是可选的，如果不提供，则根据传输协议判定默认的端口号。
#[pyclass]
#[pyo3(text_signature = "(domain, port = None)")]
pub(super) struct DomainWithPort(qiniu_sdk::http_client::DomainWithPort);

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
}

/// IP 地址和端口号
///
/// 用来表示一个七牛服务器的地址，端口号是可选的，如果不提供，则根据传输协议判定默认的端口号。
#[pyclass]
#[pyo3(text_signature = "(ip, port = None)")]
pub(super) struct IpAddrWithPort(qiniu_sdk::http_client::IpAddrWithPort);

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
}

/// 终端地址
///
/// 用来表示一个域名和端口号，或 IP 地址和端口号。
#[pyclass]
#[pyo3(text_signature = "(domain_or_ip_addr, port = None)")]
pub(super) struct Endpoint(qiniu_sdk::http_client::Endpoint);

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
}
