use anyhow::Result;
use crux_core::typegen::TypeGen;
use omnect_ui_core::{types::FactoryResetStatus, App};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=../app");

    let mut gen = TypeGen::new();

    gen.register_app::<App>()?;

    // Explicitly register enums to ensure all variants are traced
    gen.register_type::<FactoryResetStatus>()?;

    let output_root = PathBuf::from("./generated");

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
