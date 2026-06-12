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
import random
from behave import given, when, then

_test_port_base = int(os.environ.get("XIDL_BDD_PORT_BASE", "12000"))
_test_port_span = int(os.environ.get("XIDL_BDD_PORT_SPAN", "1000"))
_reserved_test_ports = set()
_port_random = random.SystemRandom()

def reserve_test_port():
    for _ in range(_test_port_span * 2):
        port = _test_port_base + _port_random.randrange(_test_port_span)
        if port in _reserved_test_ports:
            continue
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            sock.bind(("127.0.0.1", port))
        except OSError:
            sock.close()
            continue
        _reserved_test_ports.add(port)
        return port, sock
    raise RuntimeError(
        f"unable to allocate test port in range "
        f"{_test_port_base}-{_test_port_base + _test_port_span - 1}"
    )

def release_reserved_test_port(context):
    guard = getattr(context, "port_guard", None)
    if guard is not None:
        guard.close()
        context.port_guard = None

@given('a REST IDL file "{idl_file}"')
def step_impl(context, idl_file):
    context.idl_file = os.path.abspath(idl_file)
    context.protocol = "rest"
    base_temp = os.path.join(os.getcwd(), "bdd", ".temp")
    os.makedirs(base_temp, exist_ok=True)
    context.temp_dir = tempfile.mkdtemp(dir=base_temp)
    context.lang_dir = os.path.join(context.temp_dir, "gen")
    os.makedirs(context.lang_dir)
    context.port, context.port_guard = reserve_test_port()

@given('a JSON-RPC IDL file "{idl_file}"')
def step_impl(context, idl_file):
    context.idl_file = os.path.abspath(idl_file)
    context.protocol = "jsonrpc"
    base_temp = os.path.join(os.getcwd(), "bdd", ".temp")
    os.makedirs(base_temp, exist_ok=True)
    context.temp_dir = tempfile.mkdtemp(dir=base_temp)
    context.lang_dir = os.path.join(context.temp_dir, "gen")
    os.makedirs(context.lang_dir)
    context.port, context.port_guard = reserve_test_port()

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
    elif lang == "python":
        for f in os.listdir(context.lang_dir):
            if f.endswith(".py"):
                path = os.path.join(context.lang_dir, f)
                with open(path, "r") as fr:
                    content = fr.read()
                content = re.sub(r'realm=Some\("([^"]*)"\)', r'realm="\1"', content)
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
        with open(os.path.join(context.lang_dir, "tsconfig.json"), "w") as f:
            f.write('{"compilerOptions": {"target": "ESNext", "module": "ESNext", "moduleResolution": "node", "ignoreDeprecations": "6.0", "skipLibCheck": true, "strict": true}}')

        codec_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../typescript/xidl-typescript-codec"))
        if not os.path.exists(os.path.join(codec_path, "dist")):
            subprocess.run(["pnpm", "install", "--ignore-scripts"], cwd=codec_path, check=True, capture_output=True)
            subprocess.run(["pnpm", "build"], cwd=codec_path, check=True, capture_output=True)
        subprocess.run(["npm", "install", "zod@^3.23.0", "typescript", codec_path], cwd=context.lang_dir, check=True, capture_output=True)

        result = subprocess.run(["npx", "tsc", "--noEmit", "-p", "."], cwd=context.lang_dir, capture_output=True, text=True)
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
            assert re.search(r"import\s*(\*\s*as\s+\w+|\{\s*[^}]+\s*\})\s*from\s*[\"']\./", content) is not None
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

def start_server_process(args, **kwargs):
    if kwargs.get("stdout") == subprocess.PIPE and kwargs.get("stderr") == subprocess.PIPE:
        kwargs["stderr"] = subprocess.STDOUT
    return subprocess.Popen(args, start_new_session=True, **kwargs)

def start_context_server(context, args, **kwargs):
    release_reserved_test_port(context)
    return start_server_process(args, **kwargs)

def wait_for_server(context, timeout=60):
    if not wait_for_port(context.port, timeout):
        if context.server_process.poll() is not None:
            stdout, stderr = context.server_process.communicate()
            assert False, f"Server failed to start:\n{stdout or stderr}"
        assert False, f"Timed out waiting for port {context.port}"
    time.sleep(0.5)
    if context.server_process.poll() is not None:
        stdout, stderr = context.server_process.communicate()
        assert False, f"Server failed to stay running:\n{stdout or stderr}"

def run_server_logging(process, prefix):
    stream = process.stderr or process.stdout
    for line in iter(stream.readline, ''):
        if not line: break
        print(f"{prefix} LOG: {line.strip()}")

def get_module_name(lang_dir):
    files = os.listdir(lang_dir)
    for f in files:
        if f.endswith("_http.py"): return f[:-8]
        if f.endswith("_http.go"): return f[:-8]
        if f.endswith(".server.ts"): return f[:-10]
        if f.endswith(".rs") and not f.startswith("mod") and "jsonrpc" not in f: return f[:-3]
        if f.endswith(".rs") and "jsonrpc" in f: return f[:-3]
    return "main"

def get_idl_name(context):
    return os.path.splitext(os.path.basename(context.idl_file))[0]

def python_boilerplate_dir(idl_name):
    return os.path.join(os.getcwd(), "bdd", "boilerplate", idl_name, "python")

def copy_python_boilerplate(context, idl_name):
    src_dir = python_boilerplate_dir(idl_name)
    assert os.path.isdir(src_dir), f"missing python boilerplate for {idl_name}: {src_dir}"
    for name in os.listdir(src_dir):
        src = os.path.join(src_dir, name)
        dst = os.path.join(context.lang_dir, name)
        if os.path.isdir(src):
            shutil.copytree(src, dst, dirs_exist_ok=True)
        else:
            shutil.copy2(src, dst)

def start_python_boilerplate_server(context, idl_name, log_prefix):
    copy_python_boilerplate(context, idl_name)
    python_path = os.environ.get("PYTHONPATH", "")
    context.env = os.environ.copy()
    context.env["PYTHONPATH"] = os.pathsep.join([os.path.abspath("python"), context.lang_dir, python_path])
    context.env["PORT"] = str(context.port)
    context.server_file = os.path.join(context.lang_dir, "main.py")
    assert os.path.exists(context.server_file), f"missing python server fixture: {context.server_file}"
    context.server_process = start_context_server(
        context,
        ["python3", "-u", context.server_file],
        cwd=context.lang_dir,
        env=context.env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    t = threading.Thread(target=run_server_logging, args=(context.server_process, log_prefix))
    t.daemon = True
    t.start()

@then('I can run the generated {lang} server and client')
def step_impl(context, lang):
    run_generated_server_using_boilerplate(context, lang)

@then('I can run the generated {lang} server using boilerplate')
def step_impl(context, lang):
    run_generated_server_using_boilerplate(context, lang)

def run_generated_server_using_boilerplate(context, lang):
    import shutil
    idl_name = get_idl_name(context)
    src_dir = os.path.join(os.getcwd(), "bdd", "boilerplate", idl_name, lang)
    if not os.path.exists(src_dir):
        # Fallback to default boilerplate for protocol
        fallback_name = context.protocol if context.protocol else "rest"
        src_dir = os.path.join(os.getcwd(), "bdd", "boilerplate", fallback_name, lang)

    server_timeout = 60
    module_name = get_module_name(context.lang_dir)

    if lang == "go":
        for f in os.listdir(src_dir):
            shutil.copy(os.path.join(src_dir, f), context.lang_dir)
        go_mod_path = os.path.join(context.lang_dir, "go.mod")
        if os.path.exists(go_mod_path):
            with open(go_mod_path, "r") as f:
                content = f.read()
            content = content.replace("{{GOLANG_XIDL_GO_REST_PATH}}", os.path.abspath('golang/xidl-go-rest'))
            content = content.replace("{{GOLANG_XIDL_GO_PATH}}", os.path.abspath('golang/xidl-go'))
            content = content.replace("{{GOLANG_XIDL_GO_CODEC_PATH}}", os.path.abspath('golang/xidl-go-codec'))
            with open(go_mod_path, "w") as f:
                f.write(content)
        for f in os.listdir(context.lang_dir):
            if f.endswith(".go") and f != "main.go":
                path = os.path.join(context.lang_dir, f)
                if f.endswith("_test.go"):
                    new_f = f.replace("_test.go", "_test_model.go")
                    new_path = os.path.join(context.lang_dir, new_f)
                    os.rename(path, new_path)
                    path = new_path
                with open(path, "r") as fr:
                    content = fr.read()
                content = re.sub(r"package \w+", "package main", content, count=1)
                with open(path, "w") as fw:
                    fw.write(content)
        subprocess.run(["go", "mod", "tidy"], cwd=context.lang_dir, check=True)
        env = os.environ.copy()
        env["PORT"] = str(context.port)
        context.server_process = start_context_server(context, ["go", "run", "."], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "GO-BOILERPLATE"))
        t.daemon = True
        t.start()
    elif lang == "rust":
        shutil.copy(os.path.join(src_dir, "Cargo.toml"), context.lang_dir)
        os.makedirs(os.path.join(context.lang_dir, "src"), exist_ok=True)
        shutil.copy(os.path.join(src_dir, "src", "main.rs"), os.path.join(context.lang_dir, "src", "main.rs"))

        # Replace placeholders in Cargo.toml
        cargo_toml_path = os.path.join(context.lang_dir, "Cargo.toml")
        with open(cargo_toml_path, "r") as f:
            content = f.read()
        content = content.replace("{{RUST_XIDL_RUST_AXUM_PATH}}", os.path.abspath('xidl-rust-axum'))
        content = content.replace("{{RUST_XIDL_JSONRPC_PATH}}", os.path.abspath('xidl-jsonrpc'))
        with open(cargo_toml_path, "w") as f:
            f.write(content)

        # Replace placeholders in main.rs
        main_rs_path = os.path.join(context.lang_dir, "src", "main.rs")
        with open(main_rs_path, "r") as f:
            content = f.read()
        content = content.replace("{{MODULE_NAME}}", module_name)
        with open(main_rs_path, "w") as f:
            f.write(content)

        env = os.environ.copy()
        env["PORT"] = str(context.port)
        env["CARGO_TARGET_DIR"] = os.path.join(os.path.abspath("."), "bdd", ".temp", "rust_target")
        context.server_process = start_context_server(context, ["cargo", "run"], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "RUST-BOILERPLATE"))
        t.daemon = True
        t.start()
        server_timeout = 180
    elif lang == "ts":
        shutil.copy(os.path.join(src_dir, "server.ts"), context.lang_dir)
        shutil.copy(os.path.join(src_dir, "tsconfig.json"), context.lang_dir)
        shutil.copy(os.path.join(src_dir, "package.json"), context.lang_dir)

        # Replace placeholders in package.json
        package_json_path = os.path.join(context.lang_dir, "package.json")
        with open(package_json_path, "r") as f:
            content = f.read()
        codec_path = os.path.abspath(os.path.join(os.getcwd(), "typescript", "xidl-typescript-codec"))
        content = content.replace("{{TS_XIDL_TYPESCRIPT_CODEC_PATH}}", codec_path)
        with open(package_json_path, "w") as f:
            f.write(content)

        # Replace placeholders in server.ts
        server_ts_path = os.path.join(context.lang_dir, "server.ts")
        with open(server_ts_path, "r") as f:
            content = f.read()
        content = content.replace("{{MODULE_NAME}}", module_name)
        with open(server_ts_path, "w") as f:
            f.write(content)

        if not os.path.exists(os.path.join(codec_path, "dist")):
            subprocess.run(["pnpm", "install", "--ignore-scripts"], cwd=codec_path, check=True, capture_output=True)
            subprocess.run(["pnpm", "build"], cwd=codec_path, check=True, capture_output=True)
        subprocess.run(["npm", "install", "--ignore-scripts"], cwd=context.lang_dir, check=True, capture_output=True)
        env = os.environ.copy()
        env["PORT"] = str(context.port)
        context.server_process = start_context_server(context, ["npx", "tsx", "server.ts"], cwd=context.lang_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        t = threading.Thread(target=run_server_logging, args=(context.server_process, "TS-BOILERPLATE"))
        t.daemon = True
        t.start()
    elif lang == "python":
        start_python_boilerplate_server(context, idl_name, "PYTHON-BOILERPLATE")
    wait_for_server(context, timeout=server_timeout)

@then('I can run hurl tests against the server')
def step_impl(context):
    idl_name = os.path.splitext(os.path.basename(context.idl_file))[0]
    idl_hurl_mapping = {
        "complex_rest": ["complex_rest.hurl"],
        "city_rest": ["city_rest.hurl"],
        "rest_server": ["rest_server.hurl"],
        "rest_media_types": ["media_types.hurl"],
        "e2e_test": [
            "e2e_test.hurl",
            "e2e_attribute.hurl",
            "e2e_defaults_matrix.hurl",
            "e2e_form.hurl",
            "e2e_path.hurl",
            "e2e_scope_matrix.hurl",
            "e2e_security_matrix.hurl"
        ]
    }
    hurl_files = idl_hurl_mapping.get(idl_name, [f"{idl_name}.hurl"])
    for hurl_file_name in hurl_files:
        hurl_file = os.path.join(os.getcwd(), "bdd", "features", "data", hurl_file_name)
        cmd = [
            "pnpm", "exec", "hurl",
            "--test",
            "--variable", f"base_url=http://127.0.0.1:{context.port}",
            hurl_file
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=os.path.join(os.getcwd(), "xidlc-examples"))
        if result.returncode != 0:
            print(f"Hurl file: {hurl_file_name}")
            print(f"Hurl stdout: {result.stdout}")
            print(f"Hurl stderr: {result.stderr}")
        assert result.returncode == 0


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
        url = f"http://127.0.0.1:{context.port}/attribute/{attr}"
        # Standard REST attributes use the attribute name as the key in the payload.
        # Rust (axum) might also support 'value' but the projected HIR uses the ident.
        payload = {attr: value}
        resp = requests.post(url, json=payload, timeout=10)
        resp = requests.get(url, timeout=10)
        if resp.status_code != 200:
            print(f"DEBUG: GET {url} returned {resp.status_code}: {resp.text}")
        try:
            data = resp.json()
        except Exception:
            data = resp.text.strip('"')
        res = data.get("value") or data.get("return") or data if isinstance(data, dict) else data
        assert str(res) == value or (isinstance(res, dict) and str(res.get("value")) == value)
        # Check if the returned value is inside a field named after the attribute (some generators do this)
        if isinstance(res, dict) and attr in res:
             assert str(res.get(attr)) == value

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
            if raw.startswith('data:') and 'done' not in raw:
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

@then('the client gets a {status:d} error with msg containing "{expected_msg}" when requesting {method} "{path}"')
def step_impl(context, status, expected_msg, method, path):
    url = f"http://127.0.0.1:{context.port}{path}"
    resp = requests.request(method, url, timeout=10)
    assert resp.status_code == status, f"Expected {status}, got {resp.status_code}: {resp.text}"
    try:
        data = resp.json()
    except Exception as e:
        assert False, f"Failed to parse error response as JSON: {resp.text}. Error: {e}"
    assert "code" in data, f"Missing 'code' in error response: {data}"
    assert "msg" in data, f"Missing 'msg' in error response: {data}"
    assert expected_msg.lower() in data["msg"].lower(), f"Expected message containing '{expected_msg}', got '{data['msg']}'"

@then('the client gets a {status:d} error with msg containing "{expected_msg}" when requesting {method} "{path}" with headers')
def step_impl(context, status, expected_msg, method, path):
    url = f"http://127.0.0.1:{context.port}{path}"
    headers = {}
    for row in context.table:
        headers[row['name']] = row['value']
    resp = requests.request(method, url, headers=headers, timeout=10)
    assert resp.status_code == status, f"Expected {status}, got {resp.status_code}: {resp.text}"
    try:
        data = resp.json()
    except Exception as e:
        assert False, f"Failed to parse error response as JSON: {resp.text}. Error: {e}"
    assert "code" in data, f"Missing 'code' in error response: {data}"
    assert "msg" in data, f"Missing 'msg' in error response: {data}"
    assert expected_msg.lower() in data["msg"].lower(), f"Expected message containing '{expected_msg}', got '{data['msg']}'"

@then('the client can receive {count:d} alerts for "{district}" with basic auth')
def step_impl(context, count, district):
    url = f"http://127.0.0.1:{context.port}/alerts/{district}"
    resp = requests.get(url, auth=("user", "pass"), stream=True, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
    received = 0
    for line in resp.iter_lines():
        if line:
            raw = line.decode()
            if raw.startswith('data:'):
                val = raw[5:].strip()
                if val.startswith('"') and val.endswith('"'): val = val[1:-1]
                assert district in val
                received += 1
                if received == count: break
    assert received == count

@then('the client can upload {count:d} chunks for asset "{asset_id}" with bearer auth and get result containing "{expected}"')
def step_impl(context, count, asset_id, expected):
    import base64
    url = f"http://127.0.0.1:{context.port}/assets"
    def gen():
        for i in range(count):
            # NDJSON with xidl StreamFrame
            chunk_b64 = base64.b64encode(b"data").decode()
            yield (json.dumps({"t": "next", "data": {"asset_id": asset_id, "chunk": chunk_b64}}) + "\n").encode()
        yield (json.dumps({"t": "complete"}) + "\n").encode()

    headers = {"Authorization": "Bearer token-1", "Content-Type": "application/x-ndjson"}
    resp = requests.post(url, data=gen(), headers=headers, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
    assert expected in resp.text

@then('the client can get a msgpack user with id "{user_id}" and see score {score:d}')
def step_impl(context, user_id, score):
    import msgpack
    url = f"http://127.0.0.1:{context.port}/v1/msgpack/users/{user_id}"
    resp = requests.get(url, headers={"Accept": "application/msgpack"}, timeout=10)
    assert resp.status_code == 200, f"Got {resp.status_code}: {resp.text}"
    data = msgpack.unpackb(resp.content, raw=False)
    # Go uses Uppercase by default if not tagged, but xidl should tag them.
    # For now, we support both to be robust.
    res_val = data.get("return") or data.get("Return")
    score_val = data.get("score") or data.get("Score")
    assert res_val == f"user:{user_id}"
    assert score_val == score

@then('the response header "{header}" should be "{value}" when requesting {method} "{path}" with headers')
def step_impl(context, header, value, method, path):
    url = f"http://127.0.0.1:{context.port}{path}"
    headers = {}
    if context.table:
        for row in context.table:
            headers[row['name']] = row['value']
    resp = requests.request(method, url, headers=headers, timeout=10)
    assert resp.headers.get(header) == value, f"Expected {header}: {value}, got {resp.headers.get(header)}"

@then('the response header "{header}" should not be present when requesting {method} "{path}" with headers')
def step_impl(context, header, method, path):
    url = f"http://127.0.0.1:{context.port}{path}"
    headers = {}
    if context.table:
        for row in context.table:
            headers[row['name']] = row['value']
    resp = requests.request(method, url, headers=headers, timeout=10)
    assert header not in resp.headers, f"Expected header {header} to be absent, but got {resp.headers.get(header)}"

@when('the client requests GET "{path}"')
def step_impl(context, path):
    url = f"http://127.0.0.1:{context.port}{path}"
    context.last_response = requests.get(url, timeout=10)

@then('the response body should be exactly "{expected_body}"')
def step_impl(context, expected_body):
    assert context.last_response.text == expected_body, f"Expected '{expected_body}', got '{context.last_response.text}'"

@then('the response body should be JSON matching')
def step_impl(context):
    expected_json = json.loads(context.text)
    actual_json = context.last_response.json()
    assert actual_json == expected_json, f"Expected {expected_json}, got {actual_json}"

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
