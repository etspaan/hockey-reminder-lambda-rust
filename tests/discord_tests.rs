use hockey_reminder_lambda_rust::discord::Discord;

#[test]
fn discord_new_clones_url() {
    let url = "https://example.invalid/webhook".to_string();
    let d1 = Discord::new(url.clone());
    let d2 = d1.clone();
    // Ensure cloning retains internal URL equality by round-tripping a debug string
    let dbg1 = format!("{:?}", d1);
    let dbg2 = format!("{:?}", d2);
    assert!(dbg1.contains("Discord"));
    assert_eq!(dbg1, dbg2);
    // Avoid network: don't call post/post_with_attachment here
    let _ = url; // silence unused
}
