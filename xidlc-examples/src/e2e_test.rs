#![allow(deprecated)]
include!(concat!(env!("OUT_DIR"), "/e2e_test.rs"));

use std::collections::BTreeMap;

pub struct LogicE2ePathSever;

#[async_trait::async_trait]
impl E2ePathSever for LogicE2ePathSever {
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

pub struct LogicE2eHttpRouteAndBody;

#[async_trait::async_trait]
impl E2eHttpRouteAndBody for LogicE2eHttpRouteAndBody {
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

pub struct LogicE2eHttpSecurity;

#[async_trait::async_trait]
impl E2eHttpSecurity for LogicE2eHttpSecurity {
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

pub struct LogicE2eTypeServer;

#[async_trait::async_trait]
impl E2eTypeServer for LogicE2eTypeServer {
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
    async fn simple_op_with_return2(&self) -> Result<EnumEmpty, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn simple_op_with_return3(&self) -> Result<EnumSimple1, xidl_rust_axum::Error> {
        Ok(EnumSimple1::V1)
    }
    async fn simple_op_with_return4(&self) -> Result<StructEmpty, xidl_rust_axum::Error> {
        Ok(StructEmpty {})
    }
    async fn simple_op_with_return5(&self) -> Result<StructSimple, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn return_with_sequence1(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["s1".to_string(), "s2".to_string()])
    }
    async fn return_with_sequence2(&self) -> Result<Vec<EnumEmpty>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn return_with_sequence3(&self) -> Result<Vec<EnumSimple1>, xidl_rust_axum::Error> {
        Ok(vec![EnumSimple1::V1, EnumSimple1::V2])
    }
    async fn return_with_sequence4(&self) -> Result<Vec<StructEmpty>, xidl_rust_axum::Error> {
        Ok(vec![StructEmpty {}])
    }
    async fn return_with_sequence5(&self) -> Result<Vec<StructSimple>, xidl_rust_axum::Error> {
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
    async fn parameter_op(&self, _a: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn parameter_op2(&self, _a: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn parameter_op3(
        &self,
        _a: String,
        _c: Vec<serde_json::Value>,
    ) -> Result<E2ETypeServerParameterOp3Response, xidl_rust_axum::Error> {
        Ok(E2ETypeServerParameterOp3Response { b: 3, c: vec![] })
    }
    async fn parameter_op4(
        &self,
        _c: Vec<serde_json::Value>,
    ) -> Result<E2ETypeServerParameterOp4Response, xidl_rust_axum::Error> {
        Ok(E2ETypeServerParameterOp4Response {
            a: "op4".to_string(),
            b: 4,
            c: vec![],
        })
    }
    async fn parameter_op5(
        &self,
        _c: Vec<serde_json::Value>,
    ) -> Result<E2ETypeServerParameterOp5Response, xidl_rust_axum::Error> {
        Ok(E2ETypeServerParameterOp5Response {
            r#return: vec![serde_json::json!("op5")],
            a: "op5".to_string(),
            b: 5,
            c: vec![],
        })
    }
    async fn parameter_op6(
        &self,
        _c: Vec<serde_json::Value>,
    ) -> Result<E2ETypeServerParameterOp6Response, xidl_rust_axum::Error> {
        Ok(E2ETypeServerParameterOp6Response {
            r#return: BTreeMap::new(),
            a: "op6".to_string(),
            b: 6,
            c: vec![],
        })
    }
}

pub struct LogicE2eAttribute;

#[async_trait::async_trait]
impl E2eAttribute for LogicE2eAttribute {
    async fn get_attribute_attr1(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("attr1".to_string())
    }
    async fn set_attribute_attr1(&self, _value: String) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr2(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["attr2".to_string()])
    }
    async fn get_attribute_attr3(&self) -> Result<EnumEmpty, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn set_attribute_attr3(&self, _value: EnumEmpty) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr4(&self) -> Result<EnumSimple1, xidl_rust_axum::Error> {
        Ok(EnumSimple1::V1)
    }
    async fn set_attribute_attr4(&self, _value: EnumSimple1) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr5(&self) -> Result<StructEmpty, xidl_rust_axum::Error> {
        Ok(StructEmpty {})
    }
    async fn set_attribute_attr5(&self, _value: StructEmpty) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr6(&self) -> Result<StructSimple, xidl_rust_axum::Error> {
        unimplemented!()
    }
    async fn set_attribute_attr6(&self, _value: StructSimple) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr61(&self) -> Result<UnionSimple, xidl_rust_axum::Error> {
        Ok(UnionSimple::new_case1(1))
    }
    async fn set_attribute_attr61(&self, _value: UnionSimple) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr7(&self) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec!["attr7".to_string()])
    }
    async fn set_attribute_attr7(&self, _value: Vec<String>) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr8(&self) -> Result<Vec<EnumEmpty>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn set_attribute_attr8(
        &self,
        _value: Vec<EnumEmpty>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr9(&self) -> Result<Vec<EnumSimple1>, xidl_rust_axum::Error> {
        Ok(vec![EnumSimple1::V1])
    }
    async fn set_attribute_attr9(
        &self,
        _value: Vec<EnumSimple1>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr10(&self) -> Result<Vec<StructEmpty>, xidl_rust_axum::Error> {
        Ok(vec![StructEmpty {}])
    }
    async fn set_attribute_attr10(
        &self,
        _value: Vec<StructEmpty>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn get_attribute_attr11(&self) -> Result<Vec<StructSimple>, xidl_rust_axum::Error> {
        Ok(vec![])
    }
    async fn set_attribute_attr11(
        &self,
        _value: Vec<StructSimple>,
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

#[async_trait::async_trait]
impl E2eHttpForm for LogicE2eHttpForm {
    async fn submit_profile(
        &self,
        name: String,
        age: Option<u32>,
    ) -> Result<E2EHttpFormSubmitProfileResponse, xidl_rust_axum::Error> {
        Ok(E2EHttpFormSubmitProfileResponse {
            r#return: format!("name:{name},age:{age:?}"),
            normalized_name: name.to_uppercase(),
        })
    }
}

pub struct LogicE2eHttpScopeMatrix;

#[async_trait::async_trait]
impl E2eHttpScopeMatrix for LogicE2eHttpScopeMatrix {
    async fn get_attribute_scope_inherited_attr(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("inherited".to_string())
    }
    async fn get_attribute_scope_bare_attr(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("bare".to_string())
    }
    async fn default_scope(
        &self,
        request_body: StructHttpBody,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(request_body.name)
    }
    async fn override_consumes_only(
        &self,
        name: String,
        age: Option<u32>,
    ) -> Result<E2EHttpScopeMatrixOverrideConsumesOnlyResponse, xidl_rust_axum::Error> {
        Ok(E2EHttpScopeMatrixOverrideConsumesOnlyResponse {
            r#return: format!("name:{name},age:{age:?}"),
            normalized_name: name.to_uppercase(),
        })
    }
    async fn override_produces_only(
        &self,
        resource_id: String,
    ) -> Result<E2EHttpScopeMatrixOverrideProducesOnlyResponse, xidl_rust_axum::Error> {
        Ok(E2EHttpScopeMatrixOverrideProducesOnlyResponse {
            r#return: StructHttpBody {
                name: resource_id,
                alias: None,
                tags: vec![],
                labels: BTreeMap::new(),
            },
            revision: 1,
        })
    }
    async fn override_both_media(
        &self,
        name: String,
        age: Option<u32>,
    ) -> Result<E2EHttpScopeMatrixOverrideBothMediaResponse, xidl_rust_axum::Error> {
        Ok(E2EHttpScopeMatrixOverrideBothMediaResponse {
            r#return: StructHttpBody {
                name,
                alias: None,
                tags: vec![format!("age:{age:?}")],
                labels: BTreeMap::new(),
            },
            normalized_name: "OVERRIDDEN".to_string(),
        })
    }
    async fn deprecated_plain(&self, resource_id: String) -> Result<String, xidl_rust_axum::Error> {
        Ok(resource_id)
    }
    async fn deprecated_since_only(
        &self,
        resource_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(resource_id)
    }
    async fn deprecated_window(
        &self,
        resource_id: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(resource_id)
    }
}

pub struct LogicE2eHttpDefaultsMatrix;

#[async_trait::async_trait]
impl E2eHttpDefaultsMatrix for LogicE2eHttpDefaultsMatrix {
    async fn delete_resource_default_query(
        &self,
        id: String,
        revision: u32,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{id}:{revision}"))
    }
    async fn probe_resource_default_query(
        &self,
        _id: String,
        _revision: u32,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn resource_options_default_query(
        &self,
        _id: String,
        _revision: u32,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
    async fn replace_resource_default_body(
        &self,
        id: String,
        name: String,
        alias: Option<String>,
    ) -> Result<StructHttpBody, xidl_rust_axum::Error> {
        Ok(StructHttpBody {
            name,
            alias,
            tags: vec![id],
            labels: BTreeMap::new(),
        })
    }
    async fn patch_resource_default_body(
        &self,
        id: String,
        name: String,
        alias: Option<String>,
    ) -> Result<StructHttpBody, xidl_rust_axum::Error> {
        Ok(StructHttpBody {
            name,
            alias,
            tags: vec![id],
            labels: BTreeMap::new(),
        })
    }
}

pub struct LogicE2eHttpSecurityMatrix;

#[async_trait::async_trait]
impl E2eHttpSecurityMatrix for LogicE2eHttpSecurityMatrix {
    async fn inherited_security(
        &self,
        resource_id: String,
        trace_id: String,
        _xidl_auth: xidl_rust_axum::auth::basic::BasicAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{resource_id}:{trace_id}"))
    }
    async fn bearer_or_cookie_security(
        &self,
        action: String,
        note: Option<String>,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{action}:{note:?}"))
    }
    async fn alternative_security(
        &self,
        resource_id: String,
        locale: Option<String>,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{resource_id}:{locale:?}"))
    }
    async fn oauth_security(
        &self,
        keyword: String,
        page: Option<u32>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("{keyword}:{page:?}"))
    }
    async fn public_ping(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok("pong".to_string())
    }
}
