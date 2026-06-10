import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import ServerStreamResponse, register_routes

from streaming_http import *


class MyStreaming(StreamingServiceService):
    async def ticks(self, request):
        return ServerStreamResponse(items=range(request.count))


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, streaming_service_routes(MyStreaming()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
