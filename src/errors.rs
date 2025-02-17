// Copyright 2022 CeresDB Project Authors. Licensed under Apache-2.0.

use std::fmt::Display;

use thiserror::Error as ThisError;

use crate::model::write::WriteResponse;

#[derive(Debug, ThisError)]
pub enum Error {
    /// Error from the running server
    #[error("failed in server, err:{0}")]
    Server(ServerError),

    /// Error from the rpc
    /// Note that any error caused by a running server wont be wrapped in the
    /// grpc errors.
    #[error("failed in grpc, err:{0}")]
    Rpc(tonic::Status),

    /// Error about rpc.
    /// It will be throw while connection between client and server is broken
    /// and try for reconnecting is failed(timeout).
    #[error("failed to connect, addr:{addr:?}, err:{source:?}")]
    Connect {
        addr: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error from the client and basically the rpc request has not been called
    /// yet or the rpc request has already been finished successfully.
    #[error("failed in client, msg:{0}")]
    Client(String),

    /// Error about authentication
    #[error("failed to check auth, err:{0}")]
    AuthFail(AuthFailStatus),

    /// Error from write in cluster mode, some of rows may be written
    /// successfully, and others may fail.
    #[error("failed in cluster write, err:{0}")]
    ClusterWriteError(ClusterWriteError),

    /// Error unknown
    #[error("unknown error, msg:{0}")]
    Unknown(String),
}

#[derive(Debug)]
pub struct ClusterWriteError {
    pub ok: (Vec<String>, WriteResponse), // (metrics, write_response)
    pub errors: Vec<(Vec<String>, Error)>, // [(metrics, erros)]
}

impl From<Vec<(Vec<String>, Result<WriteResponse>)>> for ClusterWriteError {
    fn from(wirte_results: Vec<(Vec<String>, Result<WriteResponse>)>) -> Self {
        let mut success_total = 0;
        let mut failed_total = 0;
        let mut ok_metrics = Vec::new();
        let mut errors = Vec::new();
        for (metrics, write_result) in wirte_results {
            match write_result {
                Ok(write_resp) => {
                    success_total += write_resp.success;
                    failed_total += write_resp.failed;
                    ok_metrics.extend(metrics);
                }
                Err(e) => {
                    errors.push((metrics, e));
                }
            }
        }

        Self {
            ok: (ok_metrics, WriteResponse::new(success_total, failed_total)),
            errors,
        }
    }
}

impl ClusterWriteError {
    pub fn all_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Display for ClusterWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterWriteError")
            .field("ok", &self.ok)
            .field("errors", &self.errors)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ServerError {
    pub code: u32,
    pub msg: String,
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerError")
            .field("code", &self.code)
            .field("msg", &self.msg)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct AuthFailStatus {
    pub code: AuthCode,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub enum AuthCode {
    Ok = 0,

    InvalidTenantMeta = 1,

    InvalidTokenMeta = 2,
}

impl Display for AuthFailStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthFailStatus")
            .field("code", &self.code)
            .field("msg", &self.msg)
            .finish()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_error_standardizing() {
        let source_error = Box::new(Error::Unknown("unknown error".to_string()));
        let connect_error = Error::Connect {
            addr: "1.1.1.1:1111".to_string(),
            source: source_error as _,
        };
        assert_eq!(
            &format!("{}", connect_error),
            r#"failed to connect, addr:"1.1.1.1:1111", err:Unknown("unknown error")"#
        );
    }
}
