use crate::openapi::{
    ComponentsBuilder, HttpMethod, Info, InfoBuilder, LicenseBuilder, ObjectBuilder, OpenApi,
    OpenApiBuilder, OpenApiVersion, PathItem, Paths, PathsBuilder, Response, Type,
    extensions::Extensions, path::OperationBuilder,
};
use insta::assert_json_snapshot;

include!("openapi_tests_basic.inc.rs");
include!("openapi_tests_merge.inc.rs");
