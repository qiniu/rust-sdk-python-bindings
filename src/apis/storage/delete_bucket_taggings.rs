// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!
//
use crate::http_client::HttpClient;
use pyo3::prelude::*;
pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "delete_bucket_taggings")?;
    m.add_class::<Client>()?;
    Ok(m)
}
#[doc = "一键删除指定存储空间的所有标签"]
# [pyclass (extends = HttpClient)]
#[pyo3(
    text_signature = "(/, http_caller = None, use_https = None, appended_user_agent = None, request_retrier = None, backoff = None, chooser = None, resolver = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)"
)]
#[derive(Clone)]
struct Client;
#[pymethods]
impl Client {
    #[new]
    #[args(
        http_caller = "None",
        use_https = "None",
        appended_user_agent = "None",
        request_retrier = "None",
        backoff = "None",
        chooser = "None",
        resolver = "None",
        uploading_progress = "None",
        receive_response_status = "None",
        receive_response_header = "None",
        to_resolve_domain = "None",
        domain_resolved = "None",
        to_choose_ips = "None",
        ips_chosen = "None",
        before_request_signed = "None",
        after_request_signed = "None",
        response_ok = "None",
        response_error = "None",
        before_backoff = "None",
        after_backoff = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        http_caller: Option<crate::http::HttpCaller>,
        use_https: Option<bool>,
        appended_user_agent: Option<&str>,
        request_retrier: Option<crate::http_client::RequestRetrier>,
        backoff: Option<crate::http_client::Backoff>,
        chooser: Option<crate::http_client::Chooser>,
        resolver: Option<crate::http_client::Resolver>,
        uploading_progress: Option<PyObject>,
        receive_response_status: Option<PyObject>,
        receive_response_header: Option<PyObject>,
        to_resolve_domain: Option<PyObject>,
        domain_resolved: Option<PyObject>,
        to_choose_ips: Option<PyObject>,
        ips_chosen: Option<PyObject>,
        before_request_signed: Option<PyObject>,
        after_request_signed: Option<PyObject>,
        response_ok: Option<PyObject>,
        response_error: Option<PyObject>,
        before_backoff: Option<PyObject>,
        after_backoff: Option<PyObject>,
    ) -> PyResult<(Self, HttpClient)> {
        let client = HttpClient::new(
            http_caller,
            use_https,
            appended_user_agent,
            request_retrier,
            backoff,
            chooser,
            resolver,
            uploading_progress,
            receive_response_status,
            receive_response_header,
            to_resolve_domain,
            domain_resolved,
            to_choose_ips,
            ips_chosen,
            before_request_signed,
            after_request_signed,
            response_ok,
            response_error,
            before_backoff,
            after_backoff,
        )?;
        Ok((Self, client))
    }
    #[doc = "发出阻塞请求"]
    #[pyo3(
        text_signature = "(endpoints, credential, /, use_https = None, version = None, headers = None, query = None, query_pairs = None, appended_user_agent = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)"
    )]
    #[args(
        r#use_https = "None",
        r#version = "None",
        r#headers = "None",
        r#query = "None",
        r#query_pairs = "None",
        r#appended_user_agent = "None",
        r#uploading_progress = "None",
        r#receive_response_status = "None",
        r#receive_response_header = "None",
        r#to_resolve_domain = "None",
        r#domain_resolved = "None",
        r#to_choose_ips = "None",
        r#ips_chosen = "None",
        r#before_request_signed = "None",
        r#after_request_signed = "None",
        r#response_ok = "None",
        r#response_error = "None",
        r#before_backoff = "None",
        r#after_backoff = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn call(
        self_: PyRef<'_, Self>,
        r#endpoints: PyObject,
        r#credential: crate::credential::CredentialProvider,
        r#use_https: Option<bool>,
        r#version: Option<crate::http::Version>,
        r#headers: Option<std::collections::HashMap<String, String>>,
        r#query: Option<String>,
        r#query_pairs: Option<PyObject>,
        r#appended_user_agent: Option<String>,
        r#uploading_progress: Option<PyObject>,
        r#receive_response_status: Option<PyObject>,
        r#receive_response_header: Option<PyObject>,
        r#to_resolve_domain: Option<PyObject>,
        r#domain_resolved: Option<PyObject>,
        r#to_choose_ips: Option<PyObject>,
        r#ips_chosen: Option<PyObject>,
        r#before_request_signed: Option<PyObject>,
        r#after_request_signed: Option<PyObject>,
        r#response_ok: Option<PyObject>,
        r#response_error: Option<PyObject>,
        r#before_backoff: Option<PyObject>,
        r#after_backoff: Option<PyObject>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::http::SyncHttpResponse>> {
        let super_ = self_.into_super();
        let (resp, parts) = super_._call(
            "DELETE".to_owned(),
            endpoints,
            Some(vec![crate::http_client::ServiceName::r#Uc]),
            use_https,
            version,
            Some("/bucketTagging".to_owned()),
            headers,
            None,
            None,
            query,
            query_pairs,
            appended_user_agent,
            Some(crate::http_client::Authorization::from(
                qiniu_sdk::http_client::Authorization::v2(credential),
            )),
            Some(crate::http_client::Idempotent::r#Default),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            uploading_progress,
            receive_response_status,
            receive_response_header,
            to_resolve_domain,
            domain_resolved,
            to_choose_ips,
            ips_chosen,
            before_request_signed,
            after_request_signed,
            response_ok,
            response_error,
            before_backoff,
            after_backoff,
            py,
        )?;
        Py::new(py, (resp, parts))
    }
    #[doc = "发出异步请求"]
    #[pyo3(
        text_signature = "(endpoints, credential, /, use_https = None, version = None, headers = None, query = None, query_pairs = None, appended_user_agent = None, uploading_progress = None, receive_response_status = None, receive_response_header = None, to_resolve_domain = None, domain_resolved = None, to_choose_ips = None, ips_chosen = None, before_request_signed = None, after_request_signed = None, response_ok = None, response_error = None, before_backoff = None, after_backoff = None)"
    )]
    #[args(
        r#use_https = "None",
        r#version = "None",
        r#headers = "None",
        r#query = "None",
        r#query_pairs = "None",
        r#appended_user_agent = "None",
        r#uploading_progress = "None",
        r#receive_response_status = "None",
        r#receive_response_header = "None",
        r#to_resolve_domain = "None",
        r#domain_resolved = "None",
        r#to_choose_ips = "None",
        r#ips_chosen = "None",
        r#before_request_signed = "None",
        r#after_request_signed = "None",
        r#response_ok = "None",
        r#response_error = "None",
        r#before_backoff = "None",
        r#after_backoff = "None"
    )]
    #[allow(clippy::too_many_arguments)]
    fn async_call<'p>(
        self_: PyRef<'p, Self>,
        r#endpoints: PyObject,
        r#credential: crate::credential::CredentialProvider,
        r#use_https: Option<bool>,
        r#version: Option<crate::http::Version>,
        r#headers: Option<std::collections::HashMap<String, String>>,
        r#query: Option<String>,
        r#query_pairs: Option<PyObject>,
        r#appended_user_agent: Option<String>,
        r#uploading_progress: Option<PyObject>,
        r#receive_response_status: Option<PyObject>,
        r#receive_response_header: Option<PyObject>,
        r#to_resolve_domain: Option<PyObject>,
        r#domain_resolved: Option<PyObject>,
        r#to_choose_ips: Option<PyObject>,
        r#ips_chosen: Option<PyObject>,
        r#before_request_signed: Option<PyObject>,
        r#after_request_signed: Option<PyObject>,
        r#response_ok: Option<PyObject>,
        r#response_error: Option<PyObject>,
        r#before_backoff: Option<PyObject>,
        r#after_backoff: Option<PyObject>,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let http_client = self_.into_super().to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let (resp, parts) = http_client
                ._async_call(
                    "DELETE".to_owned(),
                    endpoints,
                    Some(vec![crate::http_client::ServiceName::r#Uc]),
                    use_https,
                    version,
                    Some("/bucketTagging".to_owned()),
                    headers,
                    None,
                    None,
                    query,
                    query_pairs,
                    appended_user_agent,
                    Some(crate::http_client::Authorization::from(
                        qiniu_sdk::http_client::Authorization::v2(credential),
                    )),
                    Some(crate::http_client::Idempotent::r#Default),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    uploading_progress,
                    receive_response_status,
                    receive_response_header,
                    to_resolve_domain,
                    domain_resolved,
                    to_choose_ips,
                    ips_chosen,
                    before_request_signed,
                    after_request_signed,
                    response_ok,
                    response_error,
                    before_backoff,
                    after_backoff,
                )
                .await?;
            Python::with_gil(|py| Py::new(py, (resp, parts)))
        })
    }
}
