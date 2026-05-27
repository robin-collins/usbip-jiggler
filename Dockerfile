FROM rust:slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd -m jiggler
COPY --from=builder /app/target/release/mouse-jiggler /usr/local/bin/
USER jiggler
EXPOSE 3240
CMD ["mouse-jiggler"]
