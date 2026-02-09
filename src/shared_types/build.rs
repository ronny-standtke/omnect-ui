use anyhow::Result;
use crux_core::typegen::TypeGen;
use omnect_ui_core::{
    events::{AuthEvent, DeviceEvent, UiEvent, WebSocketEvent},
    types::{
        DeviceOperationState, FactoryResetStatus, NetworkChangeState, NetworkConfigRequest,
        NetworkFormData, NetworkFormState, UploadState,
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

    // Explicitly register other enums/structs to ensure all variants are traced
    gen.register_type::<FactoryResetStatus>()?;
    gen.register_type::<DeviceOperationState>()?;
    gen.register_type::<NetworkChangeState>()?;
    gen.register_type::<NetworkFormState>()?;
    gen.register_type::<UploadState>()?;
    gen.register_type::<NetworkConfigRequest>()?;
    gen.register_type::<NetworkFormData>()?;

    // Register ODS types
    gen.register_type::<omnect_ui_core::types::OdsOnlineStatus>()?;
    gen.register_type::<omnect_ui_core::types::OdsSystemInfo>()?;
    gen.register_type::<omnect_ui_core::types::OdsTimeouts>()?;
    gen.register_type::<omnect_ui_core::types::OdsNetworkStatus>()?;
    gen.register_type::<omnect_ui_core::types::OdsFactoryReset>()?;
    gen.register_type::<omnect_ui_core::types::OdsUpdateValidationStatus>()?;

    let output_root = PathBuf::from("./generated");

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
