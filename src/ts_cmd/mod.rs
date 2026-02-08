//! CLI commands for tspec management (tspec ts ...)

mod backup;
mod hash;
mod list;
mod new;
mod restore;
mod set;
mod show;

pub use backup::backup_tspec;
pub use hash::hash_tspec;
pub use list::list_tspecs;
pub use new::new_tspec;
pub use restore::restore_tspec;
pub use set::set_value;
pub use show::show_tspec;
