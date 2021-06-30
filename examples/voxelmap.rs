use amethyst::{
    assets::LoaderBundle,
    controls::{
        ArcBallControl,
        ArcBallControlBundle,
        HideCursor,
    },
    core::{
        dispatcher::DispatcherBuilder,
        transform::TransformBundle,
    },
    ecs::{
        Resources,
        World,
    },
    input::{
        is_close_requested,
        is_key_down,
        is_mouse_button_down,
        InputBundle,
        VirtualKeyCode,
    },
    renderer::{
        rendy::hal::command::ClearColor,
        types::DefaultBackend,
        RenderDebugLines,
        RenderToWindow,
        RenderingBundle,
    },
    utils::{
        application_root_dir,
        auto_fov::{
            AutoFov,
            AutoFovSystem,
        },
    },
    window::ScreenDimensions,
    winit::event::MouseButton,
    Application,
    GameData,
    SimpleState,
    SimpleTrans,
    StateData,
    StateEvent,
};
use amethyst_assets::{
    DefaultLoader,
    Loader,
    ProcessingQueue,
};
use amethyst_core::{
    math::{
        Point3,
        Vector3,
    },
    Transform,
};
use amethyst_rendy::{
    Camera,
    RenderShaded3D,
    SpriteSheet,
};
use amethyst_voxelmap::{
    storage::{
        VecStorage,
        VoxelStorage,
    },
    DrawVoxelsBoundsDefault,
    MortonEncoder,
    RenderVoxels,
    Voxel,
    VoxelMap,
};

#[derive(Clone, Debug, Default)]
pub struct ExampleVoxel(bool);

impl Voxel for ExampleVoxel {
    fn occupied(&self, _coordinates: &Point3<i32>, _world: &World, _resources: &Resources) -> bool {
        self.0
    }

    fn texture(
        &self,
        _coordinates: &amethyst_core::math::Point3<i32>,
        _world: &amethyst::ecs::World,
        _resources: &amethyst::ecs::Resources,
    ) -> Option<[usize; 6]> {
        if self.0 {
            // Use sprite number 0 from sprite sheet for all 6 faces.
            Some([0; 6])
        }
        else {
            None
        }
    }
}

struct ExampleState;

impl SimpleState for ExampleState {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        // Load spritesheet
        let sprite_sheet = {
            let loader = data.resources.get::<DefaultLoader>().expect("Get Loader");
            let texture = loader.load("spritesheet.png");
            let sprites = loader.load("spritesheet.ron");
            let sprite_sheet_store = data
                .resources
                .get::<ProcessingQueue<SpriteSheet>>()
                .unwrap();
            loader.load_from_data(SpriteSheet { texture, sprites }, (), &sprite_sheet_store)
        };

        // Create 8x8x8 voxel map.
        let n = 8;
        let mut voxel_map: VoxelMap<ExampleVoxel, VecStorage<ExampleVoxel, MortonEncoder>> =
            VoxelMap::new(
                VecStorage::from_dimensions(Vector3::repeat(n)),
                sprite_sheet,
            );

        // Fill in some voxels
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    let p = Vector3::new(x as i32, y as i32, z as i32);

                    let v = p - Vector3::repeat((n / 2) as i32);
                    let v = v.component_mul(&v);
                    if v.x + v.y + v.z < (n * n / 4) as i32 {
                        voxel_map.get_mut(&Point3::from(p)).unwrap().0 = true;
                    }
                }
            }
        }

        // Add entity with voxelmap component to world
        let entity = data.world.push((Transform::default(), voxel_map));

        // Create camera (looks along Z-axis towards origin)
        let (width, height) = {
            let dim = data
                .resources
                .get::<ScreenDimensions>()
                .expect("Read ScreenDimensions");
            (dim.width(), dim.height())
        };
        //let mut camera_transform = Transform::from(Vector3::new(-8.0, 8.0, 8.0));
        //camera_transform.face_towards(Vector3::zeros(), Vector3::new(0.0, 1.0, 0.0));

        data.world.push((
            Transform::default(),
            Camera::standard_3d(width, height),
            AutoFov::default(),
            ArcBallControl::new(entity, 12.0),
        ));
    }

    fn handle_event(&mut self, data: StateData<'_, GameData>, event: StateEvent) -> SimpleTrans {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                return SimpleTrans::Quit;
            }
            else if is_mouse_button_down(&event, MouseButton::Left) {
                let mut hide_cursor = data.resources.get_mut::<HideCursor>().unwrap();
                hide_cursor.hide = !hide_cursor.hide;
            }
        }

        SimpleTrans::None
    }
}

fn main() -> Result<(), amethyst_error::Error> {
    amethyst_core::Logger::from_config(Default::default())
        .level_for("amethyst_voxelmap", log::LevelFilter::Debug)
        .start();

    let app_root = application_root_dir()?;
    let assets_directory = app_root.join("examples/assets");
    let display_config_path = app_root.join("examples/config/display.ron");
    let key_bindings_path = app_root.join("examples/config/input.ron");

    let mut dispatcher = DispatcherBuilder::default();
    dispatcher.add_bundle(LoaderBundle);
    dispatcher.add_bundle(TransformBundle);
    dispatcher.add_bundle(InputBundle::new().with_bindings_from_file(&key_bindings_path)?);
    dispatcher.add_bundle(ArcBallControlBundle::new().with_sensitivity(0.1, 0.1));
    dispatcher.add_system(AutoFovSystem);
    dispatcher.add_bundle(
        RenderingBundle::<DefaultBackend>::new()
            .with_plugin(
                RenderToWindow::from_config_path(display_config_path)?.with_clear(ClearColor {
                    float32: [0.1, 0.03, 0.35, 1.0],
                }),
            )
            .with_plugin(RenderDebugLines::default())
            .with_plugin(RenderShaded3D::default())
            .with_plugin(RenderVoxels::<
                ExampleVoxel,
                VecStorage<ExampleVoxel, MortonEncoder>,
                DrawVoxelsBoundsDefault,
            >::default()),
    );

    let game = Application::build(assets_directory, ExampleState)?.build(dispatcher)?;
    game.run();
    Ok(())
}
