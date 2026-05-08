#![allow(deprecated)]
include!(concat!(env!("OUT_DIR"), "/e2e_test.rs"));

use std::collections::BTreeMap;

pub struct MockE2ePathSever;

#[async_trait::async_trait]
impl E2ePathSever for MockE2ePathSever {
    async fn op_with_path(&self, param1: String) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec![param1])
    }

    async fn op_with_query(
        &self,
        param1: String,
        q: String,
    ) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec![param1, q])
    }

    async fn op_with_params(
        &self,
        path_name: String,
        q: Vec<String>,
        b: Vec<u8>,
        a: BTreeMap<String, serde_json::Value>,
    ) -> Result<Vec<String>, xidl_rust_axum::Error> {
        let mut res = vec![path_name];
        res.extend(q);
        res.push(format!("{:?}", b));
        res.push(format!("{:?}", a));
        Ok(res)
    }

    async fn op_with_query2(
        &self,
        all: String,
        word: String,
        q: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{all}:{word}:{q}"))
    }
}

pub struct MockE2eHttpRouteAndBody;

#[async_trait::async_trait]
impl E2eHttpRouteAndBody for MockE2eHttpRouteAndBody {
    async fn get_resource(
        &self,
        resource_id: String,
        locale: Option<String>,
        trace_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!(
            "id:{},lang:{:?},trace:{}",
            resource_id, locale, trace_id
        ))
    }

    async fn get_file(
        &self,
        file_path: String,
        download: bool,
        version: Option<String>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!(
            "file:{},download:{},version:{:?}",
            file_path, download, version
        ))
    }

    async fn create_resource(
        &self,
        resource_body: StructHttpBody,
    ) -> Result<StructHttpBody, xidl_rust_axum::Error> {
        Ok(resource_body)
    }

    async fn replace_resource(
        &self,
        _resource_id: String,
        _etag: String,
        _payload: StructHttpBody,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn patch_resource(
        &self,
        _resource_id: String,
        _dry_run: bool,
        _session_id: String,
        changes: BTreeMap<String, serde_json::Value>,
    ) -> Result<BTreeMap<String, serde_json::Value>, xidl_rust_axum::Error> {
        Ok(changes)
    }

    async fn delete_resource(
        &self,
        _resource_id: String,
        _force: Option<bool>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn probe_resource(
        &self,
        _resource_id: String,
        _if_none_match: String,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn resource_options(&self, _resource_id: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn get_msgpack_resource(
        &self,
        _resource_id: String,
    ) -> Result<E2EHttpRouteAndBodyGetMsgpackResourceResponse, xidl_rust_axum::Error> {
        Ok(E2EHttpRouteAndBodyGetMsgpackResourceResponse {
            r#return: StructHttpBody {
                name: "msgpack".to_string(),
                alias: None,
                tags: vec![],
                labels: BTreeMap::new(),
            },
            revision: 1,
        })
    }

    async fn dedup_resource(
        &self,
        id: String,
        x_trace_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{id}:{x_trace_id}"))
    }

    async fn preview_resource(
        &self,
        resource: StructHttpBody,
    ) -> Result<StructHttpBody, xidl_rust_axum::Error> {
        Ok(resource)
    }
}

pub struct MockE2eHttpSecurity;

#[async_trait::async_trait]
impl E2eHttpSecurity for MockE2eHttpSecurity {
    async fn get_secure_user(
        &self,
        user_id: String,
        locale: Option<String>,
        trace_id: String,
        _xidl_auth: xidl_rust_axum::auth::basic::BasicAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!(
            "user:{user_id},lang:{:?},trace:{}",
            locale, trace_id
        ))
    }

    async fn search_secure_user(
        &self,
        keyword: String,
        page: Option<u32>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("keyword:{keyword},page:{:?}", page))
    }

    async fn healthz(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("ok".to_string())
    }
}
