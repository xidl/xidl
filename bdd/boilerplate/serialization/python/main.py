import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from serialization import Item
from serialization_http import *


class MySerialization(SerializationTestService):
    async def get_string(self, request):
        return SerializationTestGetStringResponse(value="hello")

    async def get_int(self, request):
        return SerializationTestGetIntResponse(value=42)

    async def get_bool(self, request):
        return SerializationTestGetBoolResponse(value=True)

    async def get_struct(self, request):
        return SerializationTestGetStructResponse(value=Item(name="world"))

    async def echo_string(self, request):
        return SerializationTestEchoStringResponse(value=request.value)

    async def echo_struct(self, request):
        return SerializationTestEchoStructResponse(value=request.value)


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, serialization_test_routes(MySerialization()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
