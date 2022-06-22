use super::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuInvalidConcurrency, QiniuInvalidLimitation, QiniuInvalidMultiply,
        QiniuInvalidObjectSize, QiniuInvalidPartSize, QiniuIoError,
    },
    http_client::RegionsProvider,
    upload_token::{on_policy_generated_callback, UploadTokenProvider},
    utils::{
        convert_api_call_error, convert_json_value_to_py_object, parse_mime, AsyncReader,
        PythonIoBase, Reader,
    },
};
use async_std::fs::File as AsyncFile;
use maybe_owned::MaybeOwned;
use pyo3::prelude::*;
use qiniu_sdk::prelude::{ResumableRecorder, SinglePartUploader};
use std::{collections::HashMap, fs::File, num::NonZeroU64, sync::Arc, time::Duration};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload")?;
    m.add_class::<UploadTokenSigner>()?;
    m.add_class::<ConcurrencyProvider>()?;
    m.add_class::<FixedConcurrencyProvider>()?;
    m.add_class::<DataPartitionProvider>()?;
    m.add_class::<FixedDataPartitionProvider>()?;
    m.add_class::<MultiplyDataPartitionProvider>()?;
    m.add_class::<LimitedDataPartitionProvider>()?;
    m.add_class::<ResumablePolicy>()?;
    m.add_class::<ResumablePolicyProvider>()?;
    m.add_class::<AlwaysSinglePart>()?;
    m.add_class::<AlwaysMultiParts>()?;
    m.add_class::<FixedThresholdResumablePolicy>()?;
    m.add_class::<MultiplePartitionsResumablePolicyProvider>()?;
    m.add_class::<SourceKey>()?;
    m.add_class::<DummyResumableRecorder>()?;
    m.add_class::<DummyResumableRecorderMedium>()?;
    m.add_class::<FileSystemResumableRecorder>()?;
    m.add_class::<FileSystemResumableRecorderMedium>()?;
    m.add_class::<FileSystemResumableRecorderAsyncMedium>()?;
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

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
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
    #[pyo3(text_signature = "(concurrency, object_size, elapsed_ns, /, error = None)")]
    #[args(error = "None")]
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

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
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
#[derive(Copy, Clone, Debug)]
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
    #[pyo3(text_signature = "(part_size, elapsed_ns, /, error = None)")]
    #[args(error = "None")]
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
#[derive(Copy, Clone, Debug)]
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
#[derive(Copy, Clone, Debug)]
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
#[derive(Copy, Clone, Debug)]
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

/// 可恢复策略
///
/// 选择使用单请求上传或分片上传
#[pyclass]
#[derive(Debug, Copy, Clone)]
enum ResumablePolicy {
    /// 单请求上传
    SinglePartUploading = 0,
    /// 分片上传
    MultiPartsUploading = 1,
}

#[pymethods]
impl ResumablePolicy {
    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<qiniu_sdk::upload::ResumablePolicy> for ResumablePolicy {
    fn from(policy: qiniu_sdk::upload::ResumablePolicy) -> Self {
        match policy {
            qiniu_sdk::upload::ResumablePolicy::MultiPartsUploading => {
                ResumablePolicy::MultiPartsUploading
            }
            qiniu_sdk::upload::ResumablePolicy::SinglePartUploading => {
                ResumablePolicy::SinglePartUploading
            }
            _ => unreachable!("Unknown Resumable Policy: {:?}", policy),
        }
    }
}

impl From<ResumablePolicy> for qiniu_sdk::upload::ResumablePolicy {
    fn from(policy: ResumablePolicy) -> Self {
        match policy {
            ResumablePolicy::SinglePartUploading => {
                qiniu_sdk::upload::ResumablePolicy::MultiPartsUploading
            }
            ResumablePolicy::MultiPartsUploading => {
                qiniu_sdk::upload::ResumablePolicy::SinglePartUploading
            }
        }
    }
}

/// 可恢复策略获取接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ResumablePolicyProvider(Box<dyn qiniu_sdk::upload::ResumablePolicyProvider>);

#[pymethods]
impl ResumablePolicyProvider {
    /// 通过数据源大小获取可恢复策略
    #[pyo3(text_signature = "(source_size)")]
    fn get_policy_from_size(&self, source_size: u64) -> ResumablePolicy {
        self.0
            .get_policy_from_size(source_size, Default::default())
            .into()
    }

    /// 通过输入流获取可恢复策略
    ///
    /// 返回选择的可恢复策略，以及经过更新的输入流
    #[pyo3(text_signature = "(reader)")]
    fn get_policy_from_reader(&self, reader: PyObject) -> PyResult<(ResumablePolicy, Reader)> {
        self.0
            .get_policy_from_reader(Box::new(PythonIoBase::new(reader)), Default::default())
            .map(|(policy, reader)| (policy.into(), reader.into()))
            .map_err(QiniuIoError::from_err)
    }

    /// 通过异步输入流获取可恢复策略
    ///
    /// 返回选择的可恢复策略，以及经过更新的异步输入流
    #[pyo3(text_signature = "(reader)")]
    fn get_policy_from_async_reader<'p>(
        &'p self,
        reader: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            provider
                .get_policy_from_async_reader(
                    Box::new(PythonIoBase::new(reader).into_async_read()),
                    Default::default(),
                )
                .await
                .map(|(policy, reader)| (ResumablePolicy::from(policy), AsyncReader::from(reader)))
                .map_err(QiniuIoError::from_err)
        })
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 总是选择单请求上传
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct AlwaysSinglePart;

#[pymethods]
impl AlwaysSinglePart {
    /// 创建单请求上传
    #[new]
    fn new() -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(qiniu_sdk::upload::AlwaysSinglePart)),
        )
    }
}

/// 总是选择分片上传
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct AlwaysMultiParts;

#[pymethods]
impl AlwaysMultiParts {
    /// 创建单请求上传
    #[new]
    fn new() -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(qiniu_sdk::upload::AlwaysMultiParts)),
        )
    }
}

/// 固定阀值的可恢复策略
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(threshold)")]
struct FixedThresholdResumablePolicy;

#[pymethods]
impl FixedThresholdResumablePolicy {
    /// 创建单请求上传
    #[new]
    fn new(threshold: u64) -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(
                qiniu_sdk::upload::FixedThresholdResumablePolicy::new(threshold),
            )),
        )
    }
}

/// 整数倍分片大小的可恢复策略
///
/// 在数据源大小超过分片大小提供者返回的分片大小的整数倍时，将使用分片上传。
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(base, multiply)")]
struct MultiplePartitionsResumablePolicyProvider;

#[pymethods]
impl MultiplePartitionsResumablePolicyProvider {
    /// 创建整数倍分片大小的可恢复策略
    ///
    /// 如果传入 `0` 则抛出异常
    #[new]
    fn new(
        base: DataPartitionProvider,
        multiply: u64,
    ) -> PyResult<(Self, ResumablePolicyProvider)> {
        let provider =
            qiniu_sdk::upload::MultiplePartitionsResumablePolicyProvider::new(base, multiply)
                .map_or_else(
                    || Err(QiniuInvalidMultiply::new_err("Invalid multiply")),
                    Ok,
                )?;
        Ok((Self, ResumablePolicyProvider(Box::new(provider))))
    }
}

/// 数据源 KEY
///
/// 用于区分不同的数据源
#[pyclass]
#[derive(Debug, Clone)]
struct SourceKey(qiniu_sdk::upload::SourceKey);

#[pymethods]
impl SourceKey {
    fn __str__(&self) -> String {
        hex::encode(&*self.0)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

macro_rules! impl_resumable_recorder {
    ($name:ident, $medium:ident, $async_medium:ident) => {
        #[pymethods]
        impl $name {
            /// 根据数据源 KEY 打开只读记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn open_for_read(&self, source_key: &SourceKey) -> PyResult<$medium> {
                self.0
                    .open_for_read(&source_key.0)
                    .map($medium)
                    .map_err(QiniuIoError::from_err)
            }

            /// 根据数据源 KEY 打开追加记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn open_for_append(&self, source_key: &SourceKey) -> PyResult<$medium> {
                self.0
                    .open_for_append(&source_key.0)
                    .map($medium)
                    .map_err(QiniuIoError::from_err)
            }

            /// 根据数据源 KEY 创建追加记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn open_for_create_new(&self, source_key: &SourceKey) -> PyResult<$medium> {
                self.0
                    .open_for_create_new(&source_key.0)
                    .map($medium)
                    .map_err(QiniuIoError::from_err)
            }

            /// 根据数据源 KEY 删除记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn delete(&self, source_key: &SourceKey) -> PyResult<()> {
                self.0.delete(&source_key.0).map_err(QiniuIoError::from_err)
            }

            /// 根据数据源 KEY 打开异步只读记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn open_for_async_read<'p>(
                &self,
                source_key: SourceKey,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let recorder = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    recorder
                        .open_for_async_read(&source_key.0)
                        .await
                        .map($async_medium)
                        .map_err(QiniuIoError::from_err)
                })
            }

            /// 根据数据源 KEY 打开异步追加记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn open_for_async_append<'p>(
                &self,
                source_key: SourceKey,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let recorder = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    recorder
                        .open_for_async_append(&source_key.0)
                        .await
                        .map($async_medium)
                        .map_err(QiniuIoError::from_err)
                })
            }

            /// 根据数据源 KEY 创建异步追加记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn open_for_async_create_new<'p>(
                &self,
                source_key: SourceKey,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let recorder = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    recorder
                        .open_for_async_create_new(&source_key.0)
                        .await
                        .map($async_medium)
                        .map_err(QiniuIoError::from_err)
                })
            }

            /// 根据数据源 KEY 异步删除记录介质
            #[pyo3(text_signature = "($self, source_key)")]
            fn async_delete<'p>(
                &self,
                source_key: SourceKey,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let recorder = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    recorder
                        .async_delete(&source_key.0)
                        .await
                        .map_err(QiniuIoError::from_err)
                })
            }
        }
    };
}

/// 无断点恢复记录器
///
/// 实现了断点恢复记录器接口，但总是返回找不到记录
#[pyclass]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "()")]
struct DummyResumableRecorder(qiniu_sdk::upload::DummyResumableRecorder);

#[pymethods]
impl DummyResumableRecorder {
    /// 创建无断点恢复记录器
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::upload::DummyResumableRecorder::new())
    }
}
impl_resumable_recorder!(
    DummyResumableRecorder,
    DummyResumableRecorderMedium,
    DummyResumableRecorderMedium
);

/// 无断点恢复记录介质
///
/// 实现了断点恢复记录介质接口，但总是返回错误
#[pyclass]
#[derive(Debug, Clone, Copy)]
struct DummyResumableRecorderMedium(qiniu_sdk::upload::DummyResumableRecorderMedium);

#[pymethods]
impl DummyResumableRecorderMedium {
    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 文件系统断点恢复记录器
///
/// 基于文件系统提供断点恢复记录功能
#[pyclass]
#[derive(Debug, Clone)]
#[pyo3(text_signature = "(path = None)")]
struct FileSystemResumableRecorder(qiniu_sdk::upload::FileSystemResumableRecorder);

#[pymethods]
impl FileSystemResumableRecorder {
    /// 创建文件系统断点恢复记录器，传入一个目录路径用于储存断点记录
    #[new]
    #[args(path = "None")]
    fn new(path: Option<String>) -> Self {
        let recorder = if let Some(path) = path {
            qiniu_sdk::upload::FileSystemResumableRecorder::new(path)
        } else {
            qiniu_sdk::upload::FileSystemResumableRecorder::default()
        };
        Self(recorder)
    }
}
impl_resumable_recorder!(
    FileSystemResumableRecorder,
    FileSystemResumableRecorderMedium,
    FileSystemResumableRecorderAsyncMedium
);

/// 文件恢复记录介质
#[pyclass]
#[derive(Debug)]
struct FileSystemResumableRecorderMedium(File);

#[pymethods]
impl FileSystemResumableRecorderMedium {
    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 异步文件恢复记录介质
#[pyclass]
#[derive(Debug)]
struct FileSystemResumableRecorderAsyncMedium(AsyncFile);

#[pymethods]
impl FileSystemResumableRecorderAsyncMedium {
    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 表单上传器
///
/// 通过七牛表单上传 API 一次上传整个数据流
#[pyclass]
#[derive(Debug, Clone)]
struct FormUploader(Arc<qiniu_sdk::upload::FormUploader>);

#[pymethods]
impl FormUploader {
    // TODO: constructor

    #[pyo3(
        text_signature = "($self, path, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn upload_path(
        &self,
        path: &str,
        region_provider: Option<RegionsProvider>,
        object_name: Option<&str>,
        file_name: Option<&str>,
        content_type: Option<&str>,
        metadata: Option<HashMap<String, String>>,
        custom_vars: Option<HashMap<String, String>>,
        uploaded_part_ttl_secs: Option<u64>,
    ) -> PyResult<PyObject> {
        let object_params = make_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
        )?;
        self.0
            .upload_path(path, object_params)
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
            .and_then(|v| convert_json_value_to_py_object(&v))
    }

    #[pyo3(
        text_signature = "($self, reader, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn upload_reader(
        &self,
        reader: PyObject,
        region_provider: Option<RegionsProvider>,
        object_name: Option<&str>,
        file_name: Option<&str>,
        content_type: Option<&str>,
        metadata: Option<HashMap<String, String>>,
        custom_vars: Option<HashMap<String, String>>,
        uploaded_part_ttl_secs: Option<u64>,
    ) -> PyResult<PyObject> {
        let object_params = make_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
        )?;
        self.0
            .upload_reader(PythonIoBase::new(reader), object_params)
            .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
            .and_then(|v| convert_json_value_to_py_object(&v))
    }

    #[pyo3(
        text_signature = "($self, path, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn async_upload_path<'p>(
        &self,
        path: String,
        region_provider: Option<RegionsProvider>,
        object_name: Option<&str>,
        file_name: Option<&str>,
        content_type: Option<&str>,
        metadata: Option<HashMap<String, String>>,
        custom_vars: Option<HashMap<String, String>>,
        uploaded_part_ttl_secs: Option<u64>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let object_params = make_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
        )?;
        let uploader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            uploader
                .async_upload_path(&path, object_params)
                .await
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }

    #[pyo3(
        text_signature = "($self, reader, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn async_upload_reader<'p>(
        &self,
        reader: PyObject,
        region_provider: Option<RegionsProvider>,
        object_name: Option<&str>,
        file_name: Option<&str>,
        content_type: Option<&str>,
        metadata: Option<HashMap<String, String>>,
        custom_vars: Option<HashMap<String, String>>,
        uploaded_part_ttl_secs: Option<u64>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let object_params = make_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
        )?;
        let uploader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            uploader
                .async_upload_reader(PythonIoBase::new(reader).into_async_read(), object_params)
                .await
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }
}

fn make_object_params(
    region_provider: Option<RegionsProvider>,
    object_name: Option<&str>,
    file_name: Option<&str>,
    content_type: Option<&str>,
    metadata: Option<HashMap<String, String>>,
    custom_vars: Option<HashMap<String, String>>,
    uploaded_part_ttl_secs: Option<u64>,
) -> PyResult<qiniu_sdk::upload::ObjectParams> {
    let mut builder = qiniu_sdk::upload::ObjectParams::builder();
    if let Some(region_provider) = region_provider {
        builder.region_provider(region_provider);
    }
    if let Some(object_name) = object_name {
        builder.object_name(object_name);
    }
    if let Some(file_name) = file_name {
        builder.file_name(file_name);
    }
    if let Some(content_type) = content_type {
        builder.content_type(parse_mime(content_type)?);
    }
    if let Some(metadata) = metadata {
        builder.metadata(metadata);
    }
    if let Some(custom_vars) = custom_vars {
        builder.custom_vars(custom_vars);
    }
    if let Some(uploaded_part_ttl_secs) = uploaded_part_ttl_secs {
        builder.uploaded_part_ttl(Duration::from_secs(uploaded_part_ttl_secs));
    }
    Ok(builder.build())
}
