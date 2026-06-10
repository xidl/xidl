import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from all_scenarios import Status
from all_scenarios_http import *


class MyAllScenarios(AllScenariosServiceService):
    def __init__(self):
        self.status = Status.ACTIVE
        self.items = {}

    async def get_item(self, request):
        value = f"Item {request.id} with {request.filter} and {request.trace_id}"
        return AllScenariosServiceGetItemResponse(value=value)

    async def create_item(self, request):
        self.items[len(self.items)] = request.name
        return AllScenariosServiceCreateItemResponse(value=42)

    async def update_item(self, request):
        return AllScenariosServiceUpdateItemResponse()

    async def delete_item(self, request):
        return AllScenariosServiceDeleteItemResponse()

    async def get_attribute_system_status(self, request):
        return AllScenariosServiceGetAttributeSystemStatusResponse(value=self.status)

    async def set_attribute_system_status(self, request):
        self.status = request.system_status
        return AllScenariosServiceSetAttributeSystemStatusResponse()

    async def get_attribute_version(self, request):
        return AllScenariosServiceGetAttributeVersionResponse(value="1.0.0")

    async def upload_form(self, request):
        return AllScenariosServiceUploadFormResponse()

    async def secure_data(self, request, xidl_auth=None):
        return AllScenariosServiceSecureDataResponse(value="Secret")


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, all_scenarios_service_routes(MyAllScenarios()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
