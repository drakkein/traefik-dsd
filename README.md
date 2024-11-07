# traefik-dsd
Dead simple discovery for Traefik via overlay network in docker.  

Basic implementation to discover containers over docker sock that are running connected to overlay network.

**DISCLAIMER: If you looking for stable solution it is advertised to use [traefik-kop](https://github.com/jittering/traefik-kop).**

# Prequesites
To use traefik-dsd you need to init swarm mode and create overlay network. Traefik and all containers must be in the same overlay network.

# Usage
```yaml
services:
  traefik-dsd:
    image: "ghcr.io/drakkein/traefik-dsd:latest"
    restart: unless-stopped
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      - "REDIS_URL=redis://valkey:6379"
      - "DOCKER_NETWORK=traefik-public"
    networks:
      - traefik-public

  whoami:
    image: traefik/whoami
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.whoami.rule=Host(`myapp.example.com`)" 
      - "traefik.http.services.whoami.loadbalancer.server.port=80"
    networks:
      - traefik-public

networks:
  traefik-public:
    external: true
```

Test on `traefik machine`
```bash
curl -v -H "Host: myapp.example.com" http://127.0.0.1/
```

# Configuration
|Variable|Description|Default|
|-|-|-|
|REDIS_URL|URL to your `redis/valkey` instance|-|
|DOCKER_NETWORK|Name of your overlay network|-|
|REDIS_TTL|Expiry time of created records. Discovery loop starts 5s before| 60|
|HOST_IP|IP address that will be passed to loadbalancer config instead of container IP|-|