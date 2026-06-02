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
import json
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
    context.protocol = "rest"
    base_temp = os.path.join(os.getcwd(), "bdd", ".temp")
    os.makedirs(base_temp, exist_ok=True)
    context.temp_dir = tempfile.mkdtemp(dir=base_temp)
    context.lang_dir = os.path.join(context.temp_dir, "gen")
    os.makedirs(context.lang_dir)
    context.port = get_free_port()

@given('a JSON-RPC IDL file "{idl_file}"')
def step_impl(context, idl_file):
    context.idl_file = os.path.abspath(idl_file)
    context.protocol = "jsonrpc"
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
    if lang == "rust" and context.protocol == "rest": cmd_lang = "rust-axum"
    elif lang == "rust" and context.protocol == "jsonrpc": cmd_lang = "rust-jsonrpc"
    elif lang == "go": cmd_lang = "go-rest"
    elif lang == "python": cmd_lang = "python-rest"
    elif lang == "ts": cmd_lang = "typescript-rest"

    cmd = ["cargo", "run", "-p", "xidlc", "--features", "cli,fmt", "--", "gen", "-o", context.lang_dir, cmd_lang]
    if lang == "ts":
        cmd.append("--client")
    else:
        cmd.extend(["--client", "--server"])
    cmd.append(context.idl_file)
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=os.getcwd())
    if result.returncode != 0:
        print(f"Gen stdout: {result.stdout}")
        print(f"Gen stderr: {result.stderr}")
    assert result.returncode == 0

    if lang == "rust":
        # Merge duplicate pub mod declarations if any
        for f in os.listdir(context.lang_dir):
            if f.endswith(".rs"):
                path = os.path.join(context.lang_dir, f)
                with open(path, "r") as fr:
                    content = fr.read()
                mod_name = f[:-3]
                pattern = r"\}\s*(?:#\[allow\([^)]+\)\]\s*)?pub\s+mod\s+" + re.escape(mod_name) + r"\s*\{"
                if len(re.findall(pattern, content)) > 0:
                    content = re.sub(pattern, "", content)
                    first_pattern = r"pub\s+mod\s+" + re.escape(mod_name) + r"\s*\{"
                    content = re.sub(first_pattern, f"pub mod {mod_name} {{\n    use crate::{mod_name};", content, count=1)
                    with open(path, "w") as fw:
                        fw.write(content)

@then('the generated {lang} code should be valid')
def step_impl(context, lang):
    files = os.listdir(context.lang_dir)
    if lang == "go": assert any(f.endswith(".go") for f in files)
    elif lang == "python": assert any(f.endswith(".py") for f in files)
    elif lang == "rust": assert any(f.endswith(".rs") for f in files)
    elif lang == "ts":
        assert any(f.endswith(".ts") for f in files)
        with open(os.path.join(context.lang_dir, "package.json"), "w") as f:
            f.write('{"name": "test-ts-gen", "version": "1.0.0", "type": "module"}')
        subprocess.run(["pnpm", "add", "zod", "typescript"], cwd=context.lang_dir, check=True, capture_output=True)
        ts_files = [f for f in files if f.endswith(".ts")]
        result = subprocess.run(["pnpm", "exec", "tsc", "--noEmit"] + ts_files, cwd=context.lang_dir, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"tsc stdout: {result.stdout}")
            print(f"tsc stderr: {result.stderr}")
        assert result.returncode == 0

@then('the generated {lang} iface zod file should import the model schemas')
def step_impl(context, lang):
    files = os.listdir(context.lang_dir)
    found_iface_zod = False
    for f in files:
        if f.endswith(".iface.zod.ts"):
            found_iface_zod = True
            content = open(os.path.join(context.lang_dir, f)).read()
            assert re.search(r"import\s*\{[^}]+\}\s*from\s*[\"']\./", content) is not None
    assert found_iface_zod, f"iface.zod.ts file not found in {files}"

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

def wait_for_port(port, timeout=60):
    start_time = time.time()
    while time.time() - start_time < timeout:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=1):
                return True
        except (socket.error, ConnectionRefusedError):
            time.sleep(0.5)
    return False

def run_server_logging(process, prefix):
    for line in iter(process.stderr.readline, ''):
        if not line: break
        print(f"{prefix} LOG: {line.strip()}")

def get_module_name(lang_dir):
    files = os.listdir(lang_dir)
    for f in files:
        if f.endswith("_http.py"): return f[:-8]
        if f.endswith("_http.go"): return f[:-8]
        if f.endswith(".rs") and not f.startswith("mod") and "jsonrpc" not in f: return f[:-3]
        if f.endswith(".rs") and "jsonrpc" in f: return f[:-3]
    return "main"

@then('I can run the generated {lang} server and client')
def step_impl(context, lang):
    module_name = get_module_name(context.lang_dir)
    if lang == "python":
        python_path = os.environ.get("PYTHONPATH", "")
        new_python_path = os.pathsep.join([os.path.abspath("python"), context.lang_dir, python_path])
        context.env = os.environ.copy(); context.env["PYTHONPATH"] = new_python_path
        if "complex_rest" in context.idl_file:
            server_code = f"""
import asyncio, logging
from {module_name} import User
from {module_name}_http import *
from xidl.http import register_routes
from xidl.fastapi import FastAPIAdapter
from fastapi import FastAPI
import uvicorn
class MyUserService(UserServiceService):
    def __init__(self): self.users = {{}}
    async def get_user(self, request): return UserServiceGetUserResponse(value=self.users.get(request.id))
    async def create_user(self, request):
        user_id = request.user.id if hasattr(request.user, 'id') else request.user['id']
        self.users[user_id] = request.user
        return UserServiceCreateUserResponse(value=request.user)
    async def list_users(self, request): return UserServiceListUsersResponse(value=list(self.users.values()))
app = FastAPI(); adapter = FastAPIAdapter(app=app); register_routes(adapter, user_service_routes(MyUserService()))
if __name__ == '__main__': logging.basicConfig(level=logging.INFO); uvicorn.run(app, host='127.0.0.1', port={context.port}, log_level='info')
"""
        elif "all_scenarios" in context.idl_file:
            server_code = f"""
import asyncio, logging
from {module_name} import Status, Payload, Metadata
from {module_name}_http import *
from xidl.http import register_routes
from xidl.fastapi import FastAPIAdapter
from fastapi import FastAPI
import uvicorn

class MyAllScenarios(AllScenariosServiceService):
    def __init__(self):
        self.status = Status.ACTIVE
        self.items = {{}}

    async def get_item(self, request):
        return AllScenariosServiceGetItemResponse(value=f"Item {{request.id}} with {{request.filter}} and {{request.trace_id}}")

    async def create_item(self, request):
        self.items[len(self.items)] = request.name
        return AllScenariosServiceCreateItemResponse(value=42)

    async def update_item(self, request): return AllScenariosServiceUpdateItemResponse()
    async def delete_item(self, request): return AllScenariosServiceDeleteItemResponse()
    async def get_attribute_system_status(self, request): return AllScenariosServiceGetSystemStatusResponse(value=self.status)
    async def set_attribute_system_status(self, request):
        self.status = request.value
        return AllScenariosServiceSetSystemStatusResponse()
    async def get_attribute_version(self, request): return AllScenariosServiceGetVersionResponse(value="1.0.0")
    async def upload_form(self, request): return AllScenariosServiceUploadFormResponse()
    async def secure_data(self, request, xidl_auth=None): return AllScenariosServiceSecureDataResponse(value="Secret")

app = FastAPI()
adapter = FastAPIAdapter(app=app)
register_routes(adapter, all_scenarios_service_routes(MyAllScenarios()))
if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port={context.port})
"""
        elif "media_types" in context.idl_file:
            server_code = f"""
import asyncio, logging
from {module_name}_http import *
from xidl.http import register_routes
from xidl.fastapi import FastAPIAdapter
from fastapi import FastAPI
import uvicorn
class MyForm(FormServiceService):
    async def submit(self, request): return FormServiceSubmitResponse(value=f'Received {{request.name}} age {{request.age}}')
app = FastAPI(); adapter = FastAPIAdapter(app=app); register_routes(adapter, form_service_routes(MyForm()))
if __name__ == '__main__': uvicorn.run(app, host='127.0.0.1', port={context.port})
"""
        else:
            server_code = f"""
from {module_name}_http import *
from xidl.http import register_routes
from xidl.fastapi import FastAPIAdapter
from fastapi import FastAPI
import uvicorn
class MyHelloWorld(HelloWorldService):
    async def hello(self, request): return HelloWorldHelloResponse(value='Hello BDD')
    async def echo(self, request): return HelloWorldEchoResponse(value=request.msg)
app = FastAPI(); adapter = FastAPIAdapter(app=app); register_routes(adapter, hello_world_routes(MyHelloWorld()))
if __name__ == '__main__': uvicorn.run(app, host='127.0.0.1', port={context.port})
"""
        context.server_file = os.path.join(context.temp_dir, "server.py")
        with open(context.server_file, "w") as f: f.write(server_code)
        context.server_process = subprocess.Popen(["python3", "-u", context.server_file], env=context.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "PYTHON")); t.daemon = True; t.start()
        wait_for_port(context.port)
    elif lang == "go":
        go_mod = f"module test\ngo 1.25\nreplace github.com/xidl/xidl/golang/xidl-go-rest => {os.path.abspath('golang/xidl-go-rest')}\nreplace github.com/xidl/xidl/golang/xidl-go => {os.path.abspath('golang/xidl-go')}\nreplace github.com/xidl/xidl/golang/xidl-go-codec => {os.path.abspath('golang/xidl-go-codec')}\nrequire github.com/xidl/xidl/golang/xidl-go-rest v0.0.0\nrequire github.com/gin-gonic/gin v1.12.0\n"
        with open(os.path.join(context.lang_dir, "go.mod"), "w") as f: f.write(go_mod)
        if "complex_rest" in context.idl_file:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "sync"; "net/http"; "fmt")\ntype MyUserService struct {{ users sync.Map }}\nfunc (s *MyUserService) GetUser(ctx context.Context, req *UserServiceGetUserRequest) (*UserServiceGetUserResponse, error) {{\n\tval, ok := s.users.Load(req.Id); if !ok {{ return &UserServiceGetUserResponse{{}}, nil }}\n\treturn &UserServiceGetUserResponse{{Return: *val.(*User)}}, nil\n}}\nfunc (s *MyUserService) CreateUser(ctx context.Context, req *UserServiceCreateUserRequest) (*UserServiceCreateUserResponse, error) {{\n\ts.users.Store(req.User.Id, &req.User); return &UserServiceCreateUserResponse{{Return: req.User}}, nil\n}}\nfunc (s *MyUserService) ListUsers(ctx context.Context, req *UserServiceListUsersRequest) (*UserServiceListUsersResponse, error) {{\n\tvar users []User; s.users.Range(func(k, v interface{{}}) bool {{ users = append(users, *v.(*User)); return true }})\n\treturn &UserServiceListUsersResponse{{Return: users}}, nil\n}}\nfunc main() {{ r := gin.Default(); svc := &MyUserService{{}}; RegisterUserServiceHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        elif "all_scenarios" in context.idl_file:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "net/http"; "fmt")\ntype MyAllScenarios struct {{ }}\nfunc (s *MyAllScenarios) GetItem(ctx context.Context, req *AllScenariosServiceGetItemRequest) (*AllScenariosServiceGetItemResponse, error) {{ return &AllScenariosServiceGetItemResponse{{Return: fmt.Sprintf("Item %d with %s and %s", req.Id, req.Filter, req.TraceId)}}, nil }}\nfunc (s *MyAllScenarios) CreateItem(ctx context.Context, req *AllScenariosServiceCreateItemRequest) (*AllScenariosServiceCreateItemResponse, error) {{ return &AllScenariosServiceCreateItemResponse{{Return: 42}}, nil }}\nfunc (s *MyAllScenarios) UpdateItem(ctx context.Context, req *AllScenariosServiceUpdateItemRequest) (*AllScenariosServiceUpdateItemResponse, error) {{ return &AllScenariosServiceUpdateItemResponse{{}}, nil }}\nfunc (s *MyAllScenarios) DeleteItem(ctx context.Context, req *AllScenariosServiceDeleteItemRequest) (*AllScenariosServiceDeleteItemResponse, error) {{ return &AllScenariosServiceDeleteItemResponse{{}}, nil }}\nfunc (s *MyAllScenarios) UploadForm(ctx context.Context, req *AllScenariosServiceUploadFormRequest) (*AllScenariosServiceUploadFormResponse, error) {{ return &AllScenariosServiceUploadFormResponse{{}}, nil }}\nfunc (s *MyAllScenarios) SecureData(ctx context.Context, req *AllScenariosServiceSecureDataRequest) (*AllScenariosServiceSecureDataResponse, error) {{ return &AllScenariosServiceSecureDataResponse{{Return: "Secret"}}, nil }}\nfunc main() {{ r := gin.Default(); svc := &MyAllScenarios{{}}; RegisterAllScenariosServiceHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        elif "media_types" in context.idl_file:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "net/http"; "fmt")\ntype MyForm struct {{ }}\nfunc (s *MyForm) Submit(ctx context.Context, req *FormServiceSubmitRequest) (*FormServiceSubmitResponse, error) {{ return &FormServiceSubmitResponse{{Return: fmt.Sprintf("Received %s age %d", req.Name, req.Age)}}, nil }}\nfunc main() {{ r := gin.Default(); svc := &MyForm{{}}; RegisterFormServiceHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        elif "issue_171" in context.idl_file:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "net/http"; "fmt"; "errors")\ntype MyRepro struct{{}}\nfunc (s *MyRepro) FlattenAny(ctx context.Context, req *Issue171ReproServiceFlattenAnyRequest) (*Issue171ReproServiceFlattenAnyResponse, error) {{\n\tm, ok := req.Payload.(map[string]any)\n\tif !ok || m["foo"] != "bar" {{\n\t\treturn nil, errors.New("invalid payload")\n\t}}\n\treturn &Issue171ReproServiceFlattenAnyResponse{{}}, nil\n}}\nfunc (s *MyRepro) FlattenStructWithAny(ctx context.Context, req *Issue171ReproServiceFlattenStructWithAnyRequest) (*Issue171ReproServiceFlattenStructWithAnyResponse, error) {{\n\tm, ok := req.Payload.Field.(map[string]any)\n\tif !ok || m["foo"] != "bar" {{\n\t\treturn nil, errors.New("invalid payload")\n\t}}\n\treturn &Issue171ReproServiceFlattenStructWithAnyResponse{{}}, nil\n}}\nfunc main() {{ r := gin.Default(); svc := &MyRepro{{}}; RegisterIssue171ReproServiceHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        else:
            server_code = f'package main\nimport ("context"; "github.com/gin-gonic/gin"; "net/http"; "fmt")\ntype MyHelloWorld struct{{}}\nfunc (s *MyHelloWorld) Hello(ctx context.Context, req *HelloWorldHelloRequest) (*HelloWorldHelloResponse, error) {{ return &HelloWorldHelloResponse{{Return: "Hello BDD"}}, nil }}\nfunc (s *MyHelloWorld) Echo(ctx context.Context, req *HelloWorldEchoRequest) (*HelloWorldEchoResponse, error) {{ return &HelloWorldEchoResponse{{Return: req.Msg}}, nil }}\nfunc main() {{ r := gin.Default(); svc := &MyHelloWorld{{}}; RegisterHelloWorldHandler(r, svc); http.ListenAndServe(fmt.Sprintf(":%d", {context.port}), r) }}'
        with open(os.path.join(context.lang_dir, "server.go"), "w") as f: f.write(server_code)
        for f in os.listdir(context.lang_dir):
            if f.endswith(".go") and f != "server.go":
                path = os.path.join(context.lang_dir, f); content = open(path).read()
                content = re.sub(r"package \w+", "package main", content, count=1)
                with open(path, "w") as fw: fw.write(content)
        subprocess.run(["go", "mod", "tidy"], cwd=context.lang_dir, check=True)
        context.server_process = subprocess.Popen(["go", "run", "."], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "GO")); t.daemon = True; t.start()
        wait_for_port(context.port)
    elif lang == "rust":
        root_dir = os.path.abspath(".")
        if context.protocol == "rest":
            cargo_toml = f'[package]\nname = "test-rust-rest"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\nxidl-rust-axum = {{ path = "{os.path.join(root_dir, "xidl-rust-axum")}", features = ["stream"] }}\ntokio = {{ version = "1", features = ["full"] }}\nasync-trait = "0.1"\nserde = {{ version = "1", features = ["derive"] }}\nserde_json = "1"\naxum = "0.8"\nfutures-util = "0.3"\n'
            if "complex_rest" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyUserService {{ users: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u32, gen::User>>>, }}\n#[async_trait]\nimpl gen::UserService for MyUserService {{\n    async fn get_user<\'a>(&\'a self, id: u32) -> Result<gen::User, xidl_rust_axum::Error> {{\n        let users = self.users.lock().unwrap(); users.get(&id).cloned().ok_or(xidl_rust_axum::Error::not_found())\n    }}\n    async fn create_user<\'a>(&\'a self, user: gen::User) -> Result<gen::User, xidl_rust_axum::Error> {{\n        let mut users = self.users.lock().unwrap(); users.insert(user.id, user.clone()); Ok(user)\n    }}\n    async fn list_users<\'a>(&\'a self, _filter: String) -> Result<Vec<gen::User>, xidl_rust_axum::Error> {{\n        let users = self.users.lock().unwrap(); Ok(users.values().cloned().collect())\n    }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let svc = gen::UserServiceServer::new(MyUserService {{ users: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())), }});\n    xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(())\n}}'
            elif "all_scenarios" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyAllScenarios {{ status: std::sync::Mutex<gen::Status> }}\n#[async_trait]\nimpl gen::AllScenariosService for MyAllScenarios {{\n    async fn get_item<\'a>(&\'a self, id: u32, filter: String, trace_id: String) -> Result<String, xidl_rust_axum::Error> {{ Ok(format!("Item {{id}} with {{filter}} and {{trace_id}}")) }}\n    async fn create_item<\'a>(&\'a self, _name: String, _payload: gen::Payload) -> Result<u32, xidl_rust_axum::Error> {{ Ok(42) }}\n    async fn update_item<\'a>(&\'a self, _id: u32, _metadata: Vec<gen::Metadata>) -> Result<(), xidl_rust_axum::Error> {{ Ok(()) }}\n    async fn delete_item<\'a>(&\'a self, _id: u32) -> Result<(), xidl_rust_axum::Error> {{ Ok(()) }}\n    async fn get_attribute_system_status(&self) -> Result<gen::Status, xidl_rust_axum::Error> {{ Ok(*self.status.lock().unwrap()) }}\n    async fn set_attribute_system_status(&self, value: gen::Status) -> Result<(), xidl_rust_axum::Error> {{ *self.status.lock().unwrap() = value; Ok(()) }}\n    async fn get_attribute_version(&self) -> Result<String, xidl_rust_axum::Error> {{ Ok("1.0.0".into()) }}\n    async fn upload_form<\'a>(&\'a self, _key: String, _value: String) -> Result<(), xidl_rust_axum::Error> {{ Ok(()) }}\n    async fn secure_data<\'a>(&\'a self, _auth: xidl_rust_axum::auth::bearer::BearerAuth) -> Result<String, xidl_rust_axum::Error> {{ Ok("Secret".into()) }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let svc = gen::AllScenariosServiceServer::new(MyAllScenarios {{ status: std::sync::Mutex::new(gen::Status::ACTIVE) }});\n    xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(())\n}}'
            elif "streaming" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nuse futures_util::stream::StreamExt;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyStream;\n#[async_trait]\nimpl gen::StreamingService for MyStream {{\n    async fn ticks<\'a>(&\'a self, req: xidl_rust_axum::Request<gen::StreamingServiceTicksRequest>) -> Result<xidl_rust_axum::stream::SseStream<i32>, xidl_rust_axum::Error> {{\n        let count = req.data.count;\n        let s = futures_util::stream::iter(0..count).map(|i| Ok::<_, xidl_rust_axum::Error>(i));\n        Ok(xidl_rust_axum::stream::boxed_sse(s))\n    }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let svc = gen::StreamingServiceServer::new(MyStream);\n    xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(())\n}}'
            elif "media_types" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyForm;\n#[async_trait]\nimpl gen::FormService for MyForm {{\n    async fn submit<\'a>(&\'a self, name: String, age: i32) -> Result<String, xidl_rust_axum::Error> {{ Ok(format!("Received {{name}} age {{age}}")) }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let svc = gen::FormServiceServer::new(MyForm);\n    xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(())\n}}'
            elif "issue_171" in context.idl_file:
                server_code = f'use async_trait::async_trait;\npub mod issue_171 {{\n    pub use crate::gen::issue_171::*;\n}}\nmod gen {{\n    include!("../{module_name}.rs");\n}}\nstruct MyRepro;\n#[async_trait]\nimpl issue_171::ReproService for MyRepro {{\n    async fn flattenAny<\'a>(&\'a self, payload: xidl_rust_axum::serde_json::Value) -> Result<(), xidl_rust_axum::Error> {{\n        if payload.get("foo").and_then(|v| v.as_str()) == Some("bar") {{\n            Ok(())\n        }} else {{\n            Err(xidl_rust_axum::Error::bad_request())\n        }}\n    }}\n    async fn flattenStructWithAny<\'a>(&\'a self, payload: issue_171::StructWithAny) -> Result<(), xidl_rust_axum::Error> {{\n        if payload.field.get("foo").and_then(|v| v.as_str()) == Some("bar") {{\n            Ok(())\n        }} else {{\n            Err(xidl_rust_axum::Error::bad_request())\n        }}\n    }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let svc = issue_171::ReproServiceServer::new(MyRepro);\n    xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(())\n}}'
            else:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyHelloWorld;\n#[async_trait] impl gen::HelloWorldService for MyHelloWorld {{ async fn hello<\'a>(&\'a self) -> Result<String, xidl_rust_axum::Error> {{ Ok("Hello BDD".into()) }} async fn echo<\'a>(&\'a self, msg: String) -> Result<String, xidl_rust_axum::Error> {{ Ok(msg) }} }}\n#[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {{ let svc = gen::HelloWorldServer::new(MyHelloWorld); xidl_rust_axum::Server::builder().with_service(svc).serve("127.0.0.1:{context.port}").await?; Ok(()) }}'
            prefix = "RUST-REST"
        elif context.protocol == "jsonrpc":
            cargo_toml = f'[package]\nname = "test-rust-jsonrpc"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\nxidl-jsonrpc = {{ path = "{os.path.join(root_dir, "xidl-jsonrpc")}", features = ["transport-tcp"] }}\ntokio = {{ version = "1", features = ["full"] }}\nasync-trait = "0.1"\nserde = {{ version = "1", features = ["derive"] }}\nserde_json = "1"\n'
            if "city_jsonrpc" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MySmartCity;\n#[async_trait]\nimpl gen::SmartCityRpcApi for MySmartCity {{\n    async fn quote_trip<\'a>(&\'a self, _rider_id: String, _zone_id: String) -> Result<gen::SmartCityRpcApiquoteTripResult, xidl_jsonrpc::Error> {{ Ok(gen::SmartCityRpcApiquoteTripResult {{ amount_cents: 100, currency: "USD".into(), r#return: "quote-1".into() }}) }}\n    async fn create_invoice<\'a>(&\'a self, _rider_id: String, _amount_cents: i32, _currency: String) -> Result<gen::SmartCityRpcApicreateInvoiceResult, xidl_jsonrpc::Error> {{ Ok(gen::SmartCityRpcApicreateInvoiceResult {{ invoice_id: "inv-1".into(), payment_url: "http://pay".into(), r#return: "inv-1".into() }}) }}\n    async fn mark_paid<\'a>(&\'a self, _invoice_id: String) -> Result<(), xidl_jsonrpc::Error> {{ Ok(()) }}\n    async fn heartbeat<\'a>(&\'a self) -> Result<(), xidl_jsonrpc::Error> {{ Ok(()) }}\n    async fn rotate_session<\'a>(&\'a self, _session_token: String) -> Result<gen::SmartCityRpcApirotateSessionResult, xidl_jsonrpc::Error> {{ Ok(gen::SmartCityRpcApirotateSessionResult {{ session_token: "new-tok".into(), expires_at_epoch_sec: 3600 }}) }}\n    async fn dispatch<\'a>(&\'a self, _vehicle_id: String, _pickup_stop: String) -> Result<gen::SmartCityRpcApidispatchResult, xidl_jsonrpc::Error> {{ Ok(gen::SmartCityRpcApidispatchResult {{ job_id: "job-1".into(), r#return: 42 }}) }}\n    async fn report_trip<\'a>(&\'a self, _order_id: String, _rider_id: String, _status: String) -> Result<(), xidl_jsonrpc::Error> {{ Ok(()) }}\n    async fn summarize<\'a>(&\'a self, _day: String) -> Result<gen::SmartCityRpcApisummarizeResult, xidl_jsonrpc::Error> {{ Ok(gen::SmartCityRpcApisummarizeResult {{ trip_count: 10, gross_revenue_cents: 1000 }}) }}\n    async fn get_attribute_region<\'a>(&\'a self) -> Result<String, xidl_jsonrpc::Error> {{ Ok("us-east".into()) }}\n    async fn get_attribute_firmware_channel<\'a>(&\'a self) -> Result<String, xidl_jsonrpc::Error> {{ Ok("stable".into()) }}\n    async fn set_attribute_firmware_channel<\'a>(&\'a self, _value: String) -> Result<(), xidl_jsonrpc::Error> {{ Ok(()) }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let server = xidl_jsonrpc::Server::builder().with_service(gen::SmartCityRpcApiServer::new(MySmartCity)).with_endpoint("tcp://127.0.0.1:{context.port}").build().await?;\n    server.serve().await?; Ok(())\n}}'
            elif "multi_interface" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyMath;\n#[async_trait] impl gen::Math for MyMath {{ async fn add<\'a>(&\'a self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error> {{ Ok(a + b) }} }}\nstruct MyStore {{ last: std::sync::Mutex<String> }}\n#[async_trait] impl gen::Store for MyStore {{ async fn save<\'a>(&\'a self, value: String) -> Result<(), xidl_jsonrpc::Error> {{ *self.last.lock().unwrap() = value; Ok(()) }} async fn last_value<\'a>(&\'a self) -> Result<String, xidl_jsonrpc::Error> {{ Ok(self.last.lock().unwrap().clone()) }} }}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let server = xidl_jsonrpc::Server::builder()\n        .with_service(gen::MathServer::new(MyMath))\n        .with_service(gen::StoreServer::new(MyStore {{ last: std::sync::Mutex::new("".into()) }}))\n        .with_endpoint("tcp://127.0.0.1:{context.port}")\n        .build().await?;\n    server.serve().await?; Ok(())\n}}'
            elif "complex" in context.idl_file:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyCalculator;\n#[async_trait]\nimpl gen::Calculator for MyCalculator {{\n    async fn calculate<\'a>(&\'a self, req: gen::AddRequest, op: gen::Operation) -> Result<gen::AddResponse, xidl_jsonrpc::Error> {{\n        let result = match op {{ gen::Operation::ADD => req.a + req.b, gen::Operation::SUBTRACT => req.a - req.b }};\n        Ok(gen::AddResponse {{ result }})\n    }}\n    async fn get_history<\'a>(&\'a self) -> Result<Vec<i32>, xidl_jsonrpc::Error> {{ Ok(vec![]) }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let server = xidl_jsonrpc::Server::builder().with_service(gen::CalculatorServer::new(MyCalculator)).with_endpoint("tcp://127.0.0.1:{context.port}").build().await?;\n    server.serve().await?; Ok(())\n}}'
            else:
                server_code = f'use async_trait::async_trait;\nmod gen {{ include!("../{module_name}.rs"); }}\nstruct MyCalculator;\n#[async_trait]\nimpl gen::Calculator for MyCalculator {{\n    async fn add<\'a>(&\'a self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error> {{ Ok(a + b) }}\n    async fn subtract<\'a>(&\'a self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error> {{ Ok(a - b) }}\n}}\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let server = xidl_jsonrpc::Server::builder().with_service(gen::CalculatorServer::new(MyCalculator)).with_endpoint("tcp://127.0.0.1:{context.port}").build().await?;\n    server.serve().await?; Ok(())\n}}'
            prefix = "RUST-JSONRPC"

        with open(os.path.join(context.lang_dir, "Cargo.toml"), "w") as f: f.write(cargo_toml)
        os.makedirs(os.path.join(context.lang_dir, "src"), exist_ok=True)
        with open(os.path.join(context.lang_dir, "src", "main.rs"), "w") as f: f.write(server_code)

        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = os.path.join(root_dir, "bdd", ".temp", "rust_target")
        context.server_process = subprocess.Popen(["cargo", "run", "--offline"], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, prefix)); t.daemon = True; t.start()
        if not wait_for_port(context.port):
            if context.server_process.poll() is not None:
                stdout, stderr = context.server_process.communicate()
                assert False, f"Server failed to start:\n{stderr}"
            assert False, f"Timed out waiting for port {context.port}"

@then('the client can call math.add({a:d}, {b:d}) to get {expected:d}')
def step_impl(context, a, b, expected):
    import socket, json
    def call_rpc(method, params):
        s = socket.create_connection(("127.0.0.1", context.port))
        s.sendall((json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}) + "\n").encode())
        res = s.recv(4096).decode(); s.close(); return json.loads(res)
    res = call_rpc("Math.add", {"a": a, "b": b})
    val = res["result"].get("value") or res["result"].get("return") or res["result"]
    assert val == expected

@then('the client can save "{value}" to store and get it back')
def step_impl(context, value):
    import socket, json
    def call_rpc(method, params):
        s = socket.create_connection(("127.0.0.1", context.port))
        s.sendall((json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}) + "\n").encode())
        res = s.recv(4096).decode(); s.close(); return json.loads(res)
    call_rpc("Store.save", {"value": value})
    res = call_rpc("Store.last_value", {})
    val = res["result"].get("value") or res["result"].get("return") or res["result"]
    assert val == value

@then('the client can create a user with name "{name}" and id {user_id:d}')
def step_impl(context, name, user_id):
    resp = requests.post(f"http://127.0.0.1:{context.port}/", json={"user": {"id": user_id, "name": name, "roles": ["admin"]}}, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"

@then('the client can get the user with id {user_id:d} and see name "{name}"')
def step_impl(context, user_id, name):
    resp = requests.get(f"http://127.0.0.1:{context.port}/{user_id}", timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
    data = resp.json(); user_data = data.get("value") or data.get("return") or data
    assert user_data["name"] == name

@then('the client can set and get the "{attr}" attribute to "{value}"')
def step_impl(context, attr, value):
    if context.protocol == "jsonrpc":
        import socket, json
        def call_rpc(method, params):
            s = socket.create_connection(("127.0.0.1", context.port))
            s.sendall((json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}) + "\n").encode())
            res = s.recv(4096).decode(); s.close(); return json.loads(res)
        call_rpc(f"SmartCityRpcApi.set_attribute_{attr}", {"value": value})
        res = call_rpc(f"SmartCityRpcApi.get_attribute_{attr}", {})
        inner = res["result"].get("value") or res["result"].get("return") or res["result"]
        assert str(inner) == value
    else:
        url = f"http://127.0.0.1:{context.port}/{attr}"
        if context.lang == "rust": url = f"http://127.0.0.1:{context.port}/attribute/{attr}"
        if context.lang == "rust" and attr == "system_status":
             resp = requests.post(url, json={"value": value}, timeout=10)
        else:
             resp = requests.put(url, json={"value": value}, timeout=10)
        resp = requests.get(url, timeout=10)
        data = resp.json(); res = data.get("value") or data.get("return") or data if isinstance(data, dict) else data
        assert str(res) == value or (isinstance(res, dict) and str(res.get("value")) == value)

@then('the client can create an item with name "{name}" and payload message "{msg}"')
def step_impl(context, name, msg):
    url = f"http://127.0.0.1:{context.port}/items"
    if context.lang == "rust": payload = {"tag": "ACTIVE", "data": msg}
    elif context.lang == "go": payload = {"status": 0, "message": msg}
    else: payload = {"status": "ACTIVE", "message": msg}
    resp = requests.post(url, json={"name": name, "payload": payload}, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"

@then('the client can get item {id:d} with filter "{filter_str}" and trace id "{trace_id}"')
def step_impl(context, id, filter_str, trace_id):
    url = f"http://127.0.0.1:{context.port}/items/{id}?filter={filter_str}"
    resp = requests.get(url, headers={"X-Trace-Id": trace_id}, timeout=10)
    assert resp.status_code == 200, f"Status: {resp.status_code}, Body: {resp.text}"
    data = resp.json(); res = data.get("value") or data.get("return") or data if isinstance(data, dict) else data
    assert f"Item {id}" in str(res); assert filter_str in str(res); assert trace_id in str(res)

@then('the client can call calculate({a:d}, {b:d}, {op}) to get {expected:d}')
def step_impl(context, a, b, op, expected):
    import socket, json
    def call_rpc(method, params):
        s = socket.create_connection(("127.0.0.1", context.port))
        s.sendall((json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}) + "\n").encode())
        response = ""
        while True:
            chunk = s.recv(4096).decode(); response += chunk
            if not chunk or "\n" in chunk: break
        s.close(); return json.loads(response) if response else {}
    params = {"req": {"a": a, "b": b}, "op": op}
    res = call_rpc("Calculator.calculate", params)
    val = res["result"].get("value") or res["result"].get("return")
    assert val["result"] == expected

@then('the client can receive {count:d} ticks from the stream')
def step_impl(context, count):
    url = f"http://127.0.0.1:{context.port}/ticks?count={count}"
    resp = requests.get(url, stream=True, timeout=10)
    if resp.status_code == 404:
        url = f"http://127.0.0.1:{context.port}/stream/ticks?count={count}"
        resp = requests.get(url, stream=True, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
    received = 0
    for line in resp.iter_lines():
        if line:
            raw = line.decode()
            if raw.startswith('data:'):
                received += 1
    assert received == count

@then('the client can submit a form with name "{name}" and age {age:d}')
def step_impl(context, name, age):
    url = f"http://127.0.0.1:{context.port}/submit"
    resp = requests.post(url, data={"name": name, "age": age}, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
    try:
        data = resp.json()
        res = data.get("value") or data.get("return") or data if isinstance(data, dict) else data
    except:
        res = resp.text
    assert name in str(res); assert str(age) in str(res)

@then('the client can send flatten any payload with key "{key}" and value "{value}"')
def step_impl(context, key, value):
    url = f"http://127.0.0.1:{context.port}/flatten-any"
    resp = requests.post(url, json={key: value}, timeout=10)
    assert resp.status_code in (200, 204), f"Got {resp.status_code}: {resp.text}"

@then('the client can send flatten struct with any field payload with key "{key}" and value "{value}"')
def step_impl(context, key, value):
    url = f"http://127.0.0.1:{context.port}/flatten-struct-with-any"
    resp = requests.post(url, json={"field": {key: value}}, timeout=10)
    assert resp.status_code in (200, 204), f"Got {resp.status_code}: {resp.text}"
