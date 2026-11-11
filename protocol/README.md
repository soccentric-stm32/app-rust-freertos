# HWLINK protocol schema

`hwlink.fbs` is the FlatBuffers schema for the HWLINK protocol
(SRS-GUI-F767 Appendix A). It is shared unchanged with BOOT, HWVAL, the GUI
and the other demo applications.

This application is `no_std` and has no heap, so the FlatBuffers wire format
for this message set is implemented directly:

- `src/protocol/hwlink/fb.rs` — fixed-buffer reader/writer
- `src/protocol/hwlink/mod.rs` — message, union and enum tables
- `src/protocol/hwlink/frame.rs` — COBS framing and CRC-32/ISO-HDLC

`Message.body` is an append-only union: add new members at the end and raise
the protocol minor version (LNK-013). The file identifier is `HWLK`.
