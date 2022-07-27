use super::{
    exceptions::{
        QiniuApiCallErrorInfo, QiniuBodySizeMissingError, QiniuHeaderValueEncodingError,
        QiniuInvalidDomainWithPortError, QiniuInvalidEndpointError, QiniuInvalidHeaderNameError,
        QiniuInvalidHeaderValueError, QiniuInvalidIpAddrError, QiniuInvalidIpAddrWithPortError,
        QiniuInvalidMethodError, QiniuInvalidPortError, QiniuInvalidStatusCodeError,
        QiniuInvalidURLError, QiniuMimeParseError, QiniuUnsupportedTypeError,
    },
    http_client::{Endpoint, EndpointsProvider, RegionsProvider},
};
use futures::{
    channel::{
        mpsc::{
            unbounded, UnboundedReceiver as MpscUnboundedReceiver,
            UnboundedSender as MpscUnboundedSender,
        },
        oneshot::{channel, Sender as OneShotSender},
    },
    future::{select, Either},
    io::Cursor,
    lock::Mutex as AsyncMutex,
    pin_mut, ready, AsyncRead, AsyncSeek, AsyncWrite, FutureExt, SinkExt, StreamExt,
};
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyTuple},
};
use qiniu_sdk::{
    http::{
        header::ToStrError, AsyncRequestBody, AsyncResponseBody, HeaderMap, HeaderName,
        HeaderValue, Method, StatusCode, SyncRequestBody, SyncResponseBody, Uri,
    },
    http_client::{DomainWithPort, IpAddrWithPort},
};
use serde_json::Map;
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
    sync::Arc,
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

    pub(super) fn into_async_write(self) -> PythonIoBaseAsyncWrite {
        PythonIoBaseAsyncWrite::new(self)
    }

    pub(super) fn into_async_read_with_local_agent(
        self,
    ) -> (PythonIoBaseAsyncRead, RemotePyCallLocalAgent) {
        PythonIoBaseAsyncRead::new_with_local_agent(self)
    }

    fn _read(&mut self, buf: &mut [u8]) -> PyResult<usize> {
        Python::with_gil(|py| {
            let retval = self.io_base.call_method1(py, READ, (buf.len(),))?;
            let bytes = extract_bytes_from_py_object(py, retval)?;
            buf[..bytes.len()].copy_from_slice(&bytes);
            Ok(bytes.len())
        })
    }

    fn _seek(&mut self, seek_from: SeekFrom) -> PyResult<u64> {
        let (offset, whence) = split_seek_from(seek_from);
        Python::with_gil(|py| {
            let retval = self.io_base.call_method1(py, SEEK, (offset, whence))?;
            retval.extract::<u64>(py)
        })
    }

    fn _write(&mut self, buf: &[u8]) -> PyResult<usize> {
        Python::with_gil(|py| {
            self.io_base
                .call_method1(py, WRITE, (buf,))?
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

trait PythonCaller: Send + Sync + Debug {
    fn call_python_method(
        &self,
        object: PyObject,
        method: &str,
        args: Option<&PyTuple>,
        py: Python<'_>,
    ) -> PyResult<Pin<Box<dyn Future<Output = PyResult<PyObject>> + Send + Sync + 'static>>>;
}

#[derive(Debug)]
struct DirectPyCall;

impl PythonCaller for DirectPyCall {
    fn call_python_method(
        &self,
        object: PyObject,
        method: &str,
        args: Option<&PyTuple>,
        py: Python<'_>,
    ) -> PyResult<Pin<Box<dyn Future<Output = PyResult<PyObject>> + Send + Sync + 'static>>> {
        let result = if let Some(args) = args {
            object.call_method1(py, method, args)
        } else {
            object.call_method0(py, method)
        };
        pyo3_asyncio::async_std::into_future(result?.as_ref(py)).map(|fut| {
            Box::pin(fut)
                as Pin<Box<dyn Future<Output = PyResult<PyObject>> + Send + Sync + 'static>>
        })
    }
}

#[derive(Debug)]
struct RemotePyCall {
    sender: Arc<AsyncMutex<MpscUnboundedSender<RemotePyCallArgs>>>,
}

struct RemotePyCallArgs {
    object: PyObject,
    method: String,
    args: Option<Py<PyTuple>>,
    sender: OneShotSender<PyResult<PyObject>>,
}

impl PythonCaller for RemotePyCall {
    fn call_python_method(
        &self,
        object: PyObject,
        method: &str,
        args: Option<&PyTuple>,
        py: Python<'_>,
    ) -> PyResult<Pin<Box<dyn Future<Output = PyResult<PyObject>> + Send + Sync + 'static>>> {
        let sender = self.sender.to_owned();
        let method = method.to_owned();
        let args = args.map(|args| args.into_py(py));
        Ok(Box::pin(async move {
            let (s, r) = channel();
            sender
                .lock()
                .await
                .send(RemotePyCallArgs {
                    object,
                    method,
                    args,
                    sender: s,
                })
                .await
                .unwrap();
            r.await.unwrap()
        }))
    }
}

pub(super) struct RemotePyCallLocalAgent {
    receiver: MpscUnboundedReceiver<RemotePyCallArgs>,
}

impl RemotePyCallLocalAgent {
    pub(super) async fn run<T>(&mut self, should_stop: impl Future<Output = T>) -> PyResult<T> {
        let direct_call = DirectPyCall;
        pin_mut!(should_stop);
        loop {
            let next_future = self.receiver.next();
            pin_mut!(next_future);
            match select(should_stop, next_future).await {
                Either::Right((Some(args), st)) => {
                    should_stop = st;
                    let retval = Python::with_gil(|py| {
                        direct_call.call_python_method(
                            args.object,
                            args.method.as_str(),
                            args.args.as_ref().map(|args| args.as_ref(py)),
                            py,
                        )
                    })?
                    .await;
                    args.sender.send(retval).ok();
                }
                Either::Right((None, _)) => {
                    unreachable!("receiver should always return something")
                }
                Either::Left((retval, _)) => return Ok(retval),
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct PythonIoBaseAsyncRead {
    base: PythonIoBase,
    step: PythonIoBaseAsyncReadStep,
    py_caller: Arc<dyn PythonCaller>,
}

#[derive(SmartDefault, Debug)]
enum PythonIoBaseAsyncReadStep {
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
            py_caller: Arc::new(DirectPyCall),
        }
    }

    fn new_with_local_agent(base: PythonIoBase) -> (Self, RemotePyCallLocalAgent) {
        let (sender, receiver) = unbounded();
        let new = Self {
            base,
            step: Default::default(),
            py_caller: Arc::new(RemotePyCall {
                sender: Arc::new(AsyncMutex::new(sender)),
            }),
        };
        (new, RemotePyCallLocalAgent { receiver })
    }
}

impl AsyncRead for PythonIoBaseAsyncRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        match &mut self.step {
            PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Waiting(fut)) => {
                match ready!(fut.poll_unpin(cx)) {
                    Ok(buffered) if buffered.is_empty() => {
                        self.step = PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Done);
                        Poll::Ready(Ok(0))
                    }
                    Ok(buffered) => {
                        self.step = PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Buffered(
                            Cursor::new(buffered),
                        ));
                        self.poll_read(cx, buf)
                    }
                    Err(err) => {
                        self.step = Default::default();
                        Poll::Ready(Err(err))
                    }
                }
            }
            PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Buffered(buffered)) => {
                match ready!(Pin::new(buffered).poll_read(cx, buf)) {
                    Ok(0) => {
                        let io_base = Python::with_gil(|py| self.base.io_base.clone_ref(py));
                        let py_caller = self.py_caller.to_owned();
                        self.step = PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Waiting(
                            Box::pin(async move {
                                let retval = Python::with_gil(|py| {
                                    py_caller.call_python_method(
                                        io_base,
                                        READ,
                                        Some(PyTuple::new(py, [1 << 20])),
                                        py,
                                    )
                                })?
                                .await?;
                                Python::with_gil(|py| extract_bytes_from_py_object(py, retval))
                                    .map_err(make_io_error_from_py_err)
                            }),
                        ));
                        self.poll_read(cx, buf)
                    }
                    result => Poll::Ready(result),
                }
            }
            PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Done) => Poll::Ready(Ok(0)),
            PythonIoBaseAsyncReadStep::Free => {
                self.step = PythonIoBaseAsyncReadStep::Reading(Default::default());
                self.poll_read(cx, buf)
            }
            PythonIoBaseAsyncReadStep::Seeking(AsyncSeekStep::Waiting { .. }) => unreachable!(),
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
            PythonIoBaseAsyncReadStep::Free
            | PythonIoBaseAsyncReadStep::Reading(AsyncReadStep::Done) => {
                let io_base = Python::with_gil(|py| self.base.io_base.clone_ref(py));
                let py_caller = self.py_caller.to_owned();
                let (offset, whence) = split_seek_from(pos);
                self.step = PythonIoBaseAsyncReadStep::Seeking(AsyncSeekStep::Waiting(Box::pin(
                    async move {
                        let retval = Python::with_gil(|py| {
                            py_caller.call_python_method(
                                io_base,
                                SEEK,
                                Some(PyTuple::new(py, [offset, whence])),
                                py,
                            )
                        })?
                        .await?;
                        Python::with_gil(|py| retval.extract::<u64>(py))
                            .map_err(make_io_error_from_py_err)
                    },
                )));
                self.poll_seek(cx, pos)
            }
            PythonIoBaseAsyncReadStep::Seeking(AsyncSeekStep::Waiting(fut)) => {
                let result = ready!(fut.poll_unpin(cx));
                self.step = PythonIoBaseAsyncReadStep::Free;
                Poll::Ready(result)
            }
            PythonIoBaseAsyncReadStep::Reading { .. } => unreachable!(),
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

#[derive(Debug)]
pub(super) struct PythonIoBaseAsyncWrite {
    base: PythonIoBase,
    step: AsyncWriteStep,
    py_caller: Arc<dyn PythonCaller>,
}

#[derive(SmartDefault)]
enum AsyncWriteStep {
    WaitingForWriting(SyncBoxFuture<'static, IoResult<usize>>),
    WaitingForFlushing(SyncBoxFuture<'static, IoResult<()>>),
    WaitingForClosing(SyncBoxFuture<'static, IoResult<()>>),
    #[default]
    Done,
}

impl AsyncWrite for PythonIoBaseAsyncWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        match &mut self.step {
            AsyncWriteStep::WaitingForWriting(fut) => match ready!(fut.poll_unpin(cx)) {
                Ok(have_written) => {
                    self.step = AsyncWriteStep::Done;
                    Poll::Ready(Ok(have_written))
                }
                Err(err) => {
                    self.step = AsyncWriteStep::Done;
                    Poll::Ready(Err(err))
                }
            },
            AsyncWriteStep::Done => {
                let io_base = Python::with_gil(|py| self.base.io_base.clone_ref(py));
                let py_caller = self.py_caller.to_owned();
                let bytes: Py<PyBytes> = Python::with_gil(|py| PyBytes::new(py, buf).into_py(py));
                self.step = AsyncWriteStep::WaitingForWriting(Box::pin(async move {
                    let retval = Python::with_gil(|py| {
                        py_caller.call_python_method(
                            io_base,
                            WRITE,
                            Some(PyTuple::new(py, [bytes])),
                            py,
                        )
                    })?
                    .await?;
                    Python::with_gil(|py| retval.extract::<usize>(py))
                        .map_err(make_io_error_from_py_err)
                }));
                self.poll_write(cx, buf)
            }
            _ => unreachable!(),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        match &mut self.step {
            AsyncWriteStep::WaitingForFlushing(fut) => match ready!(fut.poll_unpin(cx)) {
                Ok(()) => {
                    self.step = AsyncWriteStep::Done;
                    Poll::Ready(Ok(()))
                }
                Err(err) => {
                    self.step = AsyncWriteStep::Done;
                    Poll::Ready(Err(err))
                }
            },
            AsyncWriteStep::Done => {
                let io_base = Python::with_gil(|py| self.base.io_base.clone_ref(py));
                let py_caller = self.py_caller.to_owned();
                self.step = AsyncWriteStep::WaitingForFlushing(Box::pin(async move {
                    Python::with_gil(|py| py_caller.call_python_method(io_base, FLUSH, None, py))?
                        .await?;
                    Ok(())
                }));
                self.poll_flush(cx)
            }
            _ => unreachable!(),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        match &mut self.step {
            AsyncWriteStep::WaitingForClosing(fut) => match ready!(fut.poll_unpin(cx)) {
                Ok(()) => {
                    self.step = AsyncWriteStep::Done;
                    Poll::Ready(Ok(()))
                }
                Err(err) => {
                    self.step = AsyncWriteStep::Done;
                    Poll::Ready(Err(err))
                }
            },
            AsyncWriteStep::Done => {
                let io_base = Python::with_gil(|py| self.base.io_base.clone_ref(py));
                let py_caller = self.py_caller.to_owned();
                self.step = AsyncWriteStep::WaitingForClosing(Box::pin(async move {
                    Python::with_gil(|py| py_caller.call_python_method(io_base, FLUSH, None, py))?
                        .await?;
                    Ok(())
                }));
                self.poll_flush(cx)
            }
            _ => unreachable!(),
        }
    }
}

impl Debug for AsyncWriteStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WaitingForWriting(_) => f.debug_tuple("WaitingForWriting").finish(),
            Self::WaitingForFlushing(_) => f.debug_tuple("WaitingForFlushing").finish(),
            Self::WaitingForClosing(_) => f.debug_tuple("WaitingForClosing").finish(),
            Self::Done => f.debug_tuple("Done").finish(),
        }
    }
}

impl PythonIoBaseAsyncWrite {
    fn new(base: PythonIoBase) -> Self {
        Self {
            base,
            step: Default::default(),
            py_caller: Arc::new(DirectPyCall),
        }
    }
}

pub(super) fn parse_uri(url: &str) -> PyResult<Uri> {
    let url = url.parse::<Uri>().map_err(QiniuInvalidURLError::from_err)?;
    Ok(url)
}

pub(super) fn parse_method(method: &str) -> PyResult<Method> {
    let method = method
        .parse::<Method>()
        .map_err(QiniuInvalidMethodError::from_err)?;
    Ok(method)
}

pub(super) fn parse_query_pairs(
    pairs: PyObject,
) -> PyResult<Vec<qiniu_sdk::http_client::QueryPair<'static>>> {
    Python::with_gil(|py| {
        if let Ok(pairs) = pairs.extract::<HashMap<String, String>>(py) {
            Ok(pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect())
        } else {
            Ok(pairs
                .extract::<Vec<(String, String)>>(py)?
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect())
        }
    })
}

pub(super) fn parse_headers(headers: HashMap<String, String>) -> PyResult<HeaderMap> {
    headers
        .into_iter()
        .map(|(name, value)| {
            let name = parse_header_name(&name)?;
            let value = parse_header_value(&value)?;
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
        .map_err(QiniuHeaderValueEncodingError::from_err)
}

pub(super) fn parse_header_name(header_name: &str) -> PyResult<HeaderName> {
    header_name
        .parse::<HeaderName>()
        .map_err(QiniuInvalidHeaderNameError::from_err)
}

pub(super) fn parse_header_value(header_value: &str) -> PyResult<HeaderValue> {
    header_value
        .parse::<HeaderValue>()
        .map_err(QiniuInvalidHeaderValueError::from_err)
}

pub(super) fn parse_ip_addrs(ip_addrs: Vec<String>) -> PyResult<Vec<IpAddr>> {
    ip_addrs
        .into_iter()
        .map(|ip_addr| {
            ip_addr
                .parse::<IpAddr>()
                .map_err(QiniuInvalidIpAddrError::from_err)
        })
        .collect()
}

pub(super) fn parse_port(port: u16) -> PyResult<NonZeroU16> {
    if let Some(port) = NonZeroU16::new(port) {
        Ok(port)
    } else {
        Err(QiniuInvalidPortError::new_err("Invalid port"))
    }
}

pub(super) fn extract_sync_multipart(
    parts: HashMap<String, PyObject>,
) -> PyResult<qiniu_sdk::http_client::SyncMultipart<'static>> {
    Python::with_gil(|py| {
        let mut multipart = qiniu_sdk::http_client::SyncMultipart::new();
        for (field_name, part) in parts {
            let part = if let Ok((body, metadata)) = part.extract::<(PyObject, &PyDict)>(py) {
                extract_sync_part(body, Some(metadata), py)?
            } else {
                extract_sync_part(part, None, py)?
            };
            multipart = multipart.add_part(field_name, part);
        }
        Ok(multipart)
    })
}

fn extract_sync_part<'a>(
    body: PyObject,
    metadata: Option<&PyDict>,
    py: Python<'_>,
) -> PyResult<qiniu_sdk::http_client::SyncPart<'a>> {
    let metadata = metadata.map(extract_multipart_metadata).transpose()?;
    let mut part = if let Ok(text) = body.extract::<String>(py) {
        qiniu_sdk::http_client::SyncPart::text(text)
    } else if let Ok(bytes) = body.extract::<Vec<u8>>(py) {
        qiniu_sdk::http_client::SyncPart::bytes(bytes)
    } else {
        qiniu_sdk::http_client::SyncPart::stream(PythonIoBase::new(body))
    };
    if let Some(metadata) = metadata {
        part = part.metadata(metadata);
    }
    Ok(part)
}

pub(super) fn extract_async_multipart(
    parts: HashMap<String, PyObject>,
) -> PyResult<qiniu_sdk::http_client::AsyncMultipart<'static>> {
    Python::with_gil(|py| {
        let mut multipart = qiniu_sdk::http_client::AsyncMultipart::new();
        for (field_name, part) in parts {
            let part = if let Ok((body, metadata)) = part.extract::<(PyObject, &PyDict)>(py) {
                extract_async_part(body, Some(metadata), py)?
            } else {
                extract_async_part(part, None, py)?
            };
            multipart = multipart.add_part(field_name, part);
        }
        Ok(multipart)
    })
}

fn extract_async_part<'a>(
    body: PyObject,
    metadata: Option<&PyDict>,
    py: Python<'_>,
) -> PyResult<qiniu_sdk::http_client::AsyncPart<'a>> {
    let metadata = metadata.map(extract_multipart_metadata).transpose()?;
    let mut part = if let Ok(text) = body.extract::<String>(py) {
        qiniu_sdk::http_client::AsyncPart::text(text)
    } else if let Ok(bytes) = body.extract::<Vec<u8>>(py) {
        qiniu_sdk::http_client::AsyncPart::bytes(bytes)
    } else {
        qiniu_sdk::http_client::AsyncPart::stream(PythonIoBase::new(body).into_async_read())
    };
    if let Some(metadata) = metadata {
        part = part.metadata(metadata);
    }
    Ok(part)
}

fn extract_multipart_metadata(dict: &PyDict) -> PyResult<qiniu_sdk::http_client::PartMetadata> {
    let mut metadata = qiniu_sdk::http_client::PartMetadata::default();
    if let Some(mime) = dict.get_item("mime") {
        let mime = parse_mime(mime.extract::<&str>()?)?;
        metadata = metadata.mime(mime);
    }
    if let Some(headers) = dict.get_item("headers") {
        let headers = parse_headers(headers.extract::<HashMap<String, String>>()?)?;
        metadata.extend(headers);
    }
    if let Some(file_name) = dict.get_item("file_name") {
        let file_name = file_name.extract::<&str>()?;
        metadata = metadata.file_name(file_name);
    }
    Ok(metadata)
}

pub(super) fn extract_sync_request_body(
    body: PyObject,
    body_len: Option<u64>,
    py: Python<'_>,
) -> PyResult<SyncRequestBody<'static>> {
    if let Ok(body) = body.extract::<String>(py) {
        Ok(SyncRequestBody::from(body))
    } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
        Ok(SyncRequestBody::from(body))
    } else if let Some(body_len) = body_len {
        Ok(SyncRequestBody::from_reader(
            PythonIoBase::new(body),
            body_len,
        ))
    } else {
        Err(QiniuBodySizeMissingError::new_err(
            "`body_len` must be passed",
        ))
    }
}

pub(super) fn extract_async_request_body(
    body: PyObject,
    body_len: Option<u64>,
    py: Python<'_>,
) -> PyResult<(AsyncRequestBody<'static>, Option<RemotePyCallLocalAgent>)> {
    if let Ok(body) = body.extract::<String>(py) {
        Ok((AsyncRequestBody::from(body), None))
    } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
        Ok((AsyncRequestBody::from(body), None))
    } else if let Some(body_len) = body_len {
        let (body, agent) = PythonIoBase::new(body).into_async_read_with_local_agent();
        Ok((AsyncRequestBody::from_reader(body, body_len), Some(agent)))
    } else {
        Err(QiniuBodySizeMissingError::new_err(
            "`body_len` must be passed",
        ))
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

pub(super) fn parse_status_code(status_code: u16) -> PyResult<StatusCode> {
    StatusCode::from_u16(status_code).map_err(QiniuInvalidStatusCodeError::from_err)
}

pub(super) fn parse_ip_addr(ip_addr: &str) -> PyResult<IpAddr> {
    ip_addr
        .parse::<IpAddr>()
        .map_err(QiniuInvalidIpAddrError::from_err)
}

pub(super) fn parse_domain_with_port(ip_addr: &str) -> PyResult<DomainWithPort> {
    ip_addr
        .parse::<DomainWithPort>()
        .map_err(QiniuInvalidDomainWithPortError::from_err)
}

pub(super) fn extract_ip_addrs_with_port(ip_addrs: &[&str]) -> PyResult<Vec<IpAddrWithPort>> {
    ip_addrs
        .iter()
        .map(|&s| parse_ip_addr_with_port(s))
        .collect()
}

pub(super) fn parse_ip_addr_with_port(ip_addr: &str) -> PyResult<IpAddrWithPort> {
    ip_addr
        .parse::<IpAddrWithPort>()
        .map_err(QiniuInvalidIpAddrWithPortError::from_err)
}

pub(super) fn extract_endpoints(
    endpoints: Vec<&PyAny>,
) -> PyResult<Vec<qiniu_sdk::http_client::Endpoint>> {
    endpoints.into_iter().map(extract_endpoint).collect()
}

pub(super) fn extract_endpoint(endpoint: &PyAny) -> PyResult<qiniu_sdk::http_client::Endpoint> {
    if let Ok((domain_or_ip_addr, port)) = endpoint.extract::<(&str, u16)>() {
        format!("{}:{}", domain_or_ip_addr, port)
            .parse()
            .map_err(QiniuInvalidEndpointError::from_err)
    } else if let Ok(domain_or_ip_addr) = endpoint.extract::<&str>() {
        domain_or_ip_addr
            .parse()
            .map_err(QiniuInvalidEndpointError::from_err)
    } else {
        Ok(endpoint.extract::<Endpoint>()?.into())
    }
}

pub(super) fn extract_endpoints_provider(
    provider: &PyAny,
) -> PyResult<Box<dyn qiniu_sdk::http_client::EndpointsProvider>> {
    if let Ok(regions) = provider.extract::<RegionsProvider>() {
        Ok(Box::new(
            qiniu_sdk::http_client::RegionsProviderEndpoints::new(regions),
        ))
    } else {
        let endpoints = provider.extract::<EndpointsProvider>()?;
        Ok(Box::new(endpoints))
    }
}

pub(super) fn parse_mime(mime: &str) -> PyResult<qiniu_sdk::http_client::mime::Mime> {
    mime.parse::<qiniu_sdk::http_client::mime::Mime>()
        .map_err(QiniuMimeParseError::from_err)
}

pub(super) fn convert_py_any_to_json_value(any: PyObject) -> PyResult<serde_json::Value> {
    Python::with_gil(|py| {
        if let Ok(value) = any.extract::<String>(py) {
            Ok(serde_json::Value::from(value))
        } else if let Ok(value) = any.extract::<bool>(py) {
            Ok(serde_json::Value::from(value))
        } else if let Ok(value) = any.extract::<u64>(py) {
            Ok(serde_json::Value::from(value))
        } else if let Ok(value) = any.extract::<i64>(py) {
            Ok(serde_json::Value::from(value))
        } else if let Ok(value) = any.extract::<f64>(py) {
            Ok(serde_json::Value::from(value))
        } else if let Ok(values) = any.extract::<Vec<PyObject>>(py) {
            let values = values
                .into_iter()
                .map(convert_py_any_to_json_value)
                .collect::<PyResult<Vec<_>>>()?;
            Ok(serde_json::Value::from(values))
        } else if let Ok(values) = any.extract::<HashMap<String, PyObject>>(py) {
            let values = values
                .into_iter()
                .map(|(k, v)| convert_py_any_to_json_value(v).map(|v| (k, v)))
                .collect::<PyResult<Map<_, _>>>()?;
            Ok(serde_json::Value::from(values))
        } else {
            Err(QiniuUnsupportedTypeError::new_err(format!(
                "Unsupported type: {:?}",
                any
            )))
        }
    })
}

pub(super) fn convert_json_value_to_py_object(value: &serde_json::Value) -> PyResult<PyObject> {
    Python::with_gil(|py| match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Number(n) => {
            if let Some(n) = n.as_u64() {
                Ok(n.to_object(py))
            } else if let Some(n) = n.as_i64() {
                Ok(n.to_object(py))
            } else if let Some(n) = n.as_f64() {
                Ok(n.to_object(py))
            } else {
                Err(QiniuUnsupportedTypeError::new_err(format!(
                    "Unsupported number type: {:?}",
                    n
                )))
            }
        }
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        serde_json::Value::Array(array) => Ok(array
            .iter()
            .map(convert_json_value_to_py_object)
            .collect::<PyResult<Vec<_>>>()?
            .to_object(py)),
        serde_json::Value::Object(object) => Ok(object
            .into_iter()
            .map(|(k, v)| convert_json_value_to_py_object(v).map(|v| (k, v)))
            .collect::<PyResult<HashMap<_, _>>>()?
            .to_object(py)),
    })
}

fn split_seek_from(seek_from: SeekFrom) -> (i64, i64) {
    match seek_from {
        SeekFrom::Start(offset) => (offset as i64, 0),
        SeekFrom::Current(offset) => (offset, 1),
        SeekFrom::End(offset) => (offset as i64, 2),
    }
}

pub(super) fn convert_api_call_error(error: &PyErr) -> PyResult<QiniuApiCallErrorInfo> {
    Python::with_gil(|py| error.value(py).getattr("args")?.get_item(0i32)?.extract())
}
