use super::{
    credential::CredentialProvider,
    exceptions::QiniuApiCallError,
    http::{HttpResponseParts, HttpResponsePartsContext},
    http_client::{
        BucketRegionsQueryer, Endpoints, HttpClient, JsonResponse, RegionsProvider,
        RequestBuilderParts,
    },
    utils::{convert_json_value_to_py_object, parse_mime},
};
use anyhow::Result as AnyResult;
use futures::{
    lock::Mutex as AsyncMutex, stream::Peekable as AsyncPeekable, StreamExt, TryStreamExt,
};
use indexmap::IndexMap;
use mime::Mime;
use pyo3::prelude::*;
use std::{
    borrow::Cow,
    collections::HashMap,
    mem::transmute,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "objects")?;
    m.add_class::<ObjectsManager>()?;
    m.add_class::<Bucket>()?;
    m.add_class::<OperationProvider>()?;
    m.add_class::<StatObject>()?;
    m.add_class::<CopyObject>()?;
    m.add_class::<MoveObject>()?;
    m.add_class::<DeleteObject>()?;
    m.add_class::<UnfreezeObject>()?;
    m.add_class::<SetObjectType>()?;
    m.add_class::<ModifyObjectStatus>()?;
    m.add_class::<ModifyObjectMetadata>()?;
    m.add_class::<ModifyObjectLifeCycle>()?;
    m.add_class::<ObjectsIterator>()?;
    m.add_class::<ListVersion>()?;
    m.add_class::<AsyncObjectsIterator>()?;
    Ok(m)
}

/// 七牛对象管理器
#[pyclass]
#[pyo3(
    text_signature = "(credential, /, use_https = None, http_client = None, uc_endpoints = None, queryer = None)"
)]
#[derive(Clone)]
struct ObjectsManager(qiniu_sdk::objects::ObjectsManager);

#[pymethods]
impl ObjectsManager {
    #[new]
    #[args(
        use_https = "None",
        http_client = "None",
        uc_endpoints = "None",
        queryer = "None"
    )]
    fn new(
        credential: CredentialProvider,
        use_https: Option<bool>,
        http_client: Option<HttpClient>,
        uc_endpoints: Option<Endpoints>,
        queryer: Option<BucketRegionsQueryer>,
    ) -> Self {
        let mut builder = qiniu_sdk::objects::ObjectsManager::builder(credential);
        if let Some(use_https) = use_https {
            builder.use_https(use_https);
        }
        if let Some(http_client) = http_client {
            builder.http_client(http_client.into());
        }
        if let Some(uc_endpoints) = uc_endpoints {
            builder.uc_endpoints(uc_endpoints);
        }
        if let Some(queryer) = queryer {
            builder.queryer(queryer.into());
        }
        Self(builder.build())
    }

    /// 获取七牛存储空间管理器
    #[pyo3(text_signature = "($self, name, /, regions = None)")]
    #[args(regions = "None")]
    fn bucket(&self, name: &str, regions: Option<RegionsProvider>) -> Bucket {
        let bucket = if let Some(regions) = regions {
            self.0.bucket_with_region(name, regions)
        } else {
            self.0.bucket(name)
        };
        Bucket(bucket)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 七牛存储空间管理器
#[pyclass]
#[derive(Clone, Debug)]
struct Bucket(qiniu_sdk::objects::Bucket);

#[pymethods]
impl Bucket {
    /// 获取存储空间名称
    #[getter]
    fn get_name(&self) -> String {
        self.0.name().to_string()
    }

    /// 列举对象
    #[pyo3(
        text_signature = "($self, /, limit = None, prefix = None, marker = None, version = None, need_parts = None, before_request_callback = None, after_response_ok_callback = None)"
    )]
    #[args(
        limit = "None",
        prefix = "None",
        marker = "None",
        version = "None",
        need_parts = "None",
        before_request_callback = "None",
        after_response_ok_callback = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn list(
        &self,
        limit: Option<usize>,
        prefix: Option<String>,
        marker: Option<String>,
        version: Option<ListVersion>,
        need_parts: Option<bool>,
        before_request_callback: Option<PyObject>,
        after_response_ok_callback: Option<PyObject>,
    ) -> ObjectsLister {
        let params = Arc::pin(ObjectsIteratorParams {
            bucket: self.to_owned(),
            limit,
            prefix,
            marker,
            version,
            need_parts,
            before_request_callback,
            after_response_ok_callback,
        });
        ObjectsLister { params }
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<ObjectsIterator> {
        self.list(None, None, None, None, None, None, None)
            .__iter__(py)
    }

    /// 获取对象元信息
    #[pyo3(text_signature = "($self, object, /, before_request_callback = None)")]
    #[args(before_request_callback = "None")]
    fn stat_object(
        &self,
        object: String,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<StatObject>> {
        let stat_object = StatObject {
            entry: Entry::new(self.to_owned(), object),
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut stat_object.make_operation(),
            ),
        };
        Py::new(py, (stat_object, operation_provider))
    }

    /// 复制对象
    #[pyo3(
        text_signature = "($self, from_object, to_bucket, to_object, /, force = None, before_request_callback = None)"
    )]
    #[args(force = "None", before_request_callback = "None")]
    fn copy_object_to(
        &self,
        from_object: String,
        to_bucket: String,
        to_object: String,
        force: Option<bool>,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<CopyObject>> {
        let copy_object = CopyObject {
            from_entry: Entry::new(self.to_owned(), from_object),
            to_entry: SimpleEntry::new(to_bucket, to_object),
            force,
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut copy_object.make_operation(),
            ),
        };
        Py::new(py, (copy_object, operation_provider))
    }

    /// 移动对象
    #[pyo3(
        text_signature = "($self, from_object, to_bucket, to_object, /, force = None, before_request_callback = None)"
    )]
    #[args(force = "None", before_request_callback = "None")]
    fn move_object_to(
        &self,
        from_object: String,
        to_bucket: String,
        to_object: String,
        force: Option<bool>,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<MoveObject>> {
        let move_object = MoveObject {
            from_entry: Entry::new(self.to_owned(), from_object),
            to_entry: SimpleEntry::new(to_bucket, to_object),
            force,
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut move_object.make_operation(),
            ),
        };
        Py::new(py, (move_object, operation_provider))
    }

    /// 删除对象
    #[pyo3(text_signature = "($self, object, /, before_request_callback = None)")]
    #[args(before_request_callback = "None")]
    fn delete_object(
        &self,
        object: String,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<DeleteObject>> {
        let delete_object = DeleteObject {
            entry: Entry::new(self.to_owned(), object),
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut delete_object.make_operation(),
            ),
        };
        Py::new(py, (delete_object, operation_provider))
    }

    /// 解冻对象
    #[pyo3(
        text_signature = "($self, object, freeze_after_days, /, before_request_callback = None)"
    )]
    #[args(before_request_callback = "None")]
    fn restore_archived_object(
        &self,
        object: String,
        freeze_after_days: usize,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<UnfreezeObject>> {
        let restore_archived_object = UnfreezeObject {
            entry: Entry::new(self.to_owned(), object),
            freeze_after_days,
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut restore_archived_object.make_operation(),
            ),
        };
        Py::new(py, (restore_archived_object, operation_provider))
    }

    /// 设置对象类型
    #[pyo3(text_signature = "($self, object, object_type, /, before_request_callback = None)")]
    #[args(before_request_callback = "None")]
    fn set_object_type(
        &self,
        object: String,
        object_type: u8,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<SetObjectType>> {
        let set_object_type = SetObjectType {
            entry: Entry::new(self.to_owned(), object),
            file_type: qiniu_sdk::upload_token::FileType::from(object_type),
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut set_object_type.make_operation(),
            ),
        };
        Py::new(py, (set_object_type, operation_provider))
    }

    /// 设置对象状态
    #[pyo3(text_signature = "($self, object, disabled, /, before_request_callback = None)")]
    #[args(before_request_callback = "None")]
    fn modify_object_status(
        &self,
        object: String,
        disabled: bool,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<ModifyObjectStatus>> {
        let modify_object_status = ModifyObjectStatus {
            entry: Entry::new(self.to_owned(), object),
            disabled,
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut modify_object_status.make_operation(),
            ),
        };
        Py::new(py, (modify_object_status, operation_provider))
    }

    /// 设置对象状态
    #[pyo3(
        text_signature = "($self, object, mime_type, /, metadata = None, conditions = None, before_request_callback = None)"
    )]
    #[args(
        metadata = "None",
        conditions = "None",
        before_request_callback = "None"
    )]
    fn modify_object_metadata(
        &self,
        object: String,
        mime_type: &str,
        metadata: Option<HashMap<String, String>>,
        conditions: Option<HashMap<String, String>>,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<ModifyObjectMetadata>> {
        let modify_object_metadata = ModifyObjectMetadata {
            entry: Entry::new(self.to_owned(), object),
            mime_type: parse_mime(mime_type)?,
            metadata: metadata.unwrap_or_default().into_iter().collect(),
            conditions: conditions.unwrap_or_default().into_iter().collect(),
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut modify_object_metadata.make_operation(),
            ),
        };
        Py::new(py, (modify_object_metadata, operation_provider))
    }

    /// 设置对象生命周期
    #[pyo3(
        text_signature = "($self, object, mime_type, /, ia_after_days = None, archive_after_days = None, deep_archive_after_days = None, delete_after_days = None, before_request_callback = None)"
    )]
    #[args(
        ia_after_days = "None",
        archive_after_days = "None",
        deep_archive_after_days = "None",
        delete_after_days = "None",
        before_request_callback = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn modify_object_life_cycle(
        &self,
        object: String,
        ia_after_days: Option<isize>,
        archive_after_days: Option<isize>,
        deep_archive_after_days: Option<isize>,
        delete_after_days: Option<isize>,
        before_request_callback: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<ModifyObjectLifeCycle>> {
        let modify_object_life_cycle = ModifyObjectLifeCycle {
            entry: Entry::new(self.to_owned(), object),
            ia_after_days,
            archive_after_days,
            deep_archive_after_days,
            delete_after_days,
            before_request_callback,
        };
        let operation_provider = OperationProvider {
            operation: qiniu_sdk::objects::OperationProvider::to_operation(
                &mut modify_object_life_cycle.make_operation(),
            ),
        };
        Py::new(py, (modify_object_life_cycle, operation_provider))
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// 对象操作提供者接口
#[pyclass(subclass)]
#[derive(Clone, Debug)]
struct OperationProvider {
    operation: String,
}

#[pymethods]
impl OperationProvider {
    fn __str__(&self) -> String {
        self.operation.to_owned()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl qiniu_sdk::objects::OperationProvider for OperationProvider {
    fn to_operation(&mut self) -> String {
        self.operation.to_owned()
    }
}

#[derive(Clone, Debug)]
struct Entry {
    bucket: Bucket,
    object: String,
}

impl Entry {
    fn new(bucket: Bucket, object: String) -> Self {
        Self { bucket, object }
    }
}

#[derive(Clone, Debug)]
struct SimpleEntry {
    bucket: String,
    object: String,
}

impl SimpleEntry {
    fn new(bucket: String, object: String) -> Self {
        Self { bucket, object }
    }
}

/// 对象元信息获取操作构建器
///
/// 可以通过 `bucket.stat_object()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct StatObject {
    entry: Entry,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl StatObject {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let stat_object = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = stat_object
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl StatObject {
    fn make_operation(&self) -> qiniu_sdk::objects::StatObjectBuilder {
        let mut builder = self.entry.bucket.0.stat_object(&self.entry.object);
        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }
        builder
    }
}

/// 对象复制操作构建器
///
/// 可以通过 `bucket.copy_object_to()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct CopyObject {
    from_entry: Entry,
    to_entry: SimpleEntry,
    force: Option<bool>,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl CopyObject {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let copy_object = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = copy_object
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl CopyObject {
    fn make_operation(&self) -> qiniu_sdk::objects::CopyObjectBuilder {
        let mut copy_object = self.from_entry.bucket.0.copy_object_to(
            &self.from_entry.object,
            &self.to_entry.bucket,
            &self.to_entry.object,
        );
        if let Some(force) = self.force {
            copy_object.is_force(force);
        }
        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                copy_object
                    .before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }
        copy_object
    }
}

/// 对象移动操作构建器
///
/// 可以通过 `bucket.move_object_to()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct MoveObject {
    from_entry: Entry,
    to_entry: SimpleEntry,
    force: Option<bool>,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl MoveObject {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let move_object = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = move_object
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl MoveObject {
    fn make_operation(&self) -> qiniu_sdk::objects::MoveObjectBuilder {
        let mut move_object = self.from_entry.bucket.0.move_object_to(
            &self.from_entry.object,
            &self.to_entry.bucket,
            &self.to_entry.object,
        );
        if let Some(force) = self.force {
            move_object.is_force(force);
        }
        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                move_object
                    .before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }
        move_object
    }
}

/// 对象元信息删除操作构建器
///
/// 可以通过 `bucket.delete_object()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct DeleteObject {
    entry: Entry,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl DeleteObject {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let delete_object = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = delete_object
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl DeleteObject {
    fn make_operation(&self) -> qiniu_sdk::objects::DeleteObjectBuilder {
        let mut builder = self.entry.bucket.0.delete_object(&self.entry.object);

        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }

        builder
    }
}

/// 对象解冻操作构建器
///
/// 可以通过 `bucket.restore_archived_object()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct UnfreezeObject {
    entry: Entry,
    freeze_after_days: usize,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl UnfreezeObject {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let unfreeze_object = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = unfreeze_object
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl UnfreezeObject {
    fn make_operation(&self) -> qiniu_sdk::objects::UnfreezeObjectBuilder {
        let mut builder = self
            .entry
            .bucket
            .0
            .restore_archived_object(&self.entry.object, self.freeze_after_days);

        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }

        builder
    }
}

/// 对象类型设置操作构建器
///
/// 可以通过 `bucket.set_object_type()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct SetObjectType {
    entry: Entry,
    file_type: qiniu_sdk::upload_token::FileType,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl SetObjectType {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let set_object_type = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = set_object_type
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl SetObjectType {
    fn make_operation(&self) -> qiniu_sdk::objects::SetObjectTypeBuilder {
        let mut builder = self
            .entry
            .bucket
            .0
            .set_object_type(&self.entry.object, self.file_type);

        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }

        builder
    }
}

/// 修改对象状态构建器
///
/// 可以通过 `bucket::modify_object_status()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct ModifyObjectStatus {
    entry: Entry,
    disabled: bool,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl ModifyObjectStatus {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let modify_object_status = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = modify_object_status
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl ModifyObjectStatus {
    fn make_operation(&self) -> qiniu_sdk::objects::ModifyObjectStatusBuilder {
        let mut builder = self
            .entry
            .bucket
            .0
            .modify_object_status(&self.entry.object, self.disabled);

        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }

        builder
    }
}

/// 修改对象元信息构建器
///
/// 可以通过 `bucket::modify_object_metadata()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct ModifyObjectMetadata {
    entry: Entry,
    mime_type: Mime,
    metadata: IndexMap<String, String>,
    conditions: IndexMap<String, String>,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl ModifyObjectMetadata {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let modify_object_metadata = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = modify_object_metadata
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl ModifyObjectMetadata {
    fn make_operation(&self) -> qiniu_sdk::objects::ModifyObjectMetadataBuilder {
        let mut builder = self
            .entry
            .bucket
            .0
            .modify_object_metadata(&self.entry.object, self.mime_type.to_owned());
        for (key, value) in &self.metadata {
            builder.add_metadata(key, value);
        }
        for (key, value) in &self.conditions {
            builder.add_condition(key, value);
        }
        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }
        builder
    }
}

/// 修改对象生命周期构建器
///
/// 可以通过 `bucket::modify_object_life_cycle()` 方法获取该构建器。
#[pyclass(extends = OperationProvider)]
#[derive(Clone, Debug)]
struct ModifyObjectLifeCycle {
    entry: Entry,
    ia_after_days: Option<isize>,
    archive_after_days: Option<isize>,
    deep_archive_after_days: Option<isize>,
    delete_after_days: Option<isize>,
    before_request_callback: Option<PyObject>,
}

#[pymethods]
impl ModifyObjectLifeCycle {
    fn call(&self, py: Python<'_>) -> PyResult<Py<JsonResponse>> {
        let resp = py.allow_threads(|| {
            self.make_operation()
                .call()
                .map_err(QiniuApiCallError::from_err)
        })?;
        let (parts, body) = resp.into_parts_and_body();
        make_json_response(parts, body.as_ref(), py)
    }

    fn async_call<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let modify_object_metadata = self.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let resp = modify_object_metadata
                .make_operation()
                .async_call()
                .await
                .map_err(QiniuApiCallError::from_err)?;
            let (parts, body) = resp.into_parts_and_body();
            Python::with_gil(|py| make_json_response(parts, body.as_ref(), py))
        })
    }
}

impl ModifyObjectLifeCycle {
    fn make_operation(&self) -> qiniu_sdk::objects::ModifyObjectLifeCycleBuilder {
        let mut builder = self
            .entry
            .bucket
            .0
            .modify_object_life_cycle(&self.entry.object);
        if let Some(ia_after_days) = self.ia_after_days {
            builder.ia_after_days(ia_after_days.into());
        }
        if let Some(archive_after_days) = self.archive_after_days {
            builder.archive_after_days(archive_after_days.into());
        }
        if let Some(deep_archive_after_days) = self.deep_archive_after_days {
            builder.deep_archive_after_days(deep_archive_after_days.into());
        }
        if let Some(delete_after_days) = self.delete_after_days {
            builder.delete_after_days(delete_after_days.into());
        }
        if let Some(callback) = &self.before_request_callback {
            Python::with_gil(|py| {
                builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
            });
        }
        builder
    }
}

fn make_json_response(
    parts: qiniu_sdk::http::ResponseParts,
    body: &serde_json::Value,
    py: Python<'_>,
) -> PyResult<Py<JsonResponse>> {
    let json = JsonResponse::from(convert_json_value_to_py_object(body)?);
    Py::new(py, (json, HttpResponseParts::from(parts)))
}

fn before_request_callback(
    callback: PyObject,
) -> impl FnMut(&mut qiniu_sdk::http_client::RequestBuilderParts<'_>) -> AnyResult<()>
       + Send
       + Sync
       + 'static {
    move |parts| {
        Python::with_gil(|py| callback.call1(py, (RequestBuilderParts::new(parts),)))?;
        Ok(())
    }
}

fn after_response_ok_callback(
    callback: PyObject,
) -> impl FnMut(&mut qiniu_sdk::http::ResponseParts) -> AnyResult<()> + Send + Sync + 'static {
    move |parts| {
        Python::with_gil(|py| callback.call1(py, (HttpResponsePartsContext::new(parts),)))?;
        Ok(())
    }
}

/// 列举操作迭代器
///
/// 可以通过 `Bucket::list` 方法获取该迭代器。
#[pyclass]
#[derive(Debug)]
struct ObjectsLister {
    params: Pin<Arc<ObjectsIteratorParams>>,
}

#[pymethods]
impl ObjectsLister {
    fn __iter__(&self, py: Python<'_>) -> PyResult<ObjectsIterator> {
        let iter = self.make_list_builder(py).iter();
        Ok(ObjectsIterator {
            iter,
            _params: self.params.to_owned(),
        })
    }

    fn __aiter__(&self, py: Python<'_>) -> PyResult<AsyncObjectsIterator> {
        let stream = self.make_list_builder(py).stream().peekable();
        Ok(AsyncObjectsIterator {
            inner: Arc::new(AsyncObjectsIteratorInner {
                stream: AsyncMutex::new(stream),
                ended: AtomicBool::new(false),
            }),
            _params: self.params.to_owned(),
        })
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl ObjectsLister {
    fn make_list_builder(&self, py: Python<'_>) -> qiniu_sdk::objects::ListBuilder<'static> {
        let mut list_builder: qiniu_sdk::objects::ListBuilder<'static> = {
            let builder = self.params.bucket.0.list();
            unsafe { transmute(builder) }
        };
        if let Some(limit) = self.params.limit {
            list_builder.limit(limit);
        }
        if let Some(prefix) = &self.params.prefix {
            list_builder.prefix(Cow::Borrowed(unsafe { transmute(prefix.as_str()) }));
        }
        if let Some(marker) = &self.params.marker {
            list_builder.marker(Cow::Borrowed(unsafe { transmute(marker.as_str()) }));
        }
        if let Some(version) = self.params.version {
            list_builder.version(version.into());
        }
        if let Some(true) = self.params.need_parts {
            list_builder.need_parts();
        }
        if let Some(callback) = &self.params.before_request_callback {
            list_builder.before_request_callback(before_request_callback(callback.clone_ref(py)));
        }
        if let Some(callback) = &self.params.after_response_ok_callback {
            list_builder
                .after_response_ok_callback(after_response_ok_callback(callback.clone_ref(py)));
        }
        list_builder
    }
}

#[derive(Debug)]
struct ObjectsIteratorParams {
    bucket: Bucket,
    limit: Option<usize>,
    prefix: Option<String>,
    marker: Option<String>,
    version: Option<ListVersion>,
    need_parts: Option<bool>,
    before_request_callback: Option<PyObject>,
    after_response_ok_callback: Option<PyObject>,
}

/// 列举操作迭代器
#[pyclass]
#[derive(Debug)]
struct ObjectsIterator {
    _params: Pin<Arc<ObjectsIteratorParams>>,
    iter: qiniu_sdk::objects::ListIter<'static>,
}

#[pymethods]
impl ObjectsIterator {
    fn __next__(&mut self) -> PyResult<Option<PyObject>> {
        self.iter
            .next()
            .map(|result| {
                result.map(|entry| convert_json_value_to_py_object(&serde_json::Value::from(entry)))
            })
            .transpose()
            .map_err(QiniuApiCallError::from_err)?
            .transpose()
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// 异步列举操作迭代器
#[pyclass]
#[derive(Debug)]
struct AsyncObjectsIterator {
    _params: Pin<Arc<ObjectsIteratorParams>>,
    inner: Arc<AsyncObjectsIteratorInner>,
}

#[derive(Debug)]
struct AsyncObjectsIteratorInner {
    stream: AsyncMutex<AsyncPeekable<qiniu_sdk::objects::ListStream<'static>>>,
    ended: AtomicBool,
}

#[pymethods]
impl AsyncObjectsIterator {
    fn __anext__(&mut self, py: Python<'_>) -> Option<PyObject> {
        if self.inner.ended.load(Ordering::SeqCst) {
            return None;
        }
        let inner = self.inner.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let mut stream = inner.stream.lock().await;
            let entry = stream
                .try_next()
                .await
                .map(
                    |entry: Option<
                        qiniu_sdk::objects::apis::storage::get_objects::ListedObjectEntry,
                    >| {
                        entry.map(|entry: qiniu_sdk::objects::apis::storage::get_objects::ListedObjectEntry| {
                            convert_json_value_to_py_object(&serde_json::Value::from(entry))
                        })
                    },
                )
                .transpose()
                .map(|result| {
                    result.map_err(QiniuApiCallError::from_err).and_then(|res| res)
                })
                .transpose()?;
            if Pin::new(&mut *stream).peek_mut().await.is_none() {
                inner.ended.store(true, Ordering::SeqCst);
            }
            Ok(entry)
        })
        .ok()
        .map(|any| any.into_py(py))
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// 列举 API 版本
///
/// 目前支持 V1 和 V2，默认为 V2
#[pyclass]
#[derive(Copy, Clone, Debug)]
enum ListVersion {
    /// 列举 API V1
    V1 = 1,

    /// 列举 API V2
    V2 = 2,
}

impl From<ListVersion> for qiniu_sdk::objects::ListVersion {
    fn from(version: ListVersion) -> Self {
        match version {
            ListVersion::V1 => qiniu_sdk::objects::ListVersion::V1,
            ListVersion::V2 => qiniu_sdk::objects::ListVersion::V2,
        }
    }
}

impl From<qiniu_sdk::objects::ListVersion> for ListVersion {
    fn from(version: qiniu_sdk::objects::ListVersion) -> Self {
        match version {
            qiniu_sdk::objects::ListVersion::V1 => ListVersion::V1,
            qiniu_sdk::objects::ListVersion::V2 => ListVersion::V2,
            _ => unreachable!("Unrecognized ListVersion: {:?}", version),
        }
    }
}
