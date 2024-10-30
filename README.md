# traefik-dsd
Dead simple discovery for Traefik via overlay network in docker.  

Basic implementation to discover containers over docker sock that are running connected to overlay network.

**DISCLAIMER: If you looking for stable solution it is advertised to use [traefik-kop](https://github.com/jittering/traefik-kop).**

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