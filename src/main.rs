mod model;
mod daysmart;
mod discord;
mod ical;
mod handler;

use lambda_runtime::{service_fn, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize structured logging with tracing
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        // Emit a closing event for each span, which includes its total duration
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_current_span(false)
        .with_target(false)
        .with_ansi(false)
        .try_init();

    //let _ = handler(LambdaEvent::new(serde_json::from_str("{\"mode\":\"production\",\"date\":\"2026-03-28\", \"discord_hook_url\":\"https://discord.com/api/webhooks/1300382433241464854/B4pFABQURfSy-eZIzX8RaOyEuyJVhXtyd-6iYEGVHsKJfzGyCr5PwTATvpuMBzx6f9IY\",\"team_id\":\"11007\",\"company\":\"kraken\"}")?, Context::default())).await.expect("TODO: panic message");

    lambda_runtime::run(service_fn(handler::handler)).await
}
