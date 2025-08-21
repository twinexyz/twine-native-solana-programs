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
.PHONY: all build build-sbf clean test deploy help \
        update-tokens-gateway update-twine-chain \
        keygen-tokens-gateway-program-id keygen-twine-chain-program-id \
        start-validator initialize \
        create-spl-token update-token-mapping \
        deposit-native-token deposit-spl-token \
        forced-native-token-withdrawal forced-spl-token-withdrawal \
        forced-native-withdrawal execute-native-l2-withdrawal execute-spl-l2-withdrawal \
        get-all-pdas get-batch-pda get-messages-buffer-data \
        get-associated-token-account get-twine-chain-storage-data \
        get-executed-payouts-buffer-data process-native-forced-withdrawal process-native-refund

# ==============================
#        Help Target
# ==============================
help:
	@echo ""
	@echo "=== BUILD & TEST TARGETS ==="
	@echo "  build                             Build the project"
	@echo "  build-sbf                         Build the project for SBF"
	@echo "  clean                             Clean build artifacts"
	@echo "  test                              Run tests"
	@echo ""
	@echo "=== DEPLOYMENT TARGETS ==="
	@echo "  deploy                            Deploy programs"
	@echo "  update-tokens-gateway             Update the tokens gateway"
	@echo "  update-twine-chain                Update the twine chain"
	@echo ""
	@echo "=== KEY GENERATION TARGETS ==="
	@echo "  keygen-tokens-gateway-program-id  Generate pubkey for tokens gateway"
	@echo "  keygen-twine-chain-program-id     Generate pubkey for twine chain"
	@echo ""
	@echo "=== DEVELOPMENT TARGETS ==="
	@echo "  start-validator                   Start a new solana-test-validator"
	@echo "  initialize                        Initialize programs"
	@echo ""
	@echo "=== TOKEN OPERATIONS ==="
	@echo "  create-spl-token                  Create a new SPL token"
	@echo "  update-token-mapping              Update token mapping"
	@echo "  deposit-native-token              Deposit native tokens"
	@echo "  deposit-spl-token                 Deposit SPL tokens"
	@echo ""
	@echo "=== WITHDRAWAL OPERATIONS ==="
	@echo "  forced-native-withdrawal    	   Forced native token withdrawal"
	@echo "  forced-spl-token-withdrawal       Forced SPL token withdrawal"
	@echo "  execute-native-l2-withdrawal      Execute l2 initiated native token withdrawal"
	@echo "  execute-spl-l2-withdrawal         Execute l2 initiated spl token withdrawal"
	@echo "  process-native-forced-withdrawal  Process the forced withdrawal"
	@echo "  process-native-refund             Process the refund"
	@echo ""
	@echo "=== DATA TARGETS ==="
	@echo "  clear-all-pdas                    clear all pdas"
	@echo "  get-all-pdas                      Get all pdas"
	@echo "  get-batch-pda                     Get batch pda id"
	@echo "  get-messages-buffer-data          Get messages buffer data"
	@echo "  get-executed-payouts-buffer-data  Get executed payouts buffer data"
	@echo "  get-associated-token-account      Get associated token account of a wallet"
	@echo "  get-twine-chain-storage-data      Get twine chain storage data"
	@echo ""
	@echo ""

# ==============================
#        Build & Test Targets
# ==============================
build:
	$(CARGO) build --workspace

build-sbf:
	$(CARGO) build-sbf --workspace

clean:
	$(CARGO) clean

test:
	$(CARGO) test

# ==============================
#        Deployment Targets
# ==============================
deploy:
	$(CARGO) run --bin deploy_programs

update-tokens-gateway:
	$(CARGO) run --bin update_tokens_gateway

update-twine-chain:
	$(CARGO) run --bin update_twine_chain

# ==============================
#        Key Generation Targets
# ==============================
keygen-tokens-gateway-program-id:
	solana-keygen pubkey $(TOKENS_GATEWAY_KEYPAIR)

keygen-twine-chain-program-id:
	solana-keygen pubkey $(TWINE_CHAIN_KEYPAIR)

# ==============================
#        Development Targets
# ==============================
start-validator:
	@echo "Starting new solana-test-validator..."
	solana-test-validator -r 

initialize:
	$(CARGO) run --bin interaction -- initialize-programs

# ==============================
#        Token Operations
# ==============================
create-spl-token:
	$(CARGO) run --bin interaction -- create-spl-token

# Usage: make update-token-mapping l1_token=your_l1_token_here l2_token=your_l2_token_here l1_decimals=9 l2_decimals=18
update-token-mapping:
	@echo "Updating token mapping..."
	$(CARGO) run --bin interaction -- token-mapping "$(l1_token)" "$(l2_token)" "$(l1_decimals)" "$(l2_decimals)"

# Usage: make deposit-native-token l2_token=l2_token_here receiver_address=receiver_address_here amount=amount_here data=data_here
deposit-native-token:
	@echo "Depositing native tokens..."
	$(CARGO) run --bin interaction -- deposit-native-token "$(SOL_PUBKEY)" "$(l2_token)" "$(receiver_address)" "$(amount)" "$(data)"

# Usage: make deposit-spl-token l1_token=l1_token_here l2_token=l2_token_here receiver_twine_address=receiver_address_here user_token_account=user_token_account_here amount=amount_here data=data_here
deposit-spl-token:
	@echo "Depositing SPL tokens..."
	$(CARGO) run --bin interaction -- deposit-spl-token "$(l1_token)" "$(l2_token)" "$(receiver_twine_address)" "$(user_token_account)" "$(amount)" "$(data)"

# ==============================
#        Withdrawal Operations
# ==============================
# Usage: make forced-native-withdrawal l2_token=l2_token_here from_address=twine_address receiver_address=receiver_address_here private_key=your_private_key amount=amount_here   
forced-native-withdrawal:
	@echo "Forced native withdrawal..."
	$(CARGO) run --bin interaction -- forced-native-withdrawal "$(SOL_PUBKEY)" "$(l2_token)" "$(from_address)" "$(receiver_address)" "$(private_key)" "$(amount)"

# Usage: make forced-spl-withdrawal l1Token=your_l1_token_pubkey_here l2Token=your_l2_token_address_here twineAccount=your_twine_account_here privateKey=your_private_key_here receiver=your_receiver_pubkey_here amount=100 
forced-spl-withdrawal:
	@echo "Forced SPL Token Withdrawal..."
	$(CARGO) run --bin interaction -- forced-spl-withdrawal "$(l1Token)" "$(l2Token)" "$(twineAccount)" "$(privateKey)" "$(receiver)" "$(amount)"

# Usage: make execute-native-l2-withdrawal receiver=your_receiver_here publicValue=your_public_value_here executionProof=your_execution_proof_here
execute-native-l2-withdrawal:
	@echo "Execute L2 initiated native Token Withdrawal..." 
	$(CARGO) run --bin interaction -- execute-native-l2-withdrawal "$(receiver)" "$(publicValue)" "$(executionProof)"

# Usage: make execute-spl-l2-withdrawal splToken=spl_token_address receiver=receiving_address publicValue=public_values executionProof=your_execution_proof_here
execute-spl-l2-withdrawal:
	@echo "Execute L2 initiated SPL Token Withdrawal..." 
	$(CARGO) run --bin interaction -- execute-spl-l2-withdrawal "$(splToken)" "$(receiver)" "$(publicValue)" "$(executionProof)"

# Usage: make process-native-refund message_nonce=nonce_value receiver=receiver_address public_values=values proof=proof_data
process-native-refund:
	@echo "Processing native refund..."
	$(CARGO) run --bin interaction -- process-native-refund "$(message_nonce)" "$(receiver)" "$(public_values)" "$(proof)"

# Usage: make process-native-forced-withdrawal message_nonce=nonce_value receiver=receiver_address public_values=values proof=proof_data
process-native-forced-withdrawal:
	@echo "Processing native forced withdrawal..."
	$(CARGO) run --bin interaction -- process-native-forced-withdrawal "$(message_nonce)" "$(receiver)" "$(public_values)" "$(proof)"

# ==============================
#        Data Targets
# ==============================
clear-all-pdas:
	@echo "Get all pdas"
	cargo run --bin interaction -- clear-all-pdas

get-all-pdas:
	@echo "Getting all PDAs..."
	$(CARGO) run --bin interaction -- get-all-pdas

# Usage: make get-batch-pda batchNumber=the_batch_number
get-batch-pda:
	@echo "Getting batch PDA for batch number: $(batchNumber)"
	$(CARGO) run --bin interaction -- get-batch-pda "$(batchNumber)"

get-messages-buffer-data:
	@echo "Getting messages buffer data..."
	$(CARGO) run --bin interaction -- get-messages-buffer-data

get-executed-payouts-buffer-data:
	@echo "Getting executed payouts buffer data..."
	$(CARGO) run --bin interaction -- get-executed-payouts-buffer-data

# Usage: make get-associated-token-account wallet_address=your_wallet_address spl_token=your_spl_token
get-associated-token-account:
	@echo "Getting associated token account..."
	$(CARGO) run --bin interaction -- get-associated-token-account "$(wallet_address)" "$(spl_token)"

get-twine-chain-storage-data:
	@echo "Getting twine chain storage data..."
	$(CARGO) run --bin interaction -- get-twine-chain-storage-data



