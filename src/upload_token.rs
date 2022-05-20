use super::{
    credential::CredentialProvider,
    exceptions::{
        QiniuBase64Error, QiniuCallbackError, QiniuIoError, QiniuJsonError, QiniuTimeError,
        QiniuUploadTokenFormatError,
    },
    utils::{convert_json_value_to_py_object, convert_py_any_to_json_value},
};
use pyo3::{
    prelude::*,
    types::{PyString, PyTuple},
};
use qiniu_sdk::{
    prelude::UploadTokenProviderExt,
    upload_token::{
        FileType, GotAccessKey, GotUploadPolicy, ParseError, ParseResult, ToStringError,
        ToStringResult,
    },
};
use std::{
    borrow::Cow,
    collections::HashMap,
    future::Future,
    mem::take,
    pin::Pin,
    time::{Duration, SystemTime},
};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload_token")?;
    m.add_class::<UploadPolicy>()?;
    m.add_class::<UploadPolicyBuilder>()?;
    m.add_class::<UploadTokenProvider>()?;
    m.add_class::<GetAccessKeyOptions>()?;
    m.add_class::<GetPolicyOptions>()?;
    m.add_class::<ToStringOptions>()?;
    m.add_class::<StaticUploadTokenProvider>()?;
    m.add_class::<FromUploadPolicy>()?;
    m.add_class::<BucketUploadTokenProvider>()?;
    m.add_class::<ObjectUploadTokenProvider>()?;
    Ok(m)
}

/// 上传策略
///
/// 可以阅读 <https://developer.qiniu.com/kodo/manual/1206/put-policy> 了解七牛安全机制。
#[pyclass]
#[derive(Clone)]
struct UploadPolicy(qiniu_sdk::upload_token::UploadPolicy);

#[pymethods]
impl UploadPolicy {
    /// 为指定的存储空间生成的上传策略
    ///
    /// 允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, lifetime, **fields)")]
    fn new_for_bucket(
        bucket: &str,
        upload_token_lifetime: u64,
        fields: Option<HashMap<String, PyObject>>,
    ) -> PyResult<UploadPolicyBuilder> {
        UploadPolicyBuilder::new_for_bucket(bucket, upload_token_lifetime, fields)
    }

    /// 为指定的存储空间和对象名称生成的上传策略
    ///
    /// 允许用户以指定的对象名称上传文件到指定的存储空间。
    /// 上传客户端不能指定与上传策略冲突的对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, object, lifetime, **fields)")]
    fn new_for_object(
        bucket: &str,
        object: &str,
        upload_token_lifetime: u64,
        fields: Option<HashMap<String, PyObject>>,
    ) -> PyResult<UploadPolicyBuilder> {
        UploadPolicyBuilder::new_for_object(bucket, object, upload_token_lifetime, fields)
    }

    /// 为指定的存储空间和对象名称前缀生成的上传策略
    ///
    /// 允许用户以指定的对象名称前缀上传文件到指定的存储空间。
    /// 上传客户端指定包含该前缀的对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, prefix, lifetime, **fields)")]
    fn new_for_objects_with_prefix(
        bucket: &str,
        prefix: &str,
        upload_token_lifetime: u64,
        fields: Option<HashMap<String, PyObject>>,
    ) -> PyResult<UploadPolicyBuilder> {
        UploadPolicyBuilder::new_for_objects_with_prefix(
            bucket,
            prefix,
            upload_token_lifetime,
            fields,
        )
    }

    /// 解析 JSON 格式的上传凭证
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let policy = qiniu_sdk::upload_token::UploadPolicy::from_json(json)
            .map_err(QiniuJsonError::from_err)?;
        Ok(UploadPolicy(policy))
    }

    /// 存储空间约束
    #[pyo3(text_signature = "($self)")]
    fn bucket<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.bucket().map(|s| PyString::new(py, s))
    }

    /// 对象名称约束或对象名称前缀约束
    #[pyo3(text_signature = "($self)")]
    fn key<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.key().map(|s| PyString::new(py, s))
    }

    /// 是否是对象名称前缀约束
    #[pyo3(text_signature = "($self)")]
    fn use_prefixal_object_key(&self) -> bool {
        self.0.use_prefixal_object_key()
    }

    /// 是否仅允许新增对象，不允许覆盖对象
    #[pyo3(text_signature = "($self)")]
    fn is_insert_only(&self) -> bool {
        self.0.is_insert_only()
    }

    /// 是否启用 MIME 类型自动检测
    #[pyo3(text_signature = "($self)")]
    fn mime_detection_enabled(&self) -> bool {
        self.0.mime_detection_enabled()
    }

    /// 上传凭证过期时间戳
    #[pyo3(text_signature = "($self)")]
    fn token_deadline(&self) -> PyResult<Option<u64>> {
        self.0
            .token_deadline()
            .map(|deadline| {
                deadline
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
            })
            .transpose()
            .map_err(QiniuTimeError::from_err)
    }

    /// Web 端文件上传成功后，浏览器执行 303 跳转的 URL
    #[pyo3(text_signature = "($self)")]
    fn return_url<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.return_url().map(|s| PyString::new(py, s))
    }

    /// 上传成功后，自定义七牛云最终返回给上传端的数据
    #[pyo3(text_signature = "($self)")]
    fn return_body<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.return_body().map(|s| PyString::new(py, s))
    }

    /// 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表
    #[pyo3(text_signature = "($self)")]
    fn callback_urls<'p>(&self, py: Python<'p>) -> Option<Vec<&'p PyString>> {
        self.0
            .callback_urls()
            .map(|url| url.map(|s| PyString::new(py, s)).collect())
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的 `Host`
    #[pyo3(text_signature = "($self)")]
    fn callback_host<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.callback_host().map(|s| PyString::new(py, s))
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的内容
    ///
    /// 支持魔法变量和自定义变量
    #[pyo3(text_signature = "($self)")]
    fn callback_body<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.callback_body().map(|s| PyString::new(py, s))
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的 `Content-Type`
    ///
    /// 默认为 `application/x-www-form-urlencoded`，也可设置为 `application/json`
    #[pyo3(text_signature = "($self)")]
    fn callback_body_type<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.callback_body_type().map(|s| PyString::new(py, s))
    }

    /// 自定义对象名称
    ///
    /// 支持魔法变量和自定义变量
    #[pyo3(text_signature = "($self)")]
    fn save_key<'p>(&self, py: Python<'p>) -> Option<&'p PyString> {
        self.0.save_key().map(|s| PyString::new(py, s))
    }

    /// 是否忽略客户端指定的对象名称，强制使用自定义对象名称进行文件命名
    #[pyo3(text_signature = "($self)")]
    fn is_save_key_forced(&self) -> bool {
        self.0.is_save_key_forced()
    }

    /// 限定上传文件尺寸的上限，单位为字节
    #[pyo3(text_signature = "($self)")]
    fn maximum_file_size(&self) -> Option<u64> {
        self.0.file_size_limitation().1
    }

    /// 限定上传文件尺寸的下限，单位为字节
    #[pyo3(text_signature = "($self)")]
    fn minimum_file_size(&self) -> Option<u64> {
        self.0.file_size_limitation().0
    }

    /// 限定用户上传的文件类型
    ///
    /// 指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
    /// 匹配成功则允许上传，匹配失败则返回 403 状态码
    #[pyo3(text_signature = "($self)")]
    fn mime_types<'p>(&self, py: Python<'p>) -> Option<Vec<&'p PyString>> {
        self.0
            .mime_types()
            .map(|mime_type| mime_type.map(|s| PyString::new(py, s)).collect())
    }

    /// 文件类型
    #[pyo3(text_signature = "($self)")]
    fn file_type(&self) -> Option<u8> {
        self.0.file_type().map(|ft| ft.into())
    }

    /// 对象生命周期
    ///
    /// 单位为秒，但精确到天
    #[pyo3(text_signature = "($self)")]
    fn object_lifetime(&self) -> Option<u64> {
        self.0.object_lifetime().map(|dur| dur.as_secs())
    }

    /// 获取 JSON 格式的上传凭证
    #[pyo3(text_signature = "($self)")]
    fn as_json(&self) -> String {
        self.0.as_json()
    }

    /// 根据指定的上传策略字段获取相应的值
    #[pyo3(text_signature = "($self, key)")]
    fn get(&self, key: &str) -> PyResult<Option<PyObject>> {
        self.0
            .get(key)
            .map(convert_json_value_to_py_object)
            .transpose()
    }

    /// 获取上传策略的字段迭代器
    #[pyo3(text_signature = "($self)")]
    fn keys<'p>(&self, py: Python<'p>) -> Vec<&'p PyString> {
        self.0.keys().map(|s| PyString::new(py, s)).collect()
    }

    /// 获取上传策略的字段值的迭代器
    #[pyo3(text_signature = "($self)")]
    fn values(&self) -> PyResult<Vec<PyObject>> {
        self.0
            .values()
            .map(convert_json_value_to_py_object)
            .collect()
    }

    /// 将上传策略转换为动态上传凭证提供者的实例
    #[pyo3(text_signature = "($self)")]
    fn to_upload_token_provider(&self, credential: CredentialProvider) -> UploadTokenProvider {
        UploadTokenProvider(Box::new(
            self.to_owned()
                .0
                .into_dynamic_upload_token_provider(credential),
        ))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 上传策略构建器
///
/// 用于生成上传策略，一旦生成完毕，上传策略将无法被修改
#[pyclass]
#[derive(Clone)]
struct UploadPolicyBuilder(qiniu_sdk::upload_token::UploadPolicyBuilder);

#[pymethods]
impl UploadPolicyBuilder {
    /// 为指定的存储空间生成的上传策略
    ///
    /// 允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, lifetime_secs, **fields)")]
    fn new_for_bucket(
        bucket: &str,
        lifetime_secs: u64,
        fields: Option<HashMap<String, PyObject>>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::upload_token::UploadPolicy::new_for_bucket(
            bucket,
            Duration::from_secs(lifetime_secs),
        );
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields)?;
        }
        Ok(Self(builder))
    }

    /// 为指定的存储空间和对象名称生成的上传策略
    ///
    /// 允许用户以指定的对象名称上传文件到指定的存储空间。
    /// 上传客户端不能指定与上传策略冲突的对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, object, lifetime_secs, **fields)")]
    fn new_for_object(
        bucket: &str,
        object: &str,
        lifetime_secs: u64,
        fields: Option<HashMap<String, PyObject>>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::upload_token::UploadPolicy::new_for_object(
            bucket,
            object,
            Duration::from_secs(lifetime_secs),
        );
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields)?;
        }
        Ok(Self(builder))
    }

    /// 为指定的存储空间和对象名称前缀生成的上传策略
    ///
    /// 允许用户以指定的对象名称前缀上传文件到指定的存储空间。
    /// 上传客户端指定包含该前缀的对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    #[staticmethod]
    #[args(fields = "**")]
    #[pyo3(text_signature = "(bucket, prefix, lifetime_secs, **fields)")]
    fn new_for_objects_with_prefix(
        bucket: &str,
        prefix: &str,
        lifetime_secs: u64,
        fields: Option<HashMap<String, PyObject>>,
    ) -> PyResult<Self> {
        let mut builder = qiniu_sdk::upload_token::UploadPolicy::new_for_objects_with_prefix(
            bucket,
            prefix,
            Duration::from_secs(lifetime_secs),
        );
        if let Some(fields) = fields {
            Self::set_builder_from_py_dict(&mut builder, fields)?;
        }
        Ok(Self(builder))
    }

    /// 生成上传策略
    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> UploadPolicy {
        UploadPolicy(self.0.build())
    }

    /// 指定上传凭证有效期
    #[pyo3(text_signature = "($self, lifetime)")]
    fn token_lifetime(&mut self, lifetime_secs: u64) {
        self.0.token_lifetime(Duration::from_secs(lifetime_secs));
    }

    /// 指定上传凭证过期时间
    #[pyo3(text_signature = "($self, deadline)")]
    fn token_deadline(&mut self, timestamp: u64) {
        self.0
            .token_deadline(SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp));
    }

    /// 仅允许创建新的对象，不允许覆盖和修改同名对象
    #[pyo3(text_signature = "($self)")]
    fn insert_only(&mut self) {
        self.0.insert_only();
    }

    /// 启用 MIME 类型自动检测
    #[pyo3(text_signature = "($self)")]
    fn enable_mime_detection(&mut self) {
        self.0.enable_mime_detection();
    }

    /// 禁用 MIME 类型自动检测
    #[pyo3(text_signature = "($self)")]
    fn disable_mime_detection(&mut self) {
        self.0.disable_mime_detection();
    }

    /// 设置文件类型
    #[pyo3(text_signature = "($self, file_type)")]
    fn file_type(&mut self, file_type: u8) {
        self.0.file_type(FileType::from(file_type));
    }

    /// Web 端文件上传成功后，浏览器执行 303 跳转的 URL
    ///
    /// 通常用于表单上传。
    /// 文件上传成功后会跳转到 `<return_url>?upload_ret=<queryString>`，
    /// `<queryString>` 包含 `return_body()` 内容。
    /// 如不设置 `return_url`，则直接将 `return_body()` 的内容返回给客户端
    #[pyo3(text_signature = "($self, url)")]
    fn return_url(&mut self, url: &str) {
        self.0.return_url(url);
    }

    #[pyo3(text_signature = "($self, body)")]
    fn return_body(&mut self, body: &str) {
        self.0.return_body(body);
    }

    /// 上传成功后，自定义七牛云最终返回给上传端（在指定 `return_url()` 时是携带在跳转路径参数中）的数据
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
    /// `return_body` 要求是合法的 JSON 文本。
    /// 例如 `{"key": $(key), "hash": $(etag), "w": $(imageInfo.width), "h": $(imageInfo.height)}`
    #[args(host = "\"\"", body = "\"\"", body_type = "\"\"")]
    #[pyo3(text_signature = "($self, urls, host = '', body = '', body_type = '')")]
    fn callback(&mut self, urls: Vec<String>, host: &str, body: &str, body_type: &str) {
        self.0.callback(urls, host, body, body_type);
    }

    /// 自定义对象名称
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
    /// `force` 为 `false` 时，`save_as` 字段仅当用户上传的时候没有主动指定对象名时起作用，
    /// `force` 为 `true` 时，将强制按 `save_as` 字段的内容命名
    #[args(force = "false")]
    #[pyo3(text_signature = "($self, save_as, force = False)")]
    fn save_as(&mut self, save_as: &str, force: bool) {
        self.0.save_as(save_as, force);
    }

    /// 限定上传文件尺寸的范围
    ///
    /// 单位为字节
    #[args(min = "None", max = "None")]
    #[pyo3(text_signature = "($self, min = None, max = None)")]
    fn file_size_limitation(&mut self, min: Option<u64>, max: Option<u64>) {
        match (min, max) {
            (Some(min), Some(max)) => {
                self.0.file_size_limitation(min..=max);
            }
            (Some(min), None) => {
                self.0.file_size_limitation(min..);
            }
            (None, Some(max)) => {
                self.0.file_size_limitation(..=max);
            }
            _ => {}
        }
    }

    /// 限定用户上传的文件类型
    ///
    /// 指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
    /// 匹配成功则允许上传，匹配失败则返回 403 状态码
    #[pyo3(text_signature = "($self, content_types)")]
    fn mime_types(&mut self, content_types: Vec<String>) {
        self.0.mime_types(content_types);
    }

    /// 对象生命周期
    ///
    /// 单位为秒，但精确到天
    #[pyo3(text_signature = "($self, lifetime)")]
    fn object_lifetime(&mut self, lifetime_secs: u64) {
        self.0.object_lifetime(Duration::from_secs(lifetime_secs));
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ToPyObject for UploadPolicyBuilder {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.to_owned().into_py(py)
    }
}

impl UploadPolicyBuilder {
    fn set_builder_from_py_dict(
        builder: &mut qiniu_sdk::upload_token::UploadPolicyBuilder,
        fields: HashMap<String, PyObject>,
    ) -> PyResult<()> {
        for (key, value) in fields.into_iter() {
            builder.set(key, convert_py_any_to_json_value(value)?);
        }
        Ok(())
    }
}

/// 上传凭证获取接口
///
/// 可以阅读 <https://developer.qiniu.com/kodo/manual/1208/upload-token> 了解七牛安全机制。
#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub(super) struct UploadTokenProvider(Box<dyn qiniu_sdk::upload_token::UploadTokenProvider>);

#[pymethods]
impl UploadTokenProvider {
    /// 从上传凭证内获取 AccessKey
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn access_key(&self, opts: Option<GetAccessKeyOptions>, py: Python<'_>) -> PyResult<String> {
        Ok(py
            .allow_threads(|| self.0.access_key(opts.unwrap_or_default().0))
            .map_err(convert_parse_error_to_py_err)?
            .into_access_key()
            .to_string())
    }

    /// 从上传凭证内获取上传策略
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn policy(&self, opts: Option<GetPolicyOptions>, py: Python<'_>) -> PyResult<UploadPolicy> {
        Ok(UploadPolicy(
            py.allow_threads(|| self.0.policy(opts.unwrap_or_default().0))
                .map_err(convert_parse_error_to_py_err)?
                .into_upload_policy(),
        ))
    }

    /// 生成字符串
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn to_token_string(&self, opts: Option<ToStringOptions>, py: Python<'_>) -> PyResult<String> {
        Ok(py
            .allow_threads(|| self.0.to_token_string(opts.unwrap_or_default().0))
            .map_err(convert_to_string_error_to_py_err)?
            .into_owned())
    }

    /// 获取上传凭证中的存储空间名称
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn bucket_name(&self, opts: Option<GetPolicyOptions>, py: Python<'_>) -> PyResult<String> {
        Ok(py
            .allow_threads(|| self.0.bucket_name(opts.unwrap_or_default().0))
            .map_err(convert_parse_error_to_py_err)?
            .to_string())
    }

    /// 异步从上传凭证内获取 AccessKey
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn async_access_key<'p>(
        &self,
        opts: Option<GetAccessKeyOptions>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(provider
                .async_access_key(opts.unwrap_or_default().0)
                .await
                .map_err(convert_parse_error_to_py_err)?
                .into_access_key()
                .to_string())
        })
    }

    /// 异步从上传凭证内获取上传策略
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn async_policy<'p>(
        &self,
        opts: Option<GetPolicyOptions>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(UploadPolicy(
                provider
                    .async_policy(opts.unwrap_or_default().0)
                    .await
                    .map_err(convert_parse_error_to_py_err)?
                    .into_upload_policy(),
            ))
        })
    }

    /// 异步生成字符串
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn async_to_token_string<'p>(
        &self,
        opts: Option<ToStringOptions>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(provider
                .async_to_token_string(opts.unwrap_or_default().0)
                .await
                .map_err(convert_to_string_error_to_py_err)?
                .into_owned())
        })
    }

    /// 异步获取上传凭证中的存储空间名称
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn async_bucket_name<'p>(
        &self,
        opts: Option<GetPolicyOptions>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let provider = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(provider
                .async_bucket_name(opts.unwrap_or_default().0)
                .await
                .map_err(convert_parse_error_to_py_err)?
                .to_string())
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self, py: Python<'_>) -> PyResult<String> {
        self.to_token_string(Default::default(), py)
    }
}

impl qiniu_sdk::upload_token::UploadTokenProvider for UploadTokenProvider {
    fn access_key(
        &self,
        opts: qiniu_sdk::upload_token::GetAccessKeyOptions,
    ) -> ParseResult<GotAccessKey> {
        self.0.access_key(opts)
    }

    fn policy(
        &self,
        opts: qiniu_sdk::upload_token::GetPolicyOptions,
    ) -> ParseResult<GotUploadPolicy> {
        self.0.policy(opts)
    }

    fn to_token_string(
        &self,
        opts: qiniu_sdk::upload_token::ToStringOptions,
    ) -> ToStringResult<Cow<'_, str>> {
        self.0.to_token_string(opts)
    }

    fn async_access_key<'a>(
        &'a self,
        opts: qiniu_sdk::upload_token::GetAccessKeyOptions,
    ) -> Pin<Box<dyn Future<Output = ParseResult<GotAccessKey>> + 'a + Send>> {
        self.0.async_access_key(opts)
    }

    fn async_policy<'a>(
        &'a self,
        opts: qiniu_sdk::upload_token::GetPolicyOptions,
    ) -> Pin<Box<dyn Future<Output = ParseResult<GotUploadPolicy>> + 'a + Send>> {
        self.0.async_policy(opts)
    }

    fn async_to_token_string<'a>(
        &'a self,
        opts: qiniu_sdk::upload_token::ToStringOptions,
    ) -> Pin<Box<dyn Future<Output = ToStringResult<Cow<'a, str>>> + 'a + Send>> {
        self.0.async_to_token_string(opts)
    }
}

fn convert_parse_error_to_py_err(err: ParseError) -> PyErr {
    match err {
        ParseError::CredentialGetError(err) => QiniuIoError::from_err(err),
        ParseError::InvalidUploadTokenFormat => QiniuUploadTokenFormatError::from_err(err),
        ParseError::Base64DecodeError(err) => QiniuBase64Error::from_err(err),
        ParseError::JsonDecodeError(err) => QiniuJsonError::from_err(err),
        ParseError::CallbackError(err) => QiniuCallbackError::from_err(err),
        err => unreachable!("Unrecognized error {:?}", err),
    }
}

fn convert_to_string_error_to_py_err(err: ToStringError) -> PyErr {
    match err {
        ToStringError::CredentialGetError(err) => QiniuIoError::from_err(err),
        ToStringError::CallbackError(err) => QiniuCallbackError::from_err(err),
        err => unreachable!("Unrecognized error {:?}", err),
    }
}

//r 静态上传凭证提供者
///
/// 根据已经被生成好的上传凭证字符串生成上传凭证获取接口的实例，可以将上传凭证解析为 Access Token 和上传策略
#[pyclass(extends = UploadTokenProvider)]
#[pyo3(text_signature = "(upload_token)")]
struct StaticUploadTokenProvider;

#[pymethods]
impl StaticUploadTokenProvider {
    #[new]
    fn new(upload_token: &str) -> (Self, UploadTokenProvider) {
        (
            Self,
            UploadTokenProvider(Box::new(
                qiniu_sdk::upload_token::StaticUploadTokenProvider::new(upload_token),
            )),
        )
    }
}

/// 根据上传凭证生成上传策略
#[pyclass(extends = UploadTokenProvider)]
#[pyo3(text_signature = "(upload_policy, credential)")]
struct FromUploadPolicy;

#[pymethods]
impl FromUploadPolicy {
    #[new]
    fn new(
        upload_policy: UploadPolicy,
        credential: CredentialProvider,
    ) -> (Self, UploadTokenProvider) {
        (
            Self,
            UploadTokenProvider(Box::new(qiniu_sdk::upload_token::FromUploadPolicy::new(
                upload_policy.0,
                credential,
            ))),
        )
    }
}

/// 基于存储空间的动态生成
///
/// 根据存储空间的快速生成上传凭证实例
#[pyclass(extends = UploadTokenProvider)]
#[pyo3(text_signature = "(bucket, lifetime_secs, credential, /, on_policy_generated = None)")]
struct BucketUploadTokenProvider;

#[pymethods]
impl BucketUploadTokenProvider {
    #[new]
    #[args(on_policy_generated = "None")]
    fn new(
        bucket: &str,
        lifetime_secs: u64,
        credential: CredentialProvider,
        on_policy_generated: Option<&PyAny>,
    ) -> (Self, UploadTokenProvider) {
        let mut builder = qiniu_sdk::upload_token::BucketUploadTokenProvider::builder(
            bucket,
            Duration::from_secs(lifetime_secs),
            credential,
        );
        if let Some(on_policy_generated) = on_policy_generated {
            builder = set_on_policy_generated_to_builder(builder, on_policy_generated);
        }
        let provider = builder.build();
        return (Self, UploadTokenProvider(Box::new(provider)));

        fn set_on_policy_generated_to_builder<'a, C: Clone + 'a>(
            mut builder: qiniu_sdk::upload_token::BucketUploadTokenProviderBuilder<'a, C>,
            any: &PyAny,
        ) -> qiniu_sdk::upload_token::BucketUploadTokenProviderBuilder<'a, C> {
            if any.is_callable() {
                builder = Python::with_gil(|py| {
                    let obj = any.to_object(py);
                    builder.on_policy_generated(move |upload_policy_builder| {
                        let builder = UploadPolicyBuilder(take(upload_policy_builder));
                        let builder = Python::with_gil(|py| {
                            obj.call1(py, PyTuple::new(py, [builder]))
                                .and_then(|retval| retval.extract::<UploadPolicyBuilder>(py))
                        })?;
                        *upload_policy_builder = builder.0;
                        Ok(())
                    })
                });
            }
            builder
        }
    }
}

/// 基于对象的动态生成
///
/// 根据对象的快速生成上传凭证实例
#[pyclass(extends = UploadTokenProvider)]
#[pyo3(
    text_signature = "(bucket, object, lifetime_secs, credential, /, on_policy_generated = None)"
)]
struct ObjectUploadTokenProvider;

#[pymethods]
impl ObjectUploadTokenProvider {
    #[new]
    #[args(on_policy_generated = "None")]
    fn new(
        bucket: &str,
        object: &str,
        lifetime_secs: u64,
        credential: CredentialProvider,
        on_policy_generated: Option<&PyAny>,
    ) -> (Self, UploadTokenProvider) {
        let mut builder = qiniu_sdk::upload_token::ObjectUploadTokenProvider::builder(
            bucket,
            object,
            Duration::from_secs(lifetime_secs),
            credential,
        );
        if let Some(on_policy_generated) = on_policy_generated {
            builder = set_on_policy_generated_to_builder(builder, on_policy_generated);
        }
        let provider = builder.build();
        return (Self, UploadTokenProvider(Box::new(provider)));

        fn set_on_policy_generated_to_builder<'a, C: Clone + 'a>(
            mut builder: qiniu_sdk::upload_token::ObjectUploadTokenProviderBuilder<'a, C>,
            any: &PyAny,
        ) -> qiniu_sdk::upload_token::ObjectUploadTokenProviderBuilder<'a, C> {
            if any.is_callable() {
                builder = Python::with_gil(|py| {
                    let obj = any.to_object(py);
                    builder.on_policy_generated(move |upload_policy_builder| {
                        let builder = UploadPolicyBuilder(take(upload_policy_builder));
                        let builder = Python::with_gil(|py| {
                            obj.call1(py, PyTuple::new(py, [builder]))
                                .and_then(|retval| retval.extract::<UploadPolicyBuilder>(py))
                        })?;
                        *upload_policy_builder = builder.0;
                        Ok(())
                    })
                });
            }
            builder
        }
    }
}

/// 获取 Access Key 的选项
#[pyclass]
#[derive(Default, Copy, Clone)]
#[pyo3(text_signature = "()")]
struct GetAccessKeyOptions(qiniu_sdk::upload_token::GetAccessKeyOptions);

#[pymethods]
impl GetAccessKeyOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 获取上传策略的选项
#[pyclass]
#[derive(Default, Copy, Clone)]
#[pyo3(text_signature = "()")]
struct GetPolicyOptions(qiniu_sdk::upload_token::GetPolicyOptions);

#[pymethods]
impl GetPolicyOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// 获取上传凭证的选项
#[pyclass]
#[derive(Default, Copy, Clone)]
#[pyo3(text_signature = "()")]
struct ToStringOptions(qiniu_sdk::upload_token::ToStringOptions);

#[pymethods]
impl ToStringOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
