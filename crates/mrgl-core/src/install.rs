//! The install engine (task 004): a zip is unpacked into a staging folder, its addon folders (several per
//! zip, as the BigWigs packager writes them) are found, and they are swapped into `Interface\AddOns` one
//! folder at a time with a rollback when a swap fails. Only `Interface\AddOns` is written; the state sidecar
//! ([`crate::state`]) records what was installed from where.
