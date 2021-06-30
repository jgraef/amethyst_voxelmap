use std::marker::PhantomData;

use amethyst_assets::Handle;
use amethyst_core::{
    ecs::{
        Resources,
        World,
    },
    math::{
        Matrix4,
        Point3,
        Vector3,
    },
};
use amethyst_rendy::{
    bundle::{
        RenderOrder,
        RenderPlan,
        Target,
    },
    palette::Srgba,
    system::GraphAuxData,
    Backend,
    Factory,
    RenderGroupDesc,
    RenderPlugin,
    SpriteSheet,
};
// TODO: Copy these, such that we don't depend on the `tiles` feature.
pub use amethyst_tiles::{
    CoordinateEncoder,
    Region,
};
use derivative::Derivative;

pub use crate::{
    bounds::{
        DrawVoxelsBounds,
        DrawVoxelsBoundsDefault,
    },
    pass::{
        DrawVoxels,
        DrawVoxelsDesc,
    },
    storage::VoxelStorage,
};

pub trait Voxel: 'static + Clone + Default + Send + Sync {
    fn occupied(&self, coordinates: &Point3<i32>, world: &World, resources: &Resources) -> bool;

    /// Index into texture coordinates of VoxelMap for each face (+x, -x, +y,
    /// -y, +z, -z)
    fn texture(
        &self,
        coordinates: &Point3<i32>,
        world: &World,
        resources: &Resources,
    ) -> Option<[usize; 6]>;

    fn tint(
        &self,
        _coordinates: &Point3<i32>,
        _world: &World,
        _resources: &Resources,
    ) -> Option<[Srgba; 6]> {
        None
    }

    // TODO: We might be able to remove this with `occupied`. `occupied` should be
    // renamed to reflect that it's being used to determine whether the faces of
    // neighboring voxels need rendering (because for a voxel without
    // transparent faces they occlude neighbouring faces)
    fn neighbor_culling(
        &self,
        _coordinates: &Point3<i32>,
        _world: &World,
        _resources: &Resources,
    ) -> [bool; 6] {
        [true, true, true, true, true, true]
    }
}

#[derive(Debug)]
pub struct VoxelMap<V: Voxel, S: VoxelStorage<V>> {
    /// Voxel data
    data: S,

    // Sprite sheet containing the face textures.
    pub(crate) sprite_sheet: Handle<SpriteSheet>,

    /// Transform applied to the map before the actual Transform component. This
    /// is used to center
    // the rendered VoxelMap.
    pub(crate) transform: Matrix4<f32>,

    _marker: PhantomData<V>,
}

impl<V: Voxel, S: VoxelStorage<V>> VoxelMap<V, S> {
    pub fn new(data: S, sprite_sheet: Handle<SpriteSheet>) -> Self {
        let translation = data
            .bounds()
            .map(|bounds| -bounds.center().coords.map(|x| x as f32))
            .unwrap_or_default();
        let transform = Matrix4::new_translation(&translation);

        Self {
            data,
            transform,
            sprite_sheet,
            _marker: PhantomData,
        }
    }

    /// Returns array of `bool`s describing which face has a neighbouring voxel
    /// and thus can be culled.
    pub(crate) fn get_neighbors(&self, coords: Point3<i32>, aux: &GraphAuxData) -> [bool; 6] {
        let mut exists = [false; 6];

        let voxel_exists = |d| {
            self.get(&(coords + &d))
                .map(|voxel| voxel.occupied(&coords, &aux.world, &aux.resources))
                .unwrap_or(false)
        };

        exists[1] = voxel_exists(Vector3::new(0, 0, -1));
        exists[0] = voxel_exists(Vector3::new(0, 0, 1));
        exists[3] = voxel_exists(Vector3::new(0, -1, 0));
        exists[2] = voxel_exists(Vector3::new(0, 1, 0));
        exists[4] = voxel_exists(Vector3::new(-1, 0, 0));
        exists[5] = voxel_exists(Vector3::new(1, 0, 0));

        exists
    }
}

impl<V: Voxel, S: VoxelStorage<V>> VoxelStorage<V> for VoxelMap<V, S> {
    fn origin(&self) -> Point3<i32> {
        self.data.origin()
    }

    fn dimensions(&self) -> Vector3<u32> {
        self.data.dimensions()
    }

    fn get(&self, coord: &Point3<i32>) -> Option<&V> {
        self.data.get(coord)
    }

    fn get_mut(&mut self, coord: &Point3<i32>) -> Option<&mut V> {
        self.data.get_mut(coord)
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug(bound = ""), Default(bound = ""))]
pub struct RenderVoxels<V: Voxel, S: VoxelStorage<V>, Z: DrawVoxelsBounds = DrawVoxelsBoundsDefault>
{
    target: Target,
    _marker: PhantomData<(V, S, Z)>,
}

impl<B: Backend, V: Voxel, S: VoxelStorage<V>, Z: DrawVoxelsBounds> RenderPlugin<B>
    for RenderVoxels<V, S, Z>
{
    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
        _resources: &Resources,
    ) -> Result<(), amethyst_error::Error> {
        plan.extend_target(self.target, |ctx| {
            ctx.add(
                RenderOrder::BeforeTransparent,
                DrawVoxelsDesc::<V, S, Z>::default().builder(),
            )?;
            Ok(())
        });
        Ok(())
    }
}
