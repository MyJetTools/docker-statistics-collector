FROM ghcr.io/myjettools/dioxus-docker:0.7.7

ENV PORT=9001
ENV IP=0.0.0.0

COPY ./target/dx/docker-statistics-ui/release/web /target/dx/docker-statistics-ui/release/web

RUN chmod +x /target/dx/docker-statistics-ui/release/web/server
WORKDIR /target/dx/docker-statistics-ui/release/web/
ENTRYPOINT ["./server"]
