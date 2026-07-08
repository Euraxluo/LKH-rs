# FastAPI JSON Backend

This example serves a browser UI and a JSON API backed by the real `lkh_rs`
Python package. The frontend sends and receives JSON only. The backend discovers
LKH `.par` cases, keeps filesystem paths on the server side, and calls the
native solver internally.

## Run

```bash
python -m venv /tmp/lkh-rs-api-venv
source /tmp/lkh-rs-api-venv/bin/activate
python -m pip install -r examples/fastapi_backend/requirements.txt
maturin develop
uvicorn app:app --app-dir examples/fastapi_backend --host 127.0.0.1 --port 8877
```

Open:

```text
http://127.0.0.1:8877/
```

## JSON API

```bash
curl -s http://127.0.0.1:8877/api/cases
```

```bash
curl -s http://127.0.0.1:8877/api/solve-case \
  -H 'content-type: application/json' \
  -d '{
    "case_id": "copy_one_from_api_cases",
    "overrides": {
      "RUNS": 1,
      "TRACE_LEVEL": 0,
      "MAX_TRIALS": 100
    }
  }'
```

By default the backend scans repository cases and also
`/tmp/lkh-official/bench-extract` when that directory exists. Set
`LKH_RS_CASE_ROOTS` to add more case roots.

Case JSON includes `known_optimum` when the LKH parameter/problem metadata
provides `OPTIMUM`. Solve responses include both `known_optimum` and
`gap_percent` so the browser can compare the current run with the known best
value.

`GET /api/metadata` and `POST /api/solve` remain available for the typed
programmatic JSON API, but the browser case picker is backed by LKH `.par`
cases.
