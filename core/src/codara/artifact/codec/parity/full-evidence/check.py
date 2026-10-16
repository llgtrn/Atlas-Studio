#!/usr/bin/env python3
"""Check test-only SHA256 pins; BLAKE3 and full reports are checked by the core regression."""
import hashlib
import json
from pathlib import Path

parity = Path(__file__).resolve().parent.parent
oracle = json.loads((parity / "full-evidence.json").read_text())
assert oracle["test_only_oracle"] == "atlas.functions-full-evidence.v1"
assert len(oracle["cases"]) == 7
for case in oracle["cases"]:
    path = parity / case["fixture"]
    assert path.resolve().is_relative_to(parity)
    data = path.read_bytes()
    checked = case["checked_evidence"]
    assert len(data) == case["input_bytes"], case["case"]
    assert hashlib.sha256(data).hexdigest() == checked["input_sha256"], case["case"]
    assert hashlib.sha256(case["file_name"].encode()).hexdigest() == checked["file_name_sha256"], case["case"]
    if checked["checked_payload_length"] is None:
        assert checked["checked_payload_sha256"] is None
        assert checked["computed_payload_blake3"] is None
    else:
        assert len(data[80:]) == checked["checked_payload_length"], case["case"]
        assert hashlib.sha256(data[80:]).hexdigest() == checked["checked_payload_sha256"], case["case"]
print("PASS: 7 full-evidence input, filename and checked-payload SHA256 pins")
