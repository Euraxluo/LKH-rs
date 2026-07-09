# FastAPI Backend

The browser integration path is a native FastAPI service. The browser talks
JSON only. The backend discovers LKH `.par` cases, reads the corresponding
problem metadata, and calls the native solver.

Run the example:

```bash
python -m venv /tmp/lkh-rs-api-venv
source /tmp/lkh-rs-api-venv/bin/activate
python -m pip install -r examples/fastapi_backend/requirements.txt
maturin develop
uvicorn app:app --app-dir examples/fastapi_backend --host 127.0.0.1 --port 8877
```

Open `http://127.0.0.1:8877/`.

The API is JSON-only:

- `GET /api/health` checks that the backend is alive.
- `GET /api/cases` returns discovered LKH parameter-file cases.
- `GET /api/cases/{case_id}` returns one case as JSON, including editable
  parameter text, known optimum when the `.par`/problem metadata provides one,
  and a JSON solve request.
- `POST /api/solve-case` solves one backend case from JSON and returns a JSON
  report with `known_optimum` and `gap_percent` for comparison.
- `GET /api/metadata` returns enum values for the separate programmatic JSON API.
- `POST /api/solve` remains available for typed programmatic JSON problems.

By default the backend scans the repository cases under `source_code/` and
`tests/fixtures/`. If `/tmp/lkh-official/bench-extract` exists, it is also
scanned so the official benchmark `.par` files appear in the browser. Set
`LKH_RS_CASE_ROOTS` to an `os.pathsep`-separated list of additional roots to
expose more LKH case directories.

The browser never receives filesystem authority. It receives a `case_id`,
metadata, optional display coordinates, and parameter text for inspection. On
solve, the backend rewrites LKH file references into a temporary parameter file
and calls `lkh_rs.solve_parameter_file` internally.
