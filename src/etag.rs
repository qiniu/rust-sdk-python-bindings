use super::utils::PythonIoBase;
use pyo3::prelude::*;
use qiniu_sdk::etag::{FixedOutput, GenericArray, Reset, Update, ETAG_SIZE};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "etag")?;
    m.add("ETAG_SIZE", ETAG_SIZE)?;
    m.add_class::<EtagV1>()?;
    m.add_function(wrap_pyfunction!(etag_of, m)?)?;
    m.add_function(wrap_pyfunction!(async_etag_of, m)?)?;
    Ok(m)
}

macro_rules! define_etag_struct {
    ($name:ident, $rust_struct:ty, $docs:expr, $signature:expr) => {
        #[pyclass]
        #[doc = $docs]
        #[pyo3(text_signature = $signature)]
        struct $name($rust_struct);

        #[pymethods]
        impl $name {
            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }

            /// 写入数据到 Etag 计算器
            #[pyo3(text_signature = "($self, data)")]
            fn write(&mut self, data: Vec<u8>) -> usize {
                self.0.update(&data);
                data.len()
            }

            /// 重置 Etag 计算器
            #[pyo3(text_signature = "($self)")]
            fn reset(&mut self) {
                self.0.reset();
            }

            /// 获取 Etag 计算结果，并且重置计算器
            #[pyo3(text_signature = "($self)")]
            fn finalize(&mut self) -> String {
                let mut buf =
                    GenericArray::<u8, <$rust_struct as FixedOutput>::OutputSize>::default();
                self.0.finalize_into_reset(&mut buf);
                String::from_utf8(buf.to_vec()).unwrap()
            }
        }
    };
}

define_etag_struct!(
    EtagV1,
    qiniu_sdk::etag::EtagV1,
    "Etag V1 计算器\n通过 `EtagV1()` 创建",
    "()"
);

#[pymethods]
impl EtagV1 {
    /// 创建 Etag V1 计算器
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::etag::EtagV1::new())
    }
}

/// 读取 reader 中的数据并计算它的 Etag V1，生成结果
#[pyfunction]
#[pyo3(text_signature = "(reader)")]
fn etag_of(reader: PyObject) -> PyResult<String> {
    let etag = qiniu_sdk::etag::etag_of(PythonIoBase::new(reader))?;
    Ok(etag)
}

/// 异步读取 reader 中的数据并计算它的 Etag V1，生成结果
#[pyfunction]
#[pyo3(text_signature = "(reader)")]
fn async_etag_of(reader: PyObject, py: Python<'_>) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async move {
        let etag =
            qiniu_sdk::etag::async_etag_of(PythonIoBase::new(reader).into_async_read()).await?;
        Ok(etag)
    })
}
