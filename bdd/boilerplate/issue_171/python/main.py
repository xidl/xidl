import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from issue_171_http import *


class MyRepro(ReproServiceService):
    async def flatten_any(self, request):
        if not isinstance(request.payload, dict) or request.payload.get("foo") != "bar":
            raise Exception("invalid payload")
        return ReproServiceFlattenAnyResponse()

    async def flatten_struct_with_any(self, request):
        payload = request.payload
        field_val = getattr(payload, "field", None)
        if field_val is None and isinstance(payload, dict):
            field_val = payload.get("field")
        if not isinstance(field_val, dict) or field_val.get("foo") != "bar":
            raise Exception("invalid payload")
        return ReproServiceFlattenStructWithAnyResponse()


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, repro_service_routes(MyRepro()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
