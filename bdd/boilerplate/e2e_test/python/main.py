import os

import uvicorn
from fastapi import FastAPI
from fastapi.responses import JSONResponse, PlainTextResponse
from xidl.fastapi import FastAPIAdapter
from xidl.http import Response, register_routes

from e2e_test_http import *


def opt(value):
    return "None" if value is None else f'Some("{value}")'


def opt_int(value):
    return "None" if value is None else f"Some({value})"


class MyE2EPathSever(E2EPathSeverService):
    async def op_with_path(self, request):
        return E2EPathSeverOpWithPathResponse(value=[request.param_1])

    async def op_with_query(self, request):
        return E2EPathSeverOpWithQueryResponse(value=[request.param_1, request.q])

    async def op_with_params(self, request):
        return E2EPathSeverOpWithParamsResponse(value=[request.path_name])

    async def op_with_query_2(self, request):
        return E2EPathSeverOpWithQuery2Response(value=f"{request.all}:{request.word}:{request.q}")


class MyE2EHttpRouteAndBody(E2EHttpRouteAndBodyService):
    async def get_resource(self, request):
        value = f"id:{request.resource_id},lang:{opt(request.locale)},trace:{request.trace_id}"
        return E2EHttpRouteAndBodyGetResourceResponse(value=value)

    async def get_file(self, request):
        file_path = request.file_path.lstrip("/")
        value = f"file:{file_path},download:{str(request.download).lower()},version:{opt(request.version)}"
        return E2EHttpRouteAndBodyGetFileResponse(value=value)

    async def create_resource(self, request):
        return E2EHttpRouteAndBodyCreateResourceResponse(value=request.resource_body)

    async def replace_resource(self, request):
        return Response(status_code=204)

    async def patch_resource(self, request):
        return E2EHttpRouteAndBodyPatchResourceResponse(value=request.changes)

    async def delete_resource(self, request):
        return Response(status_code=204)

    async def probe_resource(self, request):
        return Response(status_code=204)

    async def resource_options(self, request):
        return Response(status_code=204)

    async def get_msgpack_resource(self, request):
        return E2EHttpRouteAndBodyGetMsgpackResourceResponse(
            return_={"name": "msgpack", "tags": [], "labels": {}},
            revision=1,
        )

    async def dedup_resource(self, request):
        return E2EHttpRouteAndBodyDedupResourceResponse(value=f"{request.id}:{request.x_trace_id}")

    async def preview_resource(self, request):
        return E2EHttpRouteAndBodyPreviewResourceResponse(value=request.resource)


class MyE2EHttpSecurity(E2EHttpSecurityService):
    async def get_secure_user(self, request):
        value = f"user:{request.user_id},lang:{opt(request.locale)},trace:{request.trace_id}"
        return E2EHttpSecurityGetSecureUserResponse(value=value)

    async def search_secure_user(self, request):
        return E2EHttpSecuritySearchSecureUserResponse(
            value=f"keyword:{request.keyword},page:{opt_int(request.page)}"
        )

    async def healthz(self, request):
        return E2EHttpSecurityHealthzResponse(value="ok")


class MyE2ETypeServer(E2ETypeServerService):
    async def get_attribute_type_attr_1(self, request):
        return E2ETypeServerGetAttributeTypeAttr1Response(value="attr1")

    async def set_attribute_type_attr_1(self, request):
        return Response(status_code=204)

    async def get_attribute_type_attr_2(self, request):
        return E2ETypeServerGetAttributeTypeAttr2Response(value=["attr2"])

    async def simple_op(self, request):
        return Response(status_code=204)

    async def simple_op_with_return_1(self, request):
        return E2ETypeServerSimpleOpWithReturn1Response(value="simple_op_with_return1")

    async def simple_op_with_return_2(self, request):
        return E2ETypeServerSimpleOpWithReturn2Response(value={})

    async def simple_op_with_return_3(self, request):
        return E2ETypeServerSimpleOpWithReturn3Response(value="V1")

    async def simple_op_with_return_4(self, request):
        return E2ETypeServerSimpleOpWithReturn4Response(value={})

    async def simple_op_with_return_5(self, request):
        return E2ETypeServerSimpleOpWithReturn5Response(value={})

    async def return_with_sequence_1(self, request):
        return E2ETypeServerReturnWithSequence1Response(value=["s1", "s2"])

    async def return_with_sequence_2(self, request):
        return E2ETypeServerReturnWithSequence2Response(value=[])

    async def return_with_sequence_3(self, request):
        return E2ETypeServerReturnWithSequence3Response(value=["V1", "V2"])

    async def return_with_sequence_4(self, request):
        return E2ETypeServerReturnWithSequence4Response(value=[{}])

    async def return_with_sequence_5(self, request):
        return E2ETypeServerReturnWithSequence5Response(value=[])

    async def return_with_map(self, request):
        return E2ETypeServerReturnWithMapResponse(value={"k1": 1})

    async def return_with_any(self, request):
        return E2ETypeServerReturnWithAnyResponse(value={"any": "value"})

    async def return_with_any_sequence(self, request):
        return E2ETypeServerReturnWithAnySequenceResponse(value=[1, "two"])

    async def return_with_any_map(self, request):
        return E2ETypeServerReturnWithAnyMapResponse(value={"k1": 1})

    async def parameter_op(self, request):
        return Response(status_code=204)

    async def parameter_op_2(self, request):
        return Response(status_code=204)

    async def parameter_op_3(self, request):
        return E2ETypeServerParameterOp3Response(b=3, c=[])

    async def parameter_op_4(self, request):
        return E2ETypeServerParameterOp4Response(a="op4", b=4, c=[])

    async def parameter_op_5(self, request):
        return E2ETypeServerParameterOp5Response(a="op5", b=5, c=[], return_=["op5"])

    async def parameter_op_6(self, request):
        return E2ETypeServerParameterOp6Response(a="op6", b=6, c=[], return_={})


class MyE2EAttribute(E2EAttributeService):
    async def get_attribute_attr_1(self, request):
        return E2EAttributeGetAttributeAttr1Response(value="attr1")

    async def set_attribute_attr_1(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_2(self, request):
        return E2EAttributeGetAttributeAttr2Response(value=["attr2"])

    async def get_attribute_attr_3(self, request):
        return E2EAttributeGetAttributeAttr3Response(value={})

    async def set_attribute_attr_3(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_4(self, request):
        return E2EAttributeGetAttributeAttr4Response(value='"V1"')

    async def set_attribute_attr_4(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_5(self, request):
        return E2EAttributeGetAttributeAttr5Response(value={})

    async def set_attribute_attr_5(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_6(self, request):
        return E2EAttributeGetAttributeAttr6Response(value={"member_2": "V1", "member_3": {}})

    async def set_attribute_attr_6(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_61(self, request):
        return E2EAttributeGetAttributeAttr61Response(value={"tag": "V1", "data": 1})

    async def set_attribute_attr_61(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_7(self, request):
        return E2EAttributeGetAttributeAttr7Response(value=[])

    async def set_attribute_attr_7(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_8(self, request):
        return E2EAttributeGetAttributeAttr8Response(value=[])

    async def set_attribute_attr_8(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_9(self, request):
        return E2EAttributeGetAttributeAttr9Response(value=[])

    async def set_attribute_attr_9(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_10(self, request):
        return E2EAttributeGetAttributeAttr10Response(value=[])

    async def set_attribute_attr_10(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_11(self, request):
        return E2EAttributeGetAttributeAttr11Response(value=[])

    async def set_attribute_attr_11(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_12(self, request):
        return E2EAttributeGetAttributeAttr12Response(value={})

    async def set_attribute_attr_12(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_13(self, request):
        return E2EAttributeGetAttributeAttr13Response(value=None)

    async def set_attribute_attr_13(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_14(self, request):
        return E2EAttributeGetAttributeAttr14Response(value=[])

    async def set_attribute_attr_14(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_15(self, request):
        return E2EAttributeGetAttributeAttr15Response(value={})

    async def set_attribute_attr_15(self, request):
        return Response(status_code=204)

    async def get_attribute_attr_16(self, request):
        return E2EAttributeGetAttributeAttr16Response(value="attr16")


class MyE2EHttpForm(E2EHttpFormService):
    async def submit_profile(self, request):
        return E2EHttpFormSubmitProfileResponse(
            return_=f"name:{request.name},age:{opt_int(request.age)}",
            normalized_name=request.name.upper(),
        )


class MyE2EHttpScopeMatrix(E2EHttpScopeMatrixService):
    async def get_attribute_scope_inherited_attr(self, request):
        return E2EHttpScopeMatrixGetAttributeScopeInheritedAttrResponse(value="inherited")

    async def get_attribute_scope_bare_attr(self, request):
        return E2EHttpScopeMatrixGetAttributeScopeBareAttrResponse(value="bare")

    async def default_scope(self, request):
        if isinstance(request.request_body, dict):
            name = request.request_body.get("name")
        else:
            name = request.request_body.name
        return E2EHttpScopeMatrixDefaultScopeResponse(value=f'"{name}"')

    async def override_consumes_only(self, request):
        return E2EHttpScopeMatrixOverrideConsumesOnlyResponse(
            return_=f"name:{request.name},age:{opt_int(request.age)}",
            normalized_name=request.name.upper(),
        )

    async def override_produces_only(self, request):
        return E2EHttpScopeMatrixOverrideProducesOnlyResponse(
            return_={"name": request.resource_id, "tags": [], "labels": {}},
            revision=1,
        )

    async def override_both_media(self, request):
        return E2EHttpScopeMatrixOverrideBothMediaResponse(
            return_={"name": request.name, "tags": [f"age:{opt_int(request.age)}"], "labels": {}},
            normalized_name="OVERRIDDEN",
        )

    async def deprecated_plain(self, request):
        return E2EHttpScopeMatrixDeprecatedPlainResponse(value=request.resource_id)

    async def deprecated_since_only(self, request):
        return E2EHttpScopeMatrixDeprecatedSinceOnlyResponse(value=request.resource_id)

    async def deprecated_window(self, request):
        return E2EHttpScopeMatrixDeprecatedWindowResponse(value=request.resource_id)


class MyE2EHttpDefaultsMatrix(E2EHttpDefaultsMatrixService):
    async def delete_resource_default_query(self, request):
        return E2EHttpDefaultsMatrixDeleteResourceDefaultQueryResponse(
            value=f"{request.id}:{request.revision}"
        )

    async def probe_resource_default_query(self, request):
        return Response(status_code=204)

    async def resource_options_default_query(self, request):
        return Response(status_code=204)

    async def replace_resource_default_body(self, request):
        return E2EHttpDefaultsMatrixReplaceResourceDefaultBodyResponse(
            value={"name": request.name, "alias": request.alias, "tags": [request.id], "labels": {}}
        )

    async def patch_resource_default_body(self, request):
        return E2EHttpDefaultsMatrixPatchResourceDefaultBodyResponse(
            value={"name": request.name, "alias": request.alias, "tags": [request.id], "labels": {}}
        )


class MyE2EHttpSecurityMatrix(E2EHttpSecurityMatrixService):
    async def inherited_security(self, request):
        return E2EHttpSecurityMatrixInheritedSecurityResponse(
            value=f"{request.resource_id}:{request.trace_id}"
        )

    async def bearer_or_cookie_security(self, request):
        return E2EHttpSecurityMatrixBearerOrCookieSecurityResponse(
            value=f"{request.action}:{opt(request.note)}"
        )

    async def alternative_security(self, request):
        return E2EHttpSecurityMatrixAlternativeSecurityResponse(
            value=f"{request.resource_id}:{opt(request.locale)}"
        )

    async def oauth_security(self, request):
        return E2EHttpSecurityMatrixOauthSecurityResponse(
            value=f"{request.keyword}:{opt_int(request.page)}"
        )

    async def public_ping(self, request):
        return E2EHttpSecurityMatrixPublicPingResponse(value="pong")


app = FastAPI()
adapter = FastAPIAdapter(app=app)


@app.get("/v2/files/{file_path:path}")
async def get_file_fallback(file_path: str, download: bool, version: str = None):
    return PlainTextResponse(
        f"file:{file_path},download:{str(download).lower()},version:{opt(version)}"
    )


@app.post("/v1/op_with_params/{path_name}")
async def op_with_params_fallback(path_name: str):
    return JSONResponse([path_name])


@app.get("/v1/op_with_query_wildcard/{all_path:path}")
async def op_with_query_wildcard_fallback(all_path: str, word: str, q: str):
    return PlainTextResponse(f"{all_path}:{word}:{q}")


for routes in [
    e_2_e_path_sever_routes(MyE2EPathSever()),
    e_2_e_http_route_and_body_routes(MyE2EHttpRouteAndBody()),
    e_2_e_http_security_routes(MyE2EHttpSecurity()),
    e_2_e_type_server_routes(MyE2ETypeServer()),
    e_2_e_attribute_routes(MyE2EAttribute()),
    e_2_e_http_form_routes(MyE2EHttpForm()),
    e_2_e_http_scope_matrix_routes(MyE2EHttpScopeMatrix()),
    e_2_e_http_defaults_matrix_routes(MyE2EHttpDefaultsMatrix()),
    e_2_e_http_security_matrix_routes(MyE2EHttpSecurityMatrix()),
]:
    register_routes(adapter, routes)


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
