FROM rust:1.83-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked
FROM debian:bookworm-slim
COPY --from=build /src/target/release/memocap /usr/local/bin/memocap
ENV MEMOCAP_DATA_DIR=/data
VOLUME ["/data"]
EXPOSE 8787
ENTRYPOINT ["memocap"]
CMD ["serve", "--bind", "0.0.0.0:8787"]
