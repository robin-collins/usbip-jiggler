## Root Cause #1 — `OP_REP_IMPORT` response is malformed (the fatal one)

Every successful connection cycle dies on the same error:

```
TRACE_LEVEL_ERROR  validate_header: USBIP_RET_* expected, got 0x100
```

This happens immediately after the driver sends its first real URB:

```
send: seqnum 3, devid 0x10001, in[0]
      cmd_submit: GET_DESCRIPTOR(0x6), wValue 0x100 ...
```

The driver sends a `USBIP_CMD_SUBMIT` (command code `0x00000001`) and your Rust server is responding with `0x100` — which is **not a valid USBIP_RET_SUBMIT response**. The driver expects `USBIP_RET_SUBMIT` (`0x00000003`) as the command field in the response header, but is reading back `0x100`.

**`0x100` is `wValue` from the GET_DESCRIPTOR setup packet** — meaning your server is either:
- Echoing back the raw setup packet bytes instead of building a proper `usbip_header` response, or
- Returning the descriptor data starting at the wrong offset, so the command field lands on descriptor content instead of `0x00000003`

The fix is in your `USBIP_RET_SUBMIT` builder. The response header must be exactly 48 bytes structured as:

```
u32 command    = 0x00000003  (USBIP_RET_SUBMIT)
u32 seqnum     = <echo from CMD_SUBMIT>
u32 devid      = 0
u32 direction  = 0
u32 ep         = 0
i32 status     = 0
i32 actual_length = <bytes returned>
i32 start_frame = 0
i32 number_of_packets = 0
i32 error_count = 0
u64 setup      = 0
[data follows]
```

Your server is almost certainly writing `0x00 0x01 0x00 0x00` (little-endian for `wValue=0x0100` from the descriptor response) into the first 4 bytes instead of `0x03 0x00 0x00 0x00`.

---

## Root Cause #2 — `OP_REP_IMPORT` returning `ST_NA` (earlier failures)

The earlier attempts (before it ever gets to device enumeration) fail with:

```
TRACE_LEVEL_ERROR  recv_op_common: code 0x3, ST_NA
NTSTATUS=E1020001
```

`ST_NA` means your server returned status `0x01` (not available / error) in the `OP_REP_IMPORT` response. This happens when the client sends `OP_REQ_IMPORT` and your server replies with a non-zero status byte. The third attempt eventually succeeds (device does get plugged in), so this may be a race or timing issue in your server — possibly accepting the TCP connection before the device is ready to be imported, then recovering on retry.

---

## Summary

| Failure | Log evidence | Your server is doing |
|---|---|---|
| `ST_NA` on import | `recv_op_common: code 0x3, ST_NA` | Returning error status in `OP_REP_IMPORT` (intermittent) |
| `0x100` header | `validate_header: USBIP_RET_* expected, got 0x100` | Writing descriptor bytes at wrong offset in `USBIP_RET_SUBMIT` header |

The second one is the hard blocker — fix the `ret_submit` serialisation so `command = 0x00000003` is the first 4 bytes, all big-endian, before anything else. The `0x100` you're seeing is almost certainly `wValue` from the GET_DESCRIPTOR leaking into the command field.