use crate::{credential::CredentialProvider, upload_token::UploadTokenProvider};
use pyo3::prelude::*;

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
    fn upload_token(provider: UploadTokenProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::uptoken(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v1 签名算法的签名
    #[staticmethod]
    fn v1(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v1(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v2 签名算法的签名
    #[staticmethod]
    fn v2(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::v2(provider))
    }

    /// 根据认证信息获取接口创建一个下载凭证签名算法的签名
    #[staticmethod]
    fn download(provider: CredentialProvider) -> Self {
        Self(qiniu_sdk::http_client::Authorization::download(provider))
    }
}
