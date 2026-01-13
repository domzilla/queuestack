//! # Commands
//!
//! CLI command implementations for qstack.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

pub mod attach;
pub mod close;
pub mod completions;
pub mod init;
pub mod list;
pub mod new;
pub mod search;
pub mod setup;
pub mod update;

pub use self::{
    attach::{
        execute_add as attach_add, execute_remove as attach_remove, AttachAddArgs, AttachRemoveArgs,
    },
    close::{execute_close, execute_reopen},
    completions::execute as completions,
    init::execute as init,
    list::{execute as list, ListMode, ListOptions, SortBy, StatusFilter},
    new::{execute as new, NewArgs},
    search::{execute as search, SearchArgs},
    setup::execute as setup,
    update::{execute as update, UpdateArgs},
};
pub use crate::ui::InteractiveArgs;
