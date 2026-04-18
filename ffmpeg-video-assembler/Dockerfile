FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

FROM alpine
RUN apk add --no-cache ffmpeg
COPY --from=builder /app/target/release/ffmpeg-video-assembler /usr/local/bin/
EXPOSE 8090
CMD ["ffmpeg-video-assembler"]
