use k8s_openapi::api::core::v1::Pod;
use kube::{api::{Api, ListParams}, Client, ResourceExt};
use log::debug;
use std::collections::HashMap;
use tokio::time::{self, Duration};
use reqwest::Client as HttpClient;

enum PodStatus {
    Up,
    Down,
}

async fn fetch_monitored_pods(client: Client) -> Vec<Pod> {
    let pods: Api<Pod> = Api::all(client);
    let lp = ListParams::default();
    let mut filtered = vec![];

    if let Ok(pod_list) = pods.list(&lp).await {
        for pod in pod_list.items {
            if let Some(annotations) = &pod.metadata.annotations {
                if annotations.contains_key("anotherland/status-webhook") {
                    filtered.push(pod);
                }
            }
        }
    }

    filtered
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let interval_secs: u64 = std::env::var("INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);


    let client = Client::try_default().await.expect("Failed to create kube client");
    let http_client = HttpClient::new();
    let mut interval = time::interval(Duration::from_secs(interval_secs));

    
    loop {
        let mut state = HashMap::new();
        let pods = fetch_monitored_pods(client.clone()).await;

        // Go trough all pods, check their status and update the state per webhook
        for pod in pods {
            let webhook = pod.metadata.annotations
                .and_then(|annotations| annotations.get("anotherland/status-webhook").cloned())
                .unwrap();

            let status = state
                .entry(webhook)
                .or_insert(PodStatus::Down);

            if let Some(pod_status) = pod.status {
                if let Some(conditions) = pod_status.conditions {
                    for cond in conditions {
                        if cond.type_ == "Ready" && cond.status == "True" {
                            *status = PodStatus::Up;
                        }
                    }
                }
            }
        }

        for (webhook, state) in state.iter() {
            let trigger = match state {
                PodStatus::Up => "up",
                PodStatus::Down => "down",
            };

            debug!("Triggering webhook {} with status {}", webhook, trigger);

            let _ = http_client.post(webhook)
                .json(&serde_json::json!({"trigger": trigger}))
                .send()
                .await;
        }

        interval.tick().await;
    }
}
