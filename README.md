# viasendraw

A tiny util built using Rust to send raw data to VIA-compatible keyboards via USB HID.

Defaults align with [QMK](https://docs.qmk.fm/features/rawhid#receiving-data-from-the-keyboard); the HID Usage Page and Usage ID for the Raw HID interface are `0xFF60` and `0x61`.

## Usage

```sh
echo "Hello" | viasendraw --vendor "0xcb00" --product "0x2006"
printf "\x01\x02\x03" | viasendraw -v "0xcb00" -p "0x2006"
```

## Example Receiver

In your `keymap.c`

```c
static char received_message[32] = {0}; // Buffer for the message

void raw_hid_receive_kb(uint8_t *data, uint8_t length) {
    // Check if this is our custom command (example: first byte = 0xFF for custom commands)
    if (length > 0 && data[0] == 0xFF) {
        memset(received_message, 0, sizeof(received_message));
        uint8_t actual_length = 0;
        for (uint8_t i = 1; i < length; i++) {
            if (data[i] == 0) break;
            received_message[actual_length] = data[i];
            actual_length++;
        }
        received_message[actual_length] = '\0';
        data[0] = 0x01; // Success response
    } else {
      data[0] = 0x00; // id_unhandled
    }
}

bool oled_task_user(void) {
		oled_set_cursor(0, 0);
	  oled_write(received_message, false);
		return false;
}
```

## License

See [LICENSE](./LICENSE)
