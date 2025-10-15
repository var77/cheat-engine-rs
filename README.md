# cheat-engine-rs

## Running Tests

### Prerequisites

Before running tests, you need to build the example program:

```bash
cargo b --example simple_program
```

### Running Standard Tests

```bash
cargo test
```

### Running Tests That Require Root Permissions

Some tests require root privileges to run. To execute these tests:

```bash
sudo su
CARGO_TARGET_DIR=/tmp/target-root cargo test -- --include-ignored
```

Note: The `CARGO_TARGET_DIR` override is used to avoid permission conflicts with the standard build directory.
