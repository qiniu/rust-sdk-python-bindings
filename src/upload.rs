use super::{
    credential::CredentialProvider,
    upload_token::{on_policy_generated_callback, UploadTokenProvider},
};
use pyo3::prelude::*;
use std::time::Duration;

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "upload")?;
    m.add_class::<UploadTokenSigner>()?;
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
