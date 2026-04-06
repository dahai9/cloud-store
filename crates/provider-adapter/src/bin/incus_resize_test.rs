use futures_util::{SinkExt, StreamExt};
use provider_adapter::{ComputeProvider, IncusProvider, NodeConnection};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let provider = IncusProvider::new()?;
    let node = NodeConnection {
        endpoint: "https://104.233.202.113:18443".to_string(),
        token: None,
    };

    let instance_id = "cs-c78e579d";

    let token = provider.get_exec_token(&node, instance_id).await?;
    let mut control_ws = provider.open_console_ws(&token.control_url).await?;
    let mut data_ws = provider.open_console_ws(&token.url).await?;

    let formats = vec![
        serde_json::json!({ "command": "window-resize", "args": { "width": "120", "height": "40" } }),
        serde_json::json!({ "command": "window-resize", "args": { "width": 110, "height": 45 } }),
        serde_json::json!({ "command": "window-resize", "width": "130", "height": "50" }),
        serde_json::json!({ "command": "window-resize", "width": 140, "height": 55 }),
        // According to LXD/Incus docs, exec uses window-resize and passes args.
        // e.g. {"command": "window-resize", "args": {"width": "80", "height": "24"}}
    ];

    // Give bash a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send a newline to clear prompt
    data_ws.send(Message::Binary(b"\n".to_vec().into())).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Clear output buffer
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(100), data_ws.next()).await {
    }

    for (i, fmt) in formats.iter().enumerate() {
        let json = fmt.to_string();
        info!("Sending Format #{}: {}", i + 1, json);
        control_ws.send(Message::Text(json.into())).await?;

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Ask bash for its current window size
        data_ws
            .send(Message::Binary(b"stty size\n".to_vec().into()))
            .await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Read response
        while let Ok(Some(msg)) =
            tokio::time::timeout(Duration::from_millis(100), data_ws.next()).await
        {
            if let Ok(Message::Binary(bin)) = msg {
                let text = String::from_utf8_lossy(&bin);
                info!(
                    "Bash responded: {}",
                    text.replace('\n', "\\n").replace('\r', "\\r")
                );
            }
        }
    }

    Ok(())
}
