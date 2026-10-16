#!/usr/bin/env python3
"""Test-only proposed lossless bytes profile; no target storage, wire or authority API."""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
PARITY = HERE.parent
PROFILE = "atlas.functions-evidence-bytes.proposal.v1"
MAX_BYTES = 8192
CHUNK_BYTES = 512
MAX_CHUNKS = 16
HEX = re.compile(r"sha256:[0-9a-f]{64}\Z")
REPORT_KEYS = {
    "schema", "verdict", "defects", "address", "digest", "object_class",
    "object_schema_version", "decoded_length", "records", "not_verified",
}
CHECKED_KEYS = {
    "input_sha256", "file_name_sha256", "expected_payload_blake3",
    "computed_payload_blake3", "checked_payload_sha256", "checked_payload_length",
}


class Refused(ValueError):
    pass


def require(condition, reason):
    if not condition:
        raise Refused(reason)


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False,
                       separators=(",", ":"), allow_nan=False) + "\n").encode("utf8")


def keys(value, expected):
    require(type(value) is dict and set(value) == set(expected), "FIELDS")


def integer(value, minimum, maximum):
    require(type(value) is int and minimum <= value <= maximum, "INTEGER_BOUND")


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "DUPLICATE_KEY")
        result[key] = value
    return result


def invalid_constant(_):
    raise Refused("NONFINITE_NUMBER")


def finite_float(text):
    value = float(text)
    require(math.isfinite(value), "NONFINITE_NUMBER")
    return value


def parse(data):
    try:
        return json.loads(data.decode("utf8"), object_pairs_hook=unique_object,
                          parse_constant=invalid_constant, parse_float=finite_float)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise Refused("JSON: " + str(error)) from error


def decode_packet(data):
    """Bounds and profile shape only; this does not authenticate a source observation."""
    require(type(data) is bytes, "BYTES")
    require(0 < len(data) <= MAX_BYTES, "CAPACITY")
    packet = parse(data)
    keys(packet, {"profile", "source", "case", "input", "checked_evidence", "source_report_utf8"})
    require(packet["profile"] == PROFILE, "PROFILE")
    keys(packet["source"], {"repository", "revision", "codec_sha256", "oracle_sha256"})
    require(all(type(v) is str for v in packet["source"].values()), "PROVENANCE")
    require(type(packet["case"]) is str, "CASE")
    keys(packet["input"], {"file_name", "bytes", "sha256"})
    require(type(packet["input"]["file_name"]) is str, "FILE_NAME")
    integer(packet["input"]["bytes"], 0, (1 << 26) + 80)
    require(type(packet["input"]["sha256"]) is str
            and HEX.fullmatch(packet["input"]["sha256"]), "INPUT_DIGEST")
    keys(packet["checked_evidence"], CHECKED_KEYS)
    text = packet["source_report_utf8"]
    require(type(text) is str and text.endswith("\n"), "REPORT_TEXT")
    try:
        report_bytes = text.encode("utf8")
        require(canonical(packet) == data, "NONCANONICAL")
    except (UnicodeError, ValueError, RecursionError) as error:
        raise Refused("SERIALIZATION: " + str(error)) from error
    report = parse(report_bytes)
    keys(report, REPORT_KEYS)
    require(report["schema"] == "atlas.atlasx-codec.v1", "REPORT_SCHEMA")
    require(report["verdict"] in ("DECODED", "REFUSED"), "REPORT_VERDICT")
    require(type(report["defects"]) is list and type(report["not_verified"]) is list,
            "REPORT_TABLES")
    return packet, report_bytes, report


def describe(data, capacity):
    require(type(data) is bytes and 0 < len(data) <= MAX_BYTES, "CAPACITY")
    integer(capacity, 0, MAX_BYTES)
    require(len(data) <= capacity, "CAPACITY")
    chunks = []
    for index, offset in enumerate(range(0, len(data), CHUNK_BYTES)):
        part = data[offset:offset + CHUNK_BYTES]
        chunks.append({"index": index, "offset": offset, "bytes": len(part), "sha256": digest(part)})
    return {"sha256": digest(data), "bytes": len(data), "chunks": chunks}


def check_descriptor(descriptor):
    keys(descriptor, {"sha256", "bytes", "chunks"})
    require(type(descriptor["sha256"]) is str and HEX.fullmatch(descriptor["sha256"]), "DIGEST")
    integer(descriptor["bytes"], 1, MAX_BYTES)
    chunks = descriptor["chunks"]
    count = (descriptor["bytes"] + CHUNK_BYTES - 1) // CHUNK_BYTES
    require(type(chunks) is list and len(chunks) == count <= MAX_CHUNKS, "CHUNK_COUNT")
    for index, chunk in enumerate(chunks):
        keys(chunk, {"index", "offset", "bytes", "sha256"})
        integer(chunk["index"], index, index)
        integer(chunk["offset"], index * CHUNK_BYTES, index * CHUNK_BYTES)
        size = min(CHUNK_BYTES, descriptor["bytes"] - index * CHUNK_BYTES)
        integer(chunk["bytes"], size, size)
        require(type(chunk["sha256"]) is str and HEX.fullmatch(chunk["sha256"]), "CHUNK_DIGEST")


def read_chunk(data, descriptor, index, capacity):
    """Pure fixture model over already-held bytes; never reads a path or regenerates data."""
    check_descriptor(descriptor)
    require(describe(data, MAX_BYTES) == descriptor, "ARTIFACT_INTEGRITY")
    integer(index, 0, len(descriptor["chunks"]) - 1)
    integer(capacity, 0, CHUNK_BYTES)
    chunk = descriptor["chunks"][index]
    require(chunk["bytes"] <= capacity, "CAPACITY")
    start = chunk["offset"]
    return index, data[start:start + chunk["bytes"]]


def assemble(descriptor, responses):
    check_descriptor(descriptor)
    require(type(responses) is list and len(responses) == len(descriptor["chunks"]), "CHUNK_COUNT")
    for chunk, response in zip(descriptor["chunks"], responses):
        require(type(response) is tuple and len(response) == 2, "RESPONSE")
        index, data = response
        integer(index, chunk["index"], chunk["index"])
        require(type(data) is bytes and len(data) == chunk["bytes"], "CHUNK_SIZE")
        require(digest(data) == chunk["sha256"], "CHUNK_INTEGRITY")
    data = b"".join(response[1] for response in responses)
    require(len(data) == descriptor["bytes"] and digest(data) == descriptor["sha256"],
            "ARTIFACT_INTEGRITY")
    return data


MANIFEST = json.loads((HERE / "vectors.json").read_text())
ORACLE_BYTES = (PARITY / "full-evidence.json").read_bytes()
ORACLE = json.loads(ORACLE_BYTES)


def compare_oracle(packet, case):
    # This fixture oracle comparison is intentionally stronger than the generic shape check.
    require(packet["source"] == MANIFEST["source"], "SOURCE_BINDING")
    require(packet["case"] == case["case"], "CASE_BINDING")
    require(packet["input"] == {"file_name": case["file_name"], "bytes": case["input_bytes"],
                                "sha256": "sha256:" + case["checked_evidence"]["input_sha256"]},
            "INPUT_BINDING")
    require(canonical(packet["checked_evidence"]) == canonical(case["checked_evidence"]),
            "CHECKED_EVIDENCE")
    require(canonical(parse(packet["source_report_utf8"].encode("utf8"))) == canonical(case["source_report"]),
            "COMPLETE_SOURCE_REPORT")


class ProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        require(MANIFEST["test_only_contract"] == PROFILE, "MANIFEST_PROFILE")
        require(MANIFEST["bounds"] == {"packet_bytes": MAX_BYTES, "chunk_bytes": CHUNK_BYTES,
                                       "chunk_count": MAX_CHUNKS}, "MANIFEST_BOUNDS")
        require("sha256:" + MANIFEST["source"]["oracle_sha256"] == digest(ORACLE_BYTES), "ORACLE_PIN")
        require(len(MANIFEST["cases"]) == len(ORACLE["cases"]) == 7, "CASE_COUNT")
        cls.vectors = []
        for vector, case in zip(MANIFEST["cases"], ORACLE["cases"]):
            require(vector["case"] == case["case"], "CASE_ORDER")
            require(vector["file"] == case["case"] + ".evidence.json", "VECTOR_PATH")
            data = (HERE / vector["file"]).read_bytes()
            packet, raw, _ = decode_packet(data)
            compare_oracle(packet, case)
            require(describe(data, MAX_BYTES) == vector["artifact"], "VECTOR_PIN")
            require(digest(raw) == vector["source_report_sha256"] and len(raw) == vector["source_report_bytes"],
                    "REPORT_BYTE_PIN")
            cls.vectors.append((vector, case, data, packet, raw))

    def test_all_seven_exact_round_trips_and_safe_chunk_retry(self):
        for vector, _, data, _, _ in self.vectors:
            descriptor = vector["artifact"]
            responses = [read_chunk(data, descriptor, i, CHUNK_BYTES)
                         for i in range(len(descriptor["chunks"]))]
            self.assertEqual(assemble(descriptor, responses), data)
            self.assertEqual(read_chunk(data, descriptor, 0, CHUNK_BYTES), responses[0])

    def test_whole_and_chunk_capacity_fail_closed(self):
        for vector, _, data, _, _ in self.vectors:
            self.assertGreater(len(data), 1024)
            self.assertEqual(describe(data, len(data)), vector["artifact"])
            for capacity in (0, 1024, len(data) - 1):
                with self.assertRaises(Refused):
                    describe(data, capacity)
            for index, chunk in enumerate(vector["artifact"]["chunks"]):
                self.assertEqual(len(read_chunk(data, vector["artifact"], index, chunk["bytes"])[1]),
                                 chunk["bytes"])
                with self.assertRaises(Refused):
                    read_chunk(data, vector["artifact"], index, chunk["bytes"] - 1)

    def test_maximum_boundary_opaque_chunks_and_invalid_descriptors(self):
        data = bytes(MAX_BYTES)
        descriptor = describe(data, MAX_BYTES)
        self.assertEqual(len(descriptor["chunks"]), MAX_CHUNKS)
        self.assertEqual(assemble(descriptor, [read_chunk(data, descriptor, i, CHUNK_BYTES)
                                              for i in range(MAX_CHUNKS)]), data)
        for bad in (b"", bytes(MAX_BYTES + 1)):
            with self.assertRaises(Refused):
                describe(bad, MAX_BYTES)
        for field, value in (("bytes", MAX_BYTES + 1), ("bytes", True), ("sha256", "00")):
            bad = copy.deepcopy(descriptor)
            bad[field] = value
            with self.assertRaises(Refused):
                check_descriptor(bad)
        bad = copy.deepcopy(descriptor)
        bad["chunks"][0]["offset"] = 1
        with self.assertRaises(Refused):
            check_descriptor(bad)
        # Split α across an opaque byte boundary; decode UTF8 only after complete assembly.
        unicode_bytes = b"x" * 511 + "α".encode() + b"z"
        descriptor = describe(unicode_bytes, MAX_BYTES)
        with self.assertRaises(UnicodeDecodeError):
            read_chunk(unicode_bytes, descriptor, 0, CHUNK_BYTES)[1].decode()
        self.assertEqual(assemble(descriptor, [read_chunk(unicode_bytes, descriptor, i, CHUNK_BYTES)
                                              for i in range(2)]).decode(), "x" * 511 + "αz")

    def test_missing_duplicate_reordered_substituted_and_corrupt_chunks_refuse(self):
        vector, _, data, _, _ = self.vectors[-1]
        descriptor = vector["artifact"]
        responses = [read_chunk(data, descriptor, i, CHUNK_BYTES) for i in range(len(descriptor["chunks"]))]
        corrupt = list(responses)
        index, part = corrupt[0]
        corrupt[0] = index, bytes([part[0] ^ 1]) + part[1:]
        truncated = list(responses)
        truncated[-1] = truncated[-1][0], truncated[-1][1][:-1]
        substituted = list(responses)
        other = self.vectors[0]
        substituted[0] = read_chunk(other[2], other[0]["artifact"], 0, CHUNK_BYTES)
        for bad in (responses[:-1], responses + responses[:1], list(reversed(responses)),
                    [responses[0]] * len(responses), corrupt, truncated, substituted):
            with self.assertRaises(Refused):
                assemble(descriptor, bad)
        bad_descriptor = copy.deepcopy(descriptor)
        bad_descriptor["sha256"] = "sha256:" + "0" * 64
        with self.assertRaises(Refused):
            assemble(bad_descriptor, responses)
        for index in (-1, len(responses), True):
            with self.assertRaises(Refused):
                read_chunk(data, descriptor, index, CHUNK_BYTES)
        with self.assertRaises(Refused):
            read_chunk(data[:-1], descriptor, 0, CHUNK_BYTES)

    def test_serialization_versions_duplicate_keys_and_noncanonical_bytes_refuse(self):
        _, _, data, packet, _ = self.vectors[0]
        bad_version = copy.deepcopy(packet)
        bad_version["profile"] = "atlas.functions-evidence-bytes.proposal.v2"
        bad_field = copy.deepcopy(packet)
        bad_field["grant"] = "invented"
        cases = [data + b"\n", b" " + data, data[:-1], b"\xff", b"{}", bytes(MAX_BYTES + 1),
                 canonical(bad_version), canonical(bad_field),
                 b'{"profile":"a","profile":"b"}\n', b'{"n":NaN}\n']
        for bad in cases:
            with self.assertRaises(Refused):
                decode_packet(bad)
        for number in ("1e999", "-1e999", "NaN", "Infinity", "-Infinity"):
            bad = copy.deepcopy(packet)
            bad["source_report_utf8"] = packet["source_report_utf8"].replace(
                '"decoded_length": null', '"decoded_length": ' + number)
            with self.assertRaises(Refused):
                decode_packet(canonical(bad))
            with self.assertRaises(Refused):
                parse(('{"n":' + number + '}').encode())

    def test_semantic_and_checked_evidence_loss_is_never_full_parity(self):
        _, case, _, original, _ = self.vectors[0]
        mutations = []
        for defects in (case["source_report"]["defects"][:1],
                        list(reversed(case["source_report"]["defects"]))):
            packet = copy.deepcopy(original)
            report = copy.deepcopy(case["source_report"])
            report["defects"] = defects
            packet["source_report_utf8"] = json.dumps(report, ensure_ascii=False) + "\n"
            mutations.append(packet)
        for field in ("expected_payload_blake3", "computed_payload_blake3", "checked_payload_sha256"):
            packet = copy.deepcopy(original)
            packet["checked_evidence"][field] = None
            mutations.append(packet)
        packet = copy.deepcopy(original)
        packet["checked_evidence"]["checked_payload_length"] = float(
            packet["checked_evidence"]["checked_payload_length"])
        mutations.append(packet)
        packet = copy.deepcopy(original)
        packet["source"]["revision"] = "0" * 40
        mutations.append(packet)
        for packet in mutations:
            with self.assertRaises(Refused):
                compare_oracle(packet, case)
        # Null is not an empty decoded table, and a claim is not a completed computation.
        _, before_hash, _, original, _ = self.vectors[5]
        for field, value in (("records", []), ("address", "0" * 64)):
            packet = copy.deepcopy(original)
            report = copy.deepcopy(before_hash["source_report"])
            report[field] = value
            packet["source_report_utf8"] = json.dumps(report) + "\n"
            with self.assertRaises(Refused):
                compare_oracle(packet, before_hash)
        _, decoded, _, original, _ = self.vectors[-1]
        for value in (True, 1.0):
            packet = copy.deepcopy(original)
            report = copy.deepcopy(decoded["source_report"])
            report["object_schema_version"] = value
            packet["source_report_utf8"] = json.dumps(report) + "\n"
            with self.assertRaises(Refused):
                compare_oracle(packet, decoded)


def check_cli(binary):
    binary = Path(binary).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="atlas-lossless-bytes-") as directory:
        directory = Path(directory)
        for vector, case in zip(MANIFEST["cases"], ORACLE["cases"]):
            packet, expected, _ = decode_packet((HERE / vector["file"]).read_bytes())
            compare_oracle(packet, case)
            source = directory / case["case"] / case["file_name"]
            source.parent.mkdir()
            data = (PARITY / case["fixture"]).read_bytes()
            require(digest(data) == packet["input"]["sha256"], "INPUT_PIN")
            source.write_bytes(data)
            command = [str(binary), "atlasx", "codec", "--decode", str(source)]
            result = subprocess.run(command, capture_output=True, check=False, timeout=30)
            require(result.returncode == (0 if case["source_report"]["verdict"] == "DECODED" else 2),
                    "CLI_VERDICT")
            require(result.stdout == expected, "CLI_EXACT_REPORT_BYTES")
    print("PASS: 7 actual CLI reports match exact preserved source_report_utf8 bytes")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, help="also compare seven reports through the actual source CLI")
    args = parser.parse_args()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(ProfileTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():
        raise SystemExit(1)
    if args.cli:
        check_cli(args.cli)
    print("PASS: proposed source-only bounded byte profile; target adoption and rights remain gated")
