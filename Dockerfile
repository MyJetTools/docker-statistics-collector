FROM ubuntu:22.04
COPY ./target/release/docker-statistics-collector ./target/release/docker-statistics-collector
ENTRYPOINT ["./target/release/docker-statistics-collector"]