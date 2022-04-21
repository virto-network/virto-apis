# Merchant API

Welcome to our merchant API, which currently supports:

* `Catalog`: publish products and services with their variations, define prices, taxes and costs in all of them!
* `Iventory`:  Administrate the inventory and dispatch of your products and services to your customers.
* `Shopping Cart`: Shopping cart

## Development

### Setup steps

Run the tests `cargo test`. It uses an in-memory database so there's no need for extra setup.

Alternatively you can setup a local database to run the server

1. Install sqlx cli `cargo install sqlx-cli`
2. Create DB `sqlx database create`
3. Run the migrations `sqlx migrate run`.
4. `cargo run -p merchant`

