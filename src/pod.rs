#![allow(clippy::default_trait_access)]
//! GPU POD data types.

use amethyst_core::math::Point3;
use amethyst_rendy::{
    pod::IntoPod,
    rendy::{
        hal::format::Format,
        mesh::{
            AsVertex,
            TexCoord,
            VertexFormat,
        },
    },
    resources::Tint as TintComponent,
};
use glsl_layout::{
    ivec3,
    mat4,
    uint,
    vec2,
    vec3,
    vec4,
    Uniform,
};

/// POD for rendering a voxel map.
///
/// ```glsl
/// layout(std140, set = 0, binding = 0) uniform VoxelMapArgs {
///     uniform mat4 proj;
///     uniform mat4 view;
///     uniform mat4 map_coordinate_transform;
///     uniform mat4 map_transform;
///     uniform vec3 voxel_dimensions;
/// };
/// ```
#[derive(Clone, Copy, Debug, Uniform)]
#[repr(C, align(16))]
pub struct VoxelMapArgs {
    /// Projection matrix
    pub proj: mat4,
    /// View matrix
    pub view: mat4,
    /// Projection matrix
    pub map_coordinate_transform: mat4,
    /// View matrix
    pub map_transform: mat4,
    /// Voxel dimensions. Because we assume tiles are uniform for a map, we can
    /// store these here.
    pub voxel_dimensions: vec3,
}

/// POD for rendering a single voxel face.
///
/// ```glsl
/// layout(location = 0) in vec2 tex_top_left;
/// layout(location = 1) in vec2 tex_bottom_right;
/// layout(location = 2) in vec4 color;
/// layout(location = 3) in ivec3 voxel_coordinate;
/// layout(location = 4) in uint face;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Uniform)]
#[repr(C, align(16))]
pub struct VoxelArgs {
    /// Upper-left coordinate of the sprite in the spritesheet
    pub u_offset: vec2,
    /// Bottom-right coordinate of the sprite in the spritesheet
    pub v_offset: vec2,
    /// Tint for this this sprite
    pub tint: vec4,
    /// Voxel coordinate
    pub voxel_coordinate: ivec3,
    // /// Face
    pub face: uint,
}

impl AsVertex for VoxelArgs {
    #[must_use]
    fn vertex() -> VertexFormat {
        VertexFormat::new((
            (Format::Rg32Sfloat, "u_offset"),
            (Format::Rg32Sfloat, "v_offset"),
            (Format::Rgba32Sfloat, "tint"),
            (Format::Rgb32Sint, "voxel_coordinate"),
            (Format::R32Uint, "face"), // TODO: How do you use a R8Uint here?
        ))
    }
}

impl VoxelArgs {
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn from_data<'a>(
        tex_coords: &'a [TexCoord; 2],
        tint: Option<&TintComponent>,
        voxel_coordinate: &Point3<i32>,
        face: usize,
    ) -> Self {
        Self {
            u_offset: [tex_coords[0].0[0], tex_coords[0].0[1]].into(),
            v_offset: [tex_coords[1].0[0], tex_coords[1].0[1]].into(),
            tint: tint.map_or([1.0; 4].into(), |t| t.0.into_pod()),
            voxel_coordinate: [voxel_coordinate.x, voxel_coordinate.y, voxel_coordinate.z].into(),
            face: (face as u32).into(),
        }
    }
}
