use super::*;
use crux_core::testing::AppTester;

#[test]
fn test_login_sets_loading() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let _command = app.update(
        Event::Login {
            password: "pass".to_string(),
        },
        &mut model,
    );

    assert!(model.is_loading);
}

#[test]
fn test_system_info_updated() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let info = SystemInfo {
        os: OsInfo {
            name: "Linux".to_string(),
            version: "5.10".to_string(),
        },
        azure_sdk_version: "1.0".to_string(),
        omnect_device_service_version: "2.0".to_string(),
        boot_time: Some("2024-01-01".to_string()),
    };

    let _command = app.update(Event::SystemInfoUpdated(info.clone()), &mut model);

    assert_eq!(model.system_info, Some(info));
}

#[test]
fn test_clear_error() {
    let app = AppTester::<App>::default();
    let mut model = Model {
        error_message: Some("Some error".to_string()),
        ..Default::default()
    };

    let _command = app.update(Event::ClearError, &mut model);

    assert_eq!(model.error_message, None);
}
