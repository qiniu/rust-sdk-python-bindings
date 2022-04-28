use super::{
    exceptions::{QiniuBodySizeMissingError, QiniuInvalidHttpVersion},
    utils::{parse_headers, parse_ip_addrs, parse_method, parse_uri, PythonIoBase},
};
use pyo3::prelude::*;
use std::{collections::HashMap, convert::TryInto};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "http")?;
    m.add_class::<SyncRequestBuilder>()?;
    Ok(m)
}

#[pyclass]
struct SyncRequestBuilder(qiniu_sdk::http::SyncRequestBuilder<'static>);

#[pymethods]
impl SyncRequestBuilder {
    #[new]
    fn new() -> Self {
        Self(qiniu_sdk::http::SyncRequestBuilder::new())
    }

    #[pyo3(text_signature = "($self, url)")]
    fn url(&mut self, url: &str) -> PyResult<()> {
        let url = parse_uri(url)?;
        self.0.url(url);
        Ok(())
    }

    #[pyo3(text_signature = "($self, method)")]
    fn method(&mut self, method: &str) -> PyResult<()> {
        let method = parse_method(method)?;
        self.0.method(method);
        Ok(())
    }

    #[pyo3(text_signature = "($self, version)")]
    fn version(&mut self, version: Version) -> PyResult<()> {
        self.0.version(version.try_into()?);
        Ok(())
    }

    #[pyo3(text_signature = "($self, headers)")]
    fn headers(&mut self, headers: HashMap<String, String>) -> PyResult<()> {
        let headers = parse_headers(headers)?;
        self.0.headers(headers);
        Ok(())
    }

    #[args(len = "None")]
    #[pyo3(text_signature = "($self, body, len)")]
    fn body(&mut self, body: PyObject, len: Option<u64>, py: Python<'_>) -> PyResult<()> {
        if let Ok(body) = body.extract::<String>(py) {
            self.0.body(qiniu_sdk::http::SyncRequestBody::from_bytes(
                body.into_bytes(),
            ));
        } else if let Ok(body) = body.extract::<Vec<u8>>(py) {
            self.0
                .body(qiniu_sdk::http::SyncRequestBody::from_bytes(body));
        } else if let Some(len) = len {
            self.0.body(qiniu_sdk::http::SyncRequestBody::from_reader(
                PythonIoBase::new(body),
                len,
            ));
        } else {
            return Err(QiniuBodySizeMissingError::new_err("`body` must be passed"));
        }
        Ok(())
    }

    #[pyo3(text_signature = "($self, user_agent)")]
    fn appended_user_agent(&mut self, user_agent: &str) {
        self.0.appended_user_agent(user_agent);
    }

    #[pyo3(text_signature = "($self, resolved_ip_addrs)")]
    fn resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<String>) -> PyResult<()> {
        let resolved_ip_addrs = parse_ip_addrs(resolved_ip_addrs)?;
        self.0.resolved_ip_addrs(resolved_ip_addrs);
        Ok(())
    }

    #[pyo3(text_signature = "($self)")]
    fn reset(&mut self) {
        self.0.reset();
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
enum Version {
    HTTP_09 = 9,
    HTTP_10 = 10,
    HTTP_11 = 11,
    HTTP_2 = 20,
    HTTP_3 = 30,
}

impl TryFrom<qiniu_sdk::http::Version> for Version {
    type Error = PyErr;

    fn try_from(version: qiniu_sdk::http::Version) -> Result<Self, Self::Error> {
        match version {
            qiniu_sdk::http::Version::HTTP_09 => Ok(Version::HTTP_09),
            qiniu_sdk::http::Version::HTTP_10 => Ok(Version::HTTP_10),
            qiniu_sdk::http::Version::HTTP_11 => Ok(Version::HTTP_11),
            qiniu_sdk::http::Version::HTTP_2 => Ok(Version::HTTP_2),
            qiniu_sdk::http::Version::HTTP_3 => Ok(Version::HTTP_3),
            version => Err(QiniuInvalidHttpVersion::new_err(format!(
                "Unknown HTTP version: {:?}",
                version
            ))),
        }
    }
}

impl From<Version> for qiniu_sdk::http::Version {
    fn from(version: Version) -> Self {
        match version {
            Version::HTTP_09 => qiniu_sdk::http::Version::HTTP_09,
            Version::HTTP_10 => qiniu_sdk::http::Version::HTTP_10,
            Version::HTTP_11 => qiniu_sdk::http::Version::HTTP_11,
            Version::HTTP_2 => qiniu_sdk::http::Version::HTTP_2,
            Version::HTTP_3 => qiniu_sdk::http::Version::HTTP_3,
        }
    }
}
