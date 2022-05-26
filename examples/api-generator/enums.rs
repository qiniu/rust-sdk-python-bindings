use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use smart_default::SmartDefault;
use std::fmt::{self, Display};

#[derive(SmartDefault, Clone, Debug, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 方法
pub(super) enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

impl Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        };
        s.fmt(f)
    }
}

#[derive(SmartDefault, Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 七牛服务名称
pub(super) enum ServiceName {
    #[default]
    Up,
    Io,
    Uc,
    Rs,
    Rsf,
    Api,
    S3,
}

impl ServiceName {
    pub(super) fn to_python_token_stream(self) -> TokenStream {
        let s = match self {
            Self::Up => "Up",
            Self::Io => "Io",
            Self::Uc => "Uc",
            Self::Rs => "Rs",
            Self::Rsf => "Rsf",
            Self::Api => "Api",
            Self::S3 => "S3",
        };
        let ident = format_ident!("r#{}", s);
        quote! {crate::http_client::ServiceName::#ident}
    }
}

#[derive(SmartDefault, Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Idempotent {
    Always,

    #[default]
    Default,

    Never,
}

impl Idempotent {
    pub(super) fn to_python_token_stream(self) -> TokenStream {
        let s = match self {
            Self::Always => "Always",
            Self::Default => "Default",
            Self::Never => "Never",
        };
        let ident = format_ident!("r#{}", s);
        quote! {crate::http_client::Idempotent::#ident}
    }
}

#[derive(Clone, Debug, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 鉴权方式
pub(super) enum Authorization {
    /// 使用 QBox 凭证鉴权
    Qbox,

    /// 使用 Qiniu 凭证鉴权
    Qiniu,

    /// 使用上传凭证鉴权
    UploadToken,
}

#[derive(SmartDefault, Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 类字符串参数类型
pub(super) enum StringLikeType {
    /// 字符串（默认）
    #[default]
    String,

    /// 整型数字
    Integer,

    /// 浮点型数字
    Float,

    /// 布尔值
    Boolean,
}

impl StringLikeType {
    pub(super) fn to_rust_type(self) -> TokenStream {
        match self {
            Self::String => quote! {String},
            Self::Integer => quote! {i64},
            Self::Float => quote! {f64},
            Self::Boolean => quote! {bool},
        }
    }
}
