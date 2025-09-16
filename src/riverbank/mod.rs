pub mod plugin;
pub mod components;
pub mod systems;
pub mod resources;
pub mod utils;

pub use plugin::RiverBankPlugin;
pub use resources::*;
pub use systems::get_river_height_modifier_detailed;