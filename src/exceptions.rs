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
    qiniu_bindings,
    QiniuUserAgentInitializeError,
    PyRuntimeError,
    "?????????????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidPortError,
    PyValueError,
    "???????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuBodySizeMissingError,
    PyTypeError,
    "???????????? body_len ????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuEmptyChainCredentialsProvider,
    PyValueError,
    "????????? ChainCredentialsProvider ??????"
);
create_exception!(
    qiniu_bindings,
    QiniuEmptyRegionsProvider,
    PyValueError,
    "????????? StaticRegionsProvider ??????"
);
create_exception!(
    qiniu_bindings,
    QiniuEmptyEndpoints,
    PyValueError,
    "????????? Endpoints ??????"
);
create_exception!(
    qiniu_bindings,
    QiniuEmptyChainedResolver,
    PyValueError,
    "????????? ChainedResolver ??????"
);
create_exception!(
    qiniu_bindings,
    QiniuUnsupportedTypeError,
    PyValueError,
    "??????????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidConcurrency,
    PyValueError,
    "?????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidObjectSize,
    PyValueError,
    "????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidPartSize,
    PyValueError,
    "????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidMultiply,
    PyValueError,
    "????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidLimitation,
    PyValueError,
    "????????????????????????"
);
create_exception!(
    qiniu_bindings,
    QiniuInvalidSourceKeyLengthError,
    PyValueError,
    "??????????????? KEY ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuCallbackError,
    "QiniuCallbackError",
    PyRuntimeError,
    QiniuCallbackErrorInfo,
    anyhow::Error,
    "??????????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuIsahcError,
    "QiniuIsahcError",
    PyRuntimeError,
    QiniuIsahcErrorInfo,
    qiniu_sdk::isahc::isahc::Error,
    "?????? Isahc ??????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuTrustDNSError,
    "QiniuTrustDNSError",
    PyRuntimeError,
    QiniuTrustDNSErrorKind,
    qiniu_sdk::http_client::trust_dns_resolver::error::ResolveError,
    "?????? Isahc ??????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidURLError,
    "QiniuInvalidURLError",
    PyValueError,
    QiniuInvalidURLErrorInfo,
    qiniu_sdk::http::uri::InvalidUri,
    "???????????? URL ??????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidStatusCodeError,
    "QiniuInvalidStatusCodeError",
    PyValueError,
    QiniuInvalidStatusCodeErrorInfo,
    qiniu_sdk::http::InvalidStatusCode,
    "???????????? HTTP ???????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidMethodError,
    "QiniuInvalidMethodError",
    PyValueError,
    QiniuInvalidMethodErrorInfo,
    qiniu_sdk::http::InvalidMethod,
    "???????????? HTTP ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidHeaderNameError,
    "QiniuInvalidHeaderNameError",
    PyValueError,
    QiniuInvalidHeaderNameErrorInfo,
    qiniu_sdk::http::InvalidHeaderName,
    "???????????? HTTP ???????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidHeaderValueError,
    "QiniuInvalidHeaderValueError",
    PyValueError,
    QiniuInvalidHeaderValueErrorInfo,
    qiniu_sdk::http::InvalidHeaderValue,
    "???????????? HTTP ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuHeaderValueEncodingError,
    "QiniuHeaderValueEncodingError",
    PyValueError,
    QiniuHeaderValueEncodingErrorInfo,
    qiniu_sdk::http::header::ToStrError,
    "?????? HTTP ???????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidIpAddrError,
    "QiniuInvalidIpAddrError",
    PyValueError,
    QiniuInvalidIpAddrErrorInfo,
    std::net::AddrParseError,
    "???????????? IP ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidDomainWithPortError,
    "QiniuInvalidDomainWithPortError",
    PyValueError,
    QiniuInvalidDomainWithPortErrorInfo,
    qiniu_sdk::http_client::DomainWithPortParseError,
    "????????????????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidIpAddrWithPortError,
    "QiniuInvalidIpAddrWithPortError",
    PyValueError,
    QiniuInvalidIpAddrWithPortErrorInfo,
    qiniu_sdk::http_client::IpAddrWithPortParseError,
    "???????????? IP ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidEndpointError,
    "QiniuInvalidEndpointError",
    PyValueError,
    QiniuInvalidEndpointErrorInfo,
    qiniu_sdk::http_client::EndpointParseError,
    "??????????????????????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuJsonError,
    "QiniuJsonError",
    PyValueError,
    QiniuJsonErrorInfo,
    serde_json::Error,
    "?????? JSON ??????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuTimeError,
    "QiniuTimeError",
    PyValueError,
    QiniuTimeErrorInfo,
    SystemTimeError,
    "??????????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuBase64Error,
    "QiniuBase64Error",
    PyValueError,
    QiniuBase64ErrorInfo,
    qiniu_sdk::utils::base64::DecodeError,
    "?????? Base64 ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuMimeParseError,
    "QiniuMimeParseError",
    PyValueError,
    QiniuMimeParseErrorInfo,
    qiniu_sdk::http_client::mime::FromStrError,
    "?????? MIME ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuUploadTokenFormatError,
    "QiniuUploadTokenFormatError",
    PyValueError,
    QiniuUploadTokenFormatErrorInfo,
    qiniu_sdk::upload_token::ParseError,
    "??????????????????????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuIoError,
    "QiniuIoError",
    PyIOError,
    QiniuIoErrorInfo,
    IoError,
    "???????????? IO ??????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuHttpCallError,
    "QiniuHttpCallError",
    PyIOError,
    QiniuHttpCallErrorInfo,
    qiniu_sdk::http::ResponseError,
    "?????? HTTP ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuApiCallError,
    "QiniuApiCallError",
    PyIOError,
    QiniuApiCallErrorInfo,
    MaybeOwned<'static, qiniu_sdk::http_client::ResponseError>,
    "?????? API ????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuDownloadError,
    "QiniuDownloadError",
    PyIOError,
    QiniuDownloadErrorInfo,
    qiniu_sdk::download::DownloadError,
    "??????????????????"
);
create_exception_with_info!(
    qiniu_bindings,
    QiniuAuthorizationError,
    "QiniuAuthorizationError",
    PyIOError,
    QiniuAuthorizationErrorInfo,
    qiniu_sdk::http_client::AuthorizationError,
    "??????????????????"
);

create_exception_with_info!(
    qiniu_bindings,
    QiniuInvalidPrefixLengthError,
    "QiniuInvalidPrefixLengthError",
    PyValueError,
    QiniuInvalidPrefixLengthErrorInfo,
    qiniu_sdk::http_client::PrefixLenError,
    "????????????????????????????????????"
);

/// HTTP ??????????????????
#[pyclass]
#[derive(Debug, Clone, Copy)]
enum QiniuHttpCallErrorKind {
    /// ????????????????????????????????????
    ProtocolError = 1,

    /// ??????????????? / ????????????
    InvalidRequestResponse = 2,

    /// ????????? URL
    InvalidUrl = 3,

    /// ????????? HTTP ???
    InvalidHeader = 4,

    /// ??????????????????
    ConnectError = 5,

    /// ??????????????????
    ProxyError = 6,

    /// DNS ?????????????????????
    DnsServerError = 7,

    /// ??????????????????
    UnknownHostError = 8,

    /// ????????????
    SendError = 9,

    /// ????????????
    ReceiveError = 10,

    /// ?????? IO ??????
    LocalIoError = 11,

    /// ????????????
    TimeoutError = 12,

    /// SSL ?????????????????????
    ClientCertError = 13,

    /// SSL ????????????????????????
    ServerCertError = 14,

    /// SSL ??????
    SslError = 15,

    /// ?????????????????????
    TooManyRedirect = 16,

    /// ????????????
    UnknownError = 17,

    /// ????????????????????????
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
    /// ?????? HTTP ??????????????????
    #[getter]
    fn get_kind(&self) -> QiniuHttpCallErrorKind {
        self.0.kind().into()
    }

    /// ??????????????? IP ??????
    #[getter]
    fn get_server_ip(&self) -> Option<String> {
        self.0.server_ip().map(|ip| ip.to_string())
    }

    /// ????????????????????????
    #[getter]
    fn get_server_port(&self) -> Option<u16> {
        self.0.server_port().map(|port| port.get())
    }

    /// ????????????????????????
    #[getter]
    fn get_metrics(&self) -> Option<Metrics> {
        self.0.metrics().cloned().map(Metrics::from)
    }
}

/// ?????? API ??????????????????
#[pyclass]
#[derive(Debug, Clone, Copy)]
enum QiniuApiCallErrorKind {
    /// HTTP ???????????????
    HttpError = 1,

    /// ?????????????????????
    StatusCodeError = 2,

    /// ?????????????????????????????? 0 - 199 ??? 300 - 399????????????????????? HttpCaller ???????????????
    UnexpectedStatusCode = 3,

    /// ?????????????????????
    ParseResponseError = 4,

    /// ?????????????????????
    UnexpectedEof = 5,

    /// ?????????????????????
    MaliciousResponse = 6,

    /// ??????????????????
    SystemCallError = 7,

    /// ????????????
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
    /// ?????? API ??????????????????
    #[getter]
    fn get_message(&self) -> Option<String> {
        use std::error::Error;

        self.0.source().map(|src| src.to_string())
    }

    /// ?????? API ??????????????????
    #[getter]
    fn get_kind(&self) -> QiniuApiCallErrorKind {
        self.0.kind().into()
    }

    /// ?????? HTTP ??????????????????
    #[getter]
    fn get_http_error_kind(&self) -> Option<QiniuHttpCallErrorKind> {
        use qiniu_sdk::http_client::ResponseErrorKind;
        match self.0.kind() {
            ResponseErrorKind::HttpError(kind) => Some(kind.into()),
            _ => None,
        }
    }

    /// ?????? HTTP ?????????
    #[getter]
    fn get_status_code(&self) -> Option<u16> {
        use qiniu_sdk::http_client::ResponseErrorKind;
        match self.0.kind() {
            ResponseErrorKind::StatusCodeError(status_code) => Some(status_code.as_u16()),
            ResponseErrorKind::UnexpectedStatusCode(status_code) => Some(status_code.as_u16()),
            _ => None,
        }
    }

    /// ?????????????????????
    #[getter]
    fn get_response_body_sample<'p>(&self, py: Python<'p>) -> &'p PyBytes {
        PyBytes::new(py, self.0.response_body_sample())
    }

    /// ??????????????? IP ??????
    #[getter]
    fn get_server_ip(&self) -> Option<String> {
        self.0.server_ip().map(|ip| ip.to_string())
    }

    /// ????????????????????????
    #[getter]
    fn get_server_port(&self) -> Option<u16> {
        self.0.server_port().map(|port| port.get())
    }

    /// ????????????????????????
    #[getter]
    fn get_metrics(&self) -> Option<Metrics> {
        self.0.metrics().cloned().map(Metrics::from)
    }

    /// ?????? HTTP ????????? X-Log ??????
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

    /// ?????? HTTP ????????? X-ReqId ??????
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
