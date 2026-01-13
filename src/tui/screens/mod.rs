//! TUI screens for interactive workflows.

mod confirm;
mod item_actions;
mod prompt;
mod select;
mod wizard;

pub use confirm::confirm;
pub use item_actions::{select_item_with_actions, ItemAction};
pub use prompt::prompt_text;
pub use select::{select_from_list, select_from_list_filtered, select_from_list_with_header};
pub use wizard::{NewItemWizard, WizardOutput};
