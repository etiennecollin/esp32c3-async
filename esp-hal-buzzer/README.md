# esp-hal-buzzer

This crate is a modified and refactored version of the `esp-hal-buzzer` library from the [esp-hal-community](https://github.com/esp-rs/esp-hal-community) repository that suits my needs.

Amongst other things, songs and lists of tones may now be played asynchronously using embassy, and dependencies and features were simplified.

Thank you to the original authors!

## Features

Of the following features, **exactly one** must be activated:

- `esp32c3`: Target the ESP32-C3.

Other features:

- `defmt`: Implement `defmt::Format` on certain types.
