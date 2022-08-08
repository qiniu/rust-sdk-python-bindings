use super::http::Metrics;
use maybe_owned::MaybeOwned;
use pyo3::{
    create_exception,
    exceptions::{PyIOError, PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    types::PyBytes,
};
use std::{io::Error as IoError, time::SystemTimeError};

pub(super) fn register(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add(
        "QiniuUserAgentInitializeError",
        py.get_type::<QiniuUserAgentInitializeError>(),
    )?;
    m.add(
        "QiniuInvalidPortError",
        py.get_type::<QiniuInvalidPortError>(),
    )?;
    m.add(
        "QiniuEmptyChainCredentialsProvider",
        py.get_type::<QiniuEmptyChainCredentialsProvider>(),
    )?;
    m.add(
        "QiniuEmptyRegionsProvider",
        py.get_type::<QiniuEmptyRegionsProvider>(),
    )?;
    m.add(
        "QiniuEmptyChainedResolver",
        py.get_type::<QiniuEmptyChainedResolver>(),
    )?;
    m.add("QiniuEmptyEndpoints", py.get_type::<QiniuEmptyEndpoints>())?;
    m.add(
        "QiniuUnsupportedTypeError",
        py.get_type::<QiniuUnsupportedTypeError>(),
    )?;
    m.add(
        "QiniuBodySizeMissingError",
        py.get_type::<QiniuBodySizeMissingError>(),
    )?;
    m.add(
        "QiniuInvalidConcurrency",
        py.get_type::<QiniuInvalidConcurrency>(),
    )?;
    m.add(
        "QiniuInvalidObjectSize",
        py.get_type::<QiniuInvalidObjectSize>(),
    )?;
    m.add(
        "QiniuInvalidPartSize",
        py.get_type::<QiniuInvalidPartSize>(),
    )?;
    m.add(
        "QiniuInvalidMultiply",
        py.get_type::<QiniuInvalidMultiply>(),
    )?;
    m.add(
        "QiniuInvalidLimitation",
        py.get_type::<QiniuInvalidLimitation>(),
    )?;
    m.add(
        "QiniuInvalidSourceKeyLengthError",
        py.get_type::<QiniuInvalidSourceKeyLengthError>(),
    )?;
    m.add_class::<QiniuHttpCallErrorKind>()?;
    m.add_class::<QiniuApiCallErrorKind>()?;

    QiniuInvalidURLError::register(py, m)?;
    QiniuInvalidStatusCodeError::register(py, m)?;
    QiniuInvalidMethodError::register(py, m)?;
    QiniuInvalidHeaderNameError::register(py, m)?;
    QiniuInvalidHeaderValueError::register(py, m)?;
    QiniuHeaderValueEncodingError::register(py, m)?;
    QiniuInvalidIpAddrError::register(py, m)?;
    QiniuInvalidEndpointError::register(py, m)?;
    QiniuJsonError::register(py, m)?;
    QiniuTimeError::register(py, m)?;
    QiniuIoError::register(py, m)?;
    QiniuUploadTokenFormatError::register(py, m)?;
    QiniuBase64Error::register(py, m)?;
    QiniuMimeParseError::register(py, m)?;
    QiniuCallbackError::register(py, m)?;
    QiniuHttpCallError::register(py, m)?;
    QiniuIsahcError::register(py, m)?;
    QiniuTrustDNSError::register(py, m)?;
    QiniuInvalidDomainWithPortError::register(py, m)?;
    QiniuInvalidIpAddrWithPortError::register(py, m)?;
    QiniuApiCallError::register(py, m)?;
    QiniuDownloadError::register(py, m)?;
    QiniuAuthorizationError::register(py, m)?;
    QiniuInvalidPrefixLengthError::register(py, m)?;
    Ok(())
}

macro_rules! create_exception_with_info {
    ($module: ident, $name: ident, $name_str: literal, $base: ty, $inner_name: ident, $inner_type:ty, $doc: expr) => {
        create_exception!($module, $name, $base, $doc);

        #[pyclass]
        #[derive(Clone, Debug)]
        pub(super) struct $inner_name(std::sync::Arc<$inner_type>);

        #[pymethods]
        impl $inner_name {
            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                format!("{}", self.0)
            }
        }

        impl From<$inner_type> for $inner_name {
            fn from(t: $inner_type) -> $inner_name {
                $inner_name(std::sync::Arc::new(t))
            }
        }

        impl AsRef<$inner_type> for $inner_name {
            fn as_ref(&self) -> &$inner_type {
                &self.0
            }
        }

        impl $name {
            fn register(py: Python<'_>, m: &PyModule) -> PyResult<()> {
                m.add($name_str, py.get_type::<$name>())?;
                m.add_class::<$inner_name>()?;
                Ok(())
            }

            #[allow(dead_code)]
            pub(super) fn from_err(err: $inner_type) -> PyErr {
                Self::new_err($inner_name::from(err))
            }
        }
    };
}

create_exception!(
    qiniu_sdk,
    QiniuUserAgentInitializeError,
    PyRuntimeError,
    "七牛用户代理初始化异常"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidPortError,
    PyValueError,
    "七牛非法端口号错误"
);
create_exception!(
    qiniu_sdk,
    QiniuBodySizeMissingError,
    PyTypeError,
    "七牛缺少 body_len 参数错误"
);
create_exception!(
    qiniu_sdk,
    QiniuEmptyChainCredentialsProvider,
    PyValueError,
    "七牛空 ChainCredentialsProvider 错误"
);
create_exception!(
    qiniu_sdk,
    QiniuEmptyRegionsProvider,
    PyValueError,
    "七牛空 StaticRegionsProvider 错误"
);
create_exception!(
    qiniu_sdk,
    QiniuEmptyEndpoints,
    PyValueError,
    "七牛空 Endpoints 错误"
);
create_exception!(
    qiniu_sdk,
    QiniuEmptyChainedResolver,
    PyValueError,
    "七牛空 ChainedResolver 错误"
);
create_exception!(
    qiniu_sdk,
    QiniuUnsupportedTypeError,
    PyValueError,
    "七牛不支持的类型错误"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidConcurrency,
    PyValueError,
    "七牛并行数错误"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidObjectSize,
    PyValueError,
    "七牛对象大小错误"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidPartSize,
    PyValueError,
    "七牛分片大小错误"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidMultiply,
    PyValueError,
    "七牛分片大小错误"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidLimitation,
    PyValueError,
    "七牛分片限制错误"
);
create_exception!(
    qiniu_sdk,
    QiniuInvalidSourceKeyLengthError,
    PyValueError,
    "七牛数据源 KEY 长度错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuCallbackError,
    "QiniuCallbackError",
    PyRuntimeError,
    QiniuCallbackErrorInfo,
    anyhow::Error,
    "七牛回调异常"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuIsahcError,
    "QiniuIsahcError",
    PyRuntimeError,
    QiniuIsahcErrorInfo,
    qiniu_sdk::isahc::isahc::Error,
    "七牛 Isahc 异常"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuTrustDNSError,
    "QiniuTrustDNSError",
    PyRuntimeError,
    QiniuTrustDNSErrorKind,
    qiniu_sdk::http_client::trust_dns_resolver::error::ResolveError,
    "七牛 Isahc 异常"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidURLError,
    "QiniuInvalidURLError",
    PyValueError,
    QiniuInvalidURLErrorInfo,
    qiniu_sdk::http::uri::InvalidUri,
    "七牛非法 URL 错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidStatusCodeError,
    "QiniuInvalidStatusCodeError",
    PyValueError,
    QiniuInvalidStatusCodeErrorInfo,
    qiniu_sdk::http::InvalidStatusCode,
    "七牛非法 HTTP 状态码错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidMethodError,
    "QiniuInvalidMethodError",
    PyValueError,
    QiniuInvalidMethodErrorInfo,
    qiniu_sdk::http::InvalidMethod,
    "七牛非法 HTTP 方法错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidHeaderNameError,
    "QiniuInvalidHeaderNameError",
    PyValueError,
    QiniuInvalidHeaderNameErrorInfo,
    qiniu_sdk::http::InvalidHeaderName,
    "七牛非法 HTTP 头名称错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidHeaderValueError,
    "QiniuInvalidHeaderValueError",
    PyValueError,
    QiniuInvalidHeaderValueErrorInfo,
    qiniu_sdk::http::InvalidHeaderValue,
    "七牛非法 HTTP 头值错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuHeaderValueEncodingError,
    "QiniuHeaderValueEncodingError",
    PyValueError,
    QiniuHeaderValueEncodingErrorInfo,
    qiniu_sdk::http::header::ToStrError,
    "七牛 HTTP 头编码错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidIpAddrError,
    "QiniuInvalidIpAddrError",
    PyValueError,
    QiniuInvalidIpAddrErrorInfo,
    std::net::AddrParseError,
    "七牛非法 IP 地址错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidDomainWithPortError,
    "QiniuInvalidDomainWithPortError",
    PyValueError,
    QiniuInvalidDomainWithPortErrorInfo,
    qiniu_sdk::http_client::DomainWithPortParseError,
    "七牛非法域名错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidIpAddrWithPortError,
    "QiniuInvalidIpAddrWithPortError",
    PyValueError,
    QiniuInvalidIpAddrWithPortErrorInfo,
    qiniu_sdk::http_client::IpAddrWithPortParseError,
    "七牛非法 IP 地址错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidEndpointError,
    "QiniuInvalidEndpointError",
    PyValueError,
    QiniuInvalidEndpointErrorInfo,
    qiniu_sdk::http_client::EndpointParseError,
    "七牛非法终端地址错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuJsonError,
    "QiniuJsonError",
    PyValueError,
    QiniuJsonErrorInfo,
    serde_json::Error,
    "七牛 JSON 错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuTimeError,
    "QiniuTimeError",
    PyValueError,
    QiniuTimeErrorInfo,
    SystemTimeError,
    "七牛时间错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuBase64Error,
    "QiniuBase64Error",
    PyValueError,
    QiniuBase64ErrorInfo,
    qiniu_sdk::utils::base64::DecodeError,
    "七牛 Base64 解析错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuMimeParseError,
    "QiniuMimeParseError",
    PyValueError,
    QiniuMimeParseErrorInfo,
    qiniu_sdk::http_client::mime::FromStrError,
    "七牛 MIME 解析错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuUploadTokenFormatError,
    "QiniuUploadTokenFormatError",
    PyValueError,
    QiniuUploadTokenFormatErrorInfo,
    qiniu_sdk::upload_token::ParseError,
    "七牛上传凭证格式错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuIoError,
    "QiniuIoError",
    PyIOError,
    QiniuIoErrorInfo,
    IoError,
    "七牛本地 IO 错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuHttpCallError,
    "QiniuHttpCallError",
    PyIOError,
    QiniuHttpCallErrorInfo,
    qiniu_sdk::http::ResponseError,
    "七牛 HTTP 调用错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuApiCallError,
    "QiniuApiCallError",
    PyIOError,
    QiniuApiCallErrorInfo,
    MaybeOwned<'static, qiniu_sdk::http_client::ResponseError>,
    "七牛 API 调用错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuDownloadError,
    "QiniuDownloadError",
    PyIOError,
    QiniuDownloadErrorInfo,
    qiniu_sdk::download::DownloadError,
    "七牛下载错误"
);
create_exception_with_info!(
    qiniu_sdk,
    QiniuAuthorizationError,
    "QiniuAuthorizationError",
    PyIOError,
    QiniuAuthorizationErrorInfo,
    qiniu_sdk::http_client::AuthorizationError,
    "七牛签名异常"
);

create_exception_with_info!(
    qiniu_sdk,
    QiniuInvalidPrefixLengthError,
    "QiniuInvalidPrefixLengthError",
    PyValueError,
    QiniuInvalidPrefixLengthErrorInfo,
    qiniu_sdk::http_client::PrefixLenError,
    "七牛子网掩码前缀长度异常"
);

/// HTTP 响应错误类型
#[pyclass]
#[derive(Debug, Clone, Copy)]
enum QiniuHttpCallErrorKind {
    /// 协议错误，该协议不能支持
    ProtocolError = 1,

    /// 非法的请求 / 响应错误
    InvalidRequestResponse = 2,

    /// 非法的 URL
    InvalidUrl = 3,

    /// 非法的 HTTP 头
    InvalidHeader = 4,

    /// 网络连接失败
    ConnectError = 5,

    /// 代理连接失败
    ProxyError = 6,

    /// DNS 服务器连接失败
    DnsServerError = 7,

    /// 域名解析失败
    UnknownHostError = 8,

    /// 发送失败
    SendError = 9,

    /// 接受失败
    ReceiveError = 10,

    /// 本地 IO 失败
    LocalIoError = 11,

    /// 超时失败
    TimeoutError = 12,

    /// SSL 客户端证书错误
    ClientCertError = 13,

    /// SSL 服务器端证书错误
    ServerCertError = 14,

    /// SSL 错误
    SslError = 15,

    /// 重定向次数过多
    TooManyRedirect = 16,

    /// 未知错误
    UnknownError = 17,

    /// 回调函数返回错误
    CallbackError = 18,
}

impl From<QiniuHttpCallErrorKind> for qiniu_sdk::http::ResponseErrorKind {
    fn from(kind: QiniuHttpCallErrorKind) -> Self {
        use {qiniu_sdk::http::ResponseErrorKind, QiniuHttpCallErrorKind as HttpCallErrorKind};
        match kind {
            HttpCallErrorKind::ProtocolError => ResponseErrorKind::ProtocolError,
            HttpCallErrorKind::InvalidRequestResponse => ResponseErrorKind::InvalidRequestResponse,
            HttpCallErrorKind::InvalidUrl => ResponseErrorKind::InvalidUrl,
            HttpCallErrorKind::InvalidHeader => ResponseErrorKind::InvalidHeader,
            HttpCallErrorKind::ConnectError => ResponseErrorKind::ConnectError,
            HttpCallErrorKind::ProxyError => ResponseErrorKind::ProxyError,
            HttpCallErrorKind::DnsServerError => ResponseErrorKind::DnsServerError,
            HttpCallErrorKind::UnknownHostError => ResponseErrorKind::UnknownHostError,
            HttpCallErrorKind::SendError => ResponseErrorKind::SendError,
            HttpCallErrorKind::ReceiveError => ResponseErrorKind::ReceiveError,
            HttpCallErrorKind::LocalIoError => ResponseErrorKind::LocalIoError,
            HttpCallErrorKind::TimeoutError => ResponseErrorKind::TimeoutError,
            HttpCallErrorKind::ClientCertError => ResponseErrorKind::ClientCertError,
            HttpCallErrorKind::ServerCertError => ResponseErrorKind::ServerCertError,
            HttpCallErrorKind::SslError => ResponseErrorKind::SslError,
            HttpCallErrorKind::TooManyRedirect => ResponseErrorKind::TooManyRedirect,
            HttpCallErrorKind::UnknownError => ResponseErrorKind::UnknownError,
            HttpCallErrorKind::CallbackError => ResponseErrorKind::CallbackError,
        }
    }
}

impl From<qiniu_sdk::http::ResponseErrorKind> for QiniuHttpCallErrorKind {
    fn from(kind: qiniu_sdk::http::ResponseErrorKind) -> Self {
        use {qiniu_sdk::http::ResponseErrorKind, QiniuHttpCallErrorKind as HttpCallErrorKind};
        match kind {
            ResponseErrorKind::ProtocolError => HttpCallErrorKind::ProtocolError,
            ResponseErrorKind::InvalidRequestResponse => HttpCallErrorKind::InvalidRequestResponse,
            ResponseErrorKind::InvalidUrl => HttpCallErrorKind::InvalidUrl,
            ResponseErrorKind::InvalidHeader => HttpCallErrorKind::InvalidHeader,
            ResponseErrorKind::ConnectError => HttpCallErrorKind::ConnectError,
            ResponseErrorKind::ProxyError => HttpCallErrorKind::ProxyError,
            ResponseErrorKind::DnsServerError => HttpCallErrorKind::DnsServerError,
            ResponseErrorKind::UnknownHostError => HttpCallErrorKind::UnknownHostError,
            ResponseErrorKind::SendError => HttpCallErrorKind::SendError,
            ResponseErrorKind::ReceiveError => HttpCallErrorKind::ReceiveError,
            ResponseErrorKind::LocalIoError => HttpCallErrorKind::LocalIoError,
            ResponseErrorKind::TimeoutError => HttpCallErrorKind::TimeoutError,
            ResponseErrorKind::ClientCertError => HttpCallErrorKind::ClientCertError,
            ResponseErrorKind::ServerCertError => HttpCallErrorKind::ServerCertError,
            ResponseErrorKind::SslError => HttpCallErrorKind::SslError,
            ResponseErrorKind::TooManyRedirect => HttpCallErrorKind::TooManyRedirect,
            ResponseErrorKind::UnknownError => HttpCallErrorKind::UnknownError,
            ResponseErrorKind::CallbackError => HttpCallErrorKind::CallbackError,
            _ => panic!("Unrecognized response error kind"),
        }
    }
}

#[pymethods]
impl QiniuHttpCallErrorInfo {
    /// 获取 HTTP 响应错误类型
    #[getter]
    fn get_kind(&self) -> QiniuHttpCallErrorKind {
        self.0.kind().into()
    }

    /// 获取服务器 IP 地址
    #[getter]
    fn get_server_ip(&self) -> Option<String> {
        self.0.server_ip().map(|ip| ip.to_string())
    }

    /// 获取服务器端口号
    #[getter]
    fn get_server_port(&self) -> Option<u16> {
        self.0.server_port().map(|port| port.get())
    }

    /// 获取响应指标信息
    #[getter]
    fn get_metrics(&self) -> Option<Metrics> {
        self.0.metrics().cloned().map(Metrics::from)
    }
}

/// 七牛 API 响应错误类型
#[pyclass]
#[derive(Debug, Clone, Copy)]
enum QiniuApiCallErrorKind {
    /// HTTP 客户端错误
    HttpError = 1,

    /// 响应状态码错误
    StatusCodeError = 2,

    /// 未预期的状态码（例如 0 - 199 或 300 - 399，理论上应该由 HttpCaller 自动处理）
    UnexpectedStatusCode = 3,

    /// 解析响应体错误
    ParseResponseError = 4,

    /// 响应体提前结束
    UnexpectedEof = 5,

    /// 疑似响应被劫持
    MaliciousResponse = 6,

    /// 系统调用失败
    SystemCallError = 7,

    /// 没有尝试
    NoTry = 8,
}

impl From<qiniu_sdk::http_client::ResponseErrorKind> for QiniuApiCallErrorKind {
    fn from(kind: qiniu_sdk::http_client::ResponseErrorKind) -> Self {
        use {qiniu_sdk::http_client::ResponseErrorKind, QiniuApiCallErrorKind as CallErrorKind};
        match kind {
            ResponseErrorKind::HttpError(_) => CallErrorKind::HttpError,
            ResponseErrorKind::StatusCodeError(_) => CallErrorKind::StatusCodeError,
            ResponseErrorKind::UnexpectedStatusCode(_) => CallErrorKind::UnexpectedStatusCode,
            ResponseErrorKind::ParseResponseError => CallErrorKind::ParseResponseError,
            ResponseErrorKind::UnexpectedEof => CallErrorKind::UnexpectedEof,
            ResponseErrorKind::MaliciousResponse => CallErrorKind::MaliciousResponse,
            ResponseErrorKind::SystemCallError => CallErrorKind::SystemCallError,
            ResponseErrorKind::NoTry => CallErrorKind::NoTry,
            _ => panic!("Unrecognized api call error kind"),
        }
    }
}

#[pymethods]
impl QiniuApiCallErrorInfo {
    /// 获取 API 调用错误消息
    #[getter]
    fn get_message(&self) -> Option<String> {
        use std::error::Error;

        self.0.source().map(|src| src.to_string())
    }

    /// 获取 API 调用错误类型
    #[getter]
    fn get_kind(&self) -> QiniuApiCallErrorKind {
        self.0.kind().into()
    }

    /// 获取 HTTP 响应错误类型
    #[getter]
    fn get_http_error_kind(&self) -> Option<QiniuHttpCallErrorKind> {
        use qiniu_sdk::http_client::ResponseErrorKind;
        match self.0.kind() {
            ResponseErrorKind::HttpError(kind) => Some(kind.into()),
            _ => None,
        }
    }

    /// 获取 HTTP 状态码
    #[getter]
    fn get_status_code(&self) -> Option<u16> {
        use qiniu_sdk::http_client::ResponseErrorKind;
        match self.0.kind() {
            ResponseErrorKind::StatusCodeError(status_code) => Some(status_code.as_u16()),
            ResponseErrorKind::UnexpectedStatusCode(status_code) => Some(status_code.as_u16()),
            _ => None,
        }
    }

    /// 获取响应体样本
    #[getter]
    fn get_response_body_sample<'p>(&self, py: Python<'p>) -> &'p PyBytes {
        PyBytes::new(py, self.0.response_body_sample())
    }

    /// 获取服务器 IP 地址
    #[getter]
    fn get_server_ip(&self) -> Option<String> {
        self.0.server_ip().map(|ip| ip.to_string())
    }

    /// 获取服务器端口号
    #[getter]
    fn get_server_port(&self) -> Option<u16> {
        self.0.server_port().map(|port| port.get())
    }

    /// 获取响应指标信息
    #[getter]
    fn get_metrics(&self) -> Option<Metrics> {
        self.0.metrics().cloned().map(Metrics::from)
    }

    /// 获取 HTTP 响应的 X-Log 信息
    #[getter]
    fn get_x_log(&self) -> PyResult<Option<String>> {
        self.0
            .x_log()
            .map(|value| {
                value
                    .to_str()
                    .map(|s| s.to_string())
                    .map_err(QiniuHeaderValueEncodingError::from_err)
            })
            .transpose()
    }

    /// 获取 HTTP 响应的 X-ReqId 信息
    #[getter]
    fn get_x_reqid(&self) -> PyResult<Option<String>> {
        self.0
            .x_reqid()
            .map(|value| {
                value
                    .to_str()
                    .map(|s| s.to_string())
                    .map_err(QiniuHeaderValueEncodingError::from_err)
            })
            .transpose()
    }
}
