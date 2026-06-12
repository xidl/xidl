import os

import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter
from xidl.http import HttpError, register_routes

from complex_rest_http import *


class MyUserService(UserServiceService):
    def __init__(self):
        self.users = {}

    async def get_user(self, request):
        user = self.users.get(request.id)
        if user is None:
            raise HttpError(404, 404, "user not found")
        return UserServiceGetUserResponse(value=user)

    async def create_user(self, request):
        user_id = request.user.get("id") if isinstance(request.user, dict) else request.user.id
        self.users[user_id] = request.user
        return UserServiceCreateUserResponse(value=request.user)

    async def list_users(self, request):
        users = [
            user
            for user in self.users.values()
            if not request.filter
            or request.filter in user.get("name", "")
            or request.filter in user.get("roles", [])
        ]
        return UserServiceListUsersResponse(value=users)


app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, user_service_routes(MyUserService()))


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(os.environ["PORT"]))
