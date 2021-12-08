# Virto APIs

This mono-repo groups all the different APIs that enable the easy creation of **decentraizable** applications.
APIs are deveoped as [Valor](https://github.com/virto-network/valor) pugins which allows them to be composed together,
easily distributed and most importantly to run in the browser as well as server environments.

Whenever possibe APIs are created to be general enough that they would work with any underlying blockchain, 
however some APIs might be chain specific like those for de-commerce apps that rely on the Virto Network.

### 🚧🛠️ Blockchain API
Based on [`sube`](https://github.com/virto-network/sube) it allows lower level interaction with any Substrate based chain.  
_Features:_
- Query storage
- Query metadata(e.g. constants)
- Encode transactions
- Submit signed transactions

### 🚧🛠️ Wallet API
Based on [`libwallet`](https://github.com/virto-network/libwallet), a blockchain and storage agnostic wallet library.  
_Features:_
- Manage multiple accounts
- Register multiple chains
- Sign and verify data
- Transaction queuing, reviewing and batch signing
- Matrix integration

### 🚧 Payments API
Secure reversible payments based on the payments pallet with built-in escrow system and future support for 
payment requests, subscriptions and chained payments.

### 🚧 Merchant API
Using Matrix as a backend, it allows merchants register their economic activity, manage their catalog and 
inventory of goods and services which can be shared with marketpaces to reach a broader audience.

### 🚧 Customer API
For users to track their shopping history, their _universal shopping cart_ and manage their prefered crypto 
and fiat payment methods as well integrations with traditional banks. State is also persisted via Matrix
in the user's homeserver.

### 🚧 Market API

### 🚧 Swap API

### 🚧 Geolocation API
