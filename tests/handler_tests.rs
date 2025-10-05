use hockey_reminder_lambda_rust::handler::{Mode, Workflow, Request};

#[test]
fn serde_mode_and_workflow_lowercase() {
    // Mode
    let m: Mode = serde_json::from_str("\"test\"").unwrap();
    matches!(m, Mode::Test);
    let m2: Mode = serde_json::from_str("\"production\"").unwrap();
    matches!(m2, Mode::Production);
    // Workflow
    let w: Workflow = serde_json::from_str("\"benchapp\"").unwrap();
    matches!(w, Workflow::Benchapp);
    let w2: Workflow = serde_json::from_str("\"daysmart\"").unwrap();
    matches!(w2, Workflow::Daysmart);
}

#[test]
fn request_deserializes_and_defaults_workflows() {
    let json = serde_json::json!({
        "mode": "test",
        "discord_hook_url": "prod",
        "test_discord_hook_url": "test",
        "team_id": "123",
        "company": "acme"
    });
    let req: Request = serde_json::from_value(json).unwrap();
    assert!(req.workflows.is_empty(), "workflows should default to empty vec");

    let json2 = serde_json::json!({
        "mode": "production",
        "discord_hook_url": "prod",
        "test_discord_hook_url": "test",
        "team_id": "123",
        "company": "acme",
        "workflows": ["benchapp", "daysmart"]
    });
    let req2: Request = serde_json::from_value(json2).unwrap();
    assert_eq!(req2.workflows.len(), 2);
    let names: Vec<String> = req2.workflows.iter().map(|w| serde_json::to_string(w).unwrap()).collect();
    assert!(names.contains(&"\"benchapp\"".to_string()));
    assert!(names.contains(&"\"daysmart\"".to_string()));
}
