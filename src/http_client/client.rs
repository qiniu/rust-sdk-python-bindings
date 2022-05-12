use crate::{
    credential::CredentialProvider,
    exceptions::QiniuAuthorizationError,
    http::{AsyncHttpRequest, SyncHttpRequest},
    upload_token::UploadTokenProvider,
};
use pyo3::prelude::*;
use qiniu_sdk::prelude::AuthorizationProvider;

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Authorization>()?;

    Ok(())
}

/// 七牛鉴权签名
#[pyclass]
struct Authorization(qiniu_sdk::http_client::Authorization<'static>);

#[pymethods]
impl Authorization {
    /// 根据上传凭证获取接口创建一个上传凭证签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(upload_token_provider)")]
    fn upload_token(provider: UploadTokenProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::uptoken(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v1 签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn v1(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v1(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v2 签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn v2(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v2(provider))
    }

    /// 根据认证信息获取接口创建一个下载凭证签名算法的签名
    #[staticmethod]
    #[pyo3(text_signature = "(credential_provider)")]
    fn download(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::download(provider))
    }

    #[pyo3(text_signature = "($self, request)")]
    fn sign(&self, request: &mut SyncHttpRequest) -> PyResult<()> {
        self.0
            .sign(request)
            .map_err(|err| QiniuAuthorizationError::new_err(err.to_string()))
    }

    #[pyo3(text_signature = "($self, request)")]
    fn async_sign<'p>(
        &self,
        request: &mut AsyncHttpRequest,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let auth = self.0.to_owned();
        let request = request.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            auth.async_sign(&mut *request.lock()?)
                .await
                .map_err(|err| QiniuAuthorizationError::new_err(err.to_string()))
        })
    }
}
