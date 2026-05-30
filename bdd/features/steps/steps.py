import subprocess
import os
import shutil
import tempfile
import time
import requests
import signal
import sys
import threading
import re
import socket
from behave import given, when, then

def get_free_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(('', 0))
    port = s.getsockname()[1]
    s.close()
    return port

@given('a REST IDL file "{idl_file}"')
def step_impl(context, idl_file):
    context.idl_file = os.path.abspath(idl_file)
    base_temp = os.path.join(os.getcwd(), "bdd", ".temp")
    os.makedirs(base_temp, exist_ok=True)
    context.temp_dir = tempfile.mkdtemp(dir=base_temp)
    context.lang_dir = os.path.join(context.temp_dir, "gen")
    os.makedirs(context.lang_dir)
    context.port = get_free_port()

@given('a JSON-RPC IDL file "{idl_file}"')
def step_impl(context, idl_file):
    context.idl_file = os.path.abspath(idl_file)
    base_temp = os.path.join(os.getcwd(), "bdd", ".temp")
    os.makedirs(base_temp, exist_ok=True)
    context.temp_dir = tempfile.mkdtemp(dir=base_temp)
    context.lang_dir = os.path.join(context.temp_dir, "gen")
    os.makedirs(context.lang_dir)
    context.port = get_free_port()

@when('I generate {lang} code for the IDL')
def step_impl(context, lang):
    context.lang = lang
    cmd_lang = lang
    if lang == "rust" and "rest" in context.idl_file: cmd_lang = "rust-axum"
    elif lang == "rust" and "jsonrpc" in context.idl_file: cmd_lang = "rust-jsonrpc"
    elif lang == "go": cmd_lang = "go-rest"
    elif lang == "python": cmd_lang = "python-rest"
    cmd = ["cargo", "run", "-p", "xidlc", "--features", "cli,fmt", "--", "gen", "-o", context.lang_dir, cmd_lang, "--client", "--server", context.idl_file]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=os.getcwd())
    if result.returncode != 0: print(result.stdout); print(result.stderr)
    assert result.returncode == 0

@then('the generated {lang} code should be valid')
def step_impl(context, lang):
    files = os.listdir(context.lang_dir)
    if lang == "go": assert any(f.endswith(".go") for f in files)
    elif lang == "python": assert any(f.endswith(".py") for f in files)
    elif lang == "rust": assert any(f.endswith(".rs") for f in files)

@then('the generated {lang} code should contain correct User struct and UserService interface')
def step_impl(context, lang):
    files = os.listdir(context.lang_dir)
    found_struct = False; found_interface = False
    for f in files:
        if (lang == "go" and f.endswith(".go")) or (lang == "python" and f.endswith(".py")) or (lang == "rust" and f.endswith(".rs")):
            content = open(os.path.join(context.lang_dir, f)).read()
            struct_marker = "type User struct" if lang == "go" else ("class User" if lang == "python" else "struct User")
            interface_marker = "type UserServiceService interface" if lang == "go" else ("class UserServiceService" if lang == "python" else "trait UserService")
            if struct_marker in content: found_struct = True
            if interface_marker in content: found_interface = True
    assert found_struct, f"User struct not found in {files}"; assert found_interface, f"UserService interface not found in {files}"

@then('the generated {lang} code should contain correct AddRequest struct and Calculator interface')
def step_impl(context, lang):
    files = os.listdir(context.lang_dir)
    found_struct = False; found_interface = False
    for f in files:
        if f.endswith(".rs"):
            content = open(os.path.join(context.lang_dir, f)).read()
            if "struct AddRequest" in content: found_struct = True
            if "trait Calculator" in content: found_interface = True
    assert found_struct, f"AddRequest struct not found in {files}"; assert found_interface, f"Calculator interface not found in {files}"

def run_server_logging(process, prefix):
    for line in iter(process.stderr.readline, ''):
        if not line: break
        print(f"{prefix} LOG: {line.strip()}")

@then('I can run the generated {lang} server and client')
def step_impl(context, lang):
    files = os.listdir(context.lang_dir)
    module_name = ""
    for f in files:
        if f.endswith("_http.py"): module_name = f[:-8]; break
        if f.endswith("_http.go"): module_name = f[:-8]; break
        if f.endswith(".rs") and not f.startswith("mod"): module_name = f[:-3]; break
    if not module_name: module_name = "main"

    if lang == "python":
        python_path = os.environ.get("PYTHONPATH", "")
        new_python_path = os.pathsep.join([os.path.abspath("python"), context.lang_dir, python_path])
        context.env = os.environ.copy(); context.env["PYTHONPATH"] = new_python_path
        if "complex_rest" in context.idl_file:
            server_code = f"import asyncio, logging\nfrom {module_name} import User\nfrom {module_name}_http import (UserServiceService, UserServiceGetUserRequest, UserServiceGetUserResponse, UserServiceCreateUserRequest, UserServiceCreateUserResponse, UserServiceListUsersRequest, UserServiceListUsersResponse, user_service_routes)\nfrom xidl.http import register_routes\nfrom xidl.fastapi import FastAPIAdapter\nfrom fastapi import FastAPI\nimport uvicorn\nclass MyUserService(UserServiceService):\n    def __init__(self): self.users = {{}}\n    async def get_user(self, request): return UserServiceGetUserResponse(value=self.users.get(request.id))\n    async def create_user(self, request):\n        user_id = request.user.id if hasattr(request.user, 'id') else request.user['id']\n        self.users[user_id] = request.user\n        return UserServiceCreateUserResponse(value=request.user)\n    async def list_users(self, request): return UserServiceListUsersResponse(value=list(self.users.values()))\napp = FastAPI(); adapter = FastAPIAdapter(app=app); register_routes(adapter, user_service_routes(MyUserService()))\nif __name__ == '__main__': logging.basicConfig(level=logging.INFO); uvicorn.run(app, host='127.0.0.1', port={context.port}, log_level='info')"
        else:
            server_code = f"from {module_name}_http import HelloWorldService, HelloWorldHelloResponse, HelloWorldEchoResponse, hello_world_routes\nfrom xidl.http import register_routes\nfrom xidl.fastapi import FastAPIAdapter\nfrom fastapi import FastAPI\nimport uvicorn\nclass MyHelloWorld(HelloWorldService):\n    async def hello(self, request): return HelloWorldHelloResponse(value='Hello BDD')\n    async def echo(self, request): return HelloWorldEchoResponse(value=request.msg)\napp = FastAPI(); adapter = FastAPIAdapter(app=app); register_routes(adapter, hello_world_routes(MyHelloWorld()))\nif __name__ == '__main__': uvicorn.run(app, host='127.0.0.1', port={context.port})"
        context.server_file = os.path.join(context.temp_dir, "server.py")
        with open(context.server_file, "w") as f: f.write(server_code)
        context.server_process = subprocess.Popen(["python3", "-u", context.server_file], env=context.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "PYTHON")); t.daemon = True; t.start()
        time.sleep(3)
    elif lang == "go":
        go_mod = f"module test\ngo 1.25\nreplace github.com/xidl/xidl/golang/xidl-go-rest => {os.path.abspath('golang/xidl-go-rest')}\nreplace github.com/xidl/xidl/golang/xidl-go => {os.path.abspath('golang/xidl-go')}\nrequire github.com/xidl/xidl/golang/xidl-go-rest v0.0.0\nrequire github.com/gin-gonic/gin v1.12.0\n"
        with open(os.path.join(context.lang_dir, "go.mod"), "w") as f: f.write(go_mod)
        if "complex_rest" in context.idl_file:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "sync"; "net/http"; "fmt")\ntype MyUserService struct {{ users sync.Map }}\nfunc (s *MyUserService) GetUser(ctx context.Context, req *UserServiceGetUserRequest) (*UserServiceGetUserResponse, error) {{\n\tval, ok := s.users.Load(req.Id); if !ok {{ return &UserServiceGetUserResponse{{}}, nil }}\n\treturn &UserServiceGetUserResponse{{Return: *val.(*User)}}, nil\n}}\nfunc (s *MyUserService) CreateUser(ctx context.Context, req *UserServiceCreateUserRequest) (*UserServiceCreateUserResponse, error) {{\n\ts.users.Store(req.User.Id, &req.User); return &UserServiceCreateUserResponse{{Return: req.User}}, nil\n}}\nfunc (s *MyUserService) ListUsers(ctx context.Context, req *UserServiceListUsersRequest) (*UserServiceListUsersResponse, error) {{\n\tvar users []User; s.users.Range(func(k, v interface{{}}) bool {{ users = append(users, *v.(*User)); return true }})\n\treturn &UserServiceListUsersResponse{{Return: users}}, nil\n}}\nfunc main() {{ r := gin.Default(); svc := &MyUserService{{}}; RegisterUserServiceHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        else:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "net/http"; "fmt")\ntype MyHelloWorld struct{{}}\nfunc (s *MyHelloWorld) Hello(ctx context.Context, req *HelloWorldHelloRequest) (*HelloWorldHelloResponse, error) {{\n\treturn &HelloWorldHelloResponse{{Return: "Hello BDD"}}, nil\n}}\nfunc (s *MyHelloWorld) Echo(ctx context.Context, req *HelloWorldEchoRequest) (*HelloWorldEchoResponse, error) {{\n\treturn &HelloWorldEchoResponse{{Return: req.Msg}}, nil\n}}\nfunc main() {{ r := gin.Default(); svc := &MyHelloWorld{{}}; RegisterHelloWorldHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        with open(os.path.join(context.lang_dir, "server.go"), "w") as f: f.write(server_code)
        for f in os.listdir(context.lang_dir):
            if f.endswith(".go") and f != "server.go":
                path = os.path.join(context.lang_dir, f); content = open(path).read()
                content = re.sub(r"package \w+", "package main", content, count=1)
                with open(path, "w") as fw: fw.write(content)
        subprocess.run(["go", "mod", "tidy"], cwd=context.lang_dir, check=True)
        context.server_process = subprocess.Popen(["go", "run", "."], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "GO")); t.daemon = True; t.start()
        time.sleep(10)
    elif lang == "rust":
        root_dir = os.path.abspath(".")
        if "rest" in context.idl_file:
            cargo_toml = f'[package]\nname = "test-rust-rest"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\nxidl-rust-axum = {{ path = "{os.path.join(root_dir, "xidl-rust-axum")}" }}\ntokio = {{ version = "1", features = ["full"] }}\nasync-trait = "0.1"\nserde = {{ version = "1", features = ["derive"] }}\nserde_json = "1"\naxum = "0.8"\n'
            with open(os.path.join(context.lang_dir, "Cargo.toml"), "w") as f: f.write(cargo_toml)
            os.makedirs(os.path.join(context.lang_dir, "src"), exist_ok=True)
            server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyUserService {{ users: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u32, gen::User>>>, }}\n#[async_trait]\nimpl gen::UserService for MyUserService {{\n    async fn get_user<\'a>(&\'a self, id: u32) -> Result<gen::User, xidl_rust_axum::Error> {{\n        let users = self.users.lock().unwrap(); users.get(&id).cloned().ok_or(xidl_rust_axum::Error::not_found())\n    }}\n    async fn create_user<\'a>(&\'a self, user: gen::User) -> Result<gen::User, xidl_rust_axum::Error> {{\n        let mut users = self.users.lock().unwrap(); users.insert(user.id, user.clone()); Ok(user)\n    }}\n    async fn list_users<\'a>(&\'a self, _filter: String) -> Result<Vec<gen::User>, xidl_rust_axum::Error> {{\n        let users = self.users.lock().unwrap(); Ok(users.values().cloned().collect())\n    }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let svc = gen::UserServiceServer::new(MyUserService {{ users: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())), }});\n    xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(())\n}}'
            with open(os.path.join(context.lang_dir, "src", "main.rs"), "w") as f: f.write(server_code)
            context.server_process = subprocess.Popen(["cargo", "run", "--offline"], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            t = threading.Thread(target=run_server_logging, args=(context.server_process, "RUST-REST")); t.daemon = True; t.start()
            time.sleep(40)
        elif "jsonrpc" in context.idl_file:
            cargo_toml = f'[package]\nname = "test-rust-jsonrpc"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\nxidl-jsonrpc = {{ path = "{os.path.join(root_dir, "xidl-jsonrpc")}", features = ["transport-tcp"] }}\ntokio = {{ version = "1", features = ["full"] }}\nasync-trait = "0.1"\nserde = {{ version = "1", features = ["derive"] }}\nserde_json = "1"\n'
            with open(os.path.join(context.lang_dir, "Cargo.toml"), "w") as f: f.write(cargo_toml)
            os.makedirs(os.path.join(context.lang_dir, "src"), exist_ok=True)
            server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyCalculator;\n#[async_trait]\nimpl gen::Calculator for MyCalculator {{\n    async fn calculate<\'a>(&\'a self, req: gen::AddRequest, op: gen::Operation) -> Result<gen::AddResponse, xidl_jsonrpc::Error> {{\n        let result = match op {{ gen::Operation::ADD => req.a + req.b, gen::Operation::SUBTRACT => req.a - req.b }};\n        Ok(gen::AddResponse {{ result }})\n    }}\n    async fn get_history<\'a>(&\'a self) -> Result<Vec<i32>, xidl_jsonrpc::Error> {{ Ok(vec![]) }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let server = xidl_jsonrpc::Server::builder().with_service(gen::CalculatorServer::new(MyCalculator)).with_endpoint("tcp://127.0.0.1:{context.port}").build().await?;\n    server.serve().await?; Ok(())\n}}'
            with open(os.path.join(context.lang_dir, "src", "main.rs"), "w") as f: f.write(server_code)
            context.server_process = subprocess.Popen(["cargo", "run", "--offline"], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            t = threading.Thread(target=run_server_logging, args=(context.server_process, "RUST-JSONRPC")); t.daemon = True; t.start()
            time.sleep(40)

@then('the client can create a user with name "{name}" and id {user_id:d}')
def step_impl(context, name, user_id):
    if context.lang in ["python", "go", "rust"]:
        resp = requests.post(f"http://127.0.0.1:{context.port}/", json={"user": {"id": user_id, "name": name, "roles": ["admin"]}}, timeout=10)
        assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"

@then('the client can get the user with id {user_id:d} and see name "{name}"')
def step_impl(context, user_id, name):
    if context.lang in ["python", "go", "rust"]:
        resp = requests.get(f"http://127.0.0.1:{context.port}/{user_id}", timeout=10)
        assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
        data = resp.json(); user_data = data.get("value") or data.get("return") or data
        assert user_data["name"] == name

@then('the client can call calculate({a:d}, {b:d}, {op}) to get {expected:d}')
def step_impl(context, a, b, op, expected):
    if context.lang == "rust" and "jsonrpc" in context.idl_file:
        import socket, json
        def call_rpc(method, params):
            s = socket.create_connection(("127.0.0.1", context.port))
            request = {"jsonrpc": "2.0", "method": method, "params": params, "id": 1}
            s.sendall((json.dumps(request) + "\n").encode())
            response = ""; 
            while True:
                chunk = s.recv(4096).decode()
                if not chunk: break
                response += chunk
                if "\n" in chunk: break
            s.close()
            return json.loads(response) if response else {}
        params = {"req": {"a": a, "b": b}, "op": op}; res = call_rpc("Calculator.calculate", params)
        assert "result" in res, f"RPC error: {res}"; assert res["result"]["return"]["result"] == expected
