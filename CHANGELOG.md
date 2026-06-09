# Changelog

<!--
Keep a changelog that is human-readable and structured. Each version shall
have its own entry and , every change ought to be categorized as one of the
following type:

- Added: for new features.
- Changed for changes in existing functionality.
- Deprecated: for soon-to-be removed features.
- Removed: for now removed features.
- Fixed: for any bug fixes.
- Security: in case of vulnerabilities.
-->

## [Unreleased; planned for 2026-06-11]

### Added

### Removed

- amaru-protocols: remove the interim batch block-fetch API and keep the streaming `FetchBlocks` API (#778)

### Fixed

- amaru-ouroboros: properly wipe KES key material in unused method `SecretKey::from_bytes` (#881)
