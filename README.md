# Doer

<div>
    <!-- build -->
    <a href="https://github.com/obamathellama114/doer">
        <img src="https://img.shields.io/github/actions/workflow/status/obamathellama114/doer/rust.yml?style=flat-square"/>
    </a>
    <!-- License -->
    <a href="">
        <img src="https://img.shields.io/crates/l/doer?style=flat-square">
    </a>
    <!-- Version -->
    <a href="https://crates.io/crates/doer">
        <img src="https://img.shields.io/crates/v/doer?style=flat-square"/>
    </a>
    <!-- Downloads -->
    <a href="https://crates.io/crates/doer">
        <img src="https://img.shields.io/crates/d/doer?style=flat-square"/>
    </a>
</div>

Doer is simple task runner that uses the TOML file format for configuration.

## Installation

```bash
cargo install doer
```

## Usage

By default, Doer will run the `default` step.\
Example:

```bash
doer
```

This will run the `default` step. This can be changed by passing in a different step, or with the `default` key in the `build.toml` file.\
Example:

```bash
doer build
```

This will run the `build` step.\
You can also pass in a different file to use for configuration.\
Example:

```bash
doer --file build.toml build
```

This will run the `build` step in the `build.toml` file.

## Configuration

Doer uses a TOML file for configuration. The default location is `./build.toml` but this can be changed with the `--file` option.\
A simple example:

```toml
[step.default]
command = "echo 'Hello, world!'"

[step.build]
command = "cargo build"
```

This will run `echo 'Hello, world!'` by default, but if you run `doer --task build` it will run `cargo build`.

### Steps

Steps are the tasks that Doer will run. They are defined in the `step` table. Each step has a name, which is used to identify it and a `command` key, which is the command that will be run.\
Example:

```toml
[step.run]
command = "cargo run"

[step.build]
command = "cargo build"
```

This will run `cargo run` when the `run` step is run, and `cargo build` when the `build` step is run.

### Default step

The default step is the step that will be run if no step is specified. By default, this is the `default` step. \
Example:

```toml
[step.default]
command = "echo 'Hello, world!'"
```

This will run `echo 'Hello, world!'` by default. If you want to change the default step, you can use the `default` key in the `build.toml` file.\
Example:

```toml
default = "build"

[step.build]
command = "cargo build"
```

### Dependencies

Steps can depend on other steps by using the `depends` key.\
Example:

```toml
[step.run]
command = "cargo run"
depends = ["build"]

[step.build]
command = "cargo build"
```

This will run `cargo build` before running `cargo run`. If the `build` step fails, the `run` step will not run.

### Environment variables

Environment variables can be set in the `env` table for each step.\
Example:

```toml
[step.run]
command = "cargo run"
env = { RUST_LOG = "debug" }
```

This will run `cargo run` with the `RUST_LOG` environment variable set to `debug`.

### Step groups

A step doesn't have to have a `command` key. Instead, it can just have a `depends` key, which is a list of steps that will be run. This is useful for grouping steps together.\
Example:

```toml
[step.default]
depends = ["build", "release"]

[step.build]
command = "cargo build"

[step.release]
command = "cargo build --release"
```

This will run both `cargo build` and `cargo build --release` when the `default` step is run.

### Asynchronous steps

Steps can be run asynchronously by setting the `async` key to `true`.\
Example:

```toml
[step.default]
depends = ["build", "release"]

[step.build]
command = "cargo build"
async = true

[step.release]
command = "cargo build --release"
async = true
```

This will run `cargo build` and `cargo build --release` at the same time.\
A step can also force all of its dependencies to run synchronously by setting the `in-order` key to `true`.\
Example:

```toml
[step.default]
depends = ["build", "release"]
in-order = true

[step.build]
command = "cargo build"
async = true

[step.release]
command = "cargo build --release"
async = true
```

This will run `cargo build` and `cargo build --release` one after the other, even though they are both asynchronous steps.

### Cross file dependencies

Steps can depend on other steps in other files by depending on the directory name and the step name, or optionally just the directory name to run the default step.\
Example:

```toml
# build.toml
[step.default]
depends = ["web", "api"]

[step.build]
depends = ["web:build", "api:build"]
```

```toml
# web/build.toml
[step.default]
command = "echo 'web default'"

[step.build]
command = "trunk build"
async = true
```

```toml
# api/build.toml
[step.default]
command = "echo 'api default'"

[step.build]
command = "cargo build"
async = true
```

In this example, the default step in `build.toml` depends on the default step in `web/build.toml` and the default step in `api/build.toml` while the `build` step in `build.toml` depends on the `build` step in `web/build.toml` and the `build` step in `api/build.toml`.
