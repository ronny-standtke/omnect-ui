use anyhow::Result;
use crux_core::typegen::TypeGen;
use omnect_ui_core::{
    events::{AuthEvent, DeviceEvent, UiEvent, WebSocketEvent},
    types::{
        DeviceOperationState, FactoryResetStatus, NetworkChangeState, NetworkFormState, UploadState,
    },
    App,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=../app");

    let mut gen = TypeGen::new();

    gen.register_app::<App>()?;

    // Explicitly register domain event enums to ensure all variants are traced
    gen.register_type::<AuthEvent>()?;
    gen.register_type::<DeviceEvent>()?;
    gen.register_type::<WebSocketEvent>()?;
    gen.register_type::<UiEvent>()?;

    // Explicitly register other enums to ensure all variants are traced
    gen.register_type::<FactoryResetStatus>()?;
    gen.register_type::<DeviceOperationState>()?;
    gen.register_type::<NetworkChangeState>()?;
    gen.register_type::<NetworkFormState>()?;
    gen.register_type::<UploadState>()?;

    let output_root = PathBuf::from("./generated");

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
