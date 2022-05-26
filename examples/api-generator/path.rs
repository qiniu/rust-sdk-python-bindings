use super::enums::StringLikeType;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径请求参数列表
pub(super) struct PathParams {
    /// HTTP URL 路径有名参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) named: Vec<NamedPathParam>,

    /// HTTP URL 路径自由参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) free: Option<FreePathParams>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径有名请求参数
pub(super) struct NamedPathParam {
    /// HTTP URL 路径段落，如果为 None，则表示参数直接追加在 URL 路径末尾
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) path_segment: Option<String>,

    /// HTTP URL 路径参数名称
    pub(super) field_name: String,

    /// HTTP URL 路径参数类型
    #[serde(rename = "type")]
    pub(super) ty: StringLikeType,

    /// HTTP URL 路径参数文档
    pub(super) documentation: String,

    /// HTTP URL 路径参数编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) encode: Option<EncodeType>,
}

impl NamedPathParam {
    pub(super) fn build_path_segments_token_stream(&self, ident: &TokenStream) -> TokenStream {
        let encode_param = encode_param(ident, self.encode);
        let encode_param = if matches!(self.ty, StringLikeType::String) {
            quote!(#encode_param)
        } else {
            quote!(#encode_param.to_string())
        };
        let path_segment_token_stream = self.path_segment.as_ref().map(|path_segment| {
            let path_segment = path_segment.as_str().trim_end_matches('/');
            quote!(#path_segment.to_owned())
        });
        let add_path_param_token_stream =
            if matches!(self.encode, Some(EncodeType::UrlSafeBase64OrNone)) {
                quote! {
                    segments.push(#encode_param);
                }
            } else {
                quote! {
                    if let Some(#ident) = #ident {
                        segments.push(#encode_param);
                    }
                }
            };
        quote! {
            {
                let mut segments = vec![#path_segment_token_stream];
                #add_path_param_token_stream
                segments
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径自由请求参数
pub(super) struct FreePathParams {
    /// HTTP URL 路径参数名称
    pub(super) field_name: String,

    /// HTTP URL 路径参数文档
    pub(super) documentation: String,

    /// HTTP URL 路径参数键编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) encode_param_key: Option<EncodeType>,

    /// HTTP URL 路径参数值编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) encode_param_value: Option<EncodeType>,
}

impl FreePathParams {
    pub(super) fn build_path_segments_token_stream(&self, ident: &TokenStream) -> TokenStream {
        let encode_param_key = encode_param(&quote!(key), self.encode_param_key);
        let encode_param_value = encode_param(&quote!(value), self.encode_param_value);
        quote! {
            {
                let mut segments = Vec::new();
                if let Some(free_params) = #ident {
                    for (key, value) in free_params {
                        segments.push(#encode_param_key);
                        segments.push(#encode_param_value);
                    }
                }
                segments
            }
        }
    }
}

#[derive(Clone, Debug, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 字符串编码类型
pub(super) enum EncodeType {
    /// 需要进行编码
    UrlSafeBase64,

    /// 需要可以将 None 编码
    UrlSafeBase64OrNone,
}

fn encode_param(ident: &TokenStream, encode_type: Option<EncodeType>) -> TokenStream {
    match encode_type {
        Some(EncodeType::UrlSafeBase64) => quote! {
            qiniu_sdk::utils::base64::urlsafe(#ident.as_bytes())
        },
        Some(EncodeType::UrlSafeBase64OrNone) => quote! {
            #ident.map(|s| qiniu_sdk::utils::base64::urlsafe(s.as_bytes())).unwrap_or_else(|| "~".to_owned())
        },
        None => quote! (#ident),
    }
}
