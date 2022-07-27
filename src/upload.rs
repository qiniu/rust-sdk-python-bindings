use super::{
    credential::CredentialProvider,
    exceptions::{
        QiniuApiCallError, QiniuInvalidConcurrency, QiniuInvalidLimitation, QiniuInvalidMultiply,
        QiniuInvalidObjectSize, QiniuInvalidPartSize, QiniuInvalidSourceKeyLengthError,
        QiniuIoError,
    },
    http::HttpResponsePartsMut,
    http_client::{
        BucketRegionsQueryer, Endpoints, HttpClient, RegionsProvider, RequestBuilderPartsRef,
    },
    upload_token::{on_policy_generated_callback, UploadTokenProvider},
    utils::{convert_api_call_error, convert_json_value_to_py_object, parse_mime, PythonIoBase},
};
use anyhow::Result as AnyResult;
use futures::{lock::Mutex as AsyncMutex, AsyncRead, AsyncReadExt, AsyncWriteExt};
use maybe_owned::MaybeOwned;
use pyo3::{exceptions::PyIOError, prelude::*, types::PyBytes};
use qiniu_sdk::{
    etag::GenericArray,
    prelude::{
        AsyncReset, InitializedParts, MultiPartsUploader, MultiPartsUploaderSchedulerExt,
        MultiPartsUploaderWithCallbacks, Reset, SinglePartUploader, UploadedPart,
        UploaderWithCallbacks,
    },
};
use sha1::{digest::OutputSizeUser, Sha1};
use std::{
    collections::HashMap, fmt::Debug, io::Read, mem::transmute, num::NonZeroU64, sync::Arc,
    time::Duration,
};

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
    m.add_class::<ResumableRecorder>()?;
    m.add_class::<ReadOnlyResumableRecorderMedium>()?;
    m.add_class::<AppendOnlyResumableRecorderMedium>()?;
    m.add_class::<ReadOnlyAsyncResumableRecorderMedium>()?;
    m.add_class::<AppendOnlyAsyncResumableRecorderMedium>()?;
    m.add_class::<DummyResumableRecorder>()?;
    m.add_class::<FileSystemResumableRecorder>()?;
    m.add_class::<DataSource>()?;
    m.add_class::<FileDataSource>()?;
    m.add_class::<UnseekableDataSource>()?;
    m.add_class::<AsyncDataSource>()?;
    m.add_class::<AsyncFileDataSource>()?;
    m.add_class::<AsyncUnseekableDataSource>()?;
    m.add_class::<DataSourceReader>()?;
    m.add_class::<AsyncDataSourceReader>()?;
    m.add_class::<UploadManager>()?;
    m.add_class::<FormUploader>()?;
    m.add_class::<MultiPartsV1Uploader>()?;
    m.add_class::<MultiPartsV1UploaderInitializedObject>()?;
    m.add_class::<AsyncMultiPartsV1UploaderInitializedObject>()?;
    m.add_class::<MultiPartsV1UploaderUploadedPart>()?;
    m.add_class::<AsyncMultiPartsV1UploaderUploadedPart>()?;
    m.add_class::<MultiPartsV2Uploader>()?;
    m.add_class::<MultiPartsV2UploaderInitializedObject>()?;
    m.add_class::<AsyncMultiPartsV2UploaderInitializedObject>()?;
    m.add_class::<MultiPartsV2UploaderUploadedPart>()?;
    m.add_class::<AsyncMultiPartsV2UploaderUploadedPart>()?;
    m.add_class::<MultiPartsUploaderScheduler>()?;
    m.add_class::<SerialMultiPartsUploaderScheduler>()?;
    m.add_class::<ConcurrentMultiPartsUploaderScheduler>()?;
    m.add_class::<UploadingProgressInfo>()?;
    m.add_class::<UploadedPartInfo>()?;
    m.add_class::<MultiPartsUploaderSchedulerPrefer>()?;
    m.add_class::<SinglePartUploaderPrefer>()?;
    m.add_class::<MultiPartsUploaderPrefer>()?;
    m.add_class::<AutoUploader>()?;
    m.add_class::<Reader>()?;
    m.add_class::<AsyncReader>()?;
    Ok(m)
}

/// 上传凭证签发器
///
/// 通过 `UploadTokenSigner.new_upload_token_provider(upload_token_provider)` 或 `UploadTokenSigner.new_credential_provider(credential, bucket_name, lifetime_secs, on_policy_generated = None)` 创建上传凭证签发器
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
/// 抽象类
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
        py: Python<'_>,
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
        let feedback = feedback_builder.build();
        py.allow_threads(|| {
            self.0.feedback(feedback);
        });
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
///
/// 通过 `FixedConcurrencyProvider(concurrency)` 创建固定并发数提供者
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
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct DataPartitionProvider(Box<dyn qiniu_sdk::upload::DataPartitionProvider>);

#[pymethods]
impl DataPartitionProvider {
    /// 获取分片大小
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
        py: Python<'_>,
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
        let feedback = feedback_builder.build();
        py.allow_threads(|| {
            self.0.feedback(feedback);
        });
        Ok(())
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
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
///
/// 通过 `FixedDataPartitionProvider(part_size)` 创建固定分片大小提供者
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
///
/// 通过 `MultiplyDataPartitionProvider(base, multiply)` 创建整数倍分片大小提供者
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
///
/// 通过 `LimitedDataPartitionProvider(base, min, max)` 创建受限的分片大小提供者
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
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ResumablePolicyProvider(Box<dyn qiniu_sdk::upload::ResumablePolicyProvider>);

#[pymethods]
impl ResumablePolicyProvider {
    /// 通过数据源大小获取可恢复策略
    #[pyo3(text_signature = "(source_size)")]
    fn get_policy_from_size(&self, source_size: u64, py: Python<'_>) -> ResumablePolicy {
        py.allow_threads(|| {
            self.0
                .get_policy_from_size(source_size, Default::default())
                .into()
        })
    }

    /// 通过输入流获取可恢复策略
    ///
    /// 返回选择的可恢复策略，以及经过更新的输入流
    #[pyo3(text_signature = "(reader)")]
    fn get_policy_from_reader(
        &self,
        reader: PyObject,
        py: Python<'_>,
    ) -> PyResult<(ResumablePolicy, Reader)> {
        py.allow_threads(|| {
            self.0
                .get_policy_from_reader(Box::new(PythonIoBase::new(reader)), Default::default())
                .map(|(policy, reader)| (policy.into(), reader.into()))
                .map_err(QiniuIoError::from_err)
        })
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

impl qiniu_sdk::upload::ResumablePolicyProvider for ResumablePolicyProvider {
    fn get_policy_from_size(
        &self,
        source_size: u64,
        opts: qiniu_sdk::upload::GetPolicyOptions,
    ) -> qiniu_sdk::upload::ResumablePolicy {
        self.0.get_policy_from_size(source_size, opts)
    }

    fn get_policy_from_reader<'a>(
        &self,
        reader: Box<dyn qiniu_sdk::upload::DynRead + 'a>,
        opts: qiniu_sdk::upload::GetPolicyOptions,
    ) -> std::io::Result<(
        qiniu_sdk::upload::ResumablePolicy,
        Box<dyn qiniu_sdk::upload::DynRead + 'a>,
    )> {
        self.0.get_policy_from_reader(reader, opts)
    }

    fn get_policy_from_async_reader<'a>(
        &self,
        reader: Box<dyn qiniu_sdk::prelude::DynAsyncRead + 'a>,
        opts: qiniu_sdk::upload::GetPolicyOptions,
    ) -> futures::future::BoxFuture<
        'a,
        std::io::Result<(
            qiniu_sdk::upload::ResumablePolicy,
            Box<dyn qiniu_sdk::prelude::DynAsyncRead + 'a>,
        )>,
    > {
        self.0.get_policy_from_async_reader(reader, opts)
    }
}

/// 总是选择单请求上传
///
/// 通过 `AlwaysSinglePart()` 创建单请求上传策略
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct AlwaysSinglePart;

#[pymethods]
impl AlwaysSinglePart {
    /// 创建单请求上传策略
    #[new]
    fn new() -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(qiniu_sdk::upload::AlwaysSinglePart)),
        )
    }
}

/// 总是选择分片上传
///
/// 通过 `AlwaysMultiParts()` 创建分片上传策略
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct AlwaysMultiParts;

#[pymethods]
impl AlwaysMultiParts {
    /// 创建分片上传策略
    #[new]
    fn new() -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(qiniu_sdk::upload::AlwaysMultiParts)),
        )
    }
}

/// 固定阀值的可恢复策略
///
/// 通过 `FixedThresholdResumablePolicy(threshold)` 创建固定阀值的可恢复策略
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(threshold)")]
struct FixedThresholdResumablePolicy;

#[pymethods]
impl FixedThresholdResumablePolicy {
    /// 创建固定阀值的可恢复策略
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
///
/// 通过 `MultiplePartitionsResumablePolicyProvider(base, multiply)` 创建整数倍分片大小的可恢复策略
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
///
/// 通过 `SourceKey(key)` 创建数据源 KEY
#[pyclass]
#[derive(Debug, Clone)]
struct SourceKey(qiniu_sdk::upload::SourceKey);

#[pymethods]
impl SourceKey {
    /// 创建数据源 KEY，只能接受 20 个字节的二进制数据
    #[new]
    fn new(key: &[u8]) -> PyResult<Self> {
        if key.len() != Sha1::output_size() {
            return Err(QiniuInvalidSourceKeyLengthError::new_err(
                "Invalid source key length, expected 20",
            ));
        }
        let arr = GenericArray::<u8, <Sha1 as OutputSizeUser>::OutputSize>::clone_from_slice(key);
        Ok(Self(qiniu_sdk::upload::SourceKey::from(arr)))
    }

    fn __str__(&self) -> String {
        hex::encode(&*self.0)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 断点恢复记录器
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ResumableRecorder(Box<dyn qiniu_sdk::upload::ResumableRecorder<HashAlgorithm = Sha1>>);

#[pymethods]
impl ResumableRecorder {
    /// 根据数据源 KEY 打开只读记录介质
    #[pyo3(text_signature = "($self, source_key)")]
    fn open_for_read(
        &self,
        source_key: &SourceKey,
        py: Python<'_>,
    ) -> PyResult<ReadOnlyResumableRecorderMedium> {
        py.allow_threads(|| {
            self.0
                .open_for_read(&source_key.0)
                .map(ReadOnlyResumableRecorderMedium::from)
                .map_err(QiniuIoError::from_err)
        })
    }

    /// 根据数据源 KEY 打开追加记录介质
    #[pyo3(text_signature = "($self, source_key)")]
    fn open_for_append(
        &self,
        source_key: &SourceKey,
        py: Python<'_>,
    ) -> PyResult<AppendOnlyResumableRecorderMedium> {
        py.allow_threads(|| {
            self.0
                .open_for_append(&source_key.0)
                .map(AppendOnlyResumableRecorderMedium::from)
                .map_err(QiniuIoError::from_err)
        })
    }

    /// 根据数据源 KEY 创建追加记录介质
    #[pyo3(text_signature = "($self, source_key)")]
    fn open_for_create_new(
        &self,
        source_key: &SourceKey,
        py: Python<'_>,
    ) -> PyResult<AppendOnlyResumableRecorderMedium> {
        py.allow_threads(|| {
            self.0
                .open_for_create_new(&source_key.0)
                .map(AppendOnlyResumableRecorderMedium::from)
                .map_err(QiniuIoError::from_err)
        })
    }

    /// 根据数据源 KEY 删除记录介质
    #[pyo3(text_signature = "($self, source_key)")]
    fn delete(&self, source_key: &SourceKey, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| self.0.delete(&source_key.0).map_err(QiniuIoError::from_err))
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
                .map(ReadOnlyAsyncResumableRecorderMedium::from)
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
                .map(AppendOnlyAsyncResumableRecorderMedium::from)
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
                .map(AppendOnlyAsyncResumableRecorderMedium::from)
                .map_err(QiniuIoError::from_err)
        })
    }

    /// 根据数据源 KEY 异步删除记录介质
    #[pyo3(text_signature = "($self, source_key)")]
    fn async_delete<'p>(&self, source_key: SourceKey, py: Python<'p>) -> PyResult<&'p PyAny> {
        let recorder = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            recorder
                .async_delete(&source_key.0)
                .await
                .map_err(QiniuIoError::from_err)
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::upload::ResumableRecorder for ResumableRecorder {
    type HashAlgorithm = Sha1;

    fn open_for_read(
        &self,
        source_key: &qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> std::io::Result<Box<dyn qiniu_sdk::prelude::ReadOnlyResumableRecorderMedium>> {
        self.0.open_for_read(source_key)
    }

    fn open_for_append(
        &self,
        source_key: &qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> std::io::Result<Box<dyn qiniu_sdk::prelude::AppendOnlyResumableRecorderMedium>> {
        self.0.open_for_append(source_key)
    }

    fn open_for_create_new(
        &self,
        source_key: &qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> std::io::Result<Box<dyn qiniu_sdk::prelude::AppendOnlyResumableRecorderMedium>> {
        self.0.open_for_create_new(source_key)
    }

    fn delete(
        &self,
        source_key: &qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> std::io::Result<()> {
        self.0.delete(source_key)
    }

    fn open_for_async_read<'a>(
        &'a self,
        source_key: &'a qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> futures::future::BoxFuture<
        'a,
        std::io::Result<Box<dyn qiniu_sdk::prelude::ReadOnlyAsyncResumableRecorderMedium>>,
    > {
        self.0.open_for_async_read(source_key)
    }

    fn open_for_async_append<'a>(
        &'a self,
        source_key: &'a qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> futures::future::BoxFuture<
        'a,
        std::io::Result<Box<dyn qiniu_sdk::prelude::AppendOnlyAsyncResumableRecorderMedium>>,
    > {
        self.0.open_for_async_append(source_key)
    }

    fn open_for_async_create_new<'a>(
        &'a self,
        source_key: &'a qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> futures::future::BoxFuture<
        'a,
        std::io::Result<Box<dyn qiniu_sdk::prelude::AppendOnlyAsyncResumableRecorderMedium>>,
    > {
        self.0.open_for_async_create_new(source_key)
    }

    fn async_delete<'a>(
        &'a self,
        source_key: &'a qiniu_sdk::upload::SourceKey<Self::HashAlgorithm>,
    ) -> futures::future::BoxFuture<'a, std::io::Result<()>> {
        self.0.async_delete(source_key)
    }
}

/// 只读介质接口
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Debug)]
struct ReadOnlyResumableRecorderMedium(Box<dyn qiniu_sdk::upload::ReadOnlyResumableRecorderMedium>);

#[pymethods]
impl ReadOnlyResumableRecorderMedium {
    /// 读取响应体数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyBytes> {
        let mut buf = Vec::new();
        py.allow_threads(|| {
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut self.0).take(size).read_to_end(&mut buf)
            } else {
                self.0.read_to_end(&mut buf)
            }
            .map_err(PyIOError::new_err)
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// 读取所有响应体数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        self.read(-1, py)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl<M: qiniu_sdk::upload::ReadOnlyResumableRecorderMedium + 'static> From<M>
    for ReadOnlyResumableRecorderMedium
{
    fn from(medium: M) -> Self {
        Self(Box::new(medium))
    }
}

/// 追加介质接口
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Debug)]
struct AppendOnlyResumableRecorderMedium(
    Box<dyn qiniu_sdk::upload::AppendOnlyResumableRecorderMedium>,
);

#[pymethods]
impl AppendOnlyResumableRecorderMedium {
    /// 写入所有数据
    #[pyo3(text_signature = "($self, b, /)")]
    fn write(&mut self, b: &[u8], py: Python<'_>) -> PyResult<usize> {
        py.allow_threads(|| self.0.write_all(b).map_err(PyIOError::new_err))?;
        Ok(b.len())
    }

    /// 刷新数据
    #[pyo3(text_signature = "($self)")]
    fn flush(&mut self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| self.0.flush().map_err(PyIOError::new_err))?;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl<M: qiniu_sdk::upload::AppendOnlyResumableRecorderMedium + 'static> From<M>
    for AppendOnlyResumableRecorderMedium
{
    fn from(medium: M) -> Self {
        Self(Box::new(medium))
    }
}

/// 异步只读介质接口
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Debug)]
struct ReadOnlyAsyncResumableRecorderMedium(
    Arc<AsyncMutex<dyn qiniu_sdk::upload::ReadOnlyAsyncResumableRecorderMedium>>,
);

#[pymethods]
impl ReadOnlyAsyncResumableRecorderMedium {
    /// 异步读取响应体数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyAny> {
        let reader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut reader = reader.lock().await;
            let mut buf = Vec::new();
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut *reader).take(size).read_to_end(&mut buf).await
            } else {
                reader.read_to_end(&mut buf).await
            }
            .map_err(PyIOError::new_err)?;
            Python::with_gil(|py| Ok(PyBytes::new(py, &buf).to_object(py)))
        })
    }

    /// 异步所有读取响应体数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        self.read(-1, py)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl<M: qiniu_sdk::upload::ReadOnlyAsyncResumableRecorderMedium + 'static> From<M>
    for ReadOnlyAsyncResumableRecorderMedium
{
    fn from(medium: M) -> Self {
        Self(Arc::new(AsyncMutex::new(medium)))
    }
}

/// 异步追加介质接口
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Debug)]
struct AppendOnlyAsyncResumableRecorderMedium(
    Arc<AsyncMutex<dyn qiniu_sdk::upload::AppendOnlyAsyncResumableRecorderMedium>>,
);

#[pymethods]
impl AppendOnlyAsyncResumableRecorderMedium {
    /// 异步写入所有数据
    #[pyo3(text_signature = "($self, b, /)")]
    fn write<'a>(&mut self, b: Vec<u8>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let writer = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut writer = writer.lock().await;
            writer.write_all(&b).await.map_err(PyIOError::new_err)?;
            Ok(b.len())
        })
    }

    /// 异步刷新数据
    #[pyo3(text_signature = "($self)")]
    fn flush<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let writer = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut writer = writer.lock().await;
            writer.flush().await.map_err(PyIOError::new_err)?;
            Ok(())
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl<M: qiniu_sdk::upload::AppendOnlyAsyncResumableRecorderMedium + 'static> From<M>
    for AppendOnlyAsyncResumableRecorderMedium
{
    fn from(medium: M) -> Self {
        Self(Arc::new(AsyncMutex::new(medium)))
    }
}

/// 无断点恢复记录器
///
/// 实现了断点恢复记录器接口，但总是返回找不到记录
///
/// 通过 `DummyResumableRecorder()` 创建无断点恢复记录器
#[pyclass(extends = ResumableRecorder)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "()")]
struct DummyResumableRecorder;

#[pymethods]
impl DummyResumableRecorder {
    /// 创建无断点恢复记录器
    #[new]
    fn new() -> (Self, ResumableRecorder) {
        (
            Self,
            ResumableRecorder(Box::new(qiniu_sdk::upload::DummyResumableRecorder::new())),
        )
    }
}

/// 文件系统断点恢复记录器
///
/// 基于文件系统提供断点恢复记录功能
///
/// 通过 `FileSystemResumableRecorder(path = None)` 创建文件系统断点恢复记录器
#[pyclass(extends = ResumableRecorder)]
#[derive(Debug, Clone)]
#[pyo3(text_signature = "(/, path = None)")]
struct FileSystemResumableRecorder;

#[pymethods]
impl FileSystemResumableRecorder {
    /// 创建文件系统断点恢复记录器，传入一个目录路径用于储存断点记录
    #[new]
    #[args(path = "None")]
    fn new(path: Option<String>) -> (Self, ResumableRecorder) {
        let recorder = if let Some(path) = path {
            qiniu_sdk::upload::FileSystemResumableRecorder::new(path)
        } else {
            qiniu_sdk::upload::FileSystemResumableRecorder::default()
        };
        (Self, ResumableRecorder(Box::new(recorder)))
    }
}

macro_rules! impl_uploader {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            #[pyo3(
                text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
                py: Python<'_>,
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
                py.allow_threads(|| {
                    self.0
                        .upload_path(path, object_params)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                        .and_then(|v| convert_json_value_to_py_object(&v))
                })
            }

            #[pyo3(
                text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
                py: Python<'_>,
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
                py.allow_threads(|| {
                    self.0
                        .upload_reader(PythonIoBase::new(reader), object_params)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                        .and_then(|v| convert_json_value_to_py_object(&v))
                })
            }

            #[pyo3(
                text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
                text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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

            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }
        }
    }
}

/// 数据源接口
///
/// 抽象类
///
/// 提供上传所用的数据源
#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct DataSource(Box<dyn qiniu_sdk::upload::DataSource<Sha1>>);

#[pymethods]
impl DataSource {
    /// 数据源切片
    #[pyo3(text_signature = "($self, size)")]
    fn slice(&self, size: u64, py: Python<'_>) -> PyResult<Option<DataSourceReader>> {
        let part_size = qiniu_sdk::upload::PartSize::new(size).map_or_else(
            || Err(QiniuInvalidPartSize::new_err("part_size must not be zero")),
            Ok,
        )?;
        let reader = py
            .allow_threads(|| self.0.slice(part_size))
            .map_err(PyIOError::new_err)?
            .map(DataSourceReader);
        Ok(reader)
    }

    /// 获取数据源 KEY
    ///
    /// 用于区分不同的数据源
    #[pyo3(text_signature = "($self)")]
    fn source_key(&self, py: Python<'_>) -> PyResult<Option<SourceKey>> {
        py.allow_threads(|| self.0.source_key())
            .map(|s| s.map(SourceKey))
            .map_err(PyIOError::new_err)
    }

    /// 获取数据源大小
    #[pyo3(text_signature = "($self)")]
    fn total_size(&self, py: Python<'_>) -> PyResult<Option<u64>> {
        py.allow_threads(|| self.0.total_size())
            .map_err(PyIOError::new_err)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::upload::DataSource<Sha1> for DataSource {
    fn slice(
        &self,
        size: qiniu_sdk::upload::PartSize,
    ) -> std::io::Result<Option<qiniu_sdk::upload::DataSourceReader>> {
        self.0.slice(size)
    }

    fn source_key(&self) -> std::io::Result<Option<qiniu_sdk::upload::SourceKey<Sha1>>> {
        self.0.source_key()
    }

    fn total_size(&self) -> std::io::Result<Option<u64>> {
        self.0.total_size()
    }
}

/// 文件数据源
///
/// 基于一个文件实现了数据源接口
///
/// 通过 `FileDataSource(path)` 创建文件数据源
#[pyclass(extends = DataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(path)")]
struct FileDataSource;

#[pymethods]
impl FileDataSource {
    /// 创建文件数据源
    #[new]
    fn new(path: &str) -> (Self, DataSource) {
        (
            Self,
            DataSource(Box::new(qiniu_sdk::upload::FileDataSource::new(path))),
        )
    }
}

/// 不可寻址的数据源
///
/// 基于一个不可寻址的阅读器实现了数据源接口
///
/// 通过 `UnseekableDataSource(source)` 创建不可寻址的数据源
#[pyclass(extends = DataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(source)")]
struct UnseekableDataSource;

#[pymethods]
impl UnseekableDataSource {
    /// 创建不可寻址的数据源
    #[new]
    fn new(source: PyObject) -> (Self, DataSource) {
        (
            Self,
            DataSource(Box::new(qiniu_sdk::upload::UnseekableDataSource::new(
                PythonIoBase::new(source),
            ))),
        )
    }
}

/// 异步数据源接口
///
/// 抽象类
///
/// 提供上传所用的数据源
#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct AsyncDataSource(Box<dyn qiniu_sdk::upload::AsyncDataSource<Sha1>>);

#[pymethods]
impl AsyncDataSource {
    /// 异步数据源切片
    #[pyo3(text_signature = "($self, size)")]
    fn slice<'p>(&self, size: u64, py: Python<'p>) -> PyResult<&'p PyAny> {
        let part_size = qiniu_sdk::upload::PartSize::new(size).map_or_else(
            || Err(QiniuInvalidPartSize::new_err("part_size must not be zero")),
            Ok,
        )?;
        let source = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            source
                .slice(part_size)
                .await
                .map(|r| r.map(|r| AsyncDataSourceReader(Arc::new(AsyncMutex::new(r)))))
                .map_err(PyIOError::new_err)
        })
    }

    /// 异步获取数据源 KEY
    ///
    /// 用于区分不同的数据源
    #[pyo3(text_signature = "($self)")]
    fn source_key<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let source = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            source
                .source_key()
                .await
                .map_err(PyIOError::new_err)
                .map(|k| k.map(SourceKey))
        })
    }

    /// 异步获取数据源大小
    #[pyo3(text_signature = "($self)")]
    fn total_size<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let source = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            source.total_size().await.map_err(PyIOError::new_err)
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::upload::AsyncDataSource<Sha1> for AsyncDataSource {
    fn slice(
        &self,
        size: qiniu_sdk::upload::PartSize,
    ) -> futures::future::BoxFuture<std::io::Result<Option<qiniu_sdk::upload::AsyncDataSourceReader>>>
    {
        self.0.slice(size)
    }

    fn source_key(
        &self,
    ) -> futures::future::BoxFuture<std::io::Result<Option<qiniu_sdk::upload::SourceKey<Sha1>>>>
    {
        self.0.source_key()
    }

    fn total_size(&self) -> futures::future::BoxFuture<std::io::Result<Option<u64>>> {
        self.0.total_size()
    }
}

/// 异步文件数据源
///
/// 基于一个文件实现了数据源接口
///
/// 通过 `AsyncFileDataSource(path)` 创建异步文件数据源
#[pyclass(extends = AsyncDataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(path)")]
struct AsyncFileDataSource;

#[pymethods]
impl AsyncFileDataSource {
    /// 创建异步文件数据源
    #[new]
    fn new(path: &str) -> (Self, AsyncDataSource) {
        (
            Self,
            AsyncDataSource(Box::new(qiniu_sdk::upload::AsyncFileDataSource::new(path))),
        )
    }
}

/// 不可寻址的异步数据源
///
/// 基于一个不可寻址的异步阅读器实现了异步数据源接口
///
/// 通过 `AsyncUnseekableDataSource(source)` 创建不可寻址的异步数据源
#[pyclass(extends = AsyncDataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(source)")]
struct AsyncUnseekableDataSource;

#[pymethods]
impl AsyncUnseekableDataSource {
    /// 创建不可寻址的异步数据源
    #[new]
    fn new(source: PyObject) -> (Self, AsyncDataSource) {
        (
            Self,
            AsyncDataSource(Box::new(qiniu_sdk::upload::AsyncUnseekableDataSource::new(
                PythonIoBase::new(source).into_async_read(),
            ))),
        )
    }
}

/// 追加介质接口
///
/// 抽象类
#[pyclass]
#[derive(Debug)]
struct DataSourceReader(qiniu_sdk::upload::DataSourceReader);

#[pymethods]
impl DataSourceReader {
    /// 读取响应体数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyBytes> {
        let mut buf = Vec::new();
        py.allow_threads(|| {
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut self.0).take(size).read_to_end(&mut buf)
            } else {
                self.0.read_to_end(&mut buf)
            }
            .map_err(PyIOError::new_err)
        })?;
        Ok(PyBytes::new(py, &buf))
    }

    /// 读取所有响应体数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        self.read(-1, py)
    }

    /// 从头读取数据
    #[pyo3(text_signature = "($self)")]
    fn reset(&mut self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| self.0.reset())
            .map_err(PyIOError::new_err)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 异步只读介质接口
///
/// 抽象类
#[pyclass]
#[derive(Debug)]
struct AsyncDataSourceReader(Arc<AsyncMutex<qiniu_sdk::upload::AsyncDataSourceReader>>);

#[pymethods]
impl AsyncDataSourceReader {
    /// 异步读取响应体数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyAny> {
        let reader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut reader = reader.lock().await;
            let mut buf = Vec::new();
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut *reader).take(size).read_to_end(&mut buf).await
            } else {
                reader.read_to_end(&mut buf).await
            }
            .map_err(PyIOError::new_err)?;
            Python::with_gil(|py| Ok(PyBytes::new(py, &buf).to_object(py)))
        })
    }

    /// 异步所有读取响应体数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        self.read(-1, py)
    }

    /// 从头读取数据
    #[pyo3(text_signature = "($self)")]
    fn reset<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let reader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            reader
                .lock()
                .await
                .reset()
                .await
                .map_err(PyIOError::new_err)
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 上传管理器
///
/// 通过 `UploadManager(signer, http_client = None, use_https = None, queryer = None, uc_endpoints = None)` 创建上传管理器
#[pyclass]
#[derive(Debug, Clone)]
#[pyo3(
    text_signature = "(signer, /, http_client = None, use_https = None, queryer = None, uc_endpoints = None)"
)]
struct UploadManager(qiniu_sdk::upload::UploadManager);

#[pymethods]
impl UploadManager {
    /// 创建上传管理器
    #[new]
    #[args(
        http_client = "None",
        use_https = "None",
        queryer = "None",
        uc_endpoints = "None"
    )]
    fn new(
        signer: UploadTokenSigner,
        http_client: Option<HttpClient>,
        use_https: Option<bool>,
        queryer: Option<BucketRegionsQueryer>,
        uc_endpoints: Option<Endpoints>,
    ) -> Self {
        let mut builder = qiniu_sdk::upload::UploadManager::builder(signer.0);
        if let Some(http_client) = http_client {
            builder.http_client(http_client.into());
        }
        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        if let Some(queryer) = queryer {
            builder.queryer(queryer.into());
        }
        if let Some(uc_endpoints) = uc_endpoints {
            builder.uc_endpoints(uc_endpoints);
        }
        Self(builder.build())
    }

    /// 创建表单上传器
    #[pyo3(
        text_signature = "($self, /, before_request = None, upload_progress = None, response_ok = None, response_error = None)"
    )]
    #[args(
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None"
    )]
    fn form_uploader(
        &self,
        before_request: Option<PyObject>,
        upload_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
    ) -> FormUploader {
        let mut uploader = self.0.form_uploader();
        if let Some(before_request) = before_request {
            uploader.on_before_request(on_before_request(before_request));
        }
        if let Some(upload_progress) = upload_progress {
            uploader.on_upload_progress(on_upload_progress(upload_progress));
        }
        if let Some(response_ok) = response_ok {
            uploader.on_response_ok(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            uploader.on_response_error(on_error(response_error));
        }
        FormUploader(uploader)
    }

    /// 创建分片上传器 V1
    #[pyo3(
        text_signature = "($self, resumable_recorder, /, before_request = None, upload_progress = None, response_ok = None, response_error = None, part_uploaded = None)"
    )]
    #[args(
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None",
        part_uploaded = "None"
    )]
    fn multi_parts_v1_uploader(
        &self,
        resumable_recorder: ResumableRecorder,
        before_request: Option<PyObject>,
        upload_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        part_uploaded: Option<PyObject>,
    ) -> MultiPartsV1Uploader {
        let mut uploader = self.0.multi_parts_v1_uploader(resumable_recorder);
        if let Some(before_request) = before_request {
            uploader.on_before_request(on_before_request(before_request));
        }
        if let Some(upload_progress) = upload_progress {
            uploader.on_upload_progress(on_upload_progress(upload_progress));
        }
        if let Some(response_ok) = response_ok {
            uploader.on_response_ok(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            uploader.on_response_error(on_error(response_error));
        }
        if let Some(part_uploaded) = part_uploaded {
            uploader.on_part_uploaded(on_part_uploaded(part_uploaded));
        }
        MultiPartsV1Uploader(uploader)
    }

    /// 创建分片上传器 V2
    #[pyo3(
        text_signature = "($self, resumable_recorder, /, before_request = None, upload_progress = None, response_ok = None, response_error = None, part_uploaded = None)"
    )]
    #[args(
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None",
        part_uploaded = "None"
    )]

    fn multi_parts_v2_uploader(
        &self,
        resumable_recorder: ResumableRecorder,
        before_request: Option<PyObject>,
        upload_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        part_uploaded: Option<PyObject>,
    ) -> MultiPartsV2Uploader {
        let mut uploader = self.0.multi_parts_v2_uploader(resumable_recorder);
        if let Some(before_request) = before_request {
            uploader.on_before_request(on_before_request(before_request));
        }
        if let Some(upload_progress) = upload_progress {
            uploader.on_upload_progress(on_upload_progress(upload_progress));
        }
        if let Some(response_ok) = response_ok {
            uploader.on_response_ok(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            uploader.on_response_error(on_error(response_error));
        }
        if let Some(part_uploaded) = part_uploaded {
            uploader.on_part_uploaded(on_part_uploaded(part_uploaded));
        }
        MultiPartsV2Uploader(uploader)
    }

    /// 创建自动上传器
    #[pyo3(
        text_signature = "($self, /, concurrency_provider = None, data_partition_provider = None, resumable_recorder = None, resumable_policy_provider = None, before_request = None, upload_progress = None, response_ok = None, response_error = None, part_uploaded = None)"
    )]
    #[args(
        concurrency_provider = "None",
        data_partition_provider = "None",
        resumable_recorder = "None",
        resumable_policy_provider = "None",
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None",
        part_uploaded = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn auto_uploader(
        &self,
        concurrency_provider: Option<ConcurrencyProvider>,
        data_partition_provider: Option<DataPartitionProvider>,
        resumable_recorder: Option<ResumableRecorder>,
        resumable_policy_provider: Option<ResumablePolicyProvider>,
        before_request: Option<PyObject>,
        upload_progress: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        part_uploaded: Option<PyObject>,
    ) -> AutoUploader {
        let mut builder = self.0.auto_uploader_builder();
        if let Some(concurrency_provider) = concurrency_provider {
            builder.concurrency_provider(concurrency_provider);
        }
        if let Some(data_partition_provider) = data_partition_provider {
            builder.data_partition_provider(data_partition_provider);
        }
        if let Some(resumable_recorder) = resumable_recorder {
            builder.resumable_recorder(resumable_recorder);
        }
        if let Some(resumable_policy_provider) = resumable_policy_provider {
            builder.resumable_policy_provider(resumable_policy_provider);
        }
        let mut uploader = builder.build();
        if let Some(before_request) = before_request {
            uploader.on_before_request(on_before_request(before_request));
        }
        if let Some(upload_progress) = upload_progress {
            uploader.on_upload_progress(on_upload_progress(upload_progress));
        }
        if let Some(response_ok) = response_ok {
            uploader.on_response_ok(on_response(response_ok));
        }
        if let Some(response_error) = response_error {
            uploader.on_response_error(on_error(response_error));
        }
        if let Some(part_uploaded) = part_uploaded {
            uploader.on_part_uploaded(on_part_uploaded(part_uploaded));
        }
        AutoUploader(uploader)
    }
}

/// 表单上传器
///
/// 通过七牛表单上传 API 一次上传整个数据流
///
/// 通过 `upload_manager.form_uploader()` 创建表单上传器
#[pyclass]
#[derive(Debug, Clone)]
struct FormUploader(qiniu_sdk::upload::FormUploader);

impl_uploader!(FormUploader);

macro_rules! impl_multi_parts_uploader {
    ($name:ident, $initialized_parts:ident, $async_initialize_parts:ident, $uploaded_part:ident, $async_uploaded_part:ident) => {
        #[pymethods]
        impl $name {
            /// 初始化分片信息
            ///
            /// 该步骤只负责初始化分片，但不实际上传数据，如果提供了有效的断点续传记录器，则可以尝试在这一步找到记录。
            #[pyo3(
                text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
            fn initialize_parts(
                &self,
                source: DataSource,
                region_provider: Option<RegionsProvider>,
                object_name: Option<&str>,
                file_name: Option<&str>,
                content_type: Option<&str>,
                metadata: Option<HashMap<String, String>>,
                custom_vars: Option<HashMap<String, String>>,
                uploaded_part_ttl_secs: Option<u64>,
                py: Python<'_>,
            ) -> PyResult<$initialized_parts> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
                    uploaded_part_ttl_secs,
                )?;
                py.allow_threads(|| {
                    self.0
                        .initialize_parts(source, object_params)
                        .map($initialized_parts)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// 上传分片
            ///
            /// 实际上传的分片大小由提供的分片大小提供者获取。
            ///
            /// 如果返回 `None` 则表示已经没有更多分片可以上传。
            #[pyo3(text_signature = "($self, initialized, data_partitioner_provider)")]
            fn upload_part(
                &self,
                initialized: &$initialized_parts,
                data_partitioner_provider: &DataPartitionProvider,
                py: Python<'_>,
            ) -> PyResult<Option<$uploaded_part>> {
                py.allow_threads(|| {
                    self.0
                        .upload_part(&initialized.0, data_partitioner_provider)
                        .map(|p| p.map($uploaded_part))
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// 完成分片上传
            ///
            /// 在这步成功返回后，对象即可被读取。
            #[pyo3(text_signature = "($self, initialized, parts)")]
            fn complete_part(
                &self,
                initialized: &$initialized_parts,
                parts: Vec<$uploaded_part>,
                py: Python<'_>,
            ) -> PyResult<PyObject> {
                py.allow_threads(|| {
                    self.0
                        .complete_parts(
                            &initialized.0,
                            &parts.into_iter().map(|part| part.0).collect::<Vec<_>>(),
                        )
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                        .and_then(|s| convert_json_value_to_py_object(&s))
                })
            }

            /// 异步初始化分片信息
            ///
            /// 该步骤只负责初始化分片，但不实际上传数据，如果提供了有效的断点续传记录器，则可以尝试在这一步找到记录。
            #[pyo3(
                text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
            fn async_initialize_parts<'p>(
                &self,
                source: AsyncDataSource,
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
                        .async_initialize_parts(source, object_params)
                        .await
                        .map(|obj| $async_initialize_parts(Arc::new(obj)))
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// 异步上传分片
            ///
            /// 实际上传的分片大小由提供的分片大小提供者获取。
            ///
            /// 如果返回 `None` 则表示已经没有更多分片可以上传。
            #[pyo3(text_signature = "($self, initialized, data_partitioner_provider)")]
            fn async_upload_part<'p>(
                &self,
                initialized: &$async_initialize_parts,
                data_partitioner_provider: &DataPartitionProvider,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let uploader = self.0.to_owned();
                let initialized = initialized.0.to_owned();
                let data_partitioner_provider = data_partitioner_provider.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    uploader
                        .async_upload_part(&initialized, &data_partitioner_provider)
                        .await
                        .map(|p| p.map($async_uploaded_part))
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// 异步完成分片上传
            ///
            /// 在这步成功返回后，对象即可被读取。
            #[pyo3(text_signature = "($self, initialized, parts)")]
            fn async_complete_part<'p>(
                &'p self,
                initialized: &'p $async_initialize_parts,
                parts: Vec<$async_uploaded_part>,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let uploader = self.0.to_owned();
                let initialized = initialized.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    uploader
                        .async_complete_parts(
                            &initialized,
                            &parts.into_iter().map(|part| part.0).collect::<Vec<_>>(),
                        )
                        .await
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                        .and_then(|s| convert_json_value_to_py_object(&s))
                })
            }

            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }
        }
    };
}

/// 分片上传器 V1
///
/// 不推荐直接使用这个上传器，而是可以借助 `MultiPartsUploaderScheduler` 来方便地实现分片上传。
///
/// 通过 `upload_manager.multi_parts_v1_uploader()` 创建分片上传器 V1
#[pyclass]
#[derive(Debug, Clone)]
struct MultiPartsV1Uploader(qiniu_sdk::upload::MultiPartsV1Uploader);

impl_multi_parts_uploader!(
    MultiPartsV1Uploader,
    MultiPartsV1UploaderInitializedObject,
    AsyncMultiPartsV1UploaderInitializedObject,
    MultiPartsV1UploaderUploadedPart,
    AsyncMultiPartsV1UploaderUploadedPart
);

/// 分片上传器 V2
///
/// 不推荐直接使用这个上传器，而是可以借助 `MultiPartsUploaderScheduler` 来方便地实现分片上传。
///
/// 通过 `upload_manager.multi_parts_v2_uploader()` 创建分片上传器 V2
#[pyclass]
#[derive(Debug, Clone)]
struct MultiPartsV2Uploader(qiniu_sdk::upload::MultiPartsV2Uploader);

impl_multi_parts_uploader!(
    MultiPartsV2Uploader,
    MultiPartsV2UploaderInitializedObject,
    AsyncMultiPartsV2UploaderInitializedObject,
    MultiPartsV2UploaderUploadedPart,
    AsyncMultiPartsV2UploaderUploadedPart
);

macro_rules! impl_initialized_object {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// 获取对象名称
            #[getter]
            fn get_object_name(&self) -> Option<&str> {
                self.0.params().object_name()
            }

            /// 获取文件名称
            #[getter]
            fn get_file_name(&self) -> Option<&str> {
                self.0.params().file_name()
            }

            /// 获取 MIME 类型
            #[getter]
            fn get_content_type(&self) -> Option<&str> {
                self.0.params().content_type().map(|s| s.as_ref())
            }

            /// 获取对象元信息
            #[getter]
            fn get_metadata(&self) -> HashMap<String, String> {
                self.0.params().metadata().to_owned()
            }

            /// 获取对象自定义变量
            #[getter]
            fn get_custom_vars(&self) -> HashMap<String, String> {
                self.0.params().custom_vars().to_owned()
            }

            /// 获取分片上传后的有效期
            #[getter]
            fn get_uploaded_part_ttl_secs(&self) -> u64 {
                self.0.params().uploaded_part_ttl().as_secs()
            }

            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }
        }
    };
}

/// 被 分片上传器 V1 初始化的分片信息
///
/// 通过 `multi_parts_uploader_v1.initialize_parts()` 创建
#[pyclass]
struct MultiPartsV1UploaderInitializedObject(
    <qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::InitializedParts,
);
impl_initialized_object!(MultiPartsV1UploaderInitializedObject);

/// 被 分片上传器 V1 异步初始化的分片信息
///
/// 通过 `multi_parts_uploader_v1.async_initialize_parts()` 创建
#[pyclass]
#[derive(Clone)]
struct AsyncMultiPartsV1UploaderInitializedObject(
    Arc<<qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncInitializedParts>,
);
impl_initialized_object!(AsyncMultiPartsV1UploaderInitializedObject);

/// 被 分片上传器 V2 初始化的分片信息
///
/// 通过 `multi_parts_uploader_v2.initialize_parts()` 创建
#[pyclass]
struct MultiPartsV2UploaderInitializedObject(
    <qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::InitializedParts,
);
impl_initialized_object!(MultiPartsV2UploaderInitializedObject);

/// 被 分片上传器 V2 异步初始化的分片信息
///
/// 通过 `multi_parts_uploader_v2.async_initialize_parts()` 创建
#[pyclass]
#[derive(Clone)]
struct AsyncMultiPartsV2UploaderInitializedObject(
    Arc<<qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncInitializedParts>,
);
impl_initialized_object!(AsyncMultiPartsV2UploaderInitializedObject);

macro_rules! impl_uploaded_part {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// 分片大小
            #[getter]
            fn get_size(&self) -> u64 {
                self.0.size().get()
            }

            /// 分片偏移量
            #[getter]
            fn get_offset(&self) -> u64 {
                self.0.offset()
            }

            /// 是否来自于断点恢复
            #[getter]
            fn get_resumed(&self) -> bool {
                self.0.resumed()
            }

            /// 获取响应体
            #[getter]
            fn get_response_body(&self) -> PyResult<PyObject> {
                convert_json_value_to_py_object(self.0.response_body().as_ref())
            }

            fn __repr__(&self) -> String {
                format!("{:?}", self.0)
            }

            fn __str__(&self) -> String {
                self.__repr__()
            }
        }
    };
}

/// 已经通过 分片上传器 V1 上传的分片信息
///
/// 通过 `multi_parts_uploader_v1.upload_part()` 创建
#[pyclass]
#[derive(Clone, Debug)]
struct MultiPartsV1UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::UploadedPart);
impl_uploaded_part!(MultiPartsV1UploaderUploadedPart);

/// 已经通过 分片上传器 V1 异步上传的分片信息
///
/// 通过 `multi_parts_uploader_v1.async_upload_part()` 创建
#[pyclass]
#[derive(Clone, Debug)]
struct AsyncMultiPartsV1UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncUploadedPart);
impl_uploaded_part!(AsyncMultiPartsV1UploaderUploadedPart);

/// 已经通过 分片上传器 V2 上传的分片信息
///
/// 通过 `multi_parts_uploader_v2.upload_part()` 创建
#[pyclass]
#[derive(Clone, Debug)]
struct MultiPartsV2UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::UploadedPart);
impl_uploaded_part!(MultiPartsV2UploaderUploadedPart);

/// 已经通过 分片上传器 V2 异步上传的分片信息
///
/// 通过 `multi_parts_uploader_v2.async_upload_part()` 创建
#[pyclass]
#[derive(Clone, Debug)]
struct AsyncMultiPartsV2UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncUploadedPart);
impl_uploaded_part!(AsyncMultiPartsV2UploaderUploadedPart);

/// 分片上传调度器接口
///
/// 抽象类
///
/// 负责分片上传的调度，包括初始化分片信息、上传分片、完成分片上传。
#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct MultiPartsUploaderScheduler(Box<dyn qiniu_sdk::upload::MultiPartsUploaderScheduler<Sha1>>);

#[pymethods]
impl MultiPartsUploaderScheduler {
    /// 设置并发数提供者
    #[setter]
    fn set_concurrency_provider(&mut self, concurrency_provider: ConcurrencyProvider) {
        self.0.set_concurrency_provider(concurrency_provider.0);
    }

    /// 设置分片大小提供者
    #[setter]
    fn set_data_partition_provider(&mut self, data_partition_provider: DataPartitionProvider) {
        self.0
            .set_data_partition_provider(data_partition_provider.0);
    }

    /// 上传数据源
    #[pyo3(
        text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
    fn upload(
        &self,
        source: DataSource,
        region_provider: Option<RegionsProvider>,
        object_name: Option<&str>,
        file_name: Option<&str>,
        content_type: Option<&str>,
        metadata: Option<HashMap<String, String>>,
        custom_vars: Option<HashMap<String, String>>,
        uploaded_part_ttl_secs: Option<u64>,
        py: Python<'_>,
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
        py.allow_threads(|| {
            self.0
                .upload(source.0, object_params)
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }

    /// 异步上传数据源
    #[pyo3(
        text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None)"
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
    fn async_upload<'p>(
        &'p self,
        source: AsyncDataSource,
        region_provider: Option<RegionsProvider>,
        object_name: Option<&str>,
        file_name: Option<&str>,
        content_type: Option<&str>,
        metadata: Option<HashMap<String, String>>,
        custom_vars: Option<HashMap<String, String>>,
        uploaded_part_ttl_secs: Option<u64>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let scheduler = self.0.to_owned();
        let object_params = make_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
        )?;
        pyo3_asyncio::async_std::future_into_py(py, async move {
            scheduler
                .async_upload(source.0, object_params)
                .await
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }
}
impl_uploader!(MultiPartsUploaderScheduler);

/// 串行分片上传调度器
///
/// 不启动任何线程，仅在本地串行上传分片。
///
/// 通过 `SerialMultiPartsUploaderScheduler(multi_parts_uploader)` 创建串行分片上传调度器
#[pyclass(extends = MultiPartsUploaderScheduler)]
#[derive(Debug, Copy, Clone)]
#[pyo3(text_signature = "(uploader)")]
struct SerialMultiPartsUploaderScheduler;

#[pymethods]
impl SerialMultiPartsUploaderScheduler {
    /// 创建串行分片上传调度器
    #[new]
    fn new(uploader: PyObject, py: Python<'_>) -> PyResult<(Self, MultiPartsUploaderScheduler)> {
        let scheduler = if let Ok(uploader_v1) = uploader.extract::<MultiPartsV1Uploader>(py) {
            Box::new(qiniu_sdk::upload::SerialMultiPartsUploaderScheduler::new(
                uploader_v1.0,
            )) as Box<dyn qiniu_sdk::upload::MultiPartsUploaderScheduler<Sha1>>
        } else {
            let uploader_v2 = uploader.extract::<MultiPartsV2Uploader>(py)?;
            Box::new(qiniu_sdk::upload::SerialMultiPartsUploaderScheduler::new(
                uploader_v2.0,
            )) as Box<dyn qiniu_sdk::upload::MultiPartsUploaderScheduler<Sha1>>
        };
        Ok((Self, MultiPartsUploaderScheduler(scheduler)))
    }
}

/// 并行分片上传调度器
///
/// 在阻塞模式下创建线程池负责上传分片，在异步模式下使用 `async-std` 的线程池负责上传分片。
///
/// 通过 `ConcurrentMultiPartsUploaderScheduler(multi_parts_uploader)` 创建串行分片上传调度器
#[pyclass(extends = MultiPartsUploaderScheduler)]
#[derive(Debug, Copy, Clone)]
#[pyo3(text_signature = "(uploader)")]
struct ConcurrentMultiPartsUploaderScheduler;

#[pymethods]
impl ConcurrentMultiPartsUploaderScheduler {
    /// 创建串行分片上传调度器
    #[new]
    fn new(uploader: PyObject, py: Python<'_>) -> PyResult<(Self, MultiPartsUploaderScheduler)> {
        let scheduler = if let Ok(uploader_v1) = uploader.extract::<MultiPartsV1Uploader>(py) {
            Box::new(qiniu_sdk::upload::ConcurrentMultiPartsUploaderScheduler::new(uploader_v1.0))
                as Box<dyn qiniu_sdk::upload::MultiPartsUploaderScheduler<Sha1>>
        } else {
            let uploader_v2 = uploader.extract::<MultiPartsV2Uploader>(py)?;
            Box::new(qiniu_sdk::upload::ConcurrentMultiPartsUploaderScheduler::new(uploader_v2.0))
                as Box<dyn qiniu_sdk::upload::MultiPartsUploaderScheduler<Sha1>>
        };
        Ok((Self, MultiPartsUploaderScheduler(scheduler)))
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

fn on_before_request(
    callback: PyObject,
) -> impl Fn(&mut qiniu_sdk::http_client::RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'static
{
    move |parts| {
        Python::with_gil(|py| callback.call1(py, (RequestBuilderPartsRef::new(parts),)))?;
        Ok(())
    }
}

/// 上传进度信息
///
/// 通过 `UploadingProgressInfo(transferred_bytes, total_bytes = None)` 创建上传进度信息
#[pyclass]
#[derive(Clone, Copy, Debug)]
#[pyo3(text_signature = "(transferred_bytes, /, total_bytes = None)")]
struct UploadingProgressInfo(qiniu_sdk::upload::UploadingProgressInfo);

#[pymethods]
impl UploadingProgressInfo {
    #[new]
    #[args(total_bytes = "None")]
    fn new(transferred_bytes: u64, total_bytes: Option<u64>) -> Self {
        Self(qiniu_sdk::upload::UploadingProgressInfo::new(
            transferred_bytes,
            total_bytes,
        ))
    }

    /// 获取已经传输的数据量
    ///
    /// 单位为字节
    #[getter]
    fn get_transferred_bytes(&self) -> u64 {
        self.0.transferred_bytes()
    }

    /// 获取总共需要传输的数据量
    ///
    /// 单位为字节
    #[getter]
    fn get_total_bytes(&self) -> Option<u64> {
        self.0.total_bytes()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ToPyObject for UploadingProgressInfo {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.to_owned().into_py(py)
    }
}

/// 已经上传的分片信息
///
/// 该类型没有构造函数，仅限于在回调函数中使用
#[pyclass]
#[derive(Clone, Copy, Debug)]
struct UploadedPartInfo {
    size: NonZeroU64,
    offset: u64,
    resumed: bool,
}

#[pymethods]
impl UploadedPartInfo {
    #[getter]
    fn get_size(&self) -> u64 {
        self.size.get()
    }

    #[getter]
    fn get_offset(&self) -> u64 {
        self.offset
    }

    #[getter]
    fn get_resumed(&self) -> bool {
        self.resumed
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

fn on_upload_progress(
    callback: PyObject,
) -> impl Fn(&qiniu_sdk::upload::UploadingProgressInfo) -> AnyResult<()> + Send + Sync + 'static {
    move |progress| {
        Python::with_gil(|py| {
            callback.call1(
                py,
                (UploadingProgressInfo::new(
                    progress.transferred_bytes(),
                    progress.total_bytes(),
                ),),
            )
        })?;
        Ok(())
    }
}

fn on_response(
    callback: PyObject,
) -> impl Fn(&mut qiniu_sdk::http::ResponseParts) -> AnyResult<()> + Send + Sync + 'static {
    move |parts| {
        let parts = HttpResponsePartsMut::from(parts);
        Python::with_gil(|py| callback.call1(py, (parts,)))?;
        Ok(())
    }
}

fn on_error(
    callback: PyObject,
) -> impl Fn(&qiniu_sdk::http_client::ResponseError) -> AnyResult<()> + Send + Sync + 'static {
    move |error| {
        #[allow(unsafe_code)]
        let error: &'static qiniu_sdk::http_client::ResponseError = unsafe { transmute(error) };
        let error = QiniuApiCallError::from_err(MaybeOwned::Borrowed(error));
        let error = convert_api_call_error(&error)?;
        Python::with_gil(|py| callback.call1(py, (error,)))?;
        Ok(())
    }
}

fn on_part_uploaded(
    callback: PyObject,
) -> impl Fn(&dyn UploadedPart) -> AnyResult<()> + Send + Sync + 'static {
    move |part| {
        let part = UploadedPartInfo {
            size: part.size(),
            offset: part.offset(),
            resumed: part.resumed(),
        };
        Python::with_gil(|py| callback.call1(py, (part,)))?;
        Ok(())
    }
}

/// 期望的分片上传调度器
#[pyclass]
#[derive(Debug, Clone)]
enum MultiPartsUploaderSchedulerPrefer {
    /// 串行上传调度器
    Serial = 0,
    /// 并行上传调度器
    Concurrent = 1,
}

#[pymethods]
impl MultiPartsUploaderSchedulerPrefer {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<MultiPartsUploaderSchedulerPrefer>
    for qiniu_sdk::upload::MultiPartsUploaderSchedulerPrefer
{
    fn from(prefer: MultiPartsUploaderSchedulerPrefer) -> Self {
        match prefer {
            MultiPartsUploaderSchedulerPrefer::Serial => {
                qiniu_sdk::upload::MultiPartsUploaderSchedulerPrefer::Serial
            }
            MultiPartsUploaderSchedulerPrefer::Concurrent => {
                qiniu_sdk::upload::MultiPartsUploaderSchedulerPrefer::Concurrent
            }
        }
    }
}

/// 期望的对象单请求上传器
#[pyclass]
#[derive(Debug, Clone)]
enum SinglePartUploaderPrefer {
    /// 表单上传器
    Form = 0,
}

#[pymethods]
impl SinglePartUploaderPrefer {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<SinglePartUploaderPrefer> for qiniu_sdk::upload::SinglePartUploaderPrefer {
    fn from(prefer: SinglePartUploaderPrefer) -> Self {
        match prefer {
            SinglePartUploaderPrefer::Form => qiniu_sdk::upload::SinglePartUploaderPrefer::Form,
        }
    }
}

/// 期望的对象分片上传器
#[pyclass]
#[derive(Debug, Clone)]
enum MultiPartsUploaderPrefer {
    /// 分片上传器 V1
    V1 = 1,
    /// 分片上传器 V2
    V2 = 2,
}

#[pymethods]
impl MultiPartsUploaderPrefer {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<MultiPartsUploaderPrefer> for qiniu_sdk::upload::MultiPartsUploaderPrefer {
    fn from(prefer: MultiPartsUploaderPrefer) -> Self {
        match prefer {
            MultiPartsUploaderPrefer::V1 => qiniu_sdk::upload::MultiPartsUploaderPrefer::V1,
            MultiPartsUploaderPrefer::V2 => qiniu_sdk::upload::MultiPartsUploaderPrefer::V2,
        }
    }
}

/// 自动上传器
///
/// 使用设置的各种提供者，将文件或是二进制流数据上传。
///
/// 通过 `upload_manager.auto_uploader()` 创建自动上传器
#[pyclass]
#[derive(Debug, Clone)]
struct AutoUploader(qiniu_sdk::upload::AutoUploader);

#[pymethods]
impl AutoUploader {
    #[pyo3(
        text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None",
        multi_parts_uploader_scheduler_prefer = "None",
        single_part_uploader_prefer = "None",
        multi_parts_uploader_prefer = "None"
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
        multi_parts_uploader_scheduler_prefer: Option<MultiPartsUploaderSchedulerPrefer>,
        single_part_uploader_prefer: Option<SinglePartUploaderPrefer>,
        multi_parts_uploader_prefer: Option<MultiPartsUploaderPrefer>,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let object_params = make_auto_uploader_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
            multi_parts_uploader_scheduler_prefer,
            single_part_uploader_prefer,
            multi_parts_uploader_prefer,
        )?;
        py.allow_threads(|| {
            self.0
                .upload_path(path, object_params)
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }

    #[pyo3(
        text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None",
        multi_parts_uploader_scheduler_prefer = "None",
        single_part_uploader_prefer = "None",
        multi_parts_uploader_prefer = "None"
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
        multi_parts_uploader_scheduler_prefer: Option<MultiPartsUploaderSchedulerPrefer>,
        single_part_uploader_prefer: Option<SinglePartUploaderPrefer>,
        multi_parts_uploader_prefer: Option<MultiPartsUploaderPrefer>,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let object_params = make_auto_uploader_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
            multi_parts_uploader_scheduler_prefer,
            single_part_uploader_prefer,
            multi_parts_uploader_prefer,
        )?;
        py.allow_threads(|| {
            self.0
                .upload_reader(PythonIoBase::new(reader), object_params)
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }

    #[pyo3(
        text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None",
        multi_parts_uploader_scheduler_prefer = "None",
        single_part_uploader_prefer = "None",
        multi_parts_uploader_prefer = "None"
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
        multi_parts_uploader_scheduler_prefer: Option<MultiPartsUploaderSchedulerPrefer>,
        single_part_uploader_prefer: Option<SinglePartUploaderPrefer>,
        multi_parts_uploader_prefer: Option<MultiPartsUploaderPrefer>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let object_params = make_auto_uploader_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
            multi_parts_uploader_scheduler_prefer,
            single_part_uploader_prefer,
            multi_parts_uploader_prefer,
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
        text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, uploaded_part_ttl_secs=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
        uploaded_part_ttl_secs = "None",
        multi_parts_uploader_scheduler_prefer = "None",
        single_part_uploader_prefer = "None",
        multi_parts_uploader_prefer = "None"
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
        multi_parts_uploader_scheduler_prefer: Option<MultiPartsUploaderSchedulerPrefer>,
        single_part_uploader_prefer: Option<SinglePartUploaderPrefer>,
        multi_parts_uploader_prefer: Option<MultiPartsUploaderPrefer>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let object_params = make_auto_uploader_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
            uploaded_part_ttl_secs,
            multi_parts_uploader_scheduler_prefer,
            single_part_uploader_prefer,
            multi_parts_uploader_prefer,
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

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[allow(clippy::too_many_arguments)]
fn make_auto_uploader_object_params(
    region_provider: Option<RegionsProvider>,
    object_name: Option<&str>,
    file_name: Option<&str>,
    content_type: Option<&str>,
    metadata: Option<HashMap<String, String>>,
    custom_vars: Option<HashMap<String, String>>,
    uploaded_part_ttl_secs: Option<u64>,
    multi_parts_uploader_scheduler_prefer: Option<MultiPartsUploaderSchedulerPrefer>,
    single_part_uploader_prefer: Option<SinglePartUploaderPrefer>,
    multi_parts_uploader_prefer: Option<MultiPartsUploaderPrefer>,
) -> PyResult<qiniu_sdk::upload::AutoUploaderObjectParams> {
    let mut builder = qiniu_sdk::upload::AutoUploaderObjectParams::builder();
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
    if let Some(multi_parts_uploader_scheduler_prefer) = multi_parts_uploader_scheduler_prefer {
        builder.multi_parts_uploader_scheduler_prefer(multi_parts_uploader_scheduler_prefer.into());
    }
    if let Some(single_part_uploader_prefer) = single_part_uploader_prefer {
        builder.single_part_uploader_prefer(single_part_uploader_prefer.into());
    }
    if let Some(multi_parts_uploader_prefer) = multi_parts_uploader_prefer {
        builder.multi_parts_uploader_prefer(multi_parts_uploader_prefer.into());
    }
    Ok(builder.build())
}

/// 数据阅读器
///
/// 通过 `resumable_policy_provider.get_policy_from_reader()` 创建
#[pyclass]
struct Reader(Box<dyn qiniu_sdk::upload::DynRead>);

#[pymethods]
impl Reader {
    /// 读取数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyBytes> {
        let mut buf = Vec::new();
        if let Ok(size) = u64::try_from(size) {
            buf.reserve(size as usize);
            (&mut self.0).take(size).read_to_end(&mut buf)
        } else {
            self.0.read_to_end(&mut buf)
        }
        .map_err(PyIOError::new_err)?;
        Ok(PyBytes::new(py, &buf))
    }

    /// 读取所有数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        self.read(-1, py)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl<T: Read + Debug + Sync + Send + 'static> From<T> for Reader {
    fn from(reader: T) -> Self {
        Self(Box::new(reader))
    }
}

/// 异步数据阅读器
///
/// 通过 `resumable_policy_provider.get_policy_from_async_reader()` 创建
#[pyclass]

struct AsyncReader(Arc<AsyncMutex<dyn qiniu_sdk::upload::DynAsyncRead>>);

impl<T: AsyncRead + Unpin + Debug + Sync + Send + 'static> From<T> for AsyncReader {
    fn from(reader: T) -> Self {
        Self(Arc::new(AsyncMutex::new(reader)))
    }
}

#[pymethods]
impl AsyncReader {
    /// 异步读取数据
    #[pyo3(text_signature = "($self, size = -1, /)")]
    #[args(size = "-1")]
    fn read<'a>(&mut self, size: i64, py: Python<'a>) -> PyResult<&'a PyAny> {
        let reader = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut reader = reader.lock().await;
            let mut buf = Vec::new();
            if let Ok(size) = u64::try_from(size) {
                buf.reserve(size as usize);
                (&mut *reader).take(size).read_to_end(&mut buf).await
            } else {
                reader.read_to_end(&mut buf).await
            }
            .map_err(PyIOError::new_err)?;
            Python::with_gil(|py| Ok(PyBytes::new(py, &buf).to_object(py)))
        })
    }

    /// 异步所有读取数据
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        self.read(-1, py)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
