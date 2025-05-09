pub mod camera;
pub mod debug;
pub mod input;
pub mod animation;
pub mod bullet;
pub mod plane;
pub mod enemy;
pub mod spline;

pub use debug::DebugRenderPlugin;
pub use camera::CameraPlugin;
pub use input::InputPlugin;
pub use animation::AnimationPlugin;
pub use bullet::BulletPlugin;
pub use plane::PlanePlugin;
pub use enemy::EnemyPlugin;
pub use spline::SplinePlugin;
