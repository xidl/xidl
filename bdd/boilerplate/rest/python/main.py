import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from rest_http import *


class MyHelloWorld(HelloWorldService):
    async def hello(self, request):
        return HelloWorldHelloResponse(value="Hello BDD")

    async def echo(self, request):
        return HelloWorldEchoResponse(value=request.msg)


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, hello_world_routes(MyHelloWorld()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
