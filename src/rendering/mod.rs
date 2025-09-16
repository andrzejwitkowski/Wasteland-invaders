pub mod camera;
pub mod debug;
pub mod input;
pub mod animation;
pub mod bullet;
pub mod plane;
pub mod enemy;
pub mod spline;
pub mod enemy_spline_follower;
pub mod fbm_terrain;
pub mod water;
pub mod complex_water;
pub mod caustic_floor_material;

pub use debug::DebugRenderPlugin;
pub use camera::CameraPlugin;
pub use input::InputPlugin;
pub use animation::AnimationPlugin;
pub use bullet::BulletPlugin;
pub use plane::PlanePlugin;
pub use enemy_spline_follower::EnemySplineFollowerPlugin;
pub use water::WaterPlugin;
pub use complex_water::ComplexWaterPlugin; // This is a WIP
pub use fbm_terrain::FbmTerrainPlugin;
