FROM rust:latest AS build
WORKDIR /usr/src/merchant
COPY . .
RUN cargo build --release
CMD ["target/release/merchant"]

FROM scratch
COPY --from=build /usr/src/merchant/target /bin/merchant
CMD ["/bin/merchant"]
