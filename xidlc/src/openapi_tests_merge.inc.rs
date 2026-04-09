#[test]
fn merge_same_path_diff_methods() {
    let mut api_1 = OpenApiBuilder::new()
        .info(Info::new("Api", "v1"))
        .paths(
            crate::openapi::PathsBuilder::new()
                .path(
                    "/api/v1/user",
                    PathItem::new(
                        HttpMethod::Get,
                        OperationBuilder::new()
                            .response("200", Response::new("Get user success 1")),
                    ),
                )
                .extensions(Some(Extensions::from_iter([("x-v1-api", true)])))
                .build(),
        )
        .build();

    let api_2 = OpenApiBuilder::new()
        .info(Info::new("Api", "v2"))
        .paths(
            crate::openapi::PathsBuilder::new()
                .path(
                    "/api/v1/user",
                    PathItem::new(
                        HttpMethod::Get,
                        OperationBuilder::new()
                            .response("200", Response::new("This will not get added")),
                    ),
                )
                .path(
                    "/api/v1/user",
                    PathItem::new(
                        HttpMethod::Post,
                        OperationBuilder::new()
                            .response("200", Response::new("Post user success 1")),
                    ),
                )
                .path(
                    "/api/v2/user",
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
                        OperationBuilder::new()
                            .response("200", Response::new("Post user success 2")),
                    ),
                )
                .extensions(Some(Extensions::from_iter([("x-random", "Value")])))
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

#[test]
fn test_nest_open_apis() {
    let api = OpenApiBuilder::new()
        .paths(crate::openapi::PathsBuilder::new().path(
            "/api/v1/status",
            PathItem::new(
                HttpMethod::Get,
                OperationBuilder::new().description(Some("Get status")).build(),
            ),
        ))
        .build();

    let user_api = OpenApiBuilder::new()
        .paths(
            crate::openapi::PathsBuilder::new()
                .path(
                    "/",
                    PathItem::new(
                        HttpMethod::Get,
                        OperationBuilder::new()
                            .description(Some("Get user details"))
                            .build(),
                    ),
                )
                .path(
                    "/foo",
                    PathItem::new(HttpMethod::Post, OperationBuilder::new().build()),
                ),
        )
        .build();

    let nest_merged = api.nest("/api/v1/user", user_api);
    let value = serde_json::to_value(nest_merged).expect("should serialize as json");
    let paths = value.pointer("/paths").expect("paths should exits in openapi");

    assert_json_snapshot!(paths);
}

#[test]
fn openapi_custom_extension() {
    let mut api = OpenApiBuilder::new().build();
    let extensions = api.extensions.get_or_insert(Default::default());
    extensions.insert(
        String::from("x-tagGroup"),
        String::from("anything that serializes to Json").into(),
    );

    assert_json_snapshot!(api);
}
