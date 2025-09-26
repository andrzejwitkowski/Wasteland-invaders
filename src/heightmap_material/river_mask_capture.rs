use bevy::prelude::*;
use bevy::{app::{App, Plugin, Update}, asset::Handle, ecs::{component::Component, resource::Resource}, image::Image, render::{
    render_asset::RenderAssets, renderer::{RenderDevice, RenderQueue}, Render, RenderApp
}};
use bevy::render::render_resource::{ // keep what still exists in render_resource
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, Origin3d, TextureAspect, TextureDimension, TextureFormat, TextureUsages
};
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel::unbounded;

use crate::heightmap_material::{CompleteGpuHeightmapMaterial, GpuHeightmapTerrain};

/* ----------------------------- Components ----------------------------- */

#[derive(Component)]
pub struct RiverMaskTerrain;

#[derive(Component)]
pub struct RiverMaskCamera;

/* ----------------------------- Resources ------------------------------ */

/// Main-world state & UI control (also extracted each frame to render world)
#[derive(Resource, Clone, Default)]
pub struct RiverMaskTarget {
    pub image: Handle<Image>,
    pub request_capture: bool,
    pub path: Option<String>,
}

/// Channel receiver in main world (readback bytes arrive here)
#[derive(Resource)]
pub struct RiverMaskReadbackChannel {
    pub rx: Receiver<ReadbackMsg>,
}

/// Channel sender in render world
#[derive(Resource)]
pub struct RiverMaskReadbackSender {
    pub tx: Sender<ReadbackMsg>,
}

/// Render-world transient state (so we don't queue multiple copies per request)
#[derive(Resource, Default)]
pub struct RiverMaskRenderState {
    pub copy_submitted: bool,
    pub last_path: Option<String>,
}

/// Message sent from render world (after map) back to main world
pub struct ReadbackMsg {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA8
}

/* ------------------------------- Plugin ------------------------------- */

pub struct RiverMaskPlugin;

impl Plugin for RiverMaskPlugin {

    fn build(&self, app: &mut App) {
        // Channel for cross-world communication
        let (tx, rx) = unbounded::<ReadbackMsg>();

        app
            .insert_resource(RiverMaskTarget::default())
            .insert_resource(RiverMaskReadbackChannel { rx })
            .add_systems(
                Update,
                (
                    ensure_river_mask_setup,
                    river_mask_ui,
                    sync_mask_camera_transform,
                    poll_readback_and_save,
                ),
            );

        // Set up render app resources / systems
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(RiverMaskReadbackSender { tx })
            .insert_resource(RiverMaskRenderState::default())
            .add_systems(Render, queue_river_mask_readback);
    }
}

/* ------------------------------- UI System ---------------------------- */

fn river_mask_ui(
    mut contexts: bevy_egui::EguiContexts,
    mut target: ResMut<RiverMaskTarget>,
) {
    bevy_egui::egui::Window::new("River Mask").show(contexts.ctx_mut(), |ui| {
        if ui.button("Capture River Mask").clicked() && !target.request_capture {
            target.request_capture = true;
            target.path = Some("river_mask.png".into());
            info!("River mask capture requested.");
        }
        ui.label(format!(
            "Image allocated: {}",
            target.image != Handle::default()
        ));
    });
}

/* -------------------------- Offscreen Setup --------------------------- */

fn ensure_river_mask_setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    mask_terrain_q: Query<(), With<RiverMaskTerrain>>,
    mask_cam_q: Query<(), With<RiverMaskCamera>>,
    original_terrain_q: Query<
        (&Mesh3d, &MeshMaterial3d<CompleteGpuHeightmapMaterial>, &Transform),
        (With<GpuHeightmapTerrain>, Without<RiverMaskTerrain>),
    >,
    mut target: ResMut<RiverMaskTarget>,
) {
    // Create offscreen texture if needed
    if target.image == Handle::default() {
        let size = 1024;
        let mut image = Image::new_fill(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 255, 0, 255], // GREEN debug fill
            TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::all(),
        );
        image.texture_descriptor.usage |=
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC | TextureUsages::TEXTURE_BINDING;
        target.image = images.add(image);
        info!("Created offscreen river mask image ({}x{}).", size, size);
    }

    // Spawn dedicated offscreen camera (red clear) if absent
    if mask_cam_q.is_empty() {
        use bevy::render::camera::{PerspectiveProjection, Projection};
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: 100,
                target: bevy::render::camera::RenderTarget::Image(target.image.clone().into()),
                clear_color: ClearColorConfig::Custom(Color::srgb(1.0, 0.0, 0.0)), // RED clear
                ..Default::default()
            },
            Projection::from(PerspectiveProjection {
                fov: std::f32::consts::FRAC_PI_4,
                near: 0.1,
                far: 10_000.0,
                aspect_ratio: 1.0,
            }),
            Transform::from_xyz(0.0, 50.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            RiverMaskCamera,
        ));
        info!("Spawned river mask camera.");
    }

    // Clone terrain with mask material (once)
    if mask_terrain_q.is_empty() {
        if let Ok((mesh3d, mat3d, transform)) = original_terrain_q.get_single() {
            let (base_clone, ext_clone) = {
                if let Some(orig) = materials.get(&mat3d.0) {
                    let mut ext = orig.extension.clone();
                    let dbg = ext.debug_options;
                    // Force mask mode (z=1.0), keep existing margin step (dbg.y)
                    ext.debug_options = Vec4::new(0.0, dbg.y, 1.0, 0.0);
                    (orig.base.clone(), ext)
                } else {
                    return;
                }
            };
            let mask_mat = materials.add(CompleteGpuHeightmapMaterial {
                base: base_clone,
                extension: ext_clone,
            });

            commands.spawn((
                Mesh3d(mesh3d.0.clone()),
                MeshMaterial3d(mask_mat),
                *transform,
                Visibility::default(),
                InheritedVisibility::default(),
                RiverMaskTerrain,
            ));
            info!("Cloned terrain for mask pass.");
        }
    }
}

/* ---------------------- Camera Pose Synchronization ------------------- */

fn sync_mask_camera_transform(
    main_cam: Query<&Transform, (With<Camera3d>, Without<RiverMaskCamera>)>,
    mut mask_cam: Query<&mut Transform, With<RiverMaskCamera>>,
) {
    if let (Ok(src), Ok(mut dst)) = (main_cam.get_single(), mask_cam.get_single_mut()) {
        *dst = *src;
    }
}

/* -------------------- Render World: Queue Readback -------------------- */

fn queue_river_mask_readback(
    mut render_state: ResMut<RiverMaskRenderState>,
    target: Res<RiverMaskTarget>, // extracted clone NOT automatic; main & render share same handle object (Arc behind)
    gpu_images: Res<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    sender: Res<RiverMaskReadbackSender>,
) {
    // Only proceed if a capture is requested & not yet submitted
    if !target.request_capture || render_state.copy_submitted {
        return;
    }

    let Some(gpu_image) = gpu_images.get(&target.image) else {
        // Not yet uploaded/extracted this frame
        return;
    };

    let width = gpu_image.size.x;
    let height = gpu_image.size.y;
    let bytes_per_pixel = 4u32;
    let unpadded_bytes_per_row = width * bytes_per_pixel;

    // Align bytes_per_row to 256 (WebGPU requirement)
    let align = 256u32;
    let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
    let padded_size = (padded_bytes_per_row * height) as u64;

    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("river_mask_readback_buffer"),
        size: padded_size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Encode copy command
    let mut encoder =
        render_device.create_command_encoder(&CommandEncoderDescriptor { label: Some("river_mask_copy_encoder") });

    encoder.copy_texture_to_buffer(
        ImageCopyTexture {
            texture: &gpu_image.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        ImageCopyBuffer {
            buffer: &buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    render_queue.submit(std::iter::once(encoder.finish()));

    // Prepare mapping closure
    let slice = buffer.slice(..);
    let tx = sender.tx.clone();
    let path = target
        .path
        .clone()
        .unwrap_or_else(|| "river_mask.png".to_string());

    slice.map_async(MapMode::Read, move |res| {
        if res.is_ok() {
            let data_view = slice.get_mapped_range();
            // De-pad rows
            let mut out = Vec::with_capacity((width * height * bytes_per_pixel) as usize);
            for row in 0..height {
                let start = (row * padded_bytes_per_row) as usize;
                let end = start + unpadded_bytes_per_row as usize;
                out.extend_from_slice(&data_view[start..end]);
            }
            drop(data_view); // release view
            // Buffer auto-unmapped when dropped (slice drop not enough; explicit unmap would need buffer, but its lifetime ends after closure)
            // Send to main world
            let _ = tx.send(ReadbackMsg {
                path,
                width,
                height,
                data: out,
            });
        }
        // else: ignore error
    });

    render_state.copy_submitted = true;
    render_state.last_path = target.path.clone();
    info!("Queued river mask GPU readback.");
}

/* -------------------- Main World: Poll & Save PNG --------------------- */

fn poll_readback_and_save(
    target: Res<RiverMaskTarget>,
    chan: Res<RiverMaskReadbackChannel>,
    mut render_state: ResMut<RiverMaskRenderState>, // we reuse same struct (shared via Arc internal)
) {
    if !target.request_capture {
        // Nothing requested
        return;
    }
    // Drain all completed messages (usually one)
    while let Ok(msg) = chan.rx.try_recv() {
        if let Err(e) = save_rgba_png(&msg.path, msg.width, msg.height, &msg.data) {
            error!("River mask save failed: {e}");
        } else {
            info!("River mask saved: {}", msg.path);
        }
        // Allow another capture
        render_state.copy_submitted = false;
    }
}

/* ------------------------------- Helpers ------------------------------- */

fn save_rgba_png(path: &str, w: u32, h: u32, bytes: &[u8]) -> Result<(), String> {
    use image::{ImageBuffer, Rgba};
    if bytes.len() < (w * h * 4) as usize {
        return Err("Byte slice size mismatch".into());
    }
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(w, h, bytes.to_vec()).ok_or("Failed to build ImageBuffer")?;
    img.save(path).map_err(|e| e.to_string())
}

/* ------------------------------ Notes ----------------------------------

Flow:
1. ensure_river_mask_setup creates:
   - Offscreen texture
   - Offscreen camera (RED clear)
   - Duplicated terrain using mask mode (debug_options.z = 1)
2. User clicks "Capture River Mask":
   - Sets target.request_capture = true and target.path
3. queue_river_mask_readback (Render schedule):
   - Copies offscreen texture into a padded GPU buffer
   - Maps it asynchronously; on completion sends RGBA rows through channel
4. poll_readback_and_save (main world):
   - Receives bytes and writes PNG
   - Resets render_state.copy_submitted so future captures work

If final image is RED:
- Camera rendered but mask terrain didn't (material update overwrote debug_options.z)
If final image shows expected grayscale values: success.
If still GREEN: you are accidentally saving before first capture (ensure request made) or no copy system ran.

Remember to preserve debug_options.z for the cloned mask material in your main material update system:
if material.extension.debug_options.z > 0.5 { keep z = 1.0 }

------------------------------------------------------------------------- */