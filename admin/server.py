"""
Admin dashboard server for the Columbia Trading Competition.
Provides:
  - Web UI for managing the competition
  - WebSocket broadcast for timer synchronization
  - API endpoints for server/bot control, scoring, file uploads
  - Supervisord integration for process management
"""

import asyncio
import json
import os
import subprocess
import time
from pathlib import Path

import aiohttp
from aiohttp import web

MATCHING_ENGINE_URL = os.environ.get("MATCHING_ENGINE_URL", "http://127.0.0.1:8080")
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "9090"))
DATA_DIR = Path(os.environ.get("DATA_DIR", "/app/python_bots/data"))
CONFIG_PATH = Path(os.environ.get("CONFIG_PATH", "/app/matching-engine/config.json"))

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


# ── Supervisord helpers ──────────────────────────────────────────

def _supervisorctl(action: str, program: str) -> str:
    """Run a supervisorctl command and return output."""
    try:
        result = subprocess.run(
            ["supervisorctl", action, program],
            capture_output=True, text=True, timeout=10,
        )
        return result.stdout.strip() or result.stderr.strip()
    except FileNotFoundError:
        return f"supervisorctl not found (running outside container?)"
    except Exception as e:
        return str(e)


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
    """Get status of all managed processes."""
    output = _supervisorctl("status", "all")

    # Detect error messages (e.g. supervisorctl not found)
    if "not found" in output or "refused" in output or "no such file" in output.lower():
        return web.json_response({"ok": False, "error": output, "processes": {}})

    lines = [l.strip() for l in output.splitlines() if l.strip()]
    processes = {}
    for line in lines:
        parts = line.split()
        if len(parts) >= 2:
            processes[parts[0]] = parts[1]
    return web.json_response({"ok": True, "processes": processes})


async def api_restart_server(request):
    """Restart the matching engine."""
    result = _supervisorctl("restart", "matching-engine")
    return web.json_response({"ok": True, "message": result})


async def api_start_bot(request):
    """Start the price enforcer bot."""
    result = _supervisorctl("start", "price-enforcer")
    return web.json_response({"ok": True, "message": result})


async def api_stop_bot(request):
    """Stop the price enforcer bot."""
    result = _supervisorctl("stop", "price-enforcer")
    return web.json_response({"ok": True, "message": result})


async def api_restart_bot(request):
    """Restart the price enforcer bot."""
    result = _supervisorctl("restart", "price-enforcer")
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
