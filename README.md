# vmtouch-rs

[![Build Status](https://github.com/ocadaruma/vmtouch-rs/workflows/CI/badge.svg?branch=master)](https://github.com/ocadaruma/vmtouch-rs/actions?query=workflow%3ACI+branch%3Amaster+event%3Apush)

Rust-port of [vmtouch](https://github.com/hoytech/vmtouch).

## Usage

```bash
# Inspect page cache info
$ vmtouch-rs --file /path/to/file
Resident pages: 657/6196  2691072/25378816  10%

# Evict page cache
$ vmtouch-rs --file /path/to/file evict

# Load into page cache
$ vmtouch-rs --file /path/to/file touch
```
