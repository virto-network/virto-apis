# Merchant API

Welcome to our merchant API, which currently supports:

* `Catalog`: publish products and services with their variations, define prices, taxes and costs in all of them!
* `Iventory`:  Administrate the inventory and dispatch of your products and services to your customers.
* `Shopping Cart`: Shopping cart

## Development

### Setup steps

1. Run your postgresql database `docker run --name merchant-api-postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres`.
2. Run the migrations `cargo sqlx migrate run`.
3. Run the tests `cargo test`.