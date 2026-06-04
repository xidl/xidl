import os
import uvicorn
from fastapi import FastAPI
from xidl.fastapi import FastAPIAdapter, register_routes
from complex_rest import User
from complex_rest_http import (
    UserServiceService,
    UserServiceGetUserRequest,
    UserServiceGetUserResponse,
    UserServiceCreateUserRequest,
    UserServiceCreateUserResponse,
    UserServiceListUsersRequest,
    UserServiceListUsersResponse,
    user_service_routes,
)

class MyUserService(UserServiceService):
    def __init__(self):
        self.users = {}

    async def get_user(self, request: UserServiceGetUserRequest) -> UserServiceGetUserResponse:
        user = self.users.get(request.id)
        if not user:
            from xidl.http import HttpError
            raise HttpError(404, "NOT_FOUND", "user not found")
        return UserServiceGetUserResponse(value=user)

    async def create_user(self, request: UserServiceCreateUserRequest) -> UserServiceCreateUserResponse:
        self.users[request.user.id] = request.user
        return UserServiceCreateUserResponse(value=request.user)

    async def list_users(self, request: UserServiceListUsersRequest) -> UserServiceListUsersResponse:
        filtered = [
            u for u in self.users.values()
            if not request.filter or request.filter in u.name or any(request.filter in r for r in u.roles)
        ]
        return UserServiceListUsersResponse(value=filtered)

app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, user_service_routes(MyUserService()))

if __name__ == "__main__":
    port = int(os.getenv("PORT", 8080))
    print(f"Python server starting on port {port}")
    uvicorn.run(app, host="127.0.0.1", port=port)
