use async_trait::async_trait;
use std::collections::BTreeMap;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/e2e_test.rs"));
}

pub struct LogicE2ePathSever;

#[async_trait]
impl gen::E2ePathSever for LogicE2ePathSever {
    async fn op_with_path<'a>(&'a self, param1: String) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec![param1])
    }

    async fn op_with_query<'a>(
        &'a self,
        param1: String,
        q: String,
    ) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec![param1, q])
    }

    async fn op_with_params<'a>(
        &'a self,
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

    async fn op_with_query2<'a>(
        &'a self,
        all: String,
        word: String,
        q: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{all}:{word}:{q}"))
    }
}

pub struct LogicE2eHttpRouteAndBody;

#[async_trait]
impl gen::E2eHttpRouteAndBody for LogicE2eHttpRouteAndBody {
    async fn get_resource<'a>(
        &'a self,
        resource_id: String,
        locale: Option<String>,
        trace_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!(
            "id:{},lang:{:?},trace:{}",
            resource_id, locale, trace_id
        ))
    }

    async fn get_file<'a>(
        &'a self,
        file_path: String,
        download: bool,
        version: Option<String>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!(
            "file:{},download:{},version:{:?}",
            file_path, download, version
        ))
    }

    async fn create_resource<'a>(
        &'a self,
        resource_body: gen::StructHttpBody,
    ) -> Result<gen::StructHttpBody, xidl_rust_axum::Error> {
        Ok(resource_body)
    }

    async fn replace_resource<'a>(
        &'a self,
        _resource_id: String,
        _etag: String,
        _payload: gen::StructHttpBody,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn patch_resource<'a>(
        &'a self,
        _resource_id: String,
        _dry_run: bool,
        _session_id: String,
        changes: BTreeMap<String, serde_json::Value>,
    ) -> Result<BTreeMap<String, serde_json::Value>, xidl_rust_axum::Error> {
        Ok(changes)
    }

    async fn delete_resource<'a>(
        &'a self,
        _resource_id: String,
        _force: Option<bool>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn probe_resource<'a>(
        &'a self,
        _resource_id: String,
        _if_none_match: String,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn resource_options<'a>(&'a self, _resource_id: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn get_msgpack_resource<'a>(
        &'a self,
        _resource_id: String,
    ) -> Result<gen::E2EHttpRouteAndBodyGetMsgpackResourceResponse, xidl_rust_axum::Error> {
        Ok(gen::E2EHttpRouteAndBodyGetMsgpackResourceResponse {
            r#return: gen::StructHttpBody {
                name: "msgpack".to_string(),
                alias: None,
                tags: vec![],
                labels: BTreeMap::new(),
            },
            revision: 1,
        })
    }

    async fn dedup_resource<'a>(
        &'a self,
        id: String,
        x_trace_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{id}:{x_trace_id}"))
    }

    async fn preview_resource<'a>(
        &'a self,
        resource: gen::StructHttpBody,
    ) -> Result<gen::StructHttpBody, xidl_rust_axum::Error> {
        Ok(resource)
    }
}

pub struct LogicE2eHttpSecurity;

#[async_trait]
impl gen::E2eHttpSecurity for LogicE2eHttpSecurity {
    async fn get_secure_user<'a>(
        &'a self,
        user_id: String,
        locale: Option<String>,
        trace_id: String,
        _xidl_auth: xidl_rust_axum::ClientAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!(
            "user:{user_id},lang:{:?},trace:{}",
            locale, trace_id
        ))
    }

    async fn search_secure_user<'a>(
        &'a self,
        keyword: String,
        page: Option<u32>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("keyword:{keyword},page:{:?}", page))
    }

    async fn healthz(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("ok".to_string())
    }
}

pub struct LogicE2eTypeServer;

#[async_trait]
impl gen::E2eTypeServer for LogicE2eTypeServer {
    async fn get_attribute_type_attr1(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("attr1".to_string())
    }
    async fn set_attribute_type_attr1(&self, _value: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_type_attr2(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["attr2".to_string()])
    }
    async fn simple_op(&self) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn simple_op_with_return1(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("simple_op_with_return1".to_string())
    }
    async fn simple_op_with_return2(&self) -> Result<gen::EnumEmpty, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn simple_op_with_return3(&self) -> Result<gen::EnumSimple1, xidl_rust_axum::Error> {
        Ok(gen::EnumSimple1::V1)
    }
    async fn simple_op_with_return4(&self) -> Result<gen::StructEmpty, xidl_rust_axum::Error> {
        Ok(gen::StructEmpty {})
    }
    async fn simple_op_with_return5(&self) -> Result<gen::StructSimple, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn return_with_sequence1(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["s1".to_string(), "s2".to_string()])
    }
    async fn return_with_sequence2(&self) -> Result<Vec<gen::EnumEmpty>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn return_with_sequence3(&self) -> Result<Vec<gen::EnumSimple1>, xidl_rust_axum::Error> {
        Ok(vec![gen::EnumSimple1::V1, gen::EnumSimple1::V2])
    }
    async fn return_with_sequence4(&self) -> Result<Vec<gen::StructEmpty>, xidl_rust_axum::Error> {
        Ok(vec![gen::StructEmpty {}])
    }
    async fn return_with_sequence5(&self) -> Result<Vec<gen::StructSimple>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn return_with_map(&self) -> Result<BTreeMap<String, u8>, xidl_rust_axum::Error> {
        let mut m = BTreeMap::new();
        m.insert("k1".to_string(), 1);
        Ok(m)
    }
    async fn return_with_any(&self) -> Result<serde_json::Value, xidl_rust_axum::Error> {
        Ok(serde_json::json!({"any": "value"}))
    }
    async fn return_with_any_sequence(
        &self,
    ) -> Result<Vec<serde_json::Value>, xidl_rust_axum::Error> {
        Ok(vec![serde_json::json!(1), serde_json::json!("two")])
    }
    async fn return_with_any_map(
        &self,
    ) -> Result<BTreeMap<String, serde_json::Value>, xidl_rust_axum::Error> {
        let mut m = BTreeMap::new();
        m.insert("k1".to_string(), serde_json::json!(1));
        Ok(m)
    }
    async fn parameter_op<'a>(&'a self, _a: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn parameter_op2<'a>(&'a self, _a: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn parameter_op3<'a>(
        &'a self,
        _a: String,
        _c: Vec<serde_json::Value>,
    ) -> Result<gen::E2ETypeServerParameterOp3Response, xidl_rust_axum::Error> {
        Ok(gen::E2ETypeServerParameterOp3Response { b: 3, c: vec![] })
    }
    async fn parameter_op4<'a>(
        &'a self,
        _c: Vec<serde_json::Value>,
    ) -> Result<gen::E2ETypeServerParameterOp4Response, xidl_rust_axum::Error> {
        Ok(gen::E2ETypeServerParameterOp4Response {
            a: "op4".to_string(),
            b: 4,
            c: vec![],
        })
    }
    async fn parameter_op5<'a>(
        &'a self,
        _c: Vec<serde_json::Value>,
    ) -> Result<gen::E2ETypeServerParameterOp5Response, xidl_rust_axum::Error> {
        Ok(gen::E2ETypeServerParameterOp5Response {
            r#return: vec![serde_json::json!("op5")],
            a: "op5".to_string(),
            b: 5,
            c: vec![],
        })
    }
    async fn parameter_op6<'a>(
        &'a self,
        _c: Vec<serde_json::Value>,
    ) -> Result<gen::E2ETypeServerParameterOp6Response, xidl_rust_axum::Error> {
        Ok(gen::E2ETypeServerParameterOp6Response {
            r#return: BTreeMap::new(),
            a: "op6".to_string(),
            b: 6,
            c: vec![],
        })
    }
}

pub struct LogicE2eAttribute;

#[async_trait]
impl gen::E2eAttribute for LogicE2eAttribute {
    async fn get_attribute_attr1(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("attr1".to_string())
    }
    async fn set_attribute_attr1(&self, _value: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr2(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["attr2".to_string()])
    }
    async fn get_attribute_attr3(&self) -> Result<gen::EnumEmpty, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn set_attribute_attr3(&self, _value: gen::EnumEmpty) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr4(&self) -> Result<gen::EnumSimple1, xidl_rust_axum::Error> {
        Ok(gen::EnumSimple1::V1)
    }
    async fn set_attribute_attr4(&self, _value: gen::EnumSimple1) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr5(&self) -> Result<gen::StructEmpty, xidl_rust_axum::Error> {
        Ok(gen::StructEmpty {})
    }
    async fn set_attribute_attr5(&self, _value: gen::StructEmpty) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr6(&self) -> Result<gen::StructSimple, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn set_attribute_attr6(&self, _value: gen::StructSimple) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr61(&self) -> Result<gen::UnionSimple, xidl_rust_axum::Error> {
        Ok(gen::UnionSimple::new_case1(1))
    }
    async fn set_attribute_attr61(&self, _value: gen::UnionSimple) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr7(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["attr7".to_string()])
    }
    async fn set_attribute_attr7(&self, _value: Vec<String>) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr8(&self) -> Result<Vec<gen::EnumEmpty>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn set_attribute_attr8(
        &self,
        _value: Vec<gen::EnumEmpty>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr9(&self) -> Result<Vec<gen::EnumSimple1>, xidl_rust_axum::Error> {
        Ok(vec![gen::EnumSimple1::V1])
    }
    async fn set_attribute_attr9(
        &self,
        _value: Vec<gen::EnumSimple1>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr10(&self) -> Result<Vec<gen::StructEmpty>, xidl_rust_axum::Error> {
        Ok(vec![gen::StructEmpty {}])
    }
    async fn set_attribute_attr10(
        &self,
        _value: Vec<gen::StructEmpty>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr11(&self) -> Result<Vec<gen::StructSimple>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn set_attribute_attr11(
        &self,
        _value: Vec<gen::StructSimple>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr12(&self) -> Result<BTreeMap<String, u8>, xidl_rust_axum::Error> {
        Ok(BTreeMap::new())
    }
    async fn set_attribute_attr12(
        &self,
        _value: BTreeMap<String, u8>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr13(&self) -> Result<serde_json::Value, xidl_rust_axum::Error> {
        Ok(serde_json::json!({}))
    }
    async fn set_attribute_attr13(
        &self,
        _value: serde_json::Value,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr14(&self) -> Result<Vec<serde_json::Value>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn set_attribute_attr14(
        &self,
        _value: Vec<serde_json::Value>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr15(
        &self,
    ) -> Result<BTreeMap<String, serde_json::Value>, xidl_rust_axum::Error> {
        Ok(BTreeMap::new())
    }
    async fn set_attribute_attr15(
        &self,
        _value: BTreeMap<String, serde_json::Value>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr16(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("attr16".to_string())
    }
}

pub struct LogicE2eHttpForm;

#[async_trait]
impl gen::E2eHttpForm for LogicE2eHttpForm {
    async fn submit_profile<'a>(
        &'a self,
        name: String,
        age: Option<u32>,
    ) -> Result<gen::E2EHttpFormSubmitProfileResponse, xidl_rust_axum::Error> {
        Ok(gen::E2EHttpFormSubmitProfileResponse {
            r#return: format!("name:{name},age:{age:?}"),
            normalized_name: name.to_uppercase(),
        })
    }
}

pub struct LogicE2eHttpScopeMatrix;

#[async_trait]
impl gen::E2eHttpScopeMatrix for LogicE2eHttpScopeMatrix {
    async fn get_attribute_scope_inherited_attr(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("inherited".to_string())
    }
    async fn get_attribute_scope_bare_attr(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("bare".to_string())
    }
    async fn default_scope<'a>(
        &'a self,
        request_body: gen::StructHttpBody,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(request_body.name)
    }
    async fn override_consumes_only<'a>(
        &'a self,
        name: String,
        age: Option<u32>,
    ) -> Result<gen::E2EHttpScopeMatrixOverrideConsumesOnlyResponse, xidl_rust_axum::Error> {
        Ok(gen::E2EHttpScopeMatrixOverrideConsumesOnlyResponse {
            r#return: format!("name:{name},age:{age:?}"),
            normalized_name: name.to_uppercase(),
        })
    }
    async fn override_produces_only<'a>(
        &'a self,
        resource_id: String,
    ) -> Result<gen::E2EHttpScopeMatrixOverrideProducesOnlyResponse, xidl_rust_axum::Error> {
        Ok(gen::E2EHttpScopeMatrixOverrideProducesOnlyResponse {
            r#return: gen::StructHttpBody {
                name: resource_id,
                alias: None,
                tags: vec![],
                labels: BTreeMap::new(),
            },
            revision: 1,
        })
    }
    async fn override_both_media<'a>(
        &'a self,
        name: String,
        age: Option<u32>,
    ) -> Result<gen::E2EHttpScopeMatrixOverrideBothMediaResponse, xidl_rust_axum::Error> {
        Ok(gen::E2EHttpScopeMatrixOverrideBothMediaResponse {
            r#return: gen::StructHttpBody {
                name,
                alias: None,
                tags: vec![format!("age:{age:?}")],
                labels: BTreeMap::new(),
            },
            normalized_name: "OVERRIDDEN".to_string(),
        })
    }
    async fn deprecated_plain<'a>(&'a self, resource_id: String) -> Result<String, xidl_rust_axum::Error> {
        Ok(resource_id)
    }
    async fn deprecated_since_only<'a>(
        &'a self,
        resource_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(resource_id)
    }
    async fn deprecated_window<'a>(
        &'a self,
        resource_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(resource_id)
    }
}

pub struct LogicE2eHttpDefaultsMatrix;

#[async_trait]
impl gen::E2eHttpDefaultsMatrix for LogicE2eHttpDefaultsMatrix {
    async fn delete_resource_default_query<'a>(
        &'a self,
        id: String,
        revision: u32,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{id}:{revision}"))
    }
    async fn probe_resource_default_query<'a>(
        &'a self,
        _id: String,
        _revision: u32,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn resource_options_default_query<'a>(
        &'a self,
        _id: String,
        _revision: u32,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn replace_resource_default_body<'a>(
        &'a self,
        id: String,
        name: String,
        alias: Option<String>,
    ) -> Result<gen::StructHttpBody, xidl_rust_axum::Error> {
        Ok(gen::StructHttpBody {
            name,
            alias,
            tags: vec![id],
            labels: BTreeMap::new(),
        })
    }
    async fn patch_resource_default_body<'a>(
        &'a self,
        id: String,
        name: String,
        alias: Option<String>,
    ) -> Result<gen::StructHttpBody, xidl_rust_axum::Error> {
        Ok(gen::StructHttpBody {
            name,
            alias,
            tags: vec![id],
            labels: BTreeMap::new(),
        })
    }
}

pub struct LogicE2eHttpSecurityMatrix;

#[async_trait]
impl gen::E2eHttpSecurityMatrix for LogicE2eHttpSecurityMatrix {
    async fn inherited_security<'a>(
        &'a self,
        resource_id: String,
        trace_id: String,
        _xidl_auth: xidl_rust_axum::ClientAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{resource_id}:{trace_id}"))
    }
    async fn bearer_or_cookie_security<'a>(
        &'a self,
        action: String,
        note: Option<String>,
        _xidl_auth: xidl_rust_axum::ClientAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{action}:{note:?}"))
    }
    async fn alternative_security<'a>(
        &'a self,
        resource_id: String,
        locale: Option<String>,
        _xidl_auth: xidl_rust_axum::ClientAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{resource_id}:{locale:?}"))
    }
    async fn oauth_security<'a>(
        &'a self,
        keyword: String,
        page: Option<u32>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{keyword}:{page:?}"))
    }
    async fn public_ping(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("pong".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    println!("Rust server starting on {}", addr);
    xidl_rust_axum::Server::builder()
        .with_service(gen::E2ePathSeverServer::new(LogicE2ePathSever))
        .with_service(gen::E2eHttpRouteAndBodyServer::new(LogicE2eHttpRouteAndBody))
        .with_service(gen::E2eHttpSecurityServer::new(LogicE2eHttpSecurity))
        .with_service(gen::E2eTypeServerServer::new(LogicE2eTypeServer))
        .with_service(gen::E2eAttributeServer::new(LogicE2eAttribute))
        .with_service(gen::E2eHttpFormServer::new(LogicE2eHttpForm))
        .with_service(gen::E2eHttpScopeMatrixServer::new(LogicE2eHttpScopeMatrix))
        .with_service(gen::E2eHttpDefaultsMatrixServer::new(LogicE2eHttpDefaultsMatrix))
        .with_service(gen::E2eHttpSecurityMatrixServer::new(LogicE2eHttpSecurityMatrix))
        .serve(&addr)
        .await?;
    Ok(())
}
