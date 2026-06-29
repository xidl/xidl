import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from media_types_http import *


class MyForm(FormServiceService):
    async def submit(self, request):
        return FormServiceSubmitResponse(
            value=f"Received {request.name} age {request.age}"
        )


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, form_service_routes(MyForm()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
