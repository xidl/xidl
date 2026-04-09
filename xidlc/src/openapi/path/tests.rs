use super::{HttpMethod, Operation, OperationBuilder};
use crate::openapi::{PathItem, PathsBuilder, security::SecurityRequirement, server::Server};

#[test]
fn test_path_order() {
    let paths_list = PathsBuilder::new()
        .path(
            "/todo",
            PathItem::new(HttpMethod::Get, OperationBuilder::new()),
        )
        .path(
            "/todo",
            PathItem::new(HttpMethod::Post, OperationBuilder::new()),
        )
        .path(
            "/todo/{id}",
            PathItem::new(HttpMethod::Delete, OperationBuilder::new()),
        )
        .path(
            "/todo/{id}",
            PathItem::new(HttpMethod::Get, OperationBuilder::new()),
        )
        .path(
            "/todo/{id}",
            PathItem::new(HttpMethod::Put, OperationBuilder::new()),
        )
        .path(
            "/todo/search",
            PathItem::new(HttpMethod::Get, OperationBuilder::new()),
        )
        .build();

    let actual_value = paths_list
        .paths
        .iter()
        .flat_map(|(path, path_item)| {
            let mut path_methods =
                Vec::<(&str, &HttpMethod)>::with_capacity(paths_list.paths.len());
            if path_item.get.is_some() {
                path_methods.push((path, &HttpMethod::Get));
            }
            if path_item.put.is_some() {
                path_methods.push((path, &HttpMethod::Put));
            }
            if path_item.post.is_some() {
                path_methods.push((path, &HttpMethod::Post));
            }
            if path_item.delete.is_some() {
                path_methods.push((path, &HttpMethod::Delete));
            }
            if path_item.options.is_some() {
                path_methods.push((path, &HttpMethod::Options));
            }
            if path_item.head.is_some() {
                path_methods.push((path, &HttpMethod::Head));
            }
            if path_item.patch.is_some() {
                path_methods.push((path, &HttpMethod::Patch));
            }
            if path_item.trace.is_some() {
                path_methods.push((path, &HttpMethod::Trace));
            }
            path_methods
        })
        .collect::<Vec<_>>();

    let get = HttpMethod::Get;
    let post = HttpMethod::Post;
    let put = HttpMethod::Put;
    let delete = HttpMethod::Delete;
    let expected_value = vec![
        ("/todo", &get),
        ("/todo", &post),
        ("/todo/search", &get),
        ("/todo/{id}", &get),
        ("/todo/{id}", &put),
        ("/todo/{id}", &delete),
    ];
    assert_eq!(actual_value, expected_value);
}

#[test]
fn operation_new() {
    let operation = Operation::new();
    assert!(operation.tags.is_none());
    assert!(operation.summary.is_none());
    assert!(operation.description.is_none());
    assert!(operation.operation_id.is_none());
    assert!(operation.external_docs.is_none());
    assert!(operation.parameters.is_none());
    assert!(operation.request_body.is_none());
    assert!(operation.responses.responses.is_empty());
    assert!(operation.callbacks.is_none());
    assert!(operation.deprecated.is_none());
    assert!(operation.security.is_none());
    assert!(operation.servers.is_none());
}

#[test]
fn operation_builder_security() {
    let security_requirement1 =
        SecurityRequirement::new("api_oauth2_flow", ["edit:items", "read:items"]);
    let security_requirement2 = SecurityRequirement::new("api_oauth2_flow", ["remove:items"]);
    let operation = OperationBuilder::new()
        .security(security_requirement1)
        .security(security_requirement2)
        .build();
    assert!(operation.security.is_some());
}

#[test]
fn operation_builder_server() {
    let server1 = Server::new("/api");
    let server2 = Server::new("/admin");
    let operation = OperationBuilder::new()
        .server(server1)
        .server(server2)
        .build();
    assert!(operation.servers.is_some());
}
