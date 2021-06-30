use std::{
    env,
    path::{
        Path,
        PathBuf,
    },
};

use color_eyre::eyre::Error;
use shellfn::shell;

#[shell]
fn compile_shader(src: &str, output: &str) -> Result<(), Error> {
    r#"
        glslc -g -c -O -o $OUTPUT $SRC
    "#
}

fn _compile_shaders<P: AsRef<Path>>(manifest_dir: P) -> Result<(), Error> {
    let shaders_src = manifest_dir.as_ref().join("shaders/src");
    let shaders_compiled = manifest_dir.as_ref().join("shaders/compiled");

    let shaders = vec!["voxels.vert", "voxels.frag"];

    for shader in shaders {
        let shader_src = shaders_src.join(shader);
        let shader_output = shaders_compiled.join(&format!("{}.spv", shader));
        println!("cargo:rerun-if-changed={}", shader_src.display());
        log::info!("Compiling shader: {}", shader_src.display());
        compile_shader(
            shader_src.to_str().unwrap(),
            shader_output.to_str().unwrap(),
        )?;
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let _manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    //compile_shaders()?;

    Ok(())
}
