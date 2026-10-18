//! # kh2-fx
//!
//! Plays Kingdom Hearts II effect packages (PAX elements → DPX effects →
//! DPD particle data) the way the game's "ppp" particle machine does, and
//! hands out world-space triangles to draw. Written to draw KH2's real
//! Explosion and Magnet Burst inside another game; the behaviour follows
//! `docs/kh2-effects.md`.
//!
//! ```no_run
//! use std::sync::Arc;
//! use glam::{Mat4, Vec3};
//! use kh2_fx::{Camera, Effects, Frame, Library};
//!
//! # fn bytes(_: &str) -> Vec<u8> { vec![] }
//! // Once: Sora's BAR (or the raw PAX) and the shared effect textures.
//! let lib = Arc::new(Library::load(&bytes("P_EX100.a.us"), Some(&bytes("texcommon.dpd")))?);
//! let mut fx = Effects::new(lib.clone());
//! // Upload lib.textures() (RGBA8, id = index) to the GPU.
//!
//! // When the motion's trigger fires:
//! let mut bones = |code: u16| -> Option<Mat4> { Some(Mat4::IDENTITY) };
//! let h = fx.start(44, &mut bones);
//!
//! // Every game frame:
//! fx.tick(1.0, &mut bones);
//! let cam = Camera::look_at(Vec3::new(0.0, -100.0, 300.0), Vec3::new(0.0, -100.0, 0.0), Vec3::NEG_Y);
//! let mut frame = Frame::new();
//! fx.build(&cam, &mut frame);
//! for b in &frame.batches {
//!     let tris = frame.batch_vertices(b); // draw with b.texture, b.blend, depth flags
//! }
//! # Ok::<(), kh2_fx::LoadError>(())
//! ```
//!
//! ## Conventions
//!
//! - **Space**: KH2 world units (about 1 cm) and axes, **y down**. Bone
//!   matrices in, vertex positions out, both world space, glam column
//!   vectors.
//! - **Time**: [`Effects::tick`] takes 60 Hz game frames; effects tick at
//!   30 Hz internally (one tick per 2.0 frames).
//! - **Camera**: only [`Effects::build`] needs it ([`Camera`]: view space
//!   x right, y down, z forward; constructors for KH2 and D3D-style views).
//! - **Colour and blending**: see [`output`] (final colours with every
//!   multiplier applied, 1.0 = 0x80; GS blend equation per batch).
//!
//! ## What runs
//!
//! Every program type used by Explosion, Magnet Burst, Guard Break,
//! Finishing Leap and Aerial Finish (see [`supported`]): the integrators
//! (Point, Angle, Scale, Color and their Move / Accele forms), RandFloat,
//! RandIV, the matrix builders (XYZ, YXZ, ZXY, Scl, ParMatrix, DrawMatrix,
//! DrawMatrixFront, DrawMatrixWood, RyjDrawMatrixWorld, DrawMatrixLoop,
//! KeZCrct, KeZCrctShp), DrawShape(X), DrawMdl3, DrawMdlTs3, DrawMdlLoopZ,
//! VtMime, VertexAp, VertexApLc, RyjMegaBirth and EiDrawShipoly,
//! plus MatrixLoop and VertexApLcLoop (the same routines as MatrixXYZ and
//! VertexApLc in the game's table).
//! Other types are skipped and counted ([`Effects::skipped_programs`],
//! [`Library::unsupported_programs`]).
//!
//! ## Approximations
//!
//! - The game's "Z correction" (sprites drawn with their depth pulled
//!   toward the camera) is reported per batch as
//!   [`Batch::depth_offset`] for the renderer to apply to depth only.
//! - Depth-sorted draws (rare here) go after the immediate ones, farthest
//!   first; the game's ordering-table placement is inferred.
//! - Colours follow the PS2 modulate rule the PC's particle path uses; the
//!   PC shaders themselves were not read. The PC's HD texture replacements
//!   are not used (PS2 textures are decoded instead).
//! - Not used: the particle data's owner-scale and off-screen-pause flags,
//!   element sounds, two-point elements; EiDrawShipoly's scroll-draw
//!   variant and space modes 2/3 (unused by these effects).
//!
//! Format knowledge for the containers comes from OpenKH (Apache-2.0)
//! through `kh2-formats`; see `NOTICE` and `CREDITS.md`.

pub mod camera;
mod draw;
pub mod effects;
pub mod library;
pub mod math;
mod megabirth;
pub mod output;
mod player;
pub mod rng;
mod shipoly;

pub use camera::Camera;
pub use effects::{element_transform, EffectHandle, Effects};
pub use library::{Library, LoadError, Texture, TextureId, TextureSource};
pub use output::{Batch, Blend, BlendMode, D3dFactor, D3dOp, Frame, Vertex};
pub use player::supported;
pub use rng::Rng;
