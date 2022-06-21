use super::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuInvalidConcurrency, QiniuInvalidLimitation, QiniuInvalidMultiply,
        QiniuInvalidObjectSize, QiniuInvalidPartSize,
    },
    upload_token::{on_policy_generated_callback, UploadTokenProvider},
    utils::convert_api_call_error,
};
use pyo3::prelude::*;
use std::{num::NonZeroU64, time::Duration};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload")?;
    m.add_class::<UploadTokenSigner>()?;
    m.add_class::<ConcurrencyProvider>()?;
    Ok(m)
}

/// 上传凭证签发器
#[pyclass]
#[derive(Clone, Debug)]
struct UploadTokenSigner(qiniu_sdk::upload::UploadTokenSigner);

#[pymethods]
impl UploadTokenSigner {
    /// 根据上传凭证提供者创建上传凭证签发器
    #[staticmethod]
    #[pyo3(text_signature = "(upload_token_provider)")]
    fn new_upload_token_provider(upload_token_provider: UploadTokenProvider) -> Self {
        Self(qiniu_sdk::upload::UploadTokenSigner::new_upload_token_provider(upload_token_provider))
    }

    /// 根据认证信息提供者和存储空间名称创建上传凭证签发器
    #[staticmethod]
    #[pyo3(
        text_signature = "(credential, bucket_name, lifetime_secs, /, on_policy_generated = None)"
    )]
    #[args(on_policy_generated = "None")]
    fn new_credential_provider(
        credential: CredentialProvider,
        bucket_name: String,
        lifetime_secs: u64,
        on_policy_generated: Option<PyObject>,
    ) -> Self {
        let mut builder = qiniu_sdk::upload::UploadTokenSigner::new_credential_provider_builder(
            credential,
            bucket_name,
            Duration::from_secs(lifetime_secs),
        );
        if let Some(callback) = on_policy_generated {
            builder = builder.on_policy_generated(on_policy_generated_callback(callback));
        }
        Self(builder.build())
    }
}

/// 并发数获取接口
///
/// 获取分片上传时的并发数
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ConcurrencyProvider(Box<dyn qiniu_sdk::upload::ConcurrencyProvider>);

#[pymethods]
impl ConcurrencyProvider {
    /// 获取并发数
    #[getter]
    fn get_concurrency(&self) -> usize {
        self.0.concurrency().as_usize()
    }

    /// 反馈并发数结果
    fn feedback(
        &self,
        concurrency: usize,
        object_size: u64,
        elapsed_ns: u64,
        error: Option<&QiniuApiCallError>,
    ) -> PyResult<()> {
        let concurrency = qiniu_sdk::upload::Concurrency::new(concurrency).map_or_else(
            || Err(QiniuInvalidConcurrency::new_err("Invalid concurrency")),
            Ok,
        )?;
        let object_size = NonZeroU64::new(object_size).map_or_else(
            || Err(QiniuInvalidObjectSize::new_err("Invalid object size")),
            Ok,
        )?;
        let error = error.map(PyErr::from);
        let error = error.as_ref().map(convert_api_call_error).transpose()?;
        let mut feedback_builder = qiniu_sdk::upload::ConcurrencyProviderFeedback::builder(
            concurrency,
            object_size,
            Duration::from_nanos(elapsed_ns),
        );
        if let Some(error) = &error {
            feedback_builder.error(error.as_ref());
        }
        self.0.feedback(feedback_builder.build());
        Ok(())
    }
}

impl qiniu_sdk::upload::ConcurrencyProvider for ConcurrencyProvider {
    fn concurrency(&self) -> qiniu_sdk::upload::Concurrency {
        self.0.concurrency()
    }

    fn feedback(&self, feedback: qiniu_sdk::upload::ConcurrencyProviderFeedback<'_>) {
        self.0.feedback(feedback)
    }
}

/// 固定并发数提供者
#[pyclass(extends = ConcurrencyProvider)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(concurrency)")]
struct FixedConcurrencyProvider;

#[pymethods]
impl FixedConcurrencyProvider {
    /// 创建固定并发数提供者
    ///
    /// 如果传入 `0` 将抛出异常
    #[new]
    fn new(concurrency: usize) -> PyResult<(Self, ConcurrencyProvider)> {
        let provider = qiniu_sdk::upload::FixedConcurrencyProvider::new(concurrency).map_or_else(
            || Err(QiniuInvalidConcurrency::new_err("Invalid concurrency")),
            Ok,
        )?;
        Ok((Self, ConcurrencyProvider(Box::new(provider))))
    }
}

/// 分片大小获取接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct DataPartitionProvider(Box<dyn qiniu_sdk::upload::DataPartitionProvider>);

#[pymethods]
impl DataPartitionProvider {
    #[getter]
    fn get_part_size(&self) -> u64 {
        self.0.part_size().as_u64()
    }

    /// 反馈并发数结果
    fn feedback(
        &self,
        part_size: u64,
        elapsed_ns: u64,
        error: Option<&QiniuApiCallError>,
    ) -> PyResult<()> {
        let part_size = qiniu_sdk::upload::PartSize::new(part_size).map_or_else(
            || Err(QiniuInvalidPartSize::new_err("Invalid part size")),
            Ok,
        )?;
        let error = error.map(PyErr::from);
        let error = error.as_ref().map(convert_api_call_error).transpose()?;
        let extensions = qiniu_sdk::http::Extensions::new();
        let mut feedback_builder = qiniu_sdk::upload::DataPartitionProviderFeedback::builder(
            part_size,
            Duration::from_nanos(elapsed_ns),
            &extensions,
        );
        if let Some(error) = &error {
            feedback_builder.error(error.as_ref());
        }
        self.0.feedback(feedback_builder.build());
        Ok(())
    }
}

impl qiniu_sdk::upload::DataPartitionProvider for DataPartitionProvider {
    fn part_size(&self) -> qiniu_sdk::upload::PartSize {
        self.0.part_size()
    }

    fn feedback(&self, feedback: qiniu_sdk::upload::DataPartitionProviderFeedback<'_>) {
        self.0.feedback(feedback)
    }
}

/// 固定分片大小提供者
#[pyclass(extends = DataPartitionProvider)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(part_size)")]
struct FixedDataPartitionProvider;

#[pymethods]
impl FixedDataPartitionProvider {
    /// 创建固定分片大小提供者
    ///
    /// 如果传入 `0` 将抛出异常
    #[new]
    fn new(part_size: u64) -> PyResult<(Self, DataPartitionProvider)> {
        let provider = qiniu_sdk::upload::FixedDataPartitionProvider::new(part_size).map_or_else(
            || Err(QiniuInvalidPartSize::new_err("Invalid part size")),
            Ok,
        )?;
        Ok((Self, DataPartitionProvider(Box::new(provider))))
    }
}

/// 整数倍分片大小提供者
///
/// 基于一个分片大小提供者实例，如果提供的分片大小不是指定倍数的整数倍，则下调到它的整数倍
#[pyclass(extends = DataPartitionProvider)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(base, multiply)")]
struct MultiplyDataPartitionProvider;

#[pymethods]
impl MultiplyDataPartitionProvider {
    /// 创建整数倍分片大小提供者
    ///
    /// 如果传入 `0` 将抛出异常
    #[new]
    fn new(base: DataPartitionProvider, multiply: u64) -> PyResult<(Self, DataPartitionProvider)> {
        let provider = qiniu_sdk::upload::MultiplyDataPartitionProvider::new(base, multiply)
            .map_or_else(
                || Err(QiniuInvalidMultiply::new_err("Invalid multiply")),
                Ok,
            )?;
        Ok((Self, DataPartitionProvider(Box::new(provider))))
    }
}

/// 受限的分片大小提供者
///
/// 基于一个分片大小提供者实例，如果提供的分片大小在限制范围外，则调整到限制范围内。
#[pyclass(extends = DataPartitionProvider)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(base, min, max)")]
struct LimitedDataPartitionProvider;

#[pymethods]
impl LimitedDataPartitionProvider {
    /// 创建受限的分片大小提供者
    ///
    /// 如果传入 `0` 作为 `min` 或 `max` 将抛出异常
    #[new]
    fn new(
        base: DataPartitionProvider,
        min: u64,
        max: u64,
    ) -> PyResult<(Self, DataPartitionProvider)> {
        let provider = qiniu_sdk::upload::LimitedDataPartitionProvider::new(base, min, max)
            .map_or_else(
                || Err(QiniuInvalidLimitation::new_err("Invalid limitation")),
                Ok,
            )?;
        Ok((Self, DataPartitionProvider(Box::new(provider))))
    }
}
