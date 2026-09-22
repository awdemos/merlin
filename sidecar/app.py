"""Merlin feature-extraction sidecar.

Exposes laya (multilingual non-autoregressive decision engine) prompt
classification over HTTP so the Merlin router can fetch semantic features
and guardrail verdicts per incoming prompt.

Run:
    python app.py                 # 127.0.0.1:8765, preloads english+multilingual at startup
    python app.py --port 9000 --host 0.0.0.0
    python app.py --no-preload    # lazy-load checkpoints on first request instead
    python app.py --preload-all   # also preload the typed-decisions checkpoint
"""
import argparse
import json
import threading
from contextlib import asynccontextmanager
from typing import Any, Dict, Optional
from urllib.parse import parse_qs

import laya
import uvicorn
from fastapi import FastAPI, HTTPException, Request

_router: Optional[laya.Router] = None
_router_lock = threading.Lock()
_preload = True  # flipped by the CLI before startup
_preload_all = False  # --preload-all: also load the typed-decisions checkpoint

_last_model: Optional[str] = None

# Documented request body for OpenAPI; the actual parsing happens in the
# middleware below so that bare `curl -d '{...}'` (form content-type) works too.
_PROMPT_BODY = {
    "requestBody": {
        "required": True,
        "content": {
            "application/json": {
                "schema": {
                    "type": "object",
                    "properties": {"prompt": {"type": "string"}},
                    "required": ["prompt"],
                }
            }
        },
    }
}


def get_router() -> laya.Router:
    """The shared module-level Router; built on first use (startup or first request)."""
    global _router
    if _router is None:
        with _router_lock:
            if _router is None:
                router = laya.Router(preload=False)
                if _preload:
                    # These endpoints only ever route to english/multilingual
                    # (typed-decisions is opt-in via auto_task_detection), so the
                    # two served checkpoints are what we keep resident.
                    router.preload() if _preload_all else router.preload(["english", "multilingual"])
                _router = router
    return _router


@asynccontextmanager
async def lifespan(_app: FastAPI):
    get_router()  # build (and preload) checkpoints before serving traffic
    yield


app = FastAPI(title="merlin-sidecar", version="0.1.0", lifespan=lifespan)


@app.middleware("http")
async def parse_prompt_body(request: Request, call_next):
    """Extract `prompt` from the POST body regardless of the declared content type.

    Clients that send `Content-Type: application/json` work as usual; clients that
    POST raw JSON without the header (e.g. bare `curl -d '{...}'`, which defaults
    to form content-type) are handled by parsing the raw body here. The parsed
    value lands in request.state.prompt for the (synchronous) handlers.
    """
    if request.method == "POST":
        prompt: Optional[str] = None
        try:
            raw = await request.body()
            text = raw.decode("utf-8", "replace").strip()
            data: Optional[Dict[str, Any]] = None
            try:
                parsed = json.loads(text)
                if isinstance(parsed, dict):
                    data = parsed
            except json.JSONDecodeError:
                form = parse_qs(text)
                if form.get("prompt"):
                    data = {"prompt": form["prompt"][0]}
            if data is not None:
                candidate = data.get("prompt")
                if isinstance(candidate, str):
                    prompt = candidate
        except Exception:
            prompt = None
        request.scope.setdefault("state", {})
        request.scope["state"]["prompt"] = prompt
    return await call_next(request)


def _prompt_from(request: Request) -> str:
    prompt = getattr(request.state, "prompt", None)
    if isinstance(prompt, str):
        prompt = prompt.strip()
    if not prompt:
        raise HTTPException(status_code=422, detail="prompt must be a non-empty string")
    return prompt


def _track_model(result: Dict[str, Any]) -> None:
    global _last_model
    routing = result.get("routing") or {}
    if routing.get("model"):
        _last_model = routing["model"]


def _normalized_score(answer: Dict[str, Any]) -> Optional[float]:
    """Normalize a laya `score` answer to 0..1.

    laya returns the expected level over `len(criteria)` ordinal levels
    numbered 0..k-1, so the maximum is len(legend) - 1.
    """
    if not answer:
        return None
    score = answer.get("score")
    n_levels = len(answer.get("legend") or {})
    if score is None or n_levels < 2:
        return None
    value = float(score) / (n_levels - 1)
    return round(min(1.0, max(0.0, value)), 4)


def _noul_probability(answer: Dict[str, Any]) -> Optional[float]:
    """P(true) for a laya `noul` answer, already 0..1."""
    if not answer:
        return None
    return answer.get("noul")


def _choice_label(answer: Dict[str, Any]) -> str:
    if not answer:
        return "unknown"
    return answer.get("choice") or "unknown"


@app.post("/features", openapi_extra=_PROMPT_BODY)
def extract_features(request: Request) -> Dict[str, Any]:
    """Semantic features for routing: domain, difficulty, tool/sensitivity flags."""
    prompt = _prompt_from(request)
    result = get_router().predict({"request": prompt}, laya.router_questions())
    _track_model(result)
    answers = result.get("answers") or {}
    domain = answers.get("domain") or {}
    difficulty = answers.get("difficulty") or {}
    needs_tools = answers.get("needs_tools") or {}
    is_sensitive = answers.get("is_sensitive") or {}
    return {
        "domain": _choice_label(domain),
        "domain_confidence": domain.get("confidence"),
        "difficulty": _normalized_score(difficulty),
        "difficulty_confidence": difficulty.get("confidence"),
        "needs_tools": _noul_probability(needs_tools),
        "is_sensitive": _noul_probability(is_sensitive),
    }


@app.post("/guard", openapi_extra=_PROMPT_BODY)
def guardrails(request: Request) -> Dict[str, Any]:
    """Guardrail verdicts: jailbreak / injection / sensitive data / harm / topic."""
    prompt = _prompt_from(request)
    result = get_router().predict({"prompt": prompt}, laya.guard_questions())
    _track_model(result)
    answers = result.get("answers") or {}
    jailbreak = answers.get("jailbreak") or {}
    prompt_injection = answers.get("prompt_injection") or {}
    sensitive_data = answers.get("sensitive_data") or {}
    harm_severity = answers.get("harm_severity") or {}
    topic = answers.get("topic") or {}
    return {
        "jailbreak": _noul_probability(jailbreak),
        "prompt_injection": _noul_probability(prompt_injection),
        "sensitive_data": _noul_probability(sensitive_data),
        "harm_severity": _normalized_score(harm_severity),
        "topic": _choice_label(topic),
    }


@app.get("/health")
def health() -> Dict[str, Any]:
    return {"status": "ok", "model": _last_model}


def main() -> None:
    global _preload, _preload_all
    parser = argparse.ArgumentParser(description="Merlin laya feature-extraction sidecar")
    parser.add_argument("--port", type=int, default=8765, help="listen port (default 8765)")
    parser.add_argument("--host", default="127.0.0.1", help="listen host (default 127.0.0.1)")
    parser.add_argument(
        "--no-preload",
        action="store_true",
        help="do not download/build checkpoints at startup; lazy-load on first request",
    )
    parser.add_argument(
        "--preload-all",
        action="store_true",
        help="preload all three laya checkpoints (incl. typed-decisions) instead of the two served ones",
    )
    args = parser.parse_args()
    _preload = not args.no_preload
    _preload_all = args.preload_all
    uvicorn.run(app, host=args.host, port=args.port, log_level="info")


if __name__ == "__main__":
    main()
