#[test]
fn serialize_deserialize_openapi_version_success() -> Result<(), serde_json::Error> {
    assert_eq!(serde_json::to_value(&OpenApiVersion::Version31)?, "3.1.0");
    Ok(())
}

#[test]
fn serialize_openapi_json_minimal_success() {
    let openapi = OpenApi::new(
        InfoBuilder::new()
            .title("My api")
            .version("1.0.0")
            .description(Some("My api description"))
            .license(Some(
                LicenseBuilder::new()
                    .name("MIT")
                    .url(Some("http://mit.licence"))
                    .build(),
            ))
            .build(),
        Paths::new(),
    );

    assert_json_snapshot!(openapi);
}

#[test]
fn serialize_openapi_json_with_paths_success() {
    let openapi = OpenApi::new(
        Info::new("My big api", "1.1.0"),
        PathsBuilder::new()
            .path(
                "/api/v1/users",
                PathItem::new(
                    HttpMethod::Get,
                    OperationBuilder::new().response("200", Response::new("Get users list")),
                ),
            )
            .path(
                "/api/v1/users",
                PathItem::new(
                    HttpMethod::Post,
                    OperationBuilder::new().response("200", Response::new("Post new user")),
                ),
            )
            .path(
                "/api/v1/users/{id}",
                PathItem::new(
                    HttpMethod::Get,
                    OperationBuilder::new().response("200", Response::new("Get user by id")),
                ),
            ),
    );

    assert_json_snapshot!(openapi);
}

#[test]
fn merge_2_openapi_documents() {
    let mut api_1 = OpenApi::new(
        Info::new("Api", "v1"),
        PathsBuilder::new()
            .path(
                "/api/v1/user",
                PathItem::new(
                    HttpMethod::Get,
                    OperationBuilder::new().response("200", Response::new("Get user success")),
                ),
            )
            .build(),
    );

    let api_2 = OpenApiBuilder::new()
        .info(Info::new("Api", "v2"))
        .paths(
            PathsBuilder::new()
                .path(
                    "/api/v1/user",
                    PathItem::new(
                        HttpMethod::Get,
                        OperationBuilder::new()
                            .response("200", Response::new("This will not get added")),
                    ),
                )
                .path(
                    "/ap/v2/user",
                    PathItem::new(
                        HttpMethod::Get,
                        OperationBuilder::new()
                            .response("200", Response::new("Get user success 2")),
                    ),
                )
                .path(
                    "/api/v2/user",
                    PathItem::new(
                        HttpMethod::Post,
                        OperationBuilder::new().response("200", Response::new("Get user success")),
                    ),
                )
                .build(),
        )
        .components(Some(
            ComponentsBuilder::new()
                .schema(
                    "User2",
                    ObjectBuilder::new().schema_type(Type::Object).property(
                        "name",
                        ObjectBuilder::new().schema_type(Type::String).build(),
                    ),
                )
                .build(),
        ))
        .build();

    api_1.merge(api_2);

    assert_json_snapshot!(api_1, {
        ".paths" => insta::sorted_redaction()
    });
}
