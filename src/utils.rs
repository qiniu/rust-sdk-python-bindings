use futures::{future::BoxFuture, io::Cursor, ready, AsyncRead, FutureExt};
use pyo3::{prelude::*, types::PyTuple};
use smart_default::SmartDefault;
use std::{
    fmt::{self, Debug},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write},
    pin::Pin,
    task::{Context, Poll},
};

const READ: &str = "read";
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
    step: AsyncReadStep,
}

#[derive(SmartDefault)]
enum AsyncReadStep {
    Waiting(BoxFuture<'static, IoResult<Vec<u8>>>),

    #[default]
    Buffered(Cursor<Vec<u8>>),

    Done,
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
            AsyncReadStep::Waiting(fut) => match ready!(fut.poll_unpin(cx)) {
                Ok(buffered) if buffered.is_empty() => {
                    self.step = AsyncReadStep::Done;
                    Poll::Ready(Ok(0))
                }
                Ok(buffered) => {
                    self.step = AsyncReadStep::Buffered(Cursor::new(buffered));
                    self.poll_read(cx, buf)
                }
                Err(err) => {
                    self.step = Default::default();
                    Poll::Ready(Err(err))
                }
            },
            AsyncReadStep::Buffered(buffered) => {
                match ready!(Pin::new(buffered).poll_read(cx, buf)) {
                    Ok(0) => {
                        let io_base = self.base.io_base.to_owned();
                        self.step = AsyncReadStep::Waiting(Box::pin(async move {
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
                        }));
                        self.poll_read(cx, buf)
                    }
                    result => Poll::Ready(result),
                }
            }
            AsyncReadStep::Done => Poll::Ready(Ok(0)),
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
