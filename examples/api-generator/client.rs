use super::{
    enums::{Authorization, Idempotent, Method, ServiceName},
    path::PathParams,
};
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::{de::IgnoredAny, Deserialize};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// API 描述信息，可以通过 YAML 描述文件编辑
pub(super) struct ApiDetailedDescription {
    /// API 调用 HTTP 方法
    method: Method,

    /// 七牛服务名称，可以设置多个，表现有多个七牛服务都可以调用该 API
    #[serde(skip_serializing_if = "Vec::is_empty")]
    service_names: Vec<ServiceName>,

    /// API 文档
    documentation: String,

    /// 七牛 API URL 基础路径
    base_path: String,

    /// 七牛 API URL 路径后缀
    path_suffix: String,

    /// 七牛 API 调用参数
    request: ApiRequestDescription,

    /// 七牛 API 响应参数
    response: ApiResponseDescription,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ApiRequestDescription {
    /// 七牛 API 调用 URL 路径参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    path_params: Option<PathParams>,

    /// 七牛 API 调用 HTTP 头参数列表
    header_names: IgnoredAny,

    /// 七牛 API 调用 URL 查询参数列表
    query_names: IgnoredAny,

    /// 七牛 API 调用请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<RequestBody>,

    /// 七牛 API 调用鉴权参数
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization: Option<Authorization>,

    /// 七牛 API 调用幂等性
    idempotent: Idempotent,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 调用请求体
enum RequestBody {
    /// JSON 调用
    Json(IgnoredAny),

    /// URL 编码表单调用（无法上传二进制数据）
    FormUrlencoded(IgnoredAny),

    /// 复合表单调用（可以上传二进制数据）
    MultipartFormData(IgnoredAny),

    /// 二进制数据调用
    BinaryData,

    /// 文本数据调用
    PlainText,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 响应请求体
enum ResponseBody {
    /// JSON 响应
    Json(IgnoredAny),

    /// 二进制数据响应
    BinaryDataStream,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(super) struct ApiResponseDescription {
    /// 七牛 API 响应 HTTP 头参数列表
    header_names: IgnoredAny,

    /// 七牛 API 响应请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<ResponseBody>,
}

impl ApiDetailedDescription {
    pub(super) fn to_python_token_stream(&self, mod_name: &str) -> TokenStream {
        let api_docs = &self.documentation;
        let method = self.method.to_string();
        let service_names = self
            .service_names
            .iter()
            .map(|s| s.to_python_token_stream())
            .collect::<Vec<_>>();
        let (required_args, optional_args) = self.arguments();
        let required_arg_names = required_args.keys().map(|k| k.as_str()).collect::<Vec<_>>();
        let optional_arg_names = optional_args.keys().map(|k| k.as_str()).collect::<Vec<_>>();
        let optional_arg_idents = optional_arg_names
            .iter()
            .map(|n| format_ident!("r#{}", n))
            .collect::<Vec<_>>();
        let text_signature = "(".to_owned()
            + &required_arg_names.join(", ")
            + ", /, "
            + &optional_arg_names
                .iter()
                .map(|name| format!("{} = None", name))
                .collect::<Vec<_>>()
                .join(", ")
            + ")";
        let args_list = required_args
            .iter()
            .map(|(name, ty)| {
                let name = format_ident!("r#{}", name);
                quote! {#name: #ty}
            })
            .chain(optional_args.iter().map(|(name, ty)| {
                let name = format_ident!("r#{}", name);
                quote! {#name: Option<#ty>}
            }))
            .collect::<Vec<_>>();
        let build_path = self.build_path();
        let accept_json = make_optional_bool_token_stream(matches!(
            self.response.body,
            Some(ResponseBody::Json(_))
        ));
        let accept_application_octet_stream = make_optional_bool_token_stream(matches!(
            self.response.body,
            Some(ResponseBody::BinaryDataStream)
        ));
        let authorization = self.authorization();
        let idempotent = self.request.idempotent.to_python_token_stream();
        let bytes = args_or_not(&optional_args, "bytes");
        let body = args_or_not(&optional_args, "body");
        let body_len = args_or_not(&optional_args, "body_len");
        let content_type = args_or_not(&optional_args, "content_type");
        let json = args_or_not(&optional_args, "json");
        let form = args_or_not(&optional_args, "form");
        let multipart = args_or_not(&optional_args, "multipart");
        let arg_values_list = vec![
            quote! {#method.to_owned()},
            quote!(endpoints),
            quote! {Some(vec![#(#service_names),*])},
            quote!(use_https),
            quote!(version),
            build_path,
            quote!(headers),
            accept_json,
            accept_application_octet_stream,
            quote!(query),
            quote!(query_pairs),
            quote!(appended_user_agent),
            authorization,
            quote! {Some(#idempotent)},
            bytes,
            body,
            body_len,
            content_type,
            json,
            form,
            multipart,
            quote!(uploading_progress),
            quote!(receive_response_status),
            quote!(receive_response_header),
            quote!(to_resolve_domain),
            quote!(domain_resolved),
            quote!(to_choose_ips),
            quote!(ips_chosen),
            quote!(before_request_signed),
            quote!(after_request_signed),
            quote!(response_ok),
            quote!(response_error),
            quote!(before_backoff),
            quote!(after_backoff),
        ];
        let (call_sync_response_type, call_sync_response_code, call_async_response_code) = if matches!(
            self.response.body,
            Some(ResponseBody::Json(_))
        ) {
            (
                quote!(crate::http_client::JsonResponse),
                quote! {{
                    let mut body = resp;
                    let json = crate::http_client::JsonResponse::from(body.parse_json()?);
                    Py::new(py, (json, parts))
                }},
                quote! {{
                    let mut body = resp;
                    let json = crate::http_client::JsonResponse::from(body._parse_json().await?);
                    Python::with_gil(|py| Py::new(py, (json, parts)))
                }},
            )
        } else {
            (
                quote!(crate::http::SyncHttpResponse),
                quote!(Py::new(py, (resp, parts))),
                quote!(Python::with_gil(|py| Py::new(py, (resp, parts)))),
            )
        };
        quote! {
            use pyo3::prelude::*;
            use crate::http_client::HttpClient;

            pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
                let m = PyModule::new(py, #mod_name)?;
                m.add_class::<Client>()?;
                Ok(m)
            }

            #[doc = #api_docs]
            #[pyclass(extends = HttpClient)]
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
                #[pyo3(text_signature = #text_signature)]
                #[args(#(#optional_arg_idents = "None"),*)]
                #[allow(clippy::too_many_arguments)]
                fn call(
                    self_: PyRef<'_, Self>,
                    #(#args_list),*,
                    py: Python<'_>,
                ) -> PyResult<Py<#call_sync_response_type>> {
                    let super_ = self_.into_super();
                    let (resp, parts) = super_._call(#(#arg_values_list),*, py)?;
                    #call_sync_response_code
                }

                #[doc = "发出异步请求"]
                #[pyo3(text_signature = #text_signature)]
                #[args(#(#optional_arg_idents = "None"),*)]
                #[allow(clippy::too_many_arguments)]
                fn async_call<'p>(
                    self_: PyRef<'p, Self>,
                    #(#args_list),*,
                    py: Python<'p>,
                ) -> PyResult<&'p PyAny> {
                    let http_client = self_.into_super().to_owned();
                    pyo3_asyncio::async_std::future_into_py(py, async move {
                        let (resp, parts) = http_client
                            ._async_call(#(#arg_values_list),*)
                            .await?;
                        #call_async_response_code
                    })
                }
            }
        }
    }

    fn arguments(&self) -> (IndexMap<String, TokenStream>, IndexMap<String, TokenStream>) {
        let mut required_args = IndexMap::new();
        let mut optional_args = IndexMap::new();

        required_args.insert("endpoints".to_owned(), quote!(PyObject));
        if let Some(authorization) = &self.request.authorization {
            match authorization {
                Authorization::Qbox | Authorization::Qiniu => {
                    required_args.insert(
                        "credential".to_owned(),
                        quote!(crate::credential::CredentialProvider),
                    );
                }
                Authorization::UploadToken => {
                    required_args.insert(
                        "upload_token".to_owned(),
                        quote!(crate::upload_token::UploadTokenProvider),
                    );
                }
            }
        }

        optional_args.insert("use_https".to_owned(), quote!(bool));
        optional_args.insert("version".to_owned(), quote!(crate::http::Version));
        optional_args.insert(
            "headers".to_owned(),
            quote! {std::collections::HashMap<String,String>},
        );
        optional_args.insert("query".to_owned(), quote!(String));
        optional_args.insert("query_pairs".to_owned(), quote!(PyObject));
        optional_args.insert("appended_user_agent".to_owned(), quote!(String));

        if let Some(path_params) = &self.request.path_params {
            for named_param in &path_params.named {
                optional_args.insert(
                    named_param.field_name.to_case(Case::Snake),
                    named_param.ty.to_rust_type(),
                );
            }
            if let Some(free_params) = &path_params.free {
                optional_args.insert(
                    free_params.field_name.to_case(Case::Snake),
                    quote! {std::collections::HashMap<String,String>},
                );
            }
        }
        if let Some(body) = &self.request.body {
            match body {
                RequestBody::Json(_) => {
                    optional_args.insert("json".to_owned(), quote!(PyObject));
                }
                RequestBody::FormUrlencoded(_) => {
                    optional_args.insert("form".to_owned(), quote! {Vec<(String, Option<String>)>});
                }
                RequestBody::MultipartFormData(_) => {
                    optional_args.insert(
                        "multipart".to_owned(),
                        quote! {std::collections::HashMap<String, PyObject>},
                    );
                }
                RequestBody::BinaryData => {
                    optional_args.insert("bytes".to_owned(), quote!(Vec<u8>));
                    optional_args.insert("body".to_owned(), quote!(PyObject));
                    optional_args.insert("body_len".to_owned(), quote!(u64));
                    optional_args.insert("content_type".to_owned(), quote!(String));
                }
                RequestBody::PlainText => {
                    optional_args.insert("bytes".to_owned(), quote!(Vec<u8>));
                    optional_args.insert("content_type".to_owned(), quote!(String));
                }
            }
        }

        for arg_name in [
            "uploading_progress",
            "receive_response_status",
            "receive_response_header",
            "to_resolve_domain",
            "domain_resolved",
            "to_choose_ips",
            "ips_chosen",
            "before_request_signed",
            "after_request_signed",
            "response_ok",
            "response_error",
            "before_backoff",
            "after_backoff",
        ] {
            optional_args.insert(arg_name.to_owned(), quote!(PyObject));
        }

        (required_args, optional_args)
    }

    fn build_path(&self) -> TokenStream {
        let base_path_token_stream = {
            let mut base_path = self.base_path.as_str();
            if base_path.is_empty() {
                None
            } else {
                base_path = base_path.trim_end_matches('/');
                Some(quote!(#base_path.to_owned()))
            }
        };
        let path_suffix_token_stream = {
            let mut path_suffix = self.path_suffix.as_str();
            if path_suffix.is_empty() {
                None
            } else {
                path_suffix = path_suffix.trim_start_matches('/');
                Some(quote!(segments.push(#path_suffix.to_owned());))
            }
        };
        let mut code_segments = Vec::new();
        if let Some(path_params) = &self.request.path_params {
            for named_param in &path_params.named {
                let field_name = format_ident!("r#{}", named_param.field_name.to_case(Case::Snake));
                let token_stream =
                    named_param.build_path_segments_token_stream(&quote!(#field_name));
                code_segments.push(quote! { segments.extend(#token_stream); });
            }
            if let Some(free_params) = &path_params.free {
                let field_name = format_ident!("r#{}", free_params.field_name.to_case(Case::Snake));
                let token_stream =
                    free_params.build_path_segments_token_stream(&quote!(#field_name));
                code_segments.push(quote! { segments.extend(#token_stream); });
            }
        }
        if path_suffix_token_stream.is_none() && code_segments.is_empty() {
            base_path_token_stream
                .map(|base_path_token_stream| quote!(Some(#base_path_token_stream)))
                .unwrap_or_else(|| quote!(None))
        } else {
            quote! {
                {
                    let mut segments = vec![#base_path_token_stream];
                    #(#code_segments);*
                    #path_suffix_token_stream
                    Some(segments.join("/"))
                }
            }
        }
    }

    fn authorization(&self) -> TokenStream {
        match self.request.authorization {
            Some(Authorization::Qbox) => {
                quote!(Some(crate::http_client::Authorization::from(
                    qiniu_sdk::http_client::Authorization::v1(credential)
                )))
            }
            Some(Authorization::Qiniu) => quote!(Some(crate::http_client::Authorization::from(
                qiniu_sdk::http_client::Authorization::v2(credential)
            ))),
            Some(Authorization::UploadToken) => {
                quote!(Some(crate::http_client::Authorization::from(
                    qiniu_sdk::http_client::Authorization::uptoken(upload_token)
                )))
            }
            None => quote!(None),
        }
    }
}

fn make_optional_bool_token_stream(b: bool) -> TokenStream {
    if b {
        quote!(Some(true))
    } else {
        quote!(None)
    }
}

fn args_or_not(args: &IndexMap<String, TokenStream>, arg: &str) -> TokenStream {
    if args.contains_key(arg) {
        let arg = format_ident!("r#{}", arg);
        quote!(#arg)
    } else {
        quote!(None)
    }
}
