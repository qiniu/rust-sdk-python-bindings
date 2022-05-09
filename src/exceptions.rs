use pyo3::{
    create_exception,
    exceptions::{PyException, PyIOError, PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
};

pub(super) fn register(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add(
        "QiniuUserAgentInitializeError",
        py.get_type::<QiniuUserAgentInitializeError>(),
    )?;
    m.add("QiniuCallbackError", py.get_type::<QiniuCallbackError>())?;
    m.add(
        "QiniuDataLockedError",
        py.get_type::<QiniuDataLockedError>(),
    )?;
    m.add("QiniuIsahcError", py.get_type::<QiniuIsahcError>())?;
    m.add("QiniuUnknownError", py.get_type::<QiniuUnknownError>())?;
    m.add(
        "QiniuInvalidURLError",
        py.get_type::<QiniuInvalidURLError>(),
    )?;
    m.add(
        "QiniuInvalidStatusCodeError",
        py.get_type::<QiniuInvalidStatusCodeError>(),
    )?;
    m.add(
        "QiniuInvalidHttpVersionError",
        py.get_type::<QiniuInvalidHttpVersionError>(),
    )?;
    m.add(
        "QiniuInvalidMethodError",
        py.get_type::<QiniuInvalidMethodError>(),
    )?;
    m.add(
        "QiniuInvalidHeaderNameError",
        py.get_type::<QiniuInvalidHeaderNameError>(),
    )?;
    m.add(
        "QiniuInvalidHeaderValueError",
        py.get_type::<QiniuInvalidHeaderValueError>(),
    )?;
    m.add(
        "QiniuInvalidIpAddrError",
        py.get_type::<QiniuInvalidIpAddrError>(),
    )?;
    m.add(
        "QiniuInvalidDomainWithPortError",
        py.get_type::<QiniuInvalidDomainWithPortError>(),
    )?;
    m.add(
        "QiniuInvalidIpAddrWithPortError",
        py.get_type::<QiniuInvalidIpAddrWithPortError>(),
    )?;
    m.add(
        "QiniuInvalidEndpointError",
        py.get_type::<QiniuInvalidEndpointError>(),
    )?;
    m.add(
        "QiniuInvalidPortError",
        py.get_type::<QiniuInvalidPortError>(),
    )?;
    m.add(
        "QiniuEmptyChainCredentialsProvider",
        py.get_type::<QiniuEmptyChainCredentialsProvider>(),
    )?;
    m.add("QiniuJsonError", py.get_type::<QiniuJsonError>())?;
    m.add("QiniuTimeError", py.get_type::<QiniuTimeError>())?;
    m.add("QiniuBase64Error", py.get_type::<QiniuBase64Error>())?;
    m.add(
        "QiniuUploadTokenFormatError",
        py.get_type::<QiniuUploadTokenFormatError>(),
    )?;
    m.add(
        "QiniuUnsupportedTypeError",
        py.get_type::<QiniuUnsupportedTypeError>(),
    )?;
    m.add("QiniuIoError", py.get_type::<QiniuIoError>())?;
    m.add("QiniuHttpCallError", py.get_type::<QiniuHttpCallError>())?;
    m.add(
        "QiniuBodySizeMissingError",
        py.get_type::<QiniuBodySizeMissingError>(),
    )?;
    Ok(())
}

create_exception!(
    qiniu_sdk_bindings,
    QiniuUserAgentInitializeError,
    PyRuntimeError,
    "七牛用户代理初始化异常"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuCallbackError,
    PyRuntimeError,
    "七牛回调异常"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuDataLockedError,
    PyRuntimeError,
    "七牛数据锁定异常，需要等待解锁后重试"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuIsahcError,
    PyRuntimeError,
    "七牛 Isahc 异常"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuUnknownError,
    PyException,
    "七牛未知异常"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidURLError,
    PyValueError,
    "七牛非法 URL 错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidStatusCodeError,
    PyValueError,
    "七牛非法 HTTP 状态码错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidHttpVersionError,
    PyValueError,
    "七牛非法 HTTP 版本错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidMethodError,
    PyValueError,
    "七牛非法 HTTP 方法错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidHeaderNameError,
    PyValueError,
    "七牛非法 HTTP 头名称错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidHeaderValueError,
    PyValueError,
    "七牛非法 HTTP 头值错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidIpAddrError,
    PyValueError,
    "七牛非法 IP 地址错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidDomainWithPortError,
    PyValueError,
    "七牛非法域名错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidIpAddrWithPortError,
    PyValueError,
    "七牛非法 IP 地址错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuInvalidEndpointError,
    PyValueError,
    "七牛非法终端地址错误"
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
    QiniuJsonError,
    PyValueError,
    "七牛 JSON 错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuTimeError,
    PyValueError,
    "七牛时间错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuBase64Error,
    PyValueError,
    "七牛 Base64 解析错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuUploadTokenFormatError,
    PyValueError,
    "七牛上传凭证格式错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuUnsupportedTypeError,
    PyValueError,
    "七牛不支持的类型错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuIoError,
    PyIOError,
    "七牛本地 IO 错误"
);
create_exception!(
    qiniu_sdk_bindings,
    QiniuHttpCallError,
    PyIOError,
    "七牛 HTTP 调用错误"
);
