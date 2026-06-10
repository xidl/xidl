import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from rest_media_types_http import *


class MyRestMediaTypesApi(RestMediaTypesApiService):
    async def submit_profile(self, request):
        return RestMediaTypesApiSubmitProfileResponse(
            return_=f"{request.name}:{request.age}",
            normalized_name=request.name.upper(),
        )

    async def get_msgpack_user(self, request):
        return RestMediaTypesApiGetMsgpackUserResponse(
            return_=f"user:{request.user_id}",
            score=95,
        )


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, rest_media_types_api_routes(MyRestMediaTypesApi()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
