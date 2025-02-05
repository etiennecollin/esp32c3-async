# esp32c3-async

This repo contains wrappers for async embedded rust on the esp32-c3 chip.

## Dependencies

- [`espflash`](https://github.com/esp-rs/espflash/) (not `cargo-espflash`)

## Features

Of the following features, **exactly one** must be activated:

- `esp32c3`: Target the ESP32-C3.

Of the following features, **exactly one** must be activated:

- `log`: The logger will use simple logging.
- `defmt`: The logger will use [DEFMT](https://github.com/knurling-rs/defmt) for logging.

Of the following features, **exactly one** must be activated:

- `logging-auto`: The logger will automatically select the UART or JTAG connection for logging.
- `logging-jtag`: The logger will use the JTAG connection for logging.
- `logging-uart`: The logger will use the UART connection for logging.

## Important

If the `defmt` feature is **not activated** and `log` is used, make sure to edit the `./.cargo/config.toml` file:

```diff
[target.riscv32imc-unknown-none-elf]
- runner = "espflash flash --monitor -L defmt"
+ runner = "espflash flash --monitor"

[env]
DEFMT_LOG="info"
ESP_LOG="info"

[build]
target = "riscv32imc-unknown-none-elf"
rustflags = [
    # Required
    "-C", "link-arg=-Tlinkall.x",
-   # Required for defmt
-   "-C", "link-arg=-Tdefmt.x",
    # Required to obtain backtraces (e.g. when using the "esp-backtrace" crate.)
    # NOTE: May negatively impact performance of produced code
    "-C", "force-frame-pointers",
]

[unstable]
build-std = ["alloc", "core"]
```
