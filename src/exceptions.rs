use pyo3::{
    create_exception,
    exceptions::{PyIOError, PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
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
    m.add(
        "QiniuUnsupportedTypeError",
        py.get_type::<QiniuUnsupportedTypeError>(),
    )?;
    m.add(
        "QiniuBodySizeMissingError",
        py.get_type::<QiniuBodySizeMissingError>(),
    )?;

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
    QiniuAuthorizationError::register(py, m)?;
    QiniuInvalidPrefixLengthError::register(py, m)?;
    Ok(())
}

macro_rules! create_exception_with_info {
    ($module: ident, $name: ident, $name_str: literal, $base: ty, $inner_name: ident, $inner_type:ty, $doc: expr) => {
        create_exception!($module, $name, $base, $doc);

        #[pyclass]
        #[derive(Clone)]
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
    qiniu_sdk_bindings,
    QiniuUserAgentInitializeError,
    PyRuntimeError,
    "七牛用户代理初始化异常"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidPortError,
    PyValueError,
    "七牛非法端口号错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuBodySizeMissingError,
    PyTypeError,
    "七牛缺少 body_len 参数错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuEmptyChainCredentialsProvider,
    PyValueError,
    "七牛空 ChainCredentialsProvider 错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuEmptyRegionsProvider,
    PyValueError,
    "七牛空 StaticRegionsProvider 错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuEmptyChainedResolver,
    PyValueError,
    "七牛空 ChainedResolver 错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuUnsupportedTypeError,
    PyValueError,
    "七牛不支持的类型错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuCallbackError,
    "QiniuCallbackError",
    PyRuntimeError,
    QiniuCallbackErrorInfo,
    anyhow::Error,
    "七牛回调异常"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuIsahcError,
    "QiniuIsahcError",
    PyRuntimeError,
    QiniuIsahcErrorInfo,
    qiniu_sdk::isahc::isahc::Error,
    "七牛 Isahc 异常"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuTrustDNSError,
    "QiniuTrustDNSError",
    PyRuntimeError,
    QiniuTrustDNSErrorKind,
    qiniu_sdk::http_client::trust_dns_resolver::error::ResolveError,
    "七牛 Isahc 异常"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidURLError,
    "QiniuInvalidURLError",
    PyValueError,
    QiniuInvalidURLErrorInfo,
    qiniu_sdk::http::uri::InvalidUri,
    "七牛非法 URL 错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidStatusCodeError,
    "QiniuInvalidStatusCodeError",
    PyValueError,
    QiniuInvalidStatusCodeErrorInfo,
    qiniu_sdk::http::InvalidStatusCode,
    "七牛非法 HTTP 状态码错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidMethodError,
    "QiniuInvalidMethodError",
    PyValueError,
    QiniuInvalidMethodErrorInfo,
    qiniu_sdk::http::InvalidMethod,
    "七牛非法 HTTP 方法错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidHeaderNameError,
    "QiniuInvalidHeaderNameError",
    PyValueError,
    QiniuInvalidHeaderNameErrorInfo,
    qiniu_sdk::http::InvalidHeaderName,
    "七牛非法 HTTP 头名称错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidHeaderValueError,
    "QiniuInvalidHeaderValueError",
    PyValueError,
    QiniuInvalidHeaderValueErrorInfo,
    qiniu_sdk::http::InvalidHeaderValue,
    "七牛非法 HTTP 头值错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuHeaderValueEncodingError,
    "QiniuHeaderValueEncodingError",
    PyValueError,
    QiniuHeaderValueEncodingErrorInfo,
    qiniu_sdk::http::header::ToStrError,
    "七牛 HTTP 头编码错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidIpAddrError,
    "QiniuInvalidIpAddrError",
    PyValueError,
    QiniuInvalidIpAddrErrorInfo,
    std::net::AddrParseError,
    "七牛非法 IP 地址错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidDomainWithPortError,
    "QiniuInvalidDomainWithPortError",
    PyValueError,
    QiniuInvalidDomainWithPortErrorInfo,
    qiniu_sdk::http_client::DomainWithPortParseError,
    "七牛非法域名错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidIpAddrWithPortError,
    "QiniuInvalidIpAddrWithPortError",
    PyValueError,
    QiniuInvalidIpAddrWithPortErrorInfo,
    qiniu_sdk::http_client::IpAddrWithPortParseError,
    "七牛非法 IP 地址错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidEndpointError,
    "QiniuInvalidEndpointError",
    PyValueError,
    QiniuInvalidEndpointErrorInfo,
    qiniu_sdk::http_client::EndpointParseError,
    "七牛非法终端地址错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuJsonError,
    "QiniuJsonError",
    PyValueError,
    QiniuJsonErrorInfo,
    serde_json::Error,
    "七牛 JSON 错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuTimeError,
    "QiniuTimeError",
    PyValueError,
    QiniuTimeErrorInfo,
    SystemTimeError,
    "七牛时间错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuBase64Error,
    "QiniuBase64Error",
    PyValueError,
    QiniuBase64ErrorInfo,
    qiniu_sdk::utils::base64::DecodeError,
    "七牛 Base64 解析错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuMimeParseError,
    "QiniuMimeParseError",
    PyValueError,
    QiniuMimeParseErrorInfo,
    qiniu_sdk::http_client::mime::FromStrError,
    "七牛 MIME 解析错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuUploadTokenFormatError,
    "QiniuUploadTokenFormatError",
    PyValueError,
    QiniuUploadTokenFormatErrorInfo,
    qiniu_sdk::upload_token::ParseError,
    "七牛上传凭证格式错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuIoError,
    "QiniuIoError",
    PyIOError,
    QiniuIoErrorInfo,
    IoError,
    "七牛本地 IO 错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuHttpCallError,
    "QiniuHttpCallError",
    PyIOError,
    QiniuHttpCallErrorInfo,
    qiniu_sdk::http::ResponseError,
    "七牛 HTTP 调用错误"
);

create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuApiCallError,
    "QiniuApiCallError",
    PyIOError,
    QiniuApiCallErrorInfo,
    qiniu_sdk::http_client::ResponseError,
    "七牛 API 调用错误"
);
create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuAuthorizationError,
    "QiniuAuthorizationError",
    PyIOError,
    QiniuAuthorizationErrorInfo,
    qiniu_sdk::http_client::AuthorizationError,
    "七牛签名异常"
);

create_exception_with_info!(
    qiniu_sdk_bindings,
    QiniuInvalidPrefixLengthError,
    "QiniuInvalidPrefixLengthError",
    PyValueError,
    QiniuInvalidPrefixLengthErrorInfo,
    qiniu_sdk::http_client::PrefixLenError,
    "七牛子网掩码前缀长度异常"
);
