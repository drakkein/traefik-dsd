mod docker;
mod redis;

use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docker_network = env::var("DOCKER_NETWORK").unwrap_or_default();
    let redis_url = env::var("REDIS_URL").expect("missing environment variable REDIS_URL");
    let redis_ttl = env::var("REDIS_TTL")
        .unwrap_or("60".to_string())
        .parse::<u64>()?;
    let host_ip = env::var("HOST_IP").unwrap_or_default();

    if host_ip.is_empty() && docker_network.is_empty() {
        panic!("missing environment variable HOST_IP or DOCKER_NETWORK");
    }

    let mut docker_client = docker::Client::new(&docker_network, &host_ip).await?;
    let mut redis_client = redis::RedisClient::new(&redis_url)?;

    loop {
        let containers = docker_client.get_containers().await?;

        for container in &containers {
            for rule in container {
                redis_client.set_key(&rule.0, &rule.1, redis_ttl)?;
            }
        }

        tokio::time::sleep(Duration::from_secs(redis_ttl - 5)).await;
    }
}
