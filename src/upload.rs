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

/// ?????????????????????
///
/// ?????? `UploadTokenSigner.new_upload_token_provider(upload_token_provider)` ??? `UploadTokenSigner.new_credential_provider(credential, bucket_name, lifetime_secs, on_policy_generated = None)` ???????????????????????????
#[pyclass]
#[derive(Clone, Debug)]
struct UploadTokenSigner(qiniu_sdk::upload::UploadTokenSigner);

#[pymethods]
impl UploadTokenSigner {
    /// ??????????????????????????????????????????????????????
    #[staticmethod]
    #[pyo3(text_signature = "(upload_token_provider)")]
    fn new_upload_token_provider(upload_token_provider: UploadTokenProvider) -> Self {
        Self(qiniu_sdk::upload::UploadTokenSigner::new_upload_token_provider(upload_token_provider))
    }

    /// ???????????????????????????????????????????????????????????????????????????
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

/// ?????????????????????
///
/// ?????????
///
/// ?????????????????????????????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ConcurrencyProvider(Box<dyn qiniu_sdk::upload::ConcurrencyProvider>);

#[pymethods]
impl ConcurrencyProvider {
    /// ???????????????
    #[getter]
    fn get_concurrency(&self) -> usize {
        self.0.concurrency().as_usize()
    }

    /// ?????????????????????
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

/// ????????????????????????
///
/// ?????? `FixedConcurrencyProvider(concurrency)` ??????????????????????????????
#[pyclass(extends = ConcurrencyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(concurrency)")]
struct FixedConcurrencyProvider;

#[pymethods]
impl FixedConcurrencyProvider {
    /// ??????????????????????????????
    ///
    /// ???????????? `0` ???????????????
    #[new]
    fn new(concurrency: usize) -> PyResult<(Self, ConcurrencyProvider)> {
        let provider = qiniu_sdk::upload::FixedConcurrencyProvider::new(concurrency).map_or_else(
            || Err(QiniuInvalidConcurrency::new_err("Invalid concurrency")),
            Ok,
        )?;
        Ok((Self, ConcurrencyProvider(Box::new(provider))))
    }
}

/// ????????????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct DataPartitionProvider(Box<dyn qiniu_sdk::upload::DataPartitionProvider>);

#[pymethods]
impl DataPartitionProvider {
    /// ??????????????????
    #[getter]
    fn get_part_size(&self) -> u64 {
        self.0.part_size().as_u64()
    }

    /// ?????????????????????
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

/// ???????????????????????????
///
/// ?????? `FixedDataPartitionProvider(part_size)` ?????????????????????????????????
#[pyclass(extends = DataPartitionProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(part_size)")]
struct FixedDataPartitionProvider;

#[pymethods]
impl FixedDataPartitionProvider {
    /// ?????????????????????????????????
    ///
    /// ???????????? `0` ???????????????
    #[new]
    fn new(part_size: u64) -> PyResult<(Self, DataPartitionProvider)> {
        let provider = qiniu_sdk::upload::FixedDataPartitionProvider::new(part_size).map_or_else(
            || Err(QiniuInvalidPartSize::new_err("Invalid part size")),
            Ok,
        )?;
        Ok((Self, DataPartitionProvider(Box::new(provider))))
    }
}

/// ??????????????????????????????
///
/// ?????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
///
/// ?????? `MultiplyDataPartitionProvider(base, multiply)` ????????????????????????????????????
#[pyclass(extends = DataPartitionProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(base, multiply)")]
struct MultiplyDataPartitionProvider;

#[pymethods]
impl MultiplyDataPartitionProvider {
    /// ????????????????????????????????????
    ///
    /// ???????????? `0` ???????????????
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

/// ??????????????????????????????
///
/// ????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
///
/// ?????? `LimitedDataPartitionProvider(base, min, max)` ????????????????????????????????????
#[pyclass(extends = DataPartitionProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(base, min, max)")]
struct LimitedDataPartitionProvider;

#[pymethods]
impl LimitedDataPartitionProvider {
    /// ????????????????????????????????????
    ///
    /// ???????????? `0` ?????? `min` ??? `max` ???????????????
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

/// ???????????????
///
/// ??????????????????????????????????????????
#[pyclass]
#[derive(Debug, Copy, Clone)]
enum ResumablePolicy {
    /// ???????????????
    SinglePartUploading = 0,
    /// ????????????
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

/// ???????????????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ResumablePolicyProvider(Box<dyn qiniu_sdk::upload::ResumablePolicyProvider>);

#[pymethods]
impl ResumablePolicyProvider {
    /// ??????????????????????????????????????????
    #[pyo3(text_signature = "(source_size)")]
    fn get_policy_from_size(&self, source_size: u64, py: Python<'_>) -> ResumablePolicy {
        py.allow_threads(|| {
            self.0
                .get_policy_from_size(source_size, Default::default())
                .into()
        })
    }

    /// ????????????????????????????????????
    ///
    /// ???????????????????????????????????????????????????????????????
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

    /// ??????????????????????????????????????????
    ///
    /// ?????????????????????????????????????????????????????????????????????
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

/// ???????????????????????????
///
/// ?????? `AlwaysSinglePart()` ???????????????????????????
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct AlwaysSinglePart;

#[pymethods]
impl AlwaysSinglePart {
    /// ???????????????????????????
    #[new]
    fn new() -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(qiniu_sdk::upload::AlwaysSinglePart)),
        )
    }
}

/// ????????????????????????
///
/// ?????? `AlwaysMultiParts()` ????????????????????????
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "()")]
struct AlwaysMultiParts;

#[pymethods]
impl AlwaysMultiParts {
    /// ????????????????????????
    #[new]
    fn new() -> (Self, ResumablePolicyProvider) {
        (
            Self,
            ResumablePolicyProvider(Box::new(qiniu_sdk::upload::AlwaysMultiParts)),
        )
    }
}

/// ??????????????????????????????
///
/// ?????? `FixedThresholdResumablePolicy(threshold)` ????????????????????????????????????
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Copy, Clone, Debug)]
#[pyo3(text_signature = "(threshold)")]
struct FixedThresholdResumablePolicy;

#[pymethods]
impl FixedThresholdResumablePolicy {
    /// ????????????????????????????????????
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

/// ???????????????????????????????????????
///
/// ????????????????????????????????????????????????????????????????????????????????????????????????????????????
///
/// ?????? `MultiplePartitionsResumablePolicyProvider(base, multiply)` ?????????????????????????????????????????????
#[pyclass(extends = ResumablePolicyProvider)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "(base, multiply)")]
struct MultiplePartitionsResumablePolicyProvider;

#[pymethods]
impl MultiplePartitionsResumablePolicyProvider {
    /// ?????????????????????????????????????????????
    ///
    /// ???????????? `0` ???????????????
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

/// ????????? KEY
///
/// ??????????????????????????????
///
/// ?????? `SourceKey(key)` ??????????????? KEY
#[pyclass]
#[derive(Debug, Clone)]
struct SourceKey(qiniu_sdk::upload::SourceKey);

#[pymethods]
impl SourceKey {
    /// ??????????????? KEY??????????????? 20 ???????????????????????????
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

/// ?????????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct ResumableRecorder(Box<dyn qiniu_sdk::upload::ResumableRecorder<HashAlgorithm = Sha1>>);

#[pymethods]
impl ResumableRecorder {
    /// ??????????????? KEY ????????????????????????
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

    /// ??????????????? KEY ????????????????????????
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

    /// ??????????????? KEY ????????????????????????
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

    /// ??????????????? KEY ??????????????????
    #[pyo3(text_signature = "($self, source_key)")]
    fn delete(&self, source_key: &SourceKey, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| self.0.delete(&source_key.0).map_err(QiniuIoError::from_err))
    }

    /// ??????????????? KEY ??????????????????????????????
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

    /// ??????????????? KEY ??????????????????????????????
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

    /// ??????????????? KEY ??????????????????????????????
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

    /// ??????????????? KEY ????????????????????????
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

/// ??????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Debug)]
struct ReadOnlyResumableRecorderMedium(Box<dyn qiniu_sdk::upload::ReadOnlyResumableRecorderMedium>);

#[pymethods]
impl ReadOnlyResumableRecorderMedium {
    /// ?????????????????????
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

    /// ???????????????????????????
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

/// ??????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Debug)]
struct AppendOnlyResumableRecorderMedium(
    Box<dyn qiniu_sdk::upload::AppendOnlyResumableRecorderMedium>,
);

#[pymethods]
impl AppendOnlyResumableRecorderMedium {
    /// ??????????????????
    #[pyo3(text_signature = "($self, b, /)")]
    fn write(&mut self, b: &[u8], py: Python<'_>) -> PyResult<usize> {
        py.allow_threads(|| self.0.write_all(b).map_err(PyIOError::new_err))?;
        Ok(b.len())
    }

    /// ????????????
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

/// ????????????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Debug)]
struct ReadOnlyAsyncResumableRecorderMedium(
    Arc<AsyncMutex<dyn qiniu_sdk::upload::ReadOnlyAsyncResumableRecorderMedium>>,
);

#[pymethods]
impl ReadOnlyAsyncResumableRecorderMedium {
    /// ???????????????????????????
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

    /// ?????????????????????????????????
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

/// ????????????????????????
///
/// ?????????
#[pyclass(subclass)]
#[derive(Debug)]
struct AppendOnlyAsyncResumableRecorderMedium(
    Arc<AsyncMutex<dyn qiniu_sdk::upload::AppendOnlyAsyncResumableRecorderMedium>>,
);

#[pymethods]
impl AppendOnlyAsyncResumableRecorderMedium {
    /// ????????????????????????
    #[pyo3(text_signature = "($self, b, /)")]
    fn write<'a>(&mut self, b: Vec<u8>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let writer = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut writer = writer.lock().await;
            writer.write_all(&b).await.map_err(PyIOError::new_err)?;
            Ok(b.len())
        })
    }

    /// ??????????????????
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

/// ????????????????????????
///
/// ?????????????????????????????????????????????????????????????????????
///
/// ?????? `DummyResumableRecorder()` ??????????????????????????????
#[pyclass(extends = ResumableRecorder)]
#[derive(Clone, Debug)]
#[pyo3(text_signature = "()")]
struct DummyResumableRecorder;

#[pymethods]
impl DummyResumableRecorder {
    /// ??????????????????????????????
    #[new]
    fn new() -> (Self, ResumableRecorder) {
        (
            Self,
            ResumableRecorder(Box::new(qiniu_sdk::upload::DummyResumableRecorder::new())),
        )
    }
}

/// ?????????????????????????????????
///
/// ????????????????????????????????????????????????
///
/// ?????? `FileSystemResumableRecorder(path = None)` ???????????????????????????????????????
#[pyclass(extends = ResumableRecorder)]
#[derive(Debug, Clone)]
#[pyo3(text_signature = "(/, path = None)")]
struct FileSystemResumableRecorder;

#[pymethods]
impl FileSystemResumableRecorder {
    /// ??????????????????????????????????????????????????????????????????????????????????????????
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
                text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
            )]
            #[args(
                region_provider = "None",
                object_name = "None",
                file_name = "None",
                content_type = "None",
                metadata = "None",
                custom_vars = "None",
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
                py: Python<'_>,
            ) -> PyResult<PyObject> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
                )?;
                py.allow_threads(|| {
                    self.0
                        .upload_path(path, object_params)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                        .and_then(|v| convert_json_value_to_py_object(&v))
                })
            }

            #[pyo3(
                text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
            )]
            #[args(
                region_provider = "None",
                object_name = "None",
                file_name = "None",
                content_type = "None",
                metadata = "None",
                custom_vars = "None",
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
                py: Python<'_>,
            ) -> PyResult<PyObject> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
                )?;
                py.allow_threads(|| {
                    self.0
                        .upload_reader(PythonIoBase::new(reader), object_params)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                        .and_then(|v| convert_json_value_to_py_object(&v))
                })
            }

            #[pyo3(
                text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
            )]
            #[args(
                region_provider = "None",
                object_name = "None",
                file_name = "None",
                content_type = "None",
                metadata = "None",
                custom_vars = "None",
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
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
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
                text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
            )]
            #[args(
                region_provider = "None",
                object_name = "None",
                file_name = "None",
                content_type = "None",
                metadata = "None",
                custom_vars = "None",
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
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
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

/// ???????????????
///
/// ?????????
///
/// ??????????????????????????????
#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct DataSource(Box<dyn qiniu_sdk::upload::DataSource<Sha1>>);

#[pymethods]
impl DataSource {
    /// ???????????????
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

    /// ??????????????? KEY
    ///
    /// ??????????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn source_key(&self, py: Python<'_>) -> PyResult<Option<SourceKey>> {
        py.allow_threads(|| self.0.source_key())
            .map(|s| s.map(SourceKey))
            .map_err(PyIOError::new_err)
    }

    /// ?????????????????????
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

    fn reset(&self) -> std::io::Result<()> {
        self.0.reset()
    }

    fn source_key(&self) -> std::io::Result<Option<qiniu_sdk::upload::SourceKey<Sha1>>> {
        self.0.source_key()
    }

    fn total_size(&self) -> std::io::Result<Option<u64>> {
        self.0.total_size()
    }
}

/// ???????????????
///
/// ??????????????????????????????????????????
///
/// ?????? `FileDataSource(path)` ?????????????????????
#[pyclass(extends = DataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(path)")]
struct FileDataSource;

#[pymethods]
impl FileDataSource {
    /// ?????????????????????
    #[new]
    fn new(path: &str) -> (Self, DataSource) {
        (
            Self,
            DataSource(Box::new(qiniu_sdk::upload::FileDataSource::new(path))),
        )
    }
}

/// ????????????????????????
///
/// ????????????????????????????????????????????????????????????
///
/// ?????? `UnseekableDataSource(source)` ??????????????????????????????
#[pyclass(extends = DataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(source)")]
struct UnseekableDataSource;

#[pymethods]
impl UnseekableDataSource {
    /// ??????????????????????????????
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

/// ?????????????????????
///
/// ?????????
///
/// ??????????????????????????????
#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct AsyncDataSource(Box<dyn qiniu_sdk::upload::AsyncDataSource<Sha1>>);

#[pymethods]
impl AsyncDataSource {
    /// ?????????????????????
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

    /// ????????????????????? KEY
    ///
    /// ??????????????????????????????
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

    /// ???????????????????????????
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

    fn reset(&self) -> futures::future::BoxFuture<std::io::Result<()>> {
        self.0.reset()
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

/// ?????????????????????
///
/// ??????????????????????????????????????????
///
/// ?????? `AsyncFileDataSource(path)` ???????????????????????????
#[pyclass(extends = AsyncDataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(path)")]
struct AsyncFileDataSource;

#[pymethods]
impl AsyncFileDataSource {
    /// ???????????????????????????
    #[new]
    fn new(path: &str) -> (Self, AsyncDataSource) {
        (
            Self,
            AsyncDataSource(Box::new(qiniu_sdk::upload::AsyncFileDataSource::new(path))),
        )
    }
}

/// ??????????????????????????????
///
/// ????????????????????????????????????????????????????????????????????????
///
/// ?????? `AsyncUnseekableDataSource(source)` ????????????????????????????????????
#[pyclass(extends = AsyncDataSource)]
#[derive(Debug, Clone, Copy)]
#[pyo3(text_signature = "(source)")]
struct AsyncUnseekableDataSource;

#[pymethods]
impl AsyncUnseekableDataSource {
    /// ????????????????????????????????????
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

/// ??????????????????
///
/// ?????????
#[pyclass]
#[derive(Debug)]
struct DataSourceReader(qiniu_sdk::upload::DataSourceReader);

#[pymethods]
impl DataSourceReader {
    /// ?????????????????????
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

    /// ???????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        self.read(-1, py)
    }

    /// ??????????????????
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

/// ????????????????????????
///
/// ?????????
#[pyclass]
#[derive(Debug)]
struct AsyncDataSourceReader(Arc<AsyncMutex<qiniu_sdk::upload::AsyncDataSourceReader>>);

#[pymethods]
impl AsyncDataSourceReader {
    /// ???????????????????????????
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

    /// ?????????????????????????????????
    #[pyo3(text_signature = "($self)")]
    fn readall<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        self.read(-1, py)
    }

    /// ??????????????????
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

/// ???????????????
///
/// ?????? `UploadManager(signer, http_client = None, use_https = None, queryer = None, uc_endpoints = None)` ?????????????????????
#[pyclass]
#[derive(Debug, Clone)]
#[pyo3(
    text_signature = "(signer, /, http_client = None, use_https = None, queryer = None, uc_endpoints = None)"
)]
struct UploadManager(qiniu_sdk::upload::UploadManager);

#[pymethods]
impl UploadManager {
    /// ?????????????????????
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

    /// ?????????????????????
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

    /// ????????????????????? V1
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

    /// ????????????????????? V2
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

    /// ?????????????????????
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

/// ???????????????
///
/// ???????????????????????? API ???????????????????????????
///
/// ?????? `upload_manager.form_uploader()` ?????????????????????
#[pyclass]
#[derive(Debug, Clone)]
struct FormUploader(qiniu_sdk::upload::FormUploader);

impl_uploader!(FormUploader);

macro_rules! impl_multi_parts_uploader {
    ($name:ident, $initialized_parts:ident, $async_initialize_parts:ident, $uploaded_part:ident, $async_uploaded_part:ident) => {
        #[pymethods]
        impl $name {
            /// ?????????????????????
            ///
            /// ?????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
            #[pyo3(
                text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
            )]
            #[args(
                region_provider = "None",
                object_name = "None",
                file_name = "None",
                content_type = "None",
                metadata = "None",
                custom_vars = "None",
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
                py: Python<'_>,
            ) -> PyResult<$initialized_parts> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
                )?;
                py.allow_threads(|| {
                    self.0
                        .initialize_parts(source, object_params)
                        .map($initialized_parts)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// ???????????????????????????
            ///
            /// ????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
            #[pyo3(text_signature="($self, initialized, /, keep_original_region = None, refresh_regions = None, regions_provider = None)")]
            #[args(keep_original_region = "None", refresh_regions = "None", regions_provider = "None")]
            fn reinitialize_parts(&self,
                initialized: &mut $initialized_parts,
                keep_original_region: Option<bool>,
                refresh_regions:Option<bool>,
                regions_provider: Option<RegionsProvider>,
                py: Python<'_>,
            ) -> PyResult<()> {
                let options = make_reinitialize_options(keep_original_region, refresh_regions, regions_provider);
                py.allow_threads(|| {
                    self.0
                        .reinitialize_parts(&mut initialized.0, options)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// ????????????
            ///
            /// ?????????????????????????????????????????????????????????????????????
            ///
            /// ???????????? `None` ????????????????????????????????????????????????
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

            /// ??????????????????
            ///
            /// ???????????????????????????????????????????????????
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

            /// ???????????????????????????
            ///
            /// ?????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
            #[pyo3(
                text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
            )]
            #[args(
                region_provider = "None",
                object_name = "None",
                file_name = "None",
                content_type = "None",
                metadata = "None",
                custom_vars = "None",
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
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let object_params = make_object_params(
                    region_provider,
                    object_name,
                    file_name,
                    content_type,
                    metadata,
                    custom_vars,
                )?;
                let uploader = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    uploader
                        .async_initialize_parts(source, object_params)
                        .await
                        .map($async_initialize_parts)
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// ?????????????????????????????????
            ///
            /// ????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????????
            #[pyo3(text_signature="($self, initialized, /, keep_original_region = None, refresh_regions = None, regions_provider = None)")]
            #[args(keep_original_region = "None", refresh_regions = "None", regions_provider = "None")]
            fn async_reinitialize_parts<'p>(&self,
                initialized: $async_initialize_parts,
                keep_original_region: Option<bool>,
                refresh_regions:Option<bool>,
                regions_provider: Option<RegionsProvider>,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let options = make_reinitialize_options(keep_original_region, refresh_regions, regions_provider);
                let uploader = self.0.to_owned();
                let mut initialized = initialized.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    uploader
                        .async_reinitialize_parts(&mut initialized, options)
                        .await
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// ??????????????????
            ///
            /// ?????????????????????????????????????????????????????????????????????
            ///
            /// ???????????? `None` ????????????????????????????????????????????????
            #[pyo3(text_signature = "($self, initialized, data_partitioner_provider)")]
            fn async_upload_part<'p>(
                &'p self,
                initialized: $async_initialize_parts,
                data_partitioner_provider: DataPartitionProvider,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let uploader = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    uploader
                        .async_upload_part(&initialized.0, &data_partitioner_provider)
                        .await
                        .map(|p| p.map($async_uploaded_part))
                        .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                })
            }

            /// ????????????????????????
            ///
            /// ???????????????????????????????????????????????????
            #[pyo3(text_signature = "($self, initialized, parts)")]
            fn async_complete_part<'p>(
                &'p self,
                initialized: $async_initialize_parts,
                parts: Vec<$async_uploaded_part>,
                py: Python<'p>,
            ) -> PyResult<&'p PyAny> {
                let uploader = self.0.to_owned();
                pyo3_asyncio::async_std::future_into_py(py, async move {
                    uploader
                        .async_complete_parts(
                            &initialized.0,
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

/// ??????????????? V1
///
/// ????????????????????????????????????????????????????????? `MultiPartsUploaderScheduler` ?????????????????????????????????
///
/// ?????? `upload_manager.multi_parts_v1_uploader()` ????????????????????? V1
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

/// ??????????????? V2
///
/// ????????????????????????????????????????????????????????? `MultiPartsUploaderScheduler` ?????????????????????????????????
///
/// ?????? `upload_manager.multi_parts_v2_uploader()` ????????????????????? V2
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
            /// ??????????????????
            #[getter]
            fn get_object_name(&self) -> Option<&str> {
                self.0.params().object_name()
            }

            /// ??????????????????
            #[getter]
            fn get_file_name(&self) -> Option<&str> {
                self.0.params().file_name()
            }

            /// ?????? MIME ??????
            #[getter]
            fn get_content_type(&self) -> Option<&str> {
                self.0.params().content_type().map(|s| s.as_ref())
            }

            /// ?????????????????????
            #[getter]
            fn get_metadata(&self) -> HashMap<String, String> {
                self.0.params().metadata().to_owned()
            }

            /// ???????????????????????????
            #[getter]
            fn get_custom_vars(&self) -> HashMap<String, String> {
                self.0.params().custom_vars().to_owned()
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

/// ??? ??????????????? V1 ????????????????????????
///
/// ?????? `multi_parts_uploader_v1.initialize_parts()` ??????
#[pyclass]
struct MultiPartsV1UploaderInitializedObject(
    <qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::InitializedParts,
);
impl_initialized_object!(MultiPartsV1UploaderInitializedObject);

/// ??? ??????????????? V1 ??????????????????????????????
///
/// ?????? `multi_parts_uploader_v1.async_initialize_parts()` ??????
#[pyclass]
#[derive(Clone)]
struct AsyncMultiPartsV1UploaderInitializedObject(
    <qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncInitializedParts,
);
impl_initialized_object!(AsyncMultiPartsV1UploaderInitializedObject);

/// ??? ??????????????? V2 ????????????????????????
///
/// ?????? `multi_parts_uploader_v2.initialize_parts()` ??????
#[pyclass]
struct MultiPartsV2UploaderInitializedObject(
    <qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::InitializedParts,
);
impl_initialized_object!(MultiPartsV2UploaderInitializedObject);

/// ??? ??????????????? V2 ??????????????????????????????
///
/// ?????? `multi_parts_uploader_v2.async_initialize_parts()` ??????
#[pyclass]
#[derive(Clone)]
struct AsyncMultiPartsV2UploaderInitializedObject(
    <qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncInitializedParts,
);
impl_initialized_object!(AsyncMultiPartsV2UploaderInitializedObject);

macro_rules! impl_uploaded_part {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// ????????????
            #[getter]
            fn get_size(&self) -> u64 {
                self.0.size().get()
            }

            /// ???????????????
            #[getter]
            fn get_offset(&self) -> u64 {
                self.0.offset()
            }

            /// ???????????????????????????
            #[getter]
            fn get_resumed(&self) -> bool {
                self.0.resumed()
            }

            /// ???????????????
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

/// ???????????? ??????????????? V1 ?????????????????????
///
/// ?????? `multi_parts_uploader_v1.upload_part()` ??????
#[pyclass]
#[derive(Clone, Debug)]
struct MultiPartsV1UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::UploadedPart);
impl_uploaded_part!(MultiPartsV1UploaderUploadedPart);

/// ???????????? ??????????????? V1 ???????????????????????????
///
/// ?????? `multi_parts_uploader_v1.async_upload_part()` ??????
#[pyclass]
#[derive(Clone, Debug)]
struct AsyncMultiPartsV1UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV1Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncUploadedPart);
impl_uploaded_part!(AsyncMultiPartsV1UploaderUploadedPart);

/// ???????????? ??????????????? V2 ?????????????????????
///
/// ?????? `multi_parts_uploader_v2.upload_part()` ??????
#[pyclass]
#[derive(Clone, Debug)]
struct MultiPartsV2UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::UploadedPart);
impl_uploaded_part!(MultiPartsV2UploaderUploadedPart);

/// ???????????? ??????????????? V2 ???????????????????????????
///
/// ?????? `multi_parts_uploader_v2.async_upload_part()` ??????
#[pyclass]
#[derive(Clone, Debug)]
struct AsyncMultiPartsV2UploaderUploadedPart(<qiniu_sdk::upload::MultiPartsV2Uploader as qiniu_sdk::upload::MultiPartsUploader>::AsyncUploadedPart);
impl_uploaded_part!(AsyncMultiPartsV2UploaderUploadedPart);

/// ???????????????????????????
///
/// ?????????
///
/// ????????????????????????????????????????????????????????????????????????????????????????????????
#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct MultiPartsUploaderScheduler(Box<dyn qiniu_sdk::upload::MultiPartsUploaderScheduler<Sha1>>);

#[pymethods]
impl MultiPartsUploaderScheduler {
    /// ????????????????????????
    #[setter]
    fn set_concurrency_provider(&mut self, concurrency_provider: ConcurrencyProvider) {
        self.0.set_concurrency_provider(concurrency_provider.0);
    }

    /// ???????????????????????????
    #[setter]
    fn set_data_partition_provider(&mut self, data_partition_provider: DataPartitionProvider) {
        self.0
            .set_data_partition_provider(data_partition_provider.0);
    }

    /// ???????????????
    #[pyo3(
        text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None"
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
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let object_params = make_object_params(
            region_provider,
            object_name,
            file_name,
            content_type,
            metadata,
            custom_vars,
        )?;
        py.allow_threads(|| {
            self.0
                .upload(source.0, object_params)
                .map_err(|err| QiniuApiCallError::from_err(MaybeOwned::Owned(err)))
                .and_then(|v| convert_json_value_to_py_object(&v))
        })
    }

    /// ?????????????????????
    #[pyo3(
        text_signature = "($self, source, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None"
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

/// ???????????????????????????
///
/// ?????????????????????????????????????????????????????????
///
/// ?????? `SerialMultiPartsUploaderScheduler(multi_parts_uploader)` ?????????????????????????????????
#[pyclass(extends = MultiPartsUploaderScheduler)]
#[derive(Debug, Copy, Clone)]
#[pyo3(text_signature = "(uploader)")]
struct SerialMultiPartsUploaderScheduler;

#[pymethods]
impl SerialMultiPartsUploaderScheduler {
    /// ?????????????????????????????????
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

/// ???????????????????????????
///
/// ?????????????????????????????????????????????????????????????????????????????? `async-std` ?????????????????????????????????
///
/// ?????? `ConcurrentMultiPartsUploaderScheduler(multi_parts_uploader)` ?????????????????????????????????
#[pyclass(extends = MultiPartsUploaderScheduler)]
#[derive(Debug, Copy, Clone)]
#[pyo3(text_signature = "(uploader)")]
struct ConcurrentMultiPartsUploaderScheduler;

#[pymethods]
impl ConcurrentMultiPartsUploaderScheduler {
    /// ?????????????????????????????????
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
    Ok(builder.build())
}

fn make_reinitialize_options(
    keep_original_region: Option<bool>,
    refresh_regions: Option<bool>,
    region_provider: Option<RegionsProvider>,
) -> qiniu_sdk::upload::ReinitializeOptions {
    let mut builder = qiniu_sdk::upload::ReinitializeOptions::builder();
    if let Some(region_provider) = region_provider {
        builder.regions_provider(region_provider);
    }
    if let Some(true) = refresh_regions {
        builder.refresh_regions();
    }
    if let Some(true) = keep_original_region {
        builder.keep_original_region();
    }
    builder.build()
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

/// ??????????????????
///
/// ?????? `UploadingProgressInfo(transferred_bytes, total_bytes = None)` ????????????????????????
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

    /// ??????????????????????????????
    ///
    /// ???????????????
    #[getter]
    fn get_transferred_bytes(&self) -> u64 {
        self.0.transferred_bytes()
    }

    /// ????????????????????????????????????
    ///
    /// ???????????????
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

/// ???????????????????????????
///
/// ???????????????????????????????????????????????????????????????
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
) -> impl Fn(&mut qiniu_sdk::http_client::ResponseError) -> AnyResult<()> + Send + Sync + 'static {
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

/// ??????????????????????????????
#[pyclass]
#[derive(Debug, Clone)]
enum MultiPartsUploaderSchedulerPrefer {
    /// ?????????????????????
    Serial = 0,
    /// ?????????????????????
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

/// ?????????????????????????????????
#[pyclass]
#[derive(Debug, Clone)]
enum SinglePartUploaderPrefer {
    /// ???????????????
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

/// ??????????????????????????????
#[pyclass]
#[derive(Debug, Clone)]
enum MultiPartsUploaderPrefer {
    /// ??????????????? V1
    V1 = 1,
    /// ??????????????? V2
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

/// ???????????????
///
/// ???????????????????????????????????????????????????????????????????????????
///
/// ?????? `upload_manager.auto_uploader()` ?????????????????????
#[pyclass]
#[derive(Debug, Clone)]
struct AutoUploader(qiniu_sdk::upload::AutoUploader);

#[pymethods]
impl AutoUploader {
    #[pyo3(
        text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
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
        text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
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
        text_signature = "($self, path, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
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
        text_signature = "($self, reader, /, region_provider=None, object_name=None, file_name=None, content_type=None, metadata=None, custom_vars=None, multi_parts_uploader_scheduler_prefer=None, single_part_uploader_prefer=None, multi_parts_uploader_prefer=None)"
    )]
    #[args(
        region_provider = "None",
        object_name = "None",
        file_name = "None",
        content_type = "None",
        metadata = "None",
        custom_vars = "None",
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

/// ???????????????
///
/// ?????? `resumable_policy_provider.get_policy_from_reader()` ??????
#[pyclass]
struct Reader(Box<dyn qiniu_sdk::upload::DynRead>);

#[pymethods]
impl Reader {
    /// ????????????
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

    /// ??????????????????
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

/// ?????????????????????
///
/// ?????? `resumable_policy_provider.get_policy_from_async_reader()` ??????
#[pyclass]

struct AsyncReader(Arc<AsyncMutex<dyn qiniu_sdk::upload::DynAsyncRead>>);

impl<T: AsyncRead + Unpin + Debug + Sync + Send + 'static> From<T> for AsyncReader {
    fn from(reader: T) -> Self {
        Self(Arc::new(AsyncMutex::new(reader)))
    }
}

#[pymethods]
impl AsyncReader {
    /// ??????????????????
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

    /// ????????????????????????
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
