# vmtouch-rs

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
