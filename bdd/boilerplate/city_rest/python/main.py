import os

import uvicorn
from fastapi import FastAPI
from fastapi.responses import JSONResponse
from xidl.fastapi import FastAPIAdapter
from xidl.http import register_routes

from city_rest_http import *


class MySmartCityRestApi(SmartCityRestApiService):
    def __init__(self):
        self.maintenance_mode = False

    async def get_stop_eta(self, request):
        return SmartCityRestApiGetStopEtaResponse(
            return_=request.stop_id,
            eta_seconds=240,
            destination="Central Station",
        )

    async def list_nearby_stops(self, request):
        return SmartCityRestApiListNearbyStopsResponse(
            value=[f"{request.stop_id}-A", f"{request.stop_id}-B"]
        )

    async def download_asset(self, request):
        return SmartCityRestApiDownloadAssetResponse(
            return_=list(b"asset:" + request.asset_path.encode()),
            content_type="text/plain",
            etag="etag-demo",
        )

    async def probe_lot(self, request):
        return SmartCityRestApiProbeLotResponse()

    async def reserve_lot(self, request):
        return SmartCityRestApiReserveLotResponse(
            return_=f"resv-{request.lot_id}",
            reservation_state="CONFIRMED",
            expires_at="2026-03-08T10:00:00Z",
        )

    async def cancel_reservation(self, request):
        return SmartCityRestApiCancelReservationResponse()

    async def get_profile(self, request):
        return SmartCityRestApiGetProfileResponse(
            return_=request.citizen_id,
            display_name="Taylor",
            phone_number="+1-555-0101",
            language="en-US",
        )

    async def update_profile(self, request):
        return SmartCityRestApiUpdateProfileResponse(audit_id="audit-20260307-001")

    async def get_device_status(self, request):
        return SmartCityRestApiGetDeviceStatusResponse(
            return_=f"device:{request.device_id}",
            trace_echo=request.trace_id,
            session_echo=request.session_id,
        )

    async def get_attribute_api_version(self, request):
        return SmartCityRestApiGetAttributeApiVersionResponse(value="v2.0.0")

    async def get_attribute_maintenance_mode(self, request):
        return SmartCityRestApiGetAttributeMaintenanceModeResponse(value=self.maintenance_mode)

    async def set_attribute_maintenance_mode(self, request):
        self.maintenance_mode = request.maintenance_mode
        return SmartCityRestApiSetAttributeMaintenanceModeResponse()


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, smart_city_rest_api_routes(MySmartCityRestApi()))


@app.get("/v1/assets/{asset_path:path}")
async def download_asset_fallback(asset_path: str):
    return JSONResponse(
        {
            "return": list(b"asset:" + asset_path.encode()),
            "content_type": "text/plain",
            "etag": "etag-demo",
        }
    )


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
