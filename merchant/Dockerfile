FROM rust:latest

WORKDIR /usr/src/merchant
COPY . .
RUN cargo build --release
CMD ["target/release/merchant"]