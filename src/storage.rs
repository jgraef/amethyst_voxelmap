use std::convert::TryInto;

use amethyst_core::math::{
    Point3,
    Vector3,
};

use crate::{
    bounds::Bounds,
    map::CoordinateEncoder,
    Voxel,
};

/// Trait that provides access to voxels. You can use [`VecStorage`] to store
/// the voxels in an array in-memory, or implement your own provider (e.g. by
/// loading voxels from a file).
pub trait VoxelStorage<V>: Send + Sync + 'static {
    // TODO: Deprecate in favor of `bounds`
    fn origin(&self) -> Point3<i32>;

    // TODO: Deprecate in favor of `bounds`
    fn dimensions(&self) -> Vector3<u32>;

    fn bounds(&self) -> Option<Bounds> {
        let min = self.origin();
        let dimensions = self.dimensions().map(|x| x.try_into().unwrap());
        let max = min + dimensions;
        Some(Bounds::new(min, max))
    }

    fn get(&self, coord: &Point3<i32>) -> Option<&V>;

    fn get_mut(&mut self, coord: &Point3<i32>) -> Option<&mut V>;
}

#[derive(Clone, Debug)]
pub struct VecStorage<V, E> {
    dimensions: Vector3<u32>,
    voxels: Vec<V>,
    encoder: E,
}

impl<V: Voxel, E: CoordinateEncoder> VecStorage<V, E> {
    pub fn from_dimensions(dimensions: Vector3<u32>) -> Self {
        let encoder = E::from_dimensions(dimensions);
        let num_voxels = E::allocation_size(dimensions);
        let mut voxels = Vec::with_capacity(num_voxels);
        voxels.resize_with(num_voxels, V::default);

        Self {
            dimensions,
            voxels,
            encoder,
        }
    }
}

impl<V: Voxel, E: CoordinateEncoder> VoxelStorage<V> for VecStorage<V, E> {
    fn origin(&self) -> Point3<i32> {
        Point3::origin()
    }

    fn dimensions(&self) -> Vector3<u32> {
        self.dimensions
    }

    fn get(&self, coord: &Point3<i32>) -> Option<&V> {
        let x = coord.x.try_into().ok()?;
        let y = coord.y.try_into().ok()?;
        let z = coord.z.try_into().ok()?;

        let index = self.encoder.encode(x, y, z)?;
        self.voxels.get(index as usize)
    }

    fn get_mut(&mut self, coord: &Point3<i32>) -> Option<&mut V> {
        let x = coord.x.try_into().ok()?;
        let y = coord.y.try_into().ok()?;
        let z = coord.z.try_into().ok()?;

        let index = self.encoder.encode(x, y, z)?;
        self.voxels.get_mut(index as usize)
    }
}
