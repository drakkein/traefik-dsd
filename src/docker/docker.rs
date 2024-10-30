use axum::body::Body;
use hyper::http::{Method, Request, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::Value;
use std::path::PathBuf;
use tokio::net::UnixStream;
use serde_json;
use http_body_util::BodyExt;

pub struct Client<'a> {
    sender: hyper::client::conn::http1::SendRequest<Body>,
    docker_network: &'a str,
}

#[derive(Debug)]
pub struct Container {
    pub service_name: String,
    pub service_container_index: i32,
    pub service_url: String,
    pub traefik_router_rule: String,
}

impl<'a> Client<'a> {
    pub async fn new(docker_network: &'a str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = PathBuf::from("/var/run/docker.sock");
        let stream = TokioIo::new(UnixStream::connect(path).await?);
        
        let (sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
        
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                eprintln!("Connection failed: {:?}", err);
            }
        });

        Ok(Client { sender, docker_network })
    }

    pub async fn get_containers(&mut self) -> Result<Vec<Container>, Box<dyn std::error::Error>> {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/containers/json")
            .header("Host", "api")
            .body(Body::empty())?;

        let response = self.sender.send_request(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.collect().await?.to_bytes();
        let body_str = String::from_utf8(body.to_vec())?;
        let json: Value = serde_json::from_str(&body_str)?;
        let containers = self.parse_containers(json);

        Ok(containers)
    }

    fn parse_containers(&mut self, json: Value) -> Vec<Container> {
        json.as_array().unwrap_or(&vec![])
            .iter()
            .filter_map(|container| {
            // Check if the container is connected to the specified network
            if !container["NetworkSettings"]["Networks"].as_object()
                .map(|networks| networks.contains_key(self.docker_network))
                .unwrap_or(false) {
                return None;
            }

            // Check if the container is a service
            if !container["Labels"].as_object()
                .map(|labels| labels.contains_key("com.docker.compose.service"))
                .unwrap_or(false) {
                return None;
            }

            // Check if traefik is enabled for the container
            if !container["Labels"].as_object()
                .map(|labels| labels.get("traefik.enable").map(|v| v == "true").unwrap_or(false))
                .unwrap_or(false) {
                return None;
            }

            // Parse required fields
            let service_name = container["Labels"]["com.docker.compose.service"].as_str().unwrap().to_string();
            let container_network_ip = container["NetworkSettings"]["Networks"][self.docker_network]["IPAddress"].as_str().unwrap().to_string();
            let container_number_label = container["Labels"]["com.docker.compose.container-number"].as_str().unwrap(); 
            let service_container_index = container_number_label.parse::<i32>().unwrap() - 1;
    
            let port_label = format!("traefik.http.services.{}.loadbalancer.server.port", service_name);
            let rule_label = format!("traefik.http.routers.{}.rule", service_name);
    
    
            let traefik_port = container["Labels"][port_label].as_str().unwrap();
            let service_url = format!("http://{}:{}", container_network_ip, traefik_port);
            let service_url = service_url.as_str().to_string();
    
            let traefik_router_rule = container["Labels"][rule_label].as_str().unwrap().to_string();

            Some(Container {
                service_name,
                service_container_index,
                service_url,
                traefik_router_rule,
            })
        })
        .collect()
    }
}