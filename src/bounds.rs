use std::convert::TryInto;

use amethyst_core::{
    math::Point3,
    Transform,
};
use amethyst_rendy::system::GraphAuxData;

use crate::{
    storage::VoxelStorage,
    Voxel,
    VoxelMap,
};

/// Axis aligned quantized region of space represented in tile coordinates of
/// `i32`. This behaves like a bounding box volume with `min` and `max`
/// coordinates for iteration. The lower (min) coordinates are inclusive and the
/// upper (max) coordinates are exclusive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounds {
    min: Point3<i32>,
    max: Point3<i32>,
}

impl Bounds {
    /// Create a new `Region` with the given top-left and bottom-right cubic
    /// coordinates.
    pub fn new(min: Point3<i32>, max: Point3<i32>) -> Self {
        assert!(min.x <= max.x);
        assert!(min.y <= max.y);
        assert!(min.z <= max.z);

        Self { min, max }
    }

    /// Returns an empty `Region`
    pub fn empty() -> Self {
        Self {
            min: Point3::origin(),
            max: Point3::origin(),
        }
    }

    pub fn min(&self) -> Point3<i32> {
        self.min
    }

    pub fn max(&self) -> Point3<i32> {
        self.max
    }

    pub fn center(&self) -> Point3<i32> {
        self.min + self.max.coords / 2
    }

    /// Check if this cube contains the provided coordinate.
    pub fn contains(&self, target: &Point3<i32>) -> bool {
        target.x >= self.min.x
            && target.x < self.max.x
            && target.y >= self.min.y
            && target.y < self.max.y
            && target.z >= self.min.z
            && target.z < self.max.z
    }

    /// Check if this `Region` intersects with the provided `Region`
    pub fn intersects(&self, other: &Self) -> bool {
        (self.min.x < other.max.x && self.max.x > other.min.x)
            && (self.min.y < other.max.y && self.max.y > other.min.y)
            && (self.min.z < other.max.z && self.max.z > other.min.z)
    }

    /// Calculate the volume of this bounding box volume.
    pub fn volume(&self) -> u32 {
        let v = (self.max.x - self.min.x) * (self.max.y - self.min.y) * (self.max.z - self.min.z);

        // The constructor ensures the invariant that the components of `max` are >=
        // than `min`. Thus this can't panic.
        v.try_into().unwrap()
    }

    /// Create a linear iterator across this region.
    pub fn iter(&self) -> BoundsLinearIter {
        BoundsLinearIter::new(self.clone())
    }
}

impl<'a> IntoIterator for &'a Bounds {
    type Item = Point3<i32>;
    type IntoIter = BoundsLinearIter;

    #[must_use]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Linear iterator across a 3D coordinate space.
/// This iterator is inclusive of minimum and maximum coordinates.
pub struct BoundsLinearIter {
    track: Point3<i32>,
    bounds: Bounds,
}
impl BoundsLinearIter {
    /// Create a new iterator.
    pub fn new(bounds: Bounds) -> Self {
        let track = bounds.min;
        Self { bounds, track }
    }
}
impl Iterator for BoundsLinearIter {
    type Item = Point3<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.track;

        if self.track.z >= self.bounds.max.z {
            return None;
        }

        self.track.x += 1;
        if self.track.x >= self.bounds.max.x {
            self.track.x = self.bounds.min.x;
            self.track.y += 1;
            if self.track.y >= self.bounds.max.y {
                self.track.y = self.bounds.min.y;
                self.track.z += 1;
            }
        }

        Some(ret)
    }
}

pub(crate) fn compute_render_bounds<V: Voxel, S: VoxelStorage<V>, Z: DrawVoxelsBounds>(
    voxel_map: &VoxelMap<V, S>,
    map_transform: Option<&Transform>,
    aux: &GraphAuxData,
) -> Bounds {
    let render_bounds = Z::bounds(voxel_map, map_transform, aux);
    let map_bounds = voxel_map.bounds();

    match (render_bounds, map_bounds) {
        (None, None) => {
            log::error!("Computed infinite bounds for VoxelMap rendering");
            Bounds::empty()
        }
        (Some(render_bounds), None) => render_bounds,
        (None, Some(map_bounds)) => map_bounds,
        (Some(render_bounds), Some(map_bounds)) => {
            Bounds::new(
                Point3::new(
                    //bounds.min.x.max(min.x).min(max.x),
                    render_bounds
                        .min
                        .x
                        .clamp(map_bounds.min.x, map_bounds.max.x),
                    render_bounds
                        .min
                        .y
                        .clamp(map_bounds.min.y, map_bounds.max.y),
                    render_bounds
                        .min
                        .z
                        .clamp(map_bounds.min.z, map_bounds.max.z),
                ),
                Point3::new(
                    render_bounds
                        .max
                        .x
                        .clamp(map_bounds.min.x, map_bounds.max.x),
                    render_bounds
                        .max
                        .y
                        .clamp(map_bounds.min.y, map_bounds.max.y),
                    render_bounds
                        .max
                        .z
                        .clamp(map_bounds.min.z, map_bounds.max.z),
                ),
            )
        }
    }
}

pub trait DrawVoxelsBounds: 'static + std::fmt::Debug + Send + Sync {
    fn bounds<V: Voxel, S: VoxelStorage<V>>(
        map: &VoxelMap<V, S>,
        map_transform: Option<&Transform>,
        aux: &GraphAuxData,
    ) -> Option<Bounds>;
}

#[derive(Debug, Default)]
pub struct DrawVoxelsBoundsDefault;

impl DrawVoxelsBounds for DrawVoxelsBoundsDefault {
    fn bounds<V: Voxel, S: VoxelStorage<V>>(
        _map: &VoxelMap<V, S>,
        _map_transform: Option<&Transform>,
        _aux: &GraphAuxData,
    ) -> Option<Bounds> {
        None
    }
}
