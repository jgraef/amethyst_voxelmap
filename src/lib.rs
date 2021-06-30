//! Provides a `RenderPlugin` to render `VoxelMap` components. A `VoxelMap` is
//! a volumetric 3D structure containing cubic voxels. It is analogue to
//! [`amethyst_tilemap`](https://github.com/amethyst/amethyst/tree/main/amethyst_tiles)
//! in three dimensions.
//!
//! ![Screenshot](/examples/screenshot.png)

#![allow(dead_code)]

pub mod bounds;
pub mod map;
pub mod pass;
mod pod;
pub mod storage;

pub use amethyst_tiles::{
    CoordinateEncoder,
    FlatEncoder,
    MortonEncoder,
};
pub use bounds::{
    DrawVoxelsBounds,
    DrawVoxelsBoundsDefault,
};
pub use map::{
    RenderVoxels,
    Voxel,
    VoxelMap,
};
