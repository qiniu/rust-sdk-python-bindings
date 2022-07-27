use super::{
    exceptions::QiniuEmptyChainCredentialsProvider,
    utils::{parse_header_value, parse_headers, parse_method, parse_uri, PythonIoBase},
};
use pyo3::prelude::*;
use qiniu_sdk::credential::{QINIU_ACCESS_KEY_ENV_KEY, QINIU_SECRET_KEY_ENV_KEY};
use std::{collections::HashMap, future::Future, io::Result as IoResult, pin::Pin, time::Duration};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "credential")?;
    m.add("QINIU_ACCESS_KEY_ENV_KEY", QINIU_ACCESS_KEY_ENV_KEY)?;
    m.add("QINIU_SECRET_KEY_ENV_KEY", QINIU_SECRET_KEY_ENV_KEY)?;
    m.add_class::<Credential>()?;
    m.add_class::<CredentialProvider>()?;
    m.add_class::<GlobalCredentialProvider>()?;
    m.add_class::<EnvCredentialProvider>()?;
    m.add_class::<ChainCredentialsProvider>()?;
    m.add_class::<GetOptions>()?;
    Ok(m)
}

/// 认证信息
///
/// 通过 `Credential(access_key, secret_key)` 创建
#[pyclass(extends = CredentialProvider)]
#[derive(Debug, Clone)]
#[pyo3(text_signature = "(access_key, secret_key)")]
struct Credential;

#[pymethods]
impl Credential {
    /// 创建认证信息
    #[new]
    fn new(access_key: String, secret_key: String) -> (Self, CredentialProvider) {
        (
            Self,
            CredentialProvider(Box::new(qiniu_sdk::credential::Credential::new(
                access_key, secret_key,
            ))),
        )
    }

    /// 获取认证信息的 AccessKey
    #[getter]
    fn get_access_key(self_: PyRef<'_, Self>) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_.0.get(Default::default())?.access_key().to_string())
    }

    /// 获取认证信息的 SecretKey
    #[getter]
    fn get_secret_key(self_: PyRef<'_, Self>) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_.0.get(Default::default())?.secret_key().to_string())
    }

    /// 使用七牛签名算法对数据进行签名
    ///
    /// 参考 https://developer.qiniu.com/kodo/manual/1201/access-token
    #[pyo3(text_signature = "($self, data)")]
    fn sign(self_: PyRef<'_, Self>, data: Vec<u8>) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_.0.get(Default::default())?.sign(&data))
    }

    /// 使用七牛签名算法对输入流数据进行签名
    ///
    /// 参考 https://developer.qiniu.com/kodo/manual/1201/access-token
    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_reader(self_: PyRef<'_, Self>, io_base: PyObject) -> PyResult<String> {
        let super_ = self_.as_ref();
        let signature = super_
            .0
            .get(Default::default())?
            .sign_reader(&mut PythonIoBase::new(io_base))?;
        Ok(signature)
    }

    /// 使用七牛签名算法对异步输入流数据进行签名
    ///
    /// 参考 https://developer.qiniu.com/kodo/manual/1201/access-token
    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_async_reader<'p>(
        self_: PyRef<'p, Self>,
        io_base: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let super_ = self_.as_ref();
        let credential = super_.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let signature = credential
                .async_get(Default::default())
                .await?
                .sign_async_reader(&mut PythonIoBase::new(io_base).into_async_read())
                .await?;
            Ok(signature)
        })
    }

    /// 对对象的下载 URL 签名，可以生成私有存储空间的下载地址
    #[pyo3(text_signature = "($self, url, secs)")]
    fn sign_download_url(self_: PyRef<'_, Self>, url: &str, secs: u64) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_
            .0
            .get(Default::default())?
            .sign_download_url(parse_uri(url)?, Duration::from_secs(secs))
            .to_string())
    }

    /// 使用七牛签名算法 V1 对 HTTP 请求（请求体为内存数据）进行签名，返回 Authorization 的值
    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request(
        self_: PyRef<'_, Self>,
        url: &str,
        content_type: Option<&str>,
        body: &[u8],
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let url = parse_uri(url)?;
        let content_type = content_type.map(parse_header_value).transpose()?;
        Ok(super_
            .0
            .get(Default::default())?
            .authorization_v1_for_request(&url, content_type.as_ref(), body))
    }

    /// 使用七牛签名算法 V1 对 HTTP 请求（请求体为输入流）进行签名，返回 Authorization 的值
    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request_with_body_reader(
        self_: PyRef<'_, Self>,
        url: &str,
        content_type: Option<&str>,
        body: PyObject,
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let url = parse_uri(url)?;
        let content_type = content_type.map(parse_header_value).transpose()?;
        let auth = super_
            .0
            .get(Default::default())?
            .authorization_v1_for_request_with_body_reader(
                &url,
                content_type.as_ref(),
                &mut PythonIoBase::new(body),
            )?;
        Ok(auth)
    }

    /// 使用七牛签名算法 V1 对 HTTP 请求（请求体为异步输入流）进行签名，返回 Authorization 的值
    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request_with_async_body_reader<'p>(
        self_: PyRef<'_, Self>,
        url: &str,
        content_type: Option<&str>,
        body: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let super_ = self_.as_ref();
        let url = parse_uri(url)?;
        let content_type = content_type.map(parse_header_value).transpose()?;
        let credential = super_.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let auth = credential
                .async_get(Default::default())
                .await?
                .authorization_v1_for_request_with_async_body_reader(
                    &url,
                    content_type.as_ref(),
                    &mut PythonIoBase::new(body).into_async_read(),
                )
                .await?;
            Ok(auth)
        })
    }

    /// 使用七牛签名算法 V2 对 HTTP 请求（请求体为内存数据）进行签名，返回 Authorization 的值
    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request(
        self_: PyRef<'_, Self>,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: &[u8],
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let method = parse_method(method)?;
        let url = parse_uri(url)?;
        let headers = parse_headers(headers)?;
        Ok(super_
            .0
            .get(Default::default())?
            .authorization_v2_for_request(&method, &url, &headers, body))
    }

    /// 使用七牛签名算法 V2 对 HTTP 请求（请求体为输入流）进行签名，返回 Authorization 的值
    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request_with_body_reader(
        self_: PyRef<'_, Self>,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: PyObject,
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let method = parse_method(method)?;
        let url = parse_uri(url)?;
        let headers = parse_headers(headers)?;
        let auth = super_
            .0
            .get(Default::default())?
            .authorization_v2_for_request_with_body_reader(
                &method,
                &url,
                &headers,
                &mut PythonIoBase::new(body),
            )?;
        Ok(auth)
    }

    /// 使用七牛签名算法 V2 对 HTTP 请求（请求体为异步输入流）进行签名，返回 Authorization 的值
    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request_with_async_body_reader<'p>(
        self_: PyRef<'_, Self>,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let super_ = self_.as_ref();
        let method = parse_method(method)?;
        let url = parse_uri(url)?;
        let headers = parse_headers(headers)?;
        let credential = super_.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let auth = credential
                .async_get(Default::default())
                .await?
                .authorization_v2_for_request_with_async_body_reader(
                    &method,
                    &url,
                    &headers,
                    &mut PythonIoBase::new(body).into_async_read(),
                )
                .await?;
            Ok(auth)
        })
    }
}

/// 认证信息获取接口
///
/// 抽象类
#[pyclass(subclass)]
#[derive(Debug, Clone)]
pub(super) struct CredentialProvider(Box<dyn qiniu_sdk::credential::CredentialProvider>);

#[pymethods]
impl CredentialProvider {
    /// 返回七牛认证信息
    ///
    /// 该方法的异步版本为 [`Self::async_get`]。
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn get(&self, opts: Option<GetOptions>, py: Python<'_>) -> PyResult<Py<Credential>> {
        Py::new(
            py,
            (
                Credential,
                CredentialProvider(Box::new(
                    py.allow_threads(|| self.0.get(opts.unwrap_or_default().0))?
                        .into_credential(),
                )),
            ),
        )
    }

    /// 异步返回七牛认证信息
    #[args(opts = "None")]
    #[pyo3(text_signature = "($self, opts = None)")]
    fn async_get<'p>(&self, opts: Option<GetOptions>, py: Python<'p>) -> PyResult<&'p PyAny> {
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let py_initializer = (
                Credential,
                CredentialProvider(Box::new(
                    credential
                        .async_get(opts.unwrap_or_default().0)
                        .await?
                        .into_credential(),
                )),
            );
            Python::with_gil(|py| Py::new(py, py_initializer))
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl qiniu_sdk::credential::CredentialProvider for CredentialProvider {
    fn get(
        &self,
        opts: qiniu_sdk::credential::GetOptions,
    ) -> IoResult<qiniu_sdk::credential::GotCredential> {
        self.0.get(opts)
    }

    fn async_get<'a>(
        &'a self,
        opts: qiniu_sdk::credential::GetOptions,
    ) -> Pin<Box<dyn Future<Output = IoResult<qiniu_sdk::credential::GotCredential>> + 'a + Send>>
    {
        self.0.async_get(opts)
    }
}

/// 全局认证信息提供者，可以将认证信息配置在全局变量中。任何全局认证信息提供者实例都可以设置和访问全局认证信息。
///
/// 通过 `GlobalCredentialProvider()` 创建
#[pyclass(extends = CredentialProvider)]
#[pyo3(text_signature = "()")]
struct GlobalCredentialProvider;

#[pymethods]
impl GlobalCredentialProvider {
    /// 创建全局认证信息提供者
    #[new]
    fn new() -> (Self, CredentialProvider) {
        (
            Self,
            CredentialProvider(Box::new(qiniu_sdk::credential::GlobalCredentialProvider)),
        )
    }

    /// 配置全局认证信息
    #[staticmethod]
    #[pyo3(text_signature = "(credential)")]
    fn setup(credential: PyRef<'_, Credential>) -> PyResult<()> {
        qiniu_sdk::credential::GlobalCredentialProvider::setup(
            credential.into_super().0.get(Default::default())?.into(),
        );
        Ok(())
    }

    /// 清空全局认证信息
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn clear() {
        qiniu_sdk::credential::GlobalCredentialProvider::clear();
    }
}

/// 环境变量认证信息提供者，可以将认证信息配置在环境变量中。
///
/// 通过 `EnvCredentialProvider()` 创建
#[pyclass(extends = CredentialProvider)]
#[pyo3(text_signature = "()")]
struct EnvCredentialProvider;

#[pymethods]
impl EnvCredentialProvider {
    /// 创建环境变量认证信息提供者
    #[new]
    fn new() -> (Self, CredentialProvider) {
        (
            Self,
            CredentialProvider(Box::new(qiniu_sdk::credential::EnvCredentialProvider)),
        )
    }

    /// 配置环境变量认证信息提供者
    #[staticmethod]
    #[pyo3(text_signature = "(credential)")]
    fn setup(credential: PyRef<'_, Credential>) -> PyResult<()> {
        qiniu_sdk::credential::EnvCredentialProvider::setup(
            &credential.into_super().0.get(Default::default())?.into(),
        );
        Ok(())
    }

    /// 清空环境变量认证信息
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn clear() {
        qiniu_sdk::credential::EnvCredentialProvider::clear();
    }
}

/// 认证信息串提供者
///
/// 将多个认证信息提供者串联，遍历并找寻第一个可用认证信息
///
/// 通过 `ChainCredentialsProvider([credential1, credential2, credential3, ...])` 创建
#[pyclass(extends = CredentialProvider)]
#[derive(Debug, Copy, Clone, Default)]
#[pyo3(text_signature = "(creds)")]
struct ChainCredentialsProvider;

#[pymethods]
impl ChainCredentialsProvider {
    /// 创建认证信息串提供者
    #[new]
    fn new(creds: Vec<CredentialProvider>) -> PyResult<(Self, CredentialProvider)> {
        let mut builder: Option<qiniu_sdk::credential::ChainCredentialsProviderBuilder> = None;
        for cred in creds {
            if let Some(builder) = &mut builder {
                builder.append_credential(cred.0);
            } else {
                builder = Some(qiniu_sdk::credential::ChainCredentialsProvider::builder(
                    cred.0,
                ));
            }
        }
        if let Some(builder) = &mut builder {
            Ok((Self, CredentialProvider(Box::new(builder.build()))))
        } else {
            Err(QiniuEmptyChainCredentialsProvider::new_err(
                "creds is empty",
            ))
        }
    }
}

/// 获取认证信息的选项
///
/// 通过 `GetOptions()` 创建
#[pyclass]
#[derive(Default, Copy, Clone)]
#[pyo3(text_signature = "()")]
struct GetOptions(qiniu_sdk::credential::GetOptions);

#[pymethods]
impl GetOptions {
    /// 创建认证信息的选项
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
