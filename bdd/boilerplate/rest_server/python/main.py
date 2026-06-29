import os

import uvicorn
from fastapi import FastAPI, Request as FastAPIRequest
from fastapi.responses import JSONResponse, Response as FastAPIRawResponse
from xidl.fastapi import FastAPIAdapter
from xidl.http import HttpError, Response, register_routes

from rest_server_http import *


class MyRestServer(RestServerService):
    def __init__(self):
        self.host = "localhost"
        self.server_name = "rest_server"
        self.users = {}
        self.keys = {}

    async def get_attribute_host(self, request):
        return RestServerGetAttributeHostResponse(value=self.host)

    async def set_attribute_host(self, request):
        self.host = request.host
        return Response(status_code=204)

    async def get_attribute_port(self, request):
        return RestServerGetAttributePortResponse(value=8081)

    async def get_server_name(self, request):
        return RestServerGetServerNameResponse(value=self.server_name)

    async def set_server_name(self, request):
        self.server_name = request.name
        return Response(status_code=204)

    async def get_user_info(self, request):
        if request.id not in self.users:
            raise HttpError(404, 404, "Not Found")
        return RestServerGetUserInfoResponse(value=self.users[request.id])

    async def query_user_info(self, request):
        return await self.get_user_info(request)

    async def post_user_info(self, request):
        self.users[request.id] = request.info
        return Response(status_code=204)

    async def put_key_value(self, request):
        self.keys[request.key] = request.value
        return Response(status_code=204)

    async def delete_key(self, request):
        self.keys.pop(request.key, None)
        return Response(status_code=204)

    async def patch_key(self, request):
        self.keys[request.key] = request.value
        return Response(status_code=204)

    async def is_key_exists(self, request):
        if request.key_alias not in self.keys:
            raise HttpError(404, 404, "Not Found")
        return Response(status_code=204)

    async def get_key_options(self, request):
        return RestServerGetKeyOptionsResponse(exists=request.key in self.keys)

    async def get_key_1(self, request):
        if request.key not in self.keys:
            raise HttpError(404, 404, "Not Found")
        return RestServerGetKey1Response(value=self.keys[request.key])

    async def get_key_2(self, request):
        return await self.get_key_1(request)

    async def get_key_3(self, request):
        return await self.get_key_1(request)

    async def get_key_4(self, request):
        return await self.get_key_1(request)

    async def login(self, request):
        return RestServerLoginResponse(session_id="simple_session_id")

    async def login_realm(self, request):
        return RestServerLoginRealmResponse(session_id="simple_session_id")

    async def is_logined(self, request):
        return RestServerIsLoginedResponse(value=bool(request.session_id))

    async def login_bearer(self, request):
        return Response(status_code=204)

    async def get_timestamp(self, request):
        return RestServerGetTimestampResponse(value={"seconds": 0, "nanos": 0})

    async def is_admin(self, request):
        return RestServerIsAdminResponse()


app = FastAPI()


def auth_error(realm=None):
    headers = {"WWW-Authenticate": f'Basic realm="{realm}"'} if realm else {}
    return JSONResponse(
        {"code": 401, "msg": "Unauthorized"}, status_code=401, headers=headers
    )


@app.post("/login")
async def login_route(request: FastAPIRequest):
    if not request.headers.get("authorization"):
        return auth_error("login")
    return FastAPIRawResponse(
        status_code=204, headers={"Set-Cookie": "session_id=simple_session_id"}
    )


@app.post("/login_realm")
async def login_realm_route(request: FastAPIRequest):
    if not request.headers.get("authorization"):
        return auth_error("request login with realm")
    return FastAPIRawResponse(
        status_code=204, headers={"Set-Cookie": "session_id=simple_session_id"}
    )


@app.post("/login_bearer")
async def login_bearer_route(request: FastAPIRequest):
    auth = request.headers.get("authorization")
    if not auth or auth == "Bearer":
        return auth_error()
    return FastAPIRawResponse(status_code=204)


adapter = FastAPIAdapter(app=app)
register_routes(adapter, rest_server_routes(MyRestServer()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
