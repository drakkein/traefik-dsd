use axum::body::Body;
use http_body_util::BodyExt;
use hyper::http::{Method, Request, StatusCode};
use hyper::service;
use hyper_util::rt::TokioIo;
use serde_json;
use serde_json::Value;
use std::path::PathBuf;
use tokio::net::UnixStream;

pub struct Client<'a> {
    sender: hyper::client::conn::http1::SendRequest<Body>,
    docker_network: &'a str,
    host_ip: &'a str,
}

impl<'a> Client<'a> {
    pub async fn new(
        docker_network: &'a str,
        host_ip: &'a str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = PathBuf::from("/var/run/docker.sock");
        let stream = TokioIo::new(UnixStream::connect(path).await?);

        let (sender, conn) = hyper::client::conn::http1::handshake(stream).await?;

        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                eprintln!("Connection failed: {:?}", err);
            }
        });

        Ok(Client {
            sender,
            docker_network,
            host_ip,
        })
    }

    pub async fn get_containers(
        &mut self,
    ) -> Result<Vec<Vec<(String, String)>>, Box<dyn std::error::Error>> {
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

    fn is_valid_container(&self, container: &Value) -> bool {
        let is_valid_network = if !self.docker_network.is_empty() {
            container["NetworkSettings"]["Networks"]
                .as_object()
                .map(|networks| networks.contains_key(self.docker_network))
                .unwrap_or(false)
        } else {
            true
        };

        is_valid_network
            && container["Labels"]
                .as_object()
                .map(|labels| labels.contains_key("com.docker.compose.service"))
                .unwrap_or(false)
            && container["Labels"]
                .as_object()
                .map(|labels| labels.contains_key("traefik.enable"))
                .unwrap_or(false)
    }

    fn build_lb_server_configuration(
        &self,
        labels: &Vec<(String, String)>,
        service_name: &str,
        network_ip: &str,
        container_number_label: &str,
    ) -> Option<(String, String)> {
        let key = format!(
            "traefik/http/services/{}/loadbalancer/server/port",
            service_name
        );
        let lb_service_port = labels
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.to_string())
            .or_else(|| None);

        match lb_service_port {
            Some(service_port) => {
                let service_url = format!("http://{}:{}", network_ip, service_port);
                let service_container_index = container_number_label.parse::<i32>().unwrap() - 1;

                Some((
                    format!(
                        "traefik/http/services/{}/loadBalancer/servers/{}/url",
                        service_name, service_container_index
                    ),
                    service_url,
                ))
            }
            _ => None,
        }
    }

    fn build_router_service_configuration(&self, service_name: &str) -> Option<(String, String)> {
        Some((
            format!("traefik/http/routers/{}/service", service_name),
            service_name.into(),
        ))
    }

    fn implicit_service_name(&self, labels: &Vec<(String, String)>) -> Vec<String> {
        let service_names = labels
            .iter()
            .filter(|(k, _)| k.starts_with("traefik/http/services/"))
            .map(|(k, _)| {
                k.split("/")
                    .nth(3)
                    .expect("service name not found in traefik label")
                    .to_string()
            })
            .collect();
        service_names
    }

    fn parse_containers(&mut self, json: Value) -> Vec<Vec<(String, String)>> {
        json.as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|container| {
                if !self.is_valid_container(container) {
                    return None;
                }

                let mut traefik_labels: Vec<(String, String)> = container["Labels"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(k, _)| k.starts_with("traefik.http."))
                    .map(|(k, v)| {
                        (
                            k.to_string().replace(".", "/"),
                            v.clone().as_str().unwrap().to_string(),
                        )
                    })
                    .collect();

                let container_network_ip = if !self.docker_network.is_empty() {
                    container["NetworkSettings"]["Networks"][self.docker_network]["IPAddress"]
                        .as_str()
                        .unwrap()
                        .to_string()
                } else {
                    self.host_ip.to_string()
                };

                let container_number_label = container["Labels"]
                    ["com.docker.compose.container-number"]
                    .as_str()
                    .unwrap();

                let service_names = self.implicit_service_name(&traefik_labels);

                for service_name in service_names {
                    let lb_config = self.build_lb_server_configuration(
                        &traefik_labels,
                        &service_name,
                        &container_network_ip,
                        container_number_label,
                    );

                    if lb_config.is_some() {
                        traefik_labels.push(lb_config.unwrap());
                    }

                    let router_service_config =
                        self.build_router_service_configuration(&service_name);

                    if router_service_config.is_some() {
                        traefik_labels.push(router_service_config.unwrap());
                    }
                }

                traefik_labels.retain(|(k, _)| !k.ends_with("/loadbalancer/server/port"));

                Some(traefik_labels)
            })
            .collect()
    }
}
