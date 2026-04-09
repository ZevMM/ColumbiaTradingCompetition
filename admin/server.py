"""
Admin dashboard server for the Columbia Trading Competition.
Provides:
  - Web UI for managing the competition
  - WebSocket broadcast for timer synchronization
  - API endpoints for server/bot control, scoring, file uploads
  - Docker API integration for process management
"""

import asyncio
import json
import os
import time
from pathlib import Path

import aiohttp
from aiohttp import web

MATCHING_ENGINE_URL = os.environ.get("MATCHING_ENGINE_URL", "http://127.0.0.1:8080")
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "9090"))
DATA_DIR = Path(os.environ.get("DATA_DIR", "/app/python_bots/data"))
CONFIG_PATH = Path(os.environ.get("CONFIG_PATH", "/app/matching-engine/config.json"))
DOCKER_SOCKET = os.environ.get("DOCKER_SOCKET", "/var/run/docker.sock")
COMPOSE_PROJECT = os.environ.get("COMPOSE_PROJECT", "")  # auto-detected if empty

# Connected WebSocket clients (timer displays + admin dashboards)
ws_clients: set[web.WebSocketResponse] = set()

# Timer state
timer_state = {
    "running": False,
    "paused": False,
    "remaining": 0,       # seconds remaining
    "duration": 0,        # total duration set
    "start_epoch": 0,     # when the timer was started (epoch)
}


async def broadcast(msg: dict):
    """Send a JSON message to all connected WebSocket clients."""
    data = json.dumps(msg)
    dead = set()
    for ws in ws_clients:
        try:
            await ws.send_str(data)
        except Exception:
            dead.add(ws)
    ws_clients.difference_update(dead)


async def timer_tick():
    """Background task that counts down and broadcasts every second."""
    while True:
        await asyncio.sleep(1)
        if timer_state["running"] and not timer_state["paused"]:
            elapsed = time.time() - timer_state["start_epoch"]
            timer_state["remaining"] = max(0, timer_state["duration"] - int(elapsed))
            await broadcast({
                "type": "timer_tick",
                "remaining": timer_state["remaining"],
                "running": True,
                "paused": False,
            })
            if timer_state["remaining"] <= 0:
                timer_state["running"] = False
                # Auto end game when timer hits 0
                await _proxy_to_engine("end_game")
                await broadcast({"type": "game_ended", "remaining": 0, "running": False, "paused": False})


# ── Docker API helpers ──────────────────────────────────────────

def _docker_available() -> bool:
    """Check if the Docker socket is mounted."""
    return os.path.exists(DOCKER_SOCKET)


async def _docker_api(method: str, path: str, **kwargs) -> tuple[int, dict | str]:
    """Make a request to the Docker Engine API via Unix socket."""
    try:
        conn = aiohttp.UnixConnector(path=DOCKER_SOCKET)
        async with aiohttp.ClientSession(connector=conn) as session:
            url = f"http://localhost{path}"
            async with session.request(method, url, timeout=aiohttp.ClientTimeout(total=30), **kwargs) as resp:
                try:
                    body = await resp.json()
                except Exception:
                    body = await resp.text()
                return resp.status, body
    except Exception as e:
        return 500, str(e)


async def _find_compose_project() -> str:
    """Auto-detect the compose project name from our own container's labels."""
    global COMPOSE_PROJECT
    if COMPOSE_PROJECT:
        return COMPOSE_PROJECT

    # Read our own container ID from cgroup
    try:
        cgroup = Path("/proc/self/cgroup").read_text()
        for line in cgroup.splitlines():
            if "docker" in line:
                COMPOSE_PROJECT = ""  # will use label filter below
                break
    except Exception:
        pass

    # Find containers with com.docker.compose.project label that include 'admin'
    status, data = await _docker_api("GET", "/containers/json?all=true")
    if status == 200 and isinstance(data, list):
        for container in data:
            labels = container.get("Labels", {})
            service = labels.get("com.docker.compose.service", "")
            if service == "admin":
                COMPOSE_PROJECT = labels.get("com.docker.compose.project", "")
                return COMPOSE_PROJECT

    return COMPOSE_PROJECT


async def _get_compose_containers() -> dict[str, dict]:
    """Get all containers in the same compose project, keyed by service name."""
    project = await _find_compose_project()
    filters = json.dumps({"label": [f"com.docker.compose.project={project}"]}) if project else "{}"
    status, data = await _docker_api("GET", f"/containers/json?all=true&filters={filters}")

    services = {}
    if status == 200 and isinstance(data, list):
        for container in data:
            labels = container.get("Labels", {})
            service = labels.get("com.docker.compose.service", "")
            if service:
                state = container.get("State", "unknown")
                services[service] = {
                    "id": container["Id"][:12],
                    "status": container.get("Status", ""),
                    "state": state.upper(),
                }
    return services


async def _docker_container_action(service_name: str, action: str) -> str:
    """Start/stop/restart a compose service container by service name."""
    containers = await _get_compose_containers()
    info = containers.get(service_name)
    if not info:
        return f"Container for service '{service_name}' not found"

    container_id = info["id"]
    status, body = await _docker_api("POST", f"/containers/{container_id}/{action}")
    if status in (204, 304):
        return f"{service_name} {action} successful"
    elif isinstance(body, dict) and "message" in body:
        return body["message"]
    return str(body) if body else f"{service_name} {action} completed"


async def _proxy_to_engine(endpoint: str) -> tuple[int, str]:
    """Forward a request to the matching engine."""
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{MATCHING_ENGINE_URL}/{endpoint}", timeout=aiohttp.ClientTimeout(total=5)) as resp:
                body = await resp.text()
                return resp.status, body
    except Exception as e:
        return 500, str(e)


# ── WebSocket handler ────────────────────────────────────────────

async def websocket_handler(request):
    ws = web.WebSocketResponse()
    await ws.prepare(request)
    ws_clients.add(ws)

    # Send current state on connect
    await ws.send_str(json.dumps({
        "type": "timer_state",
        **timer_state,
    }))

    try:
        async for msg in ws:
            if msg.type == aiohttp.WSMsgType.TEXT:
                pass  # Timer displays are read-only
            elif msg.type == aiohttp.WSMsgType.ERROR:
                break
    finally:
        ws_clients.discard(ws)
    return ws


# ── API routes ───────────────────────────────────────────────────

async def api_start_game(request):
    """Start the game with a timer duration (minutes)."""
    try:
        body = await request.json()
        minutes = float(body.get("minutes", 10))
    except Exception:
        minutes = 10

    status, result = await _proxy_to_engine("start_game")
    if status != 200:
        return web.json_response({"ok": False, "error": result}, status=status)

    timer_state["running"] = True
    timer_state["paused"] = False
    timer_state["duration"] = int(minutes * 60)
    timer_state["remaining"] = int(minutes * 60)
    timer_state["start_epoch"] = time.time()

    await broadcast({
        "type": "game_started",
        "remaining": timer_state["remaining"],
        "duration": timer_state["duration"],
        "running": True,
        "paused": False,
    })
    return web.json_response({"ok": True, "message": result})


async def api_pause_game(request):
    """Pause/unpause the game."""
    if not timer_state["running"]:
        return web.json_response({"ok": False, "error": "Game not running"}, status=400)

    if timer_state["paused"]:
        # Unpause
        status, result = await _proxy_to_engine("start_game")
        if status != 200:
            return web.json_response({"ok": False, "error": result}, status=status)
        timer_state["paused"] = False
        # Recalculate start_epoch so remaining time is preserved
        timer_state["start_epoch"] = time.time() - (timer_state["duration"] - timer_state["remaining"])
        msg_type = "game_resumed"
    else:
        # Pause
        status, result = await _proxy_to_engine("end_game")
        if status != 200:
            return web.json_response({"ok": False, "error": result}, status=status)
        timer_state["paused"] = True
        msg_type = "game_paused"

    await broadcast({
        "type": msg_type,
        "remaining": timer_state["remaining"],
        "running": True,
        "paused": timer_state["paused"],
    })
    return web.json_response({"ok": True, "message": result})


async def api_end_game(request):
    """End the game immediately."""
    status, result = await _proxy_to_engine("end_game")
    timer_state["running"] = False
    timer_state["paused"] = False
    timer_state["remaining"] = 0

    await broadcast({
        "type": "game_ended",
        "remaining": 0,
        "running": False,
        "paused": False,
    })
    return web.json_response({"ok": True, "message": result})


async def api_tally_score(request):
    """Proxy score tally to matching engine."""
    status, result = await _proxy_to_engine("tally_score")
    try:
        data = json.loads(result)
    except Exception:
        data = result
    return web.json_response({"ok": status == 200, "data": data}, status=status)


async def api_server_status(request):
    """Get status of all compose service containers."""
    if not _docker_available():
        return web.json_response({
            "ok": False,
            "error": "Docker socket not available. Mount /var/run/docker.sock into the admin container.",
            "processes": {},
        })

    containers = await _get_compose_containers()
    if not containers:
        return web.json_response({
            "ok": False,
            "error": "No compose containers found. Check Docker socket permissions.",
            "processes": {},
        })

    processes = {name: info["state"] for name, info in containers.items()}
    return web.json_response({"ok": True, "processes": processes})


async def api_restart_server(request):
    """Restart the matching engine container."""
    if not _docker_available():
        return web.json_response({"ok": False, "error": "Docker socket not available"}, status=500)
    result = await _docker_container_action("matching-engine", "restart")
    return web.json_response({"ok": True, "message": result})


async def api_start_bot(request):
    """Start the price enforcer bot container."""
    if not _docker_available():
        return web.json_response({"ok": False, "error": "Docker socket not available"}, status=500)
    result = await _docker_container_action("price-enforcer", "start")
    return web.json_response({"ok": True, "message": result})


async def api_stop_bot(request):
    """Stop the price enforcer bot container."""
    if not _docker_available():
        return web.json_response({"ok": False, "error": "Docker socket not available"}, status=500)
    result = await _docker_container_action("price-enforcer", "stop")
    return web.json_response({"ok": True, "message": result})


async def api_restart_bot(request):
    """Restart the price enforcer bot container."""
    if not _docker_available():
        return web.json_response({"ok": False, "error": "Docker socket not available"}, status=500)
    result = await _docker_container_action("price-enforcer", "restart")
    return web.json_response({"ok": True, "message": result})


async def api_upload_config(request):
    """Upload a new config.json for the matching engine."""
    reader = await request.multipart()
    field = await reader.next()
    if field is None:
        return web.json_response({"ok": False, "error": "No file uploaded"}, status=400)

    data = await field.read(decode=False)
    # Validate JSON
    try:
        json.loads(data)
    except json.JSONDecodeError as e:
        return web.json_response({"ok": False, "error": f"Invalid JSON: {e}"}, status=400)

    CONFIG_PATH.write_bytes(data)
    return web.json_response({"ok": True, "message": f"Config uploaded to {CONFIG_PATH}"})


async def api_upload_data(request):
    """Upload price data files for the bots."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    reader = await request.multipart()
    uploaded = []
    while True:
        field = await reader.next()
        if field is None:
            break
        filename = field.filename
        if not filename:
            continue
        # Sanitize filename
        safe_name = Path(filename).name
        dest = DATA_DIR / safe_name
        data = await field.read(decode=False)
        dest.write_bytes(data)
        uploaded.append(safe_name)

    if not uploaded:
        return web.json_response({"ok": False, "error": "No files uploaded"}, status=400)
    return web.json_response({"ok": True, "files": uploaded})


async def api_get_config(request):
    """Return the current config.json."""
    try:
        data = json.loads(CONFIG_PATH.read_text())
        return web.json_response({"ok": True, "config": data})
    except Exception as e:
        return web.json_response({"ok": False, "error": str(e)}, status=500)


async def api_health(request):
    """Health check for admin server."""
    engine_status, _ = await _proxy_to_engine("health")
    return web.json_response({
        "admin": "ok",
        "matching_engine": "ok" if engine_status == 200 else "down",
    })


# ── Static file serving ─────────────────────────────────────────

async def index(request):
    return web.FileResponse(Path(__file__).parent / "static" / "index.html")


# ── App setup ────────────────────────────────────────────────────

async def on_startup(app):
    app["timer_task"] = asyncio.create_task(timer_tick())


async def on_cleanup(app):
    app["timer_task"].cancel()


def create_app():
    app = web.Application(client_max_size=50 * 1024 * 1024)  # 50MB upload limit
    app.on_startup.append(on_startup)
    app.on_cleanup.append(on_cleanup)

    # WebSocket
    app.router.add_get("/ws", websocket_handler)

    # API routes
    app.router.add_post("/api/start", api_start_game)
    app.router.add_post("/api/pause", api_pause_game)
    app.router.add_post("/api/end", api_end_game)
    app.router.add_get("/api/score", api_tally_score)
    app.router.add_get("/api/status", api_server_status)
    app.router.add_post("/api/restart-server", api_restart_server)
    app.router.add_post("/api/start-bot", api_start_bot)
    app.router.add_post("/api/stop-bot", api_stop_bot)
    app.router.add_post("/api/restart-bot", api_restart_bot)
    app.router.add_post("/api/upload/config", api_upload_config)
    app.router.add_post("/api/upload/data", api_upload_data)
    app.router.add_get("/api/config", api_get_config)
    app.router.add_get("/api/health", api_health)

    # Static dashboard files
    static_dir = Path(__file__).parent / "static"
    app.router.add_get("/", index)
    app.router.add_static("/static/", static_dir, name="static")

    return app


if __name__ == "__main__":
    web.run_app(create_app(), host="0.0.0.0", port=ADMIN_PORT)
