//! The state sidecar (D-003, task 004): `%LOCALAPPDATA%\mrgl\<flavor-id>\state.json` with, per installed
//! addon, its source, version, file list with hashes and install time. The AddOns folder is the truth and
//! the state is a cache: a missing or corrupt file forces a rebuild from the scan, never an error the user
//! has to fix.
