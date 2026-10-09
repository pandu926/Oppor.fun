#!/usr/bin/env python3
"""Validate OpenAPI and captured real integration responses against its schemas."""
import json
import re
from urllib.parse import urlsplit
from pathlib import Path
from jsonschema import Draft202012Validator, FormatChecker
from openapi_spec_validator import validate

root = Path(__file__).resolve().parent.parent
spec = json.loads((root / "openapi.json").read_text())
validate(spec)
print("OpenAPI 3.1 schema validation passed.")
records_path = root / "target/api-contract-responses.json"
if records_path.exists():
    records = json.loads(records_path.read_text())
    checked = 0
    for record in records:
        request_path=urlsplit(record["path"]).path
        matched = [path for path in spec["paths"] if re.fullmatch(re.sub(r"\{[^}]+\}", "[^/]+", path), request_path)]
        # Static paths take precedence over parameterized paths, as in Axum.
        if request_path in matched:
            matched = [request_path]
        assert len(matched) == 1, record["path"]
        operation = spec["paths"][matched[0]][record["method"]]
        response = operation["responses"].get(str(record["status"]), operation["responses"]["default"])
        content = response["content"].get("application/json")
        if not content:
            continue
        validator = Draft202012Validator({**content["schema"], "components": spec["components"]}, format_checker=FormatChecker())
        errors = list(validator.iter_errors(record["body"]))
        assert not errors, f'{record["method"]} {record["path"]}: {[str(error) for error in errors]}'
        checked += 1
    print(f"Validated {checked} captured integration responses against typed API schemas.")
