mod docker;
mod redis;

use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docker_network = env::var("DOCKER_NETWORK").expect("missing environment variable DOCKER_NETWORK");
    let redis_url = env::var("REDIS_URL").expect("missing environment variable REDIS_URL");
    let redis_ttl = env::var("REDIS_TTL").unwrap_or("60".to_string()).parse::<u64>()?;

    let mut docker_client = docker::Client::new(&docker_network).await?;
    let mut redis_client = redis::RedisClient::new(&redis_url)?;

    loop {
        let containers = docker_client.get_containers().await?;

        for container in containers {
            let service_name = container.service_name;
            let service_url = container.service_url;
            let service_container_index = container.service_container_index;
            let traefik_router_rule = container.traefik_router_rule;

            redis_client.set_key(&format!("traefik/http/routers/{}/service", service_name), &service_name, redis_ttl)?;
            redis_client.set_key(&format!("traefik/http/services/{}/loadBalancer/passHostHeader", service_name), "true", redis_ttl)?;
            redis_client.set_key(&format!("traefik/http/services/{}/loadBalancer/servers/{}/url", service_name, service_container_index), &service_url, redis_ttl)?;
            redis_client.set_key(&format!("traefik/http/routers/{}/rule", service_name), &traefik_router_rule, redis_ttl)?;
        }

        tokio::time::sleep(Duration::from_secs(redis_ttl - 5)).await;
    }
}