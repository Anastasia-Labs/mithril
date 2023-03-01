---
unlisted: true
hide_title: true
hide_table_of_contents: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Here is an up to date list of all the **Mithril Networks**, their configurations and their status:

> Last update: 02/24/2023

<Tabs>
  <TabItem value="preview" label="Preview" default>

## `pre-release-preview`

| Information | -
|------------|------------
| **Mithril Network** | `pre-release-preview` [:mag_right:](https://mithril.network/explorer?aggregator=https%3A%2F%2Faggregator.pre-release-preview.api.mithril.network%2Faggregator)
| **Cardano Network** | `preview` 
| **Cardano Magic Id** |   `2`
| **Supported** | Yes :heavy_check_mark:
| **Status** | Pre-Release 🟠
| **Aggregator Endpoint** | `https://aggregator.pre-release-preview.api.mithril.network/aggregator` [:arrow_upper_right:](https://aggregator.pre-release-preview.api.mithril.network/aggregator)  
| **Genesis Verification Key** | `https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey` [:arrow_upper_right:](https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey)  
| **Build From** |  **Latest Pre-Release** [:arrow_upper_right:](https://github.com/input-output-hk/mithril/releases?q=pre-release) 

<br/>

## `testing-preview`
> :warning: For devs only

| Information | -
|------------|------------
| **Mithril Network** | `testing-preview` [:mag_right:](https://mithril.network/explorer?aggregator=https%3A%2F%2Faggregator.testing-preview.api.mithril.network%2Faggregator)
| **Cardano Network** | `preview` 
| **Cardano Magic Id** |   `2`
| **Supported** | Yes :heavy_check_mark:
| **Status** | Unstable 🔴
| **Aggregator Endpoint** | `https://aggregator.testing-preview.api.mithril.network/aggregator` [:arrow_upper_right:](https://aggregator.testing-preview.api.mithril.network/aggregator)  
| **Genesis Verification Key** | `https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey` [:arrow_upper_right:](https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey)  
| **Build From** |  **Main Branch** [:arrow_upper_right:](https://github.com/input-output-hk/mithril/tree/main) 

  </TabItem>
  <TabItem value="preprod" label="Preprod">

## `release-preprod`

| Information | -
|------------|------------
| **Mithril Network** | `release-preprod` [:mag_right:](https://mithril.network/explorer?aggregator=https%3A%2F%2Faggregator.release-preprod.api.mithril.network%2Faggregator)
| **Cardano Network** | `preprod` 
| **Cardano Magic Id** |   `1`
| **Supported** | Yes :heavy_check_mark:
| **Status** | Release 🟢
| **Aggregator Endpoint** | `https://aggregator.release-preprod.api.mithril.network/aggregator` [:arrow_upper_right:](https://aggregator.release-preprod.api.mithril.network/aggregator)  
| **Genesis Verification Key** | `https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey` [:arrow_upper_right:](https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey)  
| **Build From** |  **Latest Release** [:arrow_upper_right:](https://github.com/input-output-hk/mithril/releases/latest) 

  </TabItem>
  <TabItem value="mainnet" label="Mainnet">

## `release-mainnet`

| Information | -
|------------|------------
| **Mithril Network** | `release-mainnet`
| **Cardano Network** | `mainnet` 
| **Cardano Magic Id** |   `-`
| **Supported** | No :x:
| **Status** | -
| **Aggregator Endpoint** | - 
| **Genesis Verification Key** | -  
| **Build From** |  -

  </TabItem>
</Tabs>

:::caution
In this documentation, we use the generic:
* ****YOUR_CARDANO_NETWORK**** identifier, but you need to replace it with the name of the network that runs on your Cardano node (e.g. `preprod`)
* ****YOUR_AGGREGATOR_ENDPOINT**** identifier, but you need to replace it with the endpoint of an aggregator that runs on the Cardano network you target (e.g. `https://aggregator.release-preprod.api.mithril.network/aggregator`)
* ****YOUR_GENESIS_VERIFICATION_KEY**** identifier, but you need to replace it with the genesis verification key url that runs on the Cardano network you target (e.g. `https://raw.githubusercontent.com/input-output-hk/mithril/main/TEST_ONLY_genesis.vkey`)

:::