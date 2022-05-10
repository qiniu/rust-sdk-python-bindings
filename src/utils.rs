use super::{
    exceptions::{
        QiniuBodySizeMissingError, QiniuInvalidEndpointError, QiniuInvalidHeaderNameError,
        QiniuInvalidHeaderValueError, QiniuInvalidIpAddrError, QiniuInvalidMethodError,
        QiniuInvalidPortError, QiniuInvalidStatusCodeError, QiniuInvalidURLError,
    },
    http_client::Endpoint,
};
use futures::{io::Cursor, ready, AsyncRead, AsyncSeek, FutureExt};
use pyo3::{prelude::*, types::PyTuple};
use qiniu_sdk::{
    http::{
        header::ToStrError, AsyncRequestBody, AsyncResponseBody, HeaderMap, HeaderName,
        HeaderValue, Method, Metrics, StatusCode, SyncRequestBody, SyncResponseBody, Uri, Version,
    },
    http_client::EndpointParseError,
};
use smart_default::SmartDefault;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    future::Future,
    io::{
        Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Seek, SeekFrom, Write,
    },
    net::IpAddr,
    num::NonZeroU16,
    pin::Pin,
    task::{Context, Poll},
};

const READ: &str = "read";
const SEEK: &str = "seek";
const WRITE: &str = "write";
const FLUSH: &str = "flush";

#[derive(Debug)]
pub(super) struct PythonIoBase {
    io_base: PyObject,
}

impl PythonIoBase {
    pub(super) fn new(io_base: PyObject) -> Self {
        Self { io_base }
    }

    pub(super) fn into_async_read(self) -> PythonIoBaseAsyncRead {
        PythonIoBaseAsyncRead::new(self)
    }

    fn _read(&mut self, buf: &mut [u8]) -> PyResult<usize> {
        Python::with_gil(|py| {
            let args = PyTuple::new(py, [buf.len()]);
            let retval = self.io_base.call_method1(py, READ, args)?;
            let bytes = extract_bytes_from_py_object(py, retval)?;
            buf[..bytes.len()].copy_from_slice(&bytes);
            Ok(bytes.len())
        })
    }

    fn _seek(&mut self, seek_from: SeekFrom) -> PyResult<u64> {
        let (offset, whence) = split_seek_from(seek_from);
        Python::with_gil(|py| {
            let args = PyTuple::new(py, [offset, whence]);
            let retval = self.io_base.call_method1(py, SEEK, args)?;
            retval.extract::<u64>(py)
        })
    }

    fn _write(&mut self, buf: &[u8]) -> PyResult<usize> {
        Python::with_gil(|py| {
            let args = PyTuple::new(py, [buf]);
            self.io_base
                .call_method1(py, WRITE, args)?
                .extract::<usize>(py)
        })
    }

    fn _flush(&mut self) -> PyResult<()> {
        Python::with_gil(|py| {
            self.io_base.call_method0(py, FLUSH)?;
            Ok(())
        })
    }
}

impl Read for PythonIoBase {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self._read(buf).map_err(make_io_error_from_py_err)
    }
}

impl Seek for PythonIoBase {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        self._seek(pos).map_err(make_io_error_from_py_err)
    }
}

impl Write for PythonIoBase {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self._write(buf).map_err(make_io_error_from_py_err)
    }

    fn flush(&mut self) -> IoResult<()> {
        self._flush().map_err(make_io_error_from_py_err)
    }
}

#[derive(Debug)]
pub(super) struct PythonIoBaseAsyncRead {
    base: PythonIoBase,
    step: AsyncStep,
}

#[derive(SmartDefault, Debug)]
enum AsyncStep {
    #[default]
    Free,
    Reading(AsyncReadStep),
    Seeking(AsyncSeekStep),
}

type SyncBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'a>>;

#[derive(SmartDefault)]
enum AsyncReadStep {
    Waiting(SyncBoxFuture<'static, IoResult<Vec<u8>>>),

    #[default]
    Buffered(Cursor<Vec<u8>>),

    Done,
}

enum AsyncSeekStep {
    Waiting(SyncBoxFuture<'static, IoResult<u64>>),
}

impl PythonIoBaseAsyncRead {
    fn new(base: PythonIoBase) -> Self {
        Self {
            base,
            step: Default::default(),
        }
    }
}

impl AsyncRead for PythonIoBaseAsyncRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        match &mut self.step {
            AsyncStep::Reading(AsyncReadStep::Waiting(fut)) => match ready!(fut.poll_unpin(cx)) {
                Ok(buffered) if buffered.is_empty() => {
                    self.step = AsyncStep::Reading(AsyncReadStep::Done);
                    Poll::Ready(Ok(0))
                }
                Ok(buffered) => {
                    self.step = AsyncStep::Reading(AsyncReadStep::Buffered(Cursor::new(buffered)));
                    self.poll_read(cx, buf)
                }
                Err(err) => {
                    self.step = Default::default();
                    Poll::Ready(Err(err))
                }
            },
            AsyncStep::Reading(AsyncReadStep::Buffered(buffered)) => {
                match ready!(Pin::new(buffered).poll_read(cx, buf)) {
                    Ok(0) => {
                        let io_base = self.base.io_base.to_owned();
                        self.step =
                            AsyncStep::Reading(AsyncReadStep::Waiting(Box::pin(async move {
                                let retval = Python::with_gil(|py| {
                                    pyo3_asyncio::async_std::into_future(
                                        io_base
                                            .call_method1(py, READ, PyTuple::new(py, [1 << 20]))?
                                            .as_ref(py),
                                    )
                                })?
                                .await?;
                                Python::with_gil(|py| extract_bytes_from_py_object(py, retval))
                                    .map_err(make_io_error_from_py_err)
                            })));
                        self.poll_read(cx, buf)
                    }
                    result => Poll::Ready(result),
                }
            }
            AsyncStep::Reading(AsyncReadStep::Done) => Poll::Ready(Ok(0)),
            AsyncStep::Free => {
                self.step = AsyncStep::Reading(Default::default());
                self.poll_read(cx, buf)
            }
            AsyncStep::Seeking(AsyncSeekStep::Waiting { .. }) => unreachable!(),
        }
    }
}

impl AsyncSeek for PythonIoBaseAsyncRead {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<IoResult<u64>> {
        match &mut self.step {
            AsyncStep::Free | AsyncStep::Reading(AsyncReadStep::Done) => {
                let io_base = self.base.io_base.to_owned();
                let (offset, whence) = split_seek_from(pos);
                self.step = AsyncStep::Seeking(AsyncSeekStep::Waiting(Box::pin(async move {
                    let retval = Python::with_gil(|py| {
                        pyo3_asyncio::async_std::into_future(
                            io_base
                                .call_method1(py, SEEK, PyTuple::new(py, [offset, whence]))?
                                .as_ref(py),
                        )
                    })?
                    .await?;
                    Python::with_gil(|py| retval.extract::<u64>(py))
                        .map_err(make_io_error_from_py_err)
                })));
                self.poll_seek(cx, pos)
            }
            AsyncStep::Seeking(AsyncSeekStep::Waiting(fut)) => {
                let result = ready!(fut.poll_unpin(cx));
                self.step = AsyncStep::Free;
                Poll::Ready(result)
            }
            AsyncStep::Reading { .. } => unreachable!(),
        }
    }
}

impl Debug for AsyncReadStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Waiting { .. } => f.debug_tuple("Waiting").finish(),
            Self::Buffered(cursor) => f.debug_tuple("Buffered").field(cursor).finish(),
            Self::Done => f.debug_tuple("Done").finish(),
        }
    }
}

impl Debug for AsyncSeekStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Waiting { .. } => f.debug_tuple("Waiting").finish(),
        }
    }
}

fn extract_bytes_from_py_object(py: Python<'_>, obj: PyObject) -> PyResult<Vec<u8>> {
    let bytes = if let Ok(str) = obj.extract::<String>(py) {
        str.into_bytes()
    } else {
        obj.extract::<Vec<u8>>(py)?
    };
    Ok(bytes)
}

fn make_io_error_from_py_err(err: PyErr) -> IoError {
    IoError::new(IoErrorKind::Other, err)
}

pub(super) fn extract_uri(url: &PyAny) -> PyResult<Uri> {
    parse_uri(url.extract::<&str>()?)
}

pub(super) fn parse_uri(url: &str) -> PyResult<Uri> {
    let url = url
        .parse::<Uri>()
        .map_err(|err| QiniuInvalidURLError::new_err(err.to_string()))?;
    Ok(url)
}

pub(super) fn extract_method(method: &PyAny) -> PyResult<Method> {
    parse_method(method.extract::<&str>()?)
}

pub(super) fn parse_method(method: &str) -> PyResult<Method> {
    let method = method
        .parse::<Method>()
        .map_err(|err| QiniuInvalidMethodError::new_err(err.to_string()))?;
    Ok(method)
}

pub(super) fn extract_version(version: &PyAny) -> PyResult<Version> {
    Ok(version.extract::<super::http::Version>()?.into())
}

pub(super) fn extract_headers(headers: &PyAny) -> PyResult<HeaderMap> {
    parse_headers(headers.extract::<HashMap<String, String>>()?)
}

pub(super) fn parse_headers(headers: HashMap<String, String>) -> PyResult<HeaderMap> {
    headers
        .into_iter()
        .map(|(name, value)| {
            let name = name
                .parse::<HeaderName>()
                .map_err(|err| QiniuInvalidHeaderNameError::new_err(err.to_string()))?;
            let value = value
                .parse::<HeaderValue>()
                .map_err(|err| QiniuInvalidHeaderValueError::new_err(err.to_string()))?;
            Ok((name, value))
        })
        .collect()
}

pub(super) fn convert_headers_to_hashmap(headers: &HeaderMap) -> PyResult<HashMap<String, String>> {
    headers
        .iter()
        .map(|(name, value)| {
            value
                .to_str()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect::<Result<_, ToStrError>>()
        .map_err(|err| QiniuInvalidHeaderValueError::new_err(err.to_string()))
}

pub(super) fn parse_header_value(header_value: Option<&str>) -> PyResult<Option<HeaderValue>> {
    if let Some(header_value) = header_value {
        let header_value = header_value
            .parse::<HeaderValue>()
            .map_err(|err| QiniuInvalidHeaderValueError::new_err(err.to_string()))?;
        Ok(Some(header_value))
    } else {
        Ok(None)
    }
}

pub(super) fn extract_ip_addrs(ip_addrs: &PyAny) -> PyResult<Vec<IpAddr>> {
    parse_ip_addrs(ip_addrs.extract::<Vec<String>>()?)
}

pub(super) fn parse_ip_addrs(ip_addrs: Vec<String>) -> PyResult<Vec<IpAddr>> {
    ip_addrs
        .into_iter()
        .map(|ip_addr| {
            ip_addr
                .parse::<IpAddr>()
                .map_err(|err| QiniuInvalidIpAddrError::new_err(err.to_string()))
        })
        .collect()
}

pub(super) fn extract_port(port: &PyAny) -> PyResult<NonZeroU16> {
    parse_port(port.extract::<u16>()?)
}

pub(super) fn parse_port(port: u16) -> PyResult<NonZeroU16> {
    if let Some(port) = NonZeroU16::new(port) {
        Ok(port)
    } else {
        Err(QiniuInvalidPortError::new_err("Invalid port"))
    }
}

pub(super) fn extract_sync_request_body(
    body: PyObject,
    body_len: Option<PyObject>,
    py: Python<'_>,
) -> PyResult<SyncRequestBody<'static>> {
    if let Ok(body) = body.extract::<String>(py) {
        Ok(SyncRequestBody::from(body))
    } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
        Ok(SyncRequestBody::from(body))
    } else if let Some(body_len) = body_len {
        Ok(SyncRequestBody::from_reader(
            PythonIoBase::new(body),
            body_len.extract::<u64>(py)?,
        ))
    } else {
        Err(QiniuBodySizeMissingError::new_err("`body` must be passed"))
    }
}

pub(super) fn extract_async_request_body(
    body: PyObject,
    body_len: Option<PyObject>,
    py: Python<'_>,
) -> PyResult<AsyncRequestBody<'static>> {
    if let Ok(body) = body.extract::<String>(py) {
        Ok(AsyncRequestBody::from(body))
    } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
        Ok(AsyncRequestBody::from(body))
    } else if let Some(body_len) = body_len {
        Ok(AsyncRequestBody::from_reader(
            PythonIoBase::new(body).into_async_read(),
            body_len.extract::<u64>(py)?,
        ))
    } else {
        Err(QiniuBodySizeMissingError::new_err("`body` must be passed"))
    }
}

pub(super) fn extract_sync_response_body(body: PyObject, py: Python<'_>) -> SyncResponseBody {
    if let Ok(body) = body.extract::<String>(py) {
        SyncResponseBody::from_bytes(body.into_bytes())
    } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
        SyncResponseBody::from_bytes(body)
    } else {
        SyncResponseBody::from_reader(PythonIoBase::new(body))
    }
}

pub(super) fn extract_async_response_body(body: PyObject, py: Python<'_>) -> AsyncResponseBody {
    if let Ok(body) = body.extract::<String>(py) {
        AsyncResponseBody::from_bytes(body.into_bytes())
    } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
        AsyncResponseBody::from_bytes(body)
    } else {
        AsyncResponseBody::from_reader(PythonIoBase::new(body).into_async_read())
    }
}

pub(super) fn extract_status_code(status_code: &PyAny) -> PyResult<StatusCode> {
    parse_status_code(status_code.extract::<u16>()?)
}

pub(super) fn parse_status_code(status_code: u16) -> PyResult<StatusCode> {
    StatusCode::from_u16(status_code)
        .map_err(|err| QiniuInvalidStatusCodeError::new_err(err.to_string()))
}

pub(super) fn extract_ip_addr(ip_addr: &PyAny) -> PyResult<IpAddr> {
    parse_ip_addr(ip_addr.extract::<&str>()?)
}

pub(super) fn parse_ip_addr(ip_addr: &str) -> PyResult<IpAddr> {
    ip_addr
        .parse::<IpAddr>()
        .map_err(|err| QiniuInvalidIpAddrError::new_err(err.to_string()))
}

pub(super) fn extract_metrics(metrics: &PyAny) -> PyResult<Metrics> {
    metrics
        .extract::<super::http::Metrics>()
        .map(|m| m.into_inner())
}

pub(super) fn extract_endpoints(
    endpoints: &PyAny,
) -> PyResult<Vec<qiniu_sdk::http_client::Endpoint>> {
    endpoints
        .extract::<Vec<&PyAny>>()?
        .into_iter()
        .map(extract_endpoint)
        .collect()
}

pub(super) fn extract_endpoint(endpoint: &PyAny) -> PyResult<qiniu_sdk::http_client::Endpoint> {
    if let Ok((domain_or_ip_addr, port)) = endpoint.extract::<(&str, u16)>() {
        format!("{}:{}", domain_or_ip_addr, port)
            .parse()
            .map_err(|err: EndpointParseError| QiniuInvalidEndpointError::new_err(err.to_string()))
    } else if let Ok(domain_or_ip_addr) = endpoint.extract::<&str>() {
        domain_or_ip_addr
            .parse()
            .map_err(|err: EndpointParseError| QiniuInvalidEndpointError::new_err(err.to_string()))
    } else {
        Ok(endpoint.extract::<Endpoint>()?.into_inner())
    }
}

fn split_seek_from(seek_from: SeekFrom) -> (i64, i64) {
    match seek_from {
        SeekFrom::Start(offset) => (offset as i64, 0),
        SeekFrom::Current(offset) => (offset, 1),
        SeekFrom::End(offset) => (offset as i64, 2),
    }
}
