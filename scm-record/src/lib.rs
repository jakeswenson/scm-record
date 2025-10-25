//! Reusable change selector UI for source control systems.

#![warn(missing_docs)]
#![warn(
    clippy::all,
    clippy::as_conversions,
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro
)]
#![allow(clippy::too_many_arguments)]

mod render;
mod types;
mod ui;
mod util;

#[cfg(feature = "tree-sitter")]
pub mod semantic;

pub mod consts;
pub mod helpers;
pub use types::{
    ChangeType, Commit, File, FileMode, RecordError, RecordState, Section, SectionChangedLine,
    SelectedChanges, SelectedContents, Tristate,
};
pub use ui::{Event, RecordInput, Recorder, TerminalKind, TestingScreenshot};
