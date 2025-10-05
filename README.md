# Hockey Reminder Lambda (Rust)

This project is an AWS Lambda function written in Rust that posts upcoming hockey game information to Discord. It can:
- Generate and post a DaySmart “next game” reminder message
- Optionally generate and post a BenchApp-compatible CSV (from an iCal feed) as a Discord attachment

The function is configured entirely via the invocation request payload (no environment variables required).


## Quick links
- Official AWS docs for Rust on Lambda (read this first):
  https://docs.aws.amazon.com/lambda/latest/dg/lambda-rust.html


## Request and response schema

The handler expects the following JSON payload:

- mode: "test" | "production"
  - Chooses which Discord webhook to use.
- discord_hook_url: string
  - Webhook for production mode.
- test_discord_hook_url: string (optional)
  - Webhook for test mode. If omitted, test mode falls back to discord_hook_url.
- ical_url: string (optional)
  - If present and the Benchapp workflow is selected, the iCal feed is fetched and converted to a BenchApp CSV.
- team_id: string
  - Your team identifier for DaySmart.
- company: string
  - Your company/organization identifier for DaySmart.
- workflows: array<string> (optional)
  - Supported values: "daysmart", "benchapp".
  - If omitted or empty, the function defaults to ["daysmart"].

The function returns:
- { "message": string }
  - A human-readable summary of what was done.

Example minimal payload (defaults to DaySmart workflow):

{
  "mode": "test",
  "discord_hook_url": "https://discord.com/api/webhooks/.../prod",
  "team_id": "12345",
  "company": "acme"
}

Example with explicit workflows and iCal:

{
  "mode": "production",
  "discord_hook_url": "https://discord.com/api/webhooks/.../prod",
  "test_discord_hook_url": "https://discord.com/api/webhooks/.../test",
  "ical_url": "https://example.com/schedule.ics",
  "team_id": "12345",
  "company": "acme",
  "workflows": ["daysmart", "benchapp"]
}

Behavioral notes:
- If there are no upcoming games, the function skips posting to Discord and returns a summary indicating it skipped.
- If "benchapp" is requested but ical_url is not provided, the BenchApp workflow is silently skipped.


## Build and deploy to AWS Lambda

You can deploy this Lambda using either:
- Manual build + zip upload (uses the custom runtime for Rust, Provided.al2)
- cargo-lambda (recommended for convenience)

The official guide (linked above) explains both in detail. Below are concise steps adapted to this project.

### 1) Prerequisites
- Rust toolchain installed
- AWS account and credentials configured (AWS CLI or your preferred method)
- A Discord Webhook URL for production mode (optionally provide a separate test webhook)
- Outbound internet access for the Lambda function (Discord webhooks require internet). If your Lambda runs in a VPC, ensure proper NAT/egress is configured.


### Option A: Manual build and upload (x86_64)

1. Build a Linux-compatible binary in release mode:
   - For x86_64:
     rustup target add x86_64-unknown-linux-gnu
     cargo build --release --target x86_64-unknown-linux-gnu

   - For better compatibility with Amazon Linux 2, you can also build in a container or use musl/zig. See the AWS docs for details and alternatives.

2. Prepare the bootstrap file expected by the custom runtime:
   - The release binary is typically at target/x86_64-unknown-linux-gnu/release/hockey_reminder_lambda_rust
   - Copy or rename it to a file named bootstrap and zip it:

     cd target/x86_64-unknown-linux-gnu/release
     copy hockey_reminder_lambda_rust bootstrap
     tar -a -c -f bootstrap.zip bootstrap

   Notes:
   - On Linux/macOS you can use `cp` and `zip`. On Windows, the above PowerShell example uses `copy` and `tar` with `-a` to create a .zip.

3. Create the Lambda function (once):
   - Runtime: Provide your own bootstrap on Amazon Linux 2 (Provided.al2)
   - Architecture: x86_64 (to match the build target)
   - Upload code: Use the bootstrap.zip created above

   You can do this in the AWS Console or with the AWS CLI, e.g.:

   aws lambda create-function ^
     --function-name hockey-reminder ^
     --runtime provided.al2 ^
     --role arn:aws:iam::123456789012:role/your-lambda-execution-role ^
     --handler bootstrap ^
     --architectures x86_64 ^
     --zip-file fileb://bootstrap.zip

4. Update code on subsequent deployments:

   aws lambda update-function-code ^
     --function-name hockey-reminder ^
     --zip-file fileb://bootstrap.zip

5. Test the function (Console or CLI):
- Use a test event with the Request payload described above.
- Check the CloudWatch Logs for details (the project uses structured logging with tracing).


### Option B: cargo-lambda (recommended)

cargo-lambda wraps the best practices for building and deploying Rust Lambdas.

1. Install cargo-lambda:

   cargo install cargo-lambda

2. Build for Lambda:

   cargo lambda build --release

   This produces an artifact suitable for upload in target/lambda/hockey_reminder_lambda_rust/bootstrap

3. Deploy:

   cargo lambda deploy hockey-reminder

   You can pass flags to set runtime/arch/role on first deploy; consult:
   https://www.cargo-lambda.info/

4. Invoke for testing:

   cargo lambda invoke hockey-reminder --data-file event.json --remote

   Where event.json contains one of the example payloads above.


## Local testing

Run the Rust tests:

cargo test

Note: The function’s external calls (Discord, DaySmart, iCal fetching) are exercised indirectly via unit tests that focus on serialization and internal logic. Integration tests against real services are not included.


## Operational considerations
- Networking: Discord webhook delivery requires outbound internet. If the Lambda runs in a VPC, configure NAT Gateway or VPC endpoints accordingly.
- Logging/Observability: Output goes to CloudWatch Logs. You can add subscriptions or log retention policies per your standards.
- Time windows: DaySmart message looks up the next game within 5 days of the invocation time (UTC now).
- Idempotency: The function does not persist state; repeated invocations within the same window will re-post unless there are no upcoming games.


## Repository layout
- src/handler.rs — Lambda handler with request/response types and workflow orchestration
- src/daysmart.rs — DaySmart integration and message generation
- src/benchapp_csv.rs — BenchApp CSV generator from an iCal feed
- src/discord.rs — Minimal Discord webhook client
- src/main.rs — Binary entry point that wires Lambda runtime to the handler
- tests/* — Unit tests


## License
MIT (or your project’s chosen license).