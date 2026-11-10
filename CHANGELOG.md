# Changelog

## [Unreleased]

### Added
- [x] Implement TCP Server Interface
- [ ] Generate FFI bindings for TCP Server Interface
- [ ] Create APP!

### Changed
- [x] Make `FileReceiver` write data concurrently
- [x] Use Async UDP Socket
- [ ] Use `quinn-udp` library for bulk io
- [x] Refactor `AckFrame`
- [x] Send One Ack for many cumulative frames
