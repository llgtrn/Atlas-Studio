# Golden file tests

## Instructions

*   Add a new test by running
    `rs_bindings_from_cc/test/golden/create_golden.sh foo`.
    This will create empty `foo.h`, `foo_rs_api.rs`, and `foo_rs_api_impl.cc`
    files. After adding your code to `foo.h`, run
    `common/golden_update.sh` to generate the bindings.
*   If a test in this directory fails, look at the output. It should contain a
    diff of the failure.
*   If you get spurious failures in this directory: Run
    `common/golden_update.sh`.
