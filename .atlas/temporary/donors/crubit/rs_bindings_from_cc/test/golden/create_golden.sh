#!/bin/bash
# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

set -euo pipefail

if [[ $# -ne 1 ]] || [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
  echo "Usage: $0 <test_name>"
  echo "Example: $0 my_new_test"
  exit 1
fi

TEST_NAME="$1"
if [[ "${TEST_NAME}" == */* ]]; then
  echo "Error: Test name '${TEST_NAME}' cannot contain slashes ('/')." >&2
  exit 1
fi

readonly GOLDEN_DIR="$(dirname "$0")"

readonly H_FILE="${GOLDEN_DIR}/${TEST_NAME}.h"
readonly RS_API_FILE="${GOLDEN_DIR}/${TEST_NAME}_rs_api.rs"
readonly RS_API_IMPL_FILE="${GOLDEN_DIR}/${TEST_NAME}_rs_api_impl.cc"

for chk in "${H_FILE}" "${RS_API_FILE}" "${RS_API_IMPL_FILE}"; do
  if [[ -e "$chk" ]]; then
    echo "Error: $chk already exists. Please choose a different test name or remove existing file."
    exit 1
  fi
done

touch "${H_FILE}" "${RS_API_FILE}" "${RS_API_IMPL_FILE}"

echo "Created empty test files: ${TEST_NAME}.h, ${TEST_NAME}_rs_api.rs, ${TEST_NAME}_rs_api_impl.cc"
echo "Next: Update ${TEST_NAME}.h with your test code, and run common/golden_update.sh"
