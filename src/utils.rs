use pyo3::{prelude::*, types::PyTuple};
use std::{
    fmt::{self, Debug},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write},
};

pub(super) struct PythonIoBase<'p> {
    io_base: &'p PyAny,
    py: Python<'p>,
}

impl<'p> PythonIoBase<'p> {
    pub(super) fn new(io_base: &'p PyAny, py: Python<'p>) -> Self {
        Self { io_base, py }
    }

    fn _read(&mut self, buf: &mut [u8]) -> PyResult<usize> {
        let args = PyTuple::new(self.py, [buf.len()]);
        let retval = self.io_base.call_method1("read", args)?;
        let bytes = if let Ok(str) = retval.extract::<String>() {
            str.into_bytes()
        } else {
            retval.extract::<Vec<u8>>()?
        };
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }

    fn _write(&mut self, buf: &[u8]) -> PyResult<usize> {
        let args = PyTuple::new(self.py, [buf]);
        self.io_base.call_method1("write", args)?.extract::<usize>()
    }

    fn _flush(&mut self) -> PyResult<()> {
        self.io_base.call_method0("flush")?;
        Ok(())
    }
}

impl Read for PythonIoBase<'_> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self._read(buf)
            .map_err(|err| IoError::new(IoErrorKind::Other, err))
    }
}

impl Write for PythonIoBase<'_> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self._write(buf)
            .map_err(|err| IoError::new(IoErrorKind::Other, err))
    }

    fn flush(&mut self) -> IoResult<()> {
        self._flush()
            .map_err(|err| IoError::new(IoErrorKind::Other, err))
    }
}

impl Debug for PythonIoBase<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PythonIoBase")
            .field("io_base", &self.io_base)
            .finish()
    }
}
