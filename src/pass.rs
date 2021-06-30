use std::marker::PhantomData;

use amethyst_assets::AssetStorage;
use amethyst_core::{
    ecs::{
        component,
        IntoQuery,
        TryRead,
    },
    math::Matrix4,
    Hidden,
    Transform,
};
use amethyst_rendy::{
    batch::{
        GroupIterator,
        OrderedTwoLevelBatch,
    },
    pipeline::{
        PipelineDescBuilder,
        PipelinesBuilder,
    },
    rendy::{
        command::{
            QueueId,
            RenderPassEncoder,
        },
        graph::{
            render::{
                PrepareResult,
                RenderGroup,
            },
            GraphContext,
            NodeBuffer,
            NodeImage,
        },
        hal::{
            self,
            device::Device,
            pso::Face,
        },
        mesh::{
            AsVertex,
            TexCoord,
        },
        shader::{
            ShaderSetBuilder,
            SpirvShader,
        },
    },
    resources::Tint,
    sprite::Sprites,
    submodules::{
        gather::CameraGatherer,
        DynamicUniform,
        DynamicVertexBuffer,
        TextureId,
        TextureSub,
    },
    system::GraphAuxData,
    Backend,
    ChangeDetection,
    Factory,
    RenderGroupDesc,
    SpriteSheet,
};
use derivative::Derivative;
use glsl_layout::Uniform;
use lazy_static::lazy_static;
use smallvec::SmallVec;

use crate::{
    bounds::{
        compute_render_bounds,
        DrawVoxelsBounds,
        DrawVoxelsBoundsDefault,
    },
    pod::{
        VoxelArgs,
        VoxelMapArgs,
    },
    storage::VoxelStorage,
    Voxel,
    VoxelMap,
};

lazy_static! {
    static ref VERTEX: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../shaders/compiled/voxels.vert.spv"),
        //&std::fs::read("amethyst_voxelmap/shaders/compiled/voxels.vert.spv").unwrap(),
        hal::pso::ShaderStageFlags::VERTEX,
        "main",
    )
    .unwrap();
    static ref FRAGMENT: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../shaders/compiled/voxels.frag.spv"),
        //&std::fs::read("amethyst_voxelmap/shaders/compiled/voxels.vert.spv").unwrap(),
        hal::pso::ShaderStageFlags::FRAGMENT,
        "main",
    )
    .unwrap();
    static ref SHADERS: ShaderSetBuilder = ShaderSetBuilder::default()
        .with_vertex(&*VERTEX)
        .unwrap()
        .with_fragment(&*FRAGMENT)
        .unwrap();
}

/// Draw opaque voxelmap without lighting.
#[derive(Clone, PartialEq, Derivative)]
#[derivative(Default(bound = ""), Debug(bound = ""))]
pub struct DrawVoxelsDesc<
    V: Voxel,
    S: VoxelStorage<V>,
    Z: DrawVoxelsBounds = DrawVoxelsBoundsDefault,
> {
    #[derivative(Debug = "ignore")]
    _marker: PhantomData<(V, S, Z)>,
}

impl<B: Backend, V: Voxel, S: VoxelStorage<V>, Z: DrawVoxelsBounds> RenderGroupDesc<B, GraphAuxData>
    for DrawVoxelsDesc<V, S, Z>
{
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &GraphAuxData,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, GraphAuxData>>, hal::pso::CreationError> {
        #[cfg(feature = "profiler")]
        profile_scope!("build");

        let env: DynamicUniform<B, VoxelMapArgs> =
            DynamicUniform::new(factory, hal::pso::ShaderStageFlags::VERTEX)?;

        let textures = TextureSub::new(factory)?;
        let vertex = DynamicVertexBuffer::new();

        let (pipeline, pipeline_layout) = build_voxels_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout(), textures.raw_layout()],
        )?;

        Ok(Box::new(DrawVoxels::<B, V, S, Z> {
            pipeline,
            pipeline_layout,
            textures,
            vertex,
            env: vec![env],
            batch: Default::default(),
            _marker: PhantomData::default(),
            change: Default::default(),
        }))
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub struct DrawVoxels<
    B: Backend,
    V: Voxel,
    S: VoxelStorage<V>,
    Z: DrawVoxelsBounds = DrawVoxelsBoundsDefault,
> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    textures: TextureSub<B>,
    vertex: DynamicVertexBuffer<B, VoxelArgs>,
    batch: OrderedTwoLevelBatch<TextureId, usize, VoxelArgs>,
    change: ChangeDetection,

    env: Vec<DynamicUniform<B, VoxelMapArgs>>,

    #[derivative(Debug = "ignore")]
    _marker: PhantomData<(V, S, Z)>,
}

impl<B: Backend, V: Voxel, S: VoxelStorage<V>, Z: DrawVoxelsBounds> RenderGroup<B, GraphAuxData>
    for DrawVoxels<B, V, S, Z>
{
    #[allow(clippy::cast_precision_loss)]
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        aux: &GraphAuxData,
    ) -> PrepareResult {
        #[cfg(feature = "profiler")]
        profile_scope!("prepare");

        let mut changed = false;

        let sprite_sheet_storage = aux
            .resources
            .get::<AssetStorage<SpriteSheet>>()
            .expect("AssetStorage<SpriteSheet> missing");

        let sprites_storage = aux
            .resources
            .get::<AssetStorage<Sprites>>()
            .expect("AssetStorage<Sprites> missing");

        let textures_ref = &mut self.textures;
        let batch_ref = &mut self.batch;

        batch_ref.swap_clear();

        let CameraGatherer { projview, .. } = CameraGatherer::gather(aux.world, aux.resources);

        let mut voxelmap_args = vec![];

        let mut query =
            <(&VoxelMap<V, S>, TryRead<Transform>)>::query().filter(!component::<Hidden>());

        for (voxel_map, transform) in query.iter(aux.world) {
            if let Some((sprite_sheet, sprites)) = sprite_sheet_storage
                .get(&voxel_map.sprite_sheet)
                .and_then(|sprite_sheet| {
                    Some((sprite_sheet, {
                        if let Some(sprites) = sprites_storage.get(&sprite_sheet.sprites) {
                            Some(sprites)
                        }
                        else {
                            log::error!("No Sprites found in SpritesStorage");
                            None
                        }
                    }?))
                })
            {
                let sprites = sprites.build_sprites();

                let voxelmap_args_index = voxelmap_args.len();

                let map_coordinate_transform: [[f32; 4]; 4] = voxel_map.transform.into();

                let map_transform: [[f32; 4]; 4] = transform.map_or_else(
                    || Matrix4::identity().into(),
                    |transform| (*transform.global_matrix()).into(),
                );

                voxelmap_args.push(VoxelMapArgs {
                    proj: projview.proj,
                    view: projview.view,
                    map_coordinate_transform: map_coordinate_transform.into(),
                    map_transform: map_transform.into(),
                    // TODO: Remove. This is unnecessarty, since you can just scale the
                    // while VoxelMap.
                    voxel_dimensions: [1.0, 1.0, 1.0].into(),
                });

                compute_render_bounds::<V, S, Z>(&voxel_map, transform, aux)
                    .iter()
                    .filter_map(|coord| {
                        let voxel = voxel_map.get(&coord).unwrap();

                        if let Some(tex_indices) = voxel.texture(&coord, aux.world, aux.resources) {
                            let tint = voxel.tint(&coord, aux.world, aux.resources);
                            let mut batch_datas = SmallVec::<[(TextureId, VoxelArgs); 6]>::new();

                            let neighbors = voxel_map.get_neighbors(coord, aux);
                            let neighbor_culling =
                                voxel.neighbor_culling(&coord, &aux.world, &aux.resources);

                            for face in 0..6 {
                                if !neighbors[face] || !neighbor_culling[face] {
                                    let (tex_id, this_changed) = {
                                        let r = textures_ref.insert(
                                            factory,
                                            aux.resources,
                                            &sprite_sheet.texture,
                                            hal::image::Layout::ShaderReadOnlyOptimal,
                                        );

                                        if r.is_none() {
                                            log::error!(
                                                "Texture missing: {:?}",
                                                sprite_sheet.texture
                                            );
                                        }

                                        r?
                                    };
                                    changed = changed || this_changed;

                                    let tint = tint.map(|t| Tint(t[face].clone()));

                                    let sprite = sprites
                                        .get(tex_indices[face])
                                        .expect("Sprite number out of range");
                                    let tex_coords = [
                                        TexCoord([sprite.tex_coords.left, sprite.tex_coords.top]),
                                        TexCoord([
                                            sprite.tex_coords.right,
                                            sprite.tex_coords.bottom,
                                        ]),
                                    ];

                                    let batch_data = VoxelArgs::from_data(
                                        &tex_coords,
                                        //Some(&TintComponent(tile.tint(coord, aux.world))),
                                        tint.as_ref(),
                                        &coord,
                                        face,
                                    );

                                    batch_datas.push((tex_id, batch_data));
                                }
                            }

                            return Some(batch_datas);
                        }
                        None
                    })
                    .flatten()
                    .for_each_group(|tex_id, batch_data| {
                        batch_ref.insert(tex_id, voxelmap_args_index, batch_data.drain(..))
                    });
            }
        }

        self.textures.maintain(factory, aux.resources);
        changed = changed || batch_ref.changed();

        {
            #[cfg(feature = "profiler")]
            profile_scope!("write");
            self.vertex.write(
                factory,
                index,
                batch_ref.count() as u64,
                Some(batch_ref.data()),
            );

            // grow tilemap_args cache if necessary, or shrink it
            if self.env.len() < voxelmap_args.len() || self.env.len() <= voxelmap_args.len() / 2 {
                self.env.resize_with(voxelmap_args.len(), || {
                    DynamicUniform::new(factory, hal::pso::ShaderStageFlags::VERTEX).unwrap()
                });
            }

            for (env, voxelmap_args) in self.env.iter_mut().zip(&voxelmap_args) {
                env.write(factory, index, voxelmap_args.std140());
            }
        }

        self.change.prepare_result(index, changed)
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _aux: &GraphAuxData,
    ) {
        #[cfg(feature = "profiler")]
        profile_scope!("draw");

        let layout = &self.pipeline_layout;
        encoder.bind_graphics_pipeline(&self.pipeline);

        self.vertex.bind(index, 0, 0, &mut encoder);
        for (&tex, ranges) in self.batch.iter() {
            if self.textures.loaded(tex) {
                self.textures.bind(layout, 1, tex, &mut encoder);

                for (voxelmap_args_index, range) in ranges {
                    let env = self.env.get(*voxelmap_args_index).unwrap();
                    env.bind(index, layout, 0, &mut encoder);
                    unsafe {
                        encoder.draw(0..4, range.to_owned());
                    }
                }
            }
        }
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _aux: &GraphAuxData) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_voxels_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), hal::pso::CreationError> {
    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, None as Option<(_, _)>)
    }?;

    let mut shaders = SHADERS.build(factory, Default::default()).map_err(|e| {
        match e {
            hal::device::ShaderError::OutOfMemory(oom) => oom.into(),
            _ => hal::pso::CreationError::Other,
        }
    })?;

    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                .with_vertex_desc(&[(VoxelArgs::vertex(), hal::pso::VertexInputRate::Instance(1))])
                .with_input_assembler(hal::pso::InputAssemblerDesc::new(
                    hal::pso::Primitive::TriangleStrip,
                ))
                .with_shaders(shaders.raw().map_err(|_| hal::pso::CreationError::Other)?)
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                .with_blend_targets(vec![hal::pso::ColorBlendDesc {
                    mask: hal::pso::ColorMask::ALL,
                    blend: Some(hal::pso::BlendState::PREMULTIPLIED_ALPHA),
                }])
                .with_depth_test(hal::pso::DepthTest {
                    fun: hal::pso::Comparison::Greater,
                    write: true,
                })
                .with_face_culling(Face::FRONT),
        )
        .build(factory, None);

    shaders.dispose(factory);

    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}
