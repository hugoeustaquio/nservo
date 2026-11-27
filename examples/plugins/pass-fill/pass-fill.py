#!/usr/bin/env python3
"""Example nservo exec plugin: autofill login forms from rbw, pass, or op."""

import json
import os
import shutil
import subprocess
import sys
from urllib.parse import urlparse


def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def query_rbw(host):
    if not shutil.which("rbw"):
        return None
    proc = subprocess.run(["rbw", "get", "--full", host], capture_output=True, text=True)
    if proc.returncode != 0 or not proc.stdout.strip():
        return None
    lines = proc.stdout.splitlines()
    password = lines[0].strip()
    username = ""
    for line in lines[1:]:
        if line.lower().startswith("username:"):
            username = line.split(":", 1)[1].strip()
            break
    return (username, password, "rbw")


def query_pass(host):
    if not shutil.which("pass"):
        return None
    proc = subprocess.run(["pass", "show", host], capture_output=True, text=True)
    if proc.returncode != 0 or not proc.stdout.strip():
        return None
    lines = proc.stdout.splitlines()
    password = lines[0].strip()
    username = ""
    for line in lines[1:]:
        low = line.lower()
        if low.startswith(("username:", "user:", "login:", "email:")):
            username = line.split(":", 1)[1].strip()
            break
    return (username, password, "pass")


def query_op(host):
    if not shutil.which("op"):
        return None
    proc = subprocess.run(
        ["op", "item", "get", host, "--fields", "username,password", "--format", "json"],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        return None
    try:
        items = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None
    username = ""
    password = ""
    for item in items if isinstance(items, list) else []:
        fid = (item.get("id") or "").lower()
        val = item.get("value") or ""
        if fid == "username":
            username = val
        elif fid == "password":
            password = val
    return (username, password, "1Password") if password else None


def build_fill_js(username, password):
    u_json = json.dumps(username)
    p_json = json.dumps(password)
    return f"""(function() {{
  function setVal(el, val) {{
    if (!el || !val) return;
    var proto = Object.getPrototypeOf(el);
    var desc = proto && Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && desc.set) desc.set.call(el, val);
    else el.value = val;
    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  }}
  var pw = document.querySelector('input[type="password"]:not([disabled])');
  var user = document.querySelector(
    'input[type="email"]:not([disabled]), input[autocomplete="username"]:not([disabled]), ' +
    'input[name*="user" i]:not([type="hidden"]):not([disabled]), ' +
    'input[name*="login" i]:not([type="hidden"]):not([disabled]), ' +
    'input[type="text"]:not([disabled])'
  );
  setVal(user, {u_json});
  setVal(pw, {p_json});
}})();"""


def main():
    raw = sys.stdin.read().strip()
    ctx = json.loads(raw) if raw else {}
    url = ctx.get("url") or os.environ.get("NSERVO_URL", "")
    args = (ctx.get("args") or os.environ.get("NSERVO_ARGS", "")).strip()

    host = args or (urlparse(url).hostname or "")
    if host.startswith("www."):
        host = host[4:]
    if not host:
        emit({"op": "error", "text": "pass-fill: no hostname on active tab"})
        return

    result = query_rbw(host) or query_pass(host) or query_op(host)
    if not result:
        emit({"op": "error", "text": f"pass-fill: no entry found for '{host}' (checked rbw, pass, op)"})
        return

    username, password, backend = result
    emit({"op": "eval_js", "code": build_fill_js(username, password)})
    emit({"op": "notice", "text": f"Filled credentials for {host} ({backend})"})


if __name__ == "__main__":
    main()
