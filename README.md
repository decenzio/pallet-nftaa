# NFTAs (NFT-as-Account) pallet

A pallet for converting NFTs into smart accounts that can execute transactions.

## Overview

The NFTAs pallet extends the NFTs pallet functionality by allowing NFTs to act as accounts, enabling:

* NFT to Account Conversion
* Account-based Transaction Execution
* Integration with Existing NFT Features
* Account-based Permission Management

To use it in your runtime, you need to implement the [`nfts::Config`](https://github.com/decenzio/polkadot-sdk/blob/d9f211f99cc6e2e899bf8286b43d6f146e396a6a/templates/parachain/runtime/src/configs/mod.rs#L382) trait. You also need to configure `pallet-nfts` and `pallet-utility` as those are required.

### Terminology

* **NFT-as-Account (NFTAA)**: An NFT that has been converted into a smart account, capable of executing transactions.
* **Proxy Call**: A transaction executed through an NFTAA's generated account address.
* **NFT Account**: A deterministically generated account address associated with a specific NFT.
* **Listed NFTAA**: An NFTAA that is currently listed for sale and cannot execute transactions.

### Goals

The NFTAs pallet in Substrate is designed to make the following possible:

* Allow NFTs to act as smart accounts with their own addresses
* Enable NFTs to execute transactions through proxy calls
* Maintain compatibility with all standard NFT features like transfers and sales
* Provide secure permission management for NFTAA execution
* Enable deterministic account generation based on NFT identifiers

## Interface

### Permissionless dispatchables

* `mint`: Create a new NFTAA by minting an NFT with an associated account.
* `proxy_call`: Execute a transaction through an NFTAA's account.

### Additional Features

The NFTAA pallet inherits all functionality from the base NFTs pallet, including:

* NFT transfers
* Collection management
* Metadata and attribute management 
* Trading features
* Burning mechanisms

### Unique NFTAA Features

* Deterministic account generation for each NFT
* Transaction execution through NFT accounts
* State management for NFTAA listings
* Permission validation for proxy calls
* Integration with existing NFT marketplace features

### Implementation Details

* NFTAAs cannot execute transactions while listed for sale
* Account addresses are generated deterministically based on collection and item IDs
* Proxy calls can only be executed by the current owner of the NFTAA
* All standard NFT features remain available for NFTAAs

## Related Modules

* [`NFTs`](https://paritytech.github.io/substrate/master/pallet_nfts/index.html)
* [`System`](https://docs.rs/frame-system/latest/frame_system/)
* [`Support`](https://docs.rs/frame-support/latest/frame_support/)

License: MIT
