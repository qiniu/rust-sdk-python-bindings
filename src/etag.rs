use super::utils::PythonIoBase;
use pyo3::prelude::*;
use qiniu_sdk::etag::{FixedOutput, GenericArray, Reset, Update};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<EtagV1>()?;
    m.add_class::<EtagV2>()?;
    m.add_class::<Etag>()?;
    m.add_class::<EtagVersion>()?;
    m.add_function(wrap_pyfunction!(etag_of, m)?)?;
    m.add_function(wrap_pyfunction!(etag_with_parts, m)?)?;
    m.add_function(wrap_pyfunction!(async_etag_of, m)?)?;
    m.add_function(wrap_pyfunction!(async_etag_with_parts, m)?)?;
    Ok(())
}

macro_rules! define_etag_struct {
    ($name:ident, $rust_struct:ty) => {
        #[pyclass]
        struct $name($rust_struct);

        #[pymethods]
        impl $name {
            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }

            fn write(&mut self, data: Vec<u8>) -> usize {
                self.0.update(&data);
                data.len()
            }

            fn reset(&mut self) {
                self.0.reset();
            }

            fn finalize(&mut self) -> String {
                let mut buf =
                    GenericArray::<u8, <$rust_struct as FixedOutput>::OutputSize>::default();
                self.0.finalize_into_reset(&mut buf);
                String::from_utf8(buf.to_vec()).unwrap()
            }
        }
    };
}

define_etag_struct!(EtagV1, qiniu_sdk::etag::EtagV1);

#[pymethods]
impl EtagV1 {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::etag::EtagV1::new())
    }
}

define_etag_struct!(EtagV2, qiniu_sdk::etag::EtagV2);

#[pymethods]
impl EtagV2 {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::etag::EtagV2::new())
    }
}

define_etag_struct!(Etag, qiniu_sdk::etag::Etag);

#[pymethods]
impl Etag {
    #[new]
    fn new(version: EtagVersion) -> Self {
        Self(qiniu_sdk::etag::Etag::new(version.into()))
    }
}

#[pyclass]
#[derive(Debug, Copy, Clone)]
enum EtagVersion {
    V1 = 1,
    V2 = 2,
}

impl From<EtagVersion> for qiniu_sdk::etag::EtagVersion {
    fn from(v: EtagVersion) -> Self {
        match v {
            EtagVersion::V1 => qiniu_sdk::etag::EtagVersion::V1,
            EtagVersion::V2 => qiniu_sdk::etag::EtagVersion::V2,
        }
    }
}

#[pyfunction]
fn etag_of(io_base: PyObject) -> PyResult<String> {
    let etag = qiniu_sdk::etag::etag_of(PythonIoBase::new(io_base))?;
    Ok(etag)
}

#[pyfunction]
fn etag_with_parts(io_base: PyObject, parts: Vec<usize>) -> PyResult<String> {
    let etag = qiniu_sdk::etag::etag_with_parts(PythonIoBase::new(io_base), &parts)?;
    Ok(etag)
}

#[pyfunction]
fn async_etag_of(io_base: PyObject, py: Python<'_>) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async move {
        let etag =
            qiniu_sdk::etag::async_etag_of(PythonIoBase::new(io_base).into_async_read()).await?;
        Ok(etag)
    })
}

#[pyfunction]
fn async_etag_with_parts(io_base: PyObject, parts: Vec<usize>, py: Python<'_>) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async move {
        let etag = qiniu_sdk::etag::async_etag_with_parts(
            PythonIoBase::new(io_base).into_async_read(),
            &parts,
        )
        .await?;
        Ok(etag)
    })
}
