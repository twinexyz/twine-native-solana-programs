# ==============================
#        Variable Section
# ==============================
CARGO = cargo
TOKENS_GATEWAY_KEYPAIR = ./target/deploy/tokens_gateway-keypair.json
TWINE_CHAIN_KEYPAIR = target/deploy/twine_chain-keypair.json
SOL_PUBKEY = 11111111111111111111111111111111


# ==============================
#        Phony Targets
# ==============================
.PHONY: all build clean test install help

# ==============================
#        Help Target
# ==============================
help:
	@echo ""
	@echo "Available targets:"
	@echo "  build                             Build the project"
	@echo "  build-sbf                         Build the project for SBF"
	@echo "  clean                             Clean build artifacts"
	@echo "  test                              Run tests"
	@echo "  deploy                            Deploy programs"
	@echo "  update-tokens-gateway             Update the tokens gateway"
	@echo "  keygen-tokens-gateway-program-id  Generate pubkey for tokens gateway"
	@echo "  keygen-twine-chain-program-id     Generate pubkey for twine chain"
	@echo "  start-validator                   Start a new solana-test-validator"
	@echo "  initialize                        Initialize programs"
	@echo "  create-spl-token                  Create a new SPL token"
	@echo "  update-token-mapping              Update token mapping"
	@echo "  deposit-native-token              Deposit native tokens"
	@echo "  deposit-spl-token                 Deposit SPL tokens"
	@echo "  forced-native-token-withdrawal    Forced native token withdrawal"
	@echo "  forced-spl-token-withdrawal       Forced SPL token withdrawal"
	@echo "  get-all-pdas                      Get all pdas"
	@echo "  get-batch-pda                     Get batch pda id"
	@echo "  get-messages-buffer-data          Get messages buffer data"
	@echo "  get-twine-chain-storage-data      Get twine chain storage data"
	@echo ""

# Build the project
build:
	$(CARGO) build --workspace

# Build the project
build-sbf:
	$(CARGO) build-sbf --workspace

# Clean build artifacts
clean:
	$(CARGO) clean

test:
	$(CARGO) test

deploy:
	$(CARGO) run --bin deploy_programs

update-tokens-gateway:
	$(CARGO) run --bin update_tokens_gateway

keygen-tokens-gateway-program-id:
	solana-keygen pubkey $(TOKENS_GATEWAY_KEYPAIR)

keygen-twine-chain-program-id:
	solana-keygen pubkey $(TWINE_CHAIN_KEYPAIR)

start-validator:
	@echo "Starting new solana-test-validator..."
	solana-test-validator -r 

initialize:
	$(CARGO) run --bin interaction -- initialize-programs

create-spl-token:
	$(CARGO) run --bin interaction -- create-spl-token

# make update-token-mapping l1_token=your_l1_token_here l2_token=your_l2_token_here l1_decimals=9 l2_decimals=18
update-token-mapping:
	@echo "Updating token mapping..."
	cargo run --bin interaction -- token-mapping "$(l1_token)" "$(l2_token)" "$(l1_decimals)" "$(l2_decimals)"

# make deposit-native-token amount=amount_here receiver_address=receiver_address_here l2_token=l2_token_here data=data_here
deposit-native-token:
	@echo "Depositing native tokens..."
	cargo run --bin interaction -- deposit-native-token "$(SOL_PUBKEY)" "$(l2_token)" "$(receiver_address)" "$(amount)" "$(data)"

# make deposit-spl-token l1_token=l1_token_here l2_token=l2_token_here receiver_address=receiver_address_here user_token_account= user_token_account_here amount=amount_here data=data_here
deposit-spl-token:
	@echo "Depositing native tokens..."
	cargo run --bin interaction -- deposit-spl-token "$(l1_token)" "$(l2_token)" "$(receiver_address)" "$(user_token_account)" "$(amount)" "$(data)"

# make forced-native-token-withdrawal  l2Token=your_l2_token_address_here twineAccount=your_twine_account_here privateKey=your_private_key_here amount=100
forced-native-token-withdrawal:
	@echo "Forced Native Token Withdrawal..."
	cargo run --bin interaction -- forced-native-withdrawal  "$(SOL_PUBKEY)" "$(l2Token)" "$(twineAccount)" "$(privateKey)" "$(amount)"

# make forced-spl-token-withdrawal l1Token=your_l1_token_pubkey_here l2Token=your_l2_token_address_here twineAccount=your_twine_account_here privateKey=your_private_key_here receiver=your_receiver_pubkey_here amount=100 
forced-spl-token-withdrawal:
	@echo "Forced Spl Token Withdrawal..."
	cargo run --bin interaction -- forced-spl-withdrawal  "$(l1Token)" "$(l2Token)" "$(twineAccount)" "$(privateKey)" "$(receiver)" "$(amount)"

execute_native_l2_withdrawal:
	@echo "Execute L2 initiated native Token Withdrawal..."
	cargo run --bin interaction -- execute-native-l2-withdrawal "${receiver}" "${publicValue}" "${executionProof}" 

get-all-pdas:
	@echo "Get all pdas"
	cargo run --bin interaction -- get-all-pdas

get-messages-buffer-data:
	@echo "Get all pdas"
	cargo run --bin interaction -- get-messages-buffer-data

get-twine-chain-storage-data:
	@echo "Get all pdas"
	cargo run --bin interaction -- get-twine-chain-storage-data

# make get-batch-pda batchNumber=the_batch_number
get-batch-pda:
	@echo "Get the pda id of the batch"
	cargo run --bin interaction -- get-batch-pda "$(batchNumber)"
