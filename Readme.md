# Twine Solana Programs : 

# Overview
Twine Chain is a multi-chain settlement network designed to aggregate chains and provide seamless cross-chain liquidity access. This repository contains the bridge and rollup programs for Twine on the Solana blockchain.

# Programs
 - tokens_gateway: bridge program

 - twine_chain: rollup program

### 1. Build and Test

- make clean
    > cleans unnecessary files and artifacts
- make build     
    >builds the project
- make build-sbf    
    >builds the project for deployment on solana by compiling to SBF format
- make test
    >Test the project

### 2. General Setup 
-  **Makefile Usage:**  

    All building, deployment, and setup operations can be executed through the Makefile.

- **Solana Configuration:**  
  Set your Solana endpoint before deployment:

   > echo -n "Enter url [default: http://127.0.0.1:8899]: " && read url && url=${url:-http://127.0.0.1:8899} && solana config set --url "$url" && export SOLANA_RPC_URL="$url"

    - **Url For Devnet:**
        - https://api.devnet.solana.com

  - **Url For Localnet:**
    - http://127.0.0.1:8899
  - If you want to deploy to your custom URL, then provide it 

### 3. Deployment Commands
- **List Available Commands:** 
    - make help
- **Fresh Deployment and Setup:**  
    - **Change** pub const INITIAL_CHAIN_ADMIN: &str = "your_wallet_address";
    - **Execute the following commands sequentially:**
        - make clean 
        - make build 
        - make build-sbf
        - make deploy 
            > deploys programs
- **Initial Setup:** 
    - make initialize
    > initialises and does initial setup

### 4. Token Operations
#### A. Native Token Deposit
- **Update Token Mapping (if not already set):**
    - make update-token-mapping l1_token=your_l1_token_here l2_token=your_l2_token_here l1_decimals=l1_token_decimals l2_decimals=l2_token_decimals 
- **Deposit Native Token:**
    - make deposit-native-token amount=amount_here receiver_address=receiver_address_here l2_token=l2_token_here data=data_here
#### B. SPL Token Deposit
- **Update Token Mapping (if not already set):**
    - make update-token-mapping l1_token=your_l1_token_here l2_token=your_l2_token_here l1_decimals=l1_token_decimals l2_decimals=l2_token_decimals 

- **Deposit Spl Token:**
     - make deposit-spl-token l1_token=l1_token_here l2_token=l2_token_here receiver_address=receiver_address_here user_token_account= user_token_account_here amount=amount_here data=data_here