# ==============================
#        Variable Section
# ==============================
CARGO = cargo
TOKENS_GATEWAY_KEYPAIR = ./target/deploy/tokens_gateway-keypair.json
TWINE_CHAIN_KEYPAIR = target/deploy/twine_chain-keypair.json
OAPP_KEYPAIR = target/deploy/oapp-keypair.json
TWINE_CHAIN_LIB     ?= twine_chain/src/lib.rs
TOKENS_GATEWAY_LIB  ?= tokens_gateway/src/lib.rs
OAPP_LIB ?= oapp/src/lib.rs
SOL_PUBKEY = 11111111111111111111111111111111

# ==============================
#        Phony Targets
# ==============================
.PHONY: all build build-sbf clean test deploy help \
		sync-keys sync-keys-twine-chain sync-keys-gateway update-admin \
        update-tokens-gateway update-twine-chain \
        keygen-tokens-gateway-program-id keygen-twine-chain-program-id \
        start-validator initialize \
        create-spl-token update-token-mapping \
        deposit-native-token deposit-spl-token \
        forced-native-token-withdrawal forced-spl-withdrawal \
        forced-native-withdrawal execute-native-l2-withdrawal execute-spl-l2-withdrawal \
        get-all-pdas get-batch-pda get-messages-buffer-data get-detailed-messages-buffer-data get-tokens-mapping-data\
        get-associated-token-account get-twine-chain-storage-data \
        process-native-forced-withdrawal process-native-refund \
		add-role-in-twine-chain add-role-in-tokens-gateway init-send-library\

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
	@echo "=== UPDATE ADMIN ==="
	@echo "  update-admin                      Update INITIAL_CHAIN_ADMIN for both programs"
	@echo ""
	@echo "=== KEY GENERATION TARGETS ==="
	@echo "  keygen-tokens-gateway-program-id  Generate pubkey for tokens gateway"
	@echo "  keygen-twine-chain-program-id     Generate pubkey for twine chain"
	@echo ""
	@echo "=== SYNC KEYS ==="
	@echo "  sync-keys                         Update key in 'declare_id' with correct pubkeys"
	@echo "  sync-keys-twine-chain             Update key in 'declare_id' with correct pubkey for twine chain"
	@echo "  sync-keys-gateway                 Update key in 'declare_id' with correct pubkey for tokens gateway"
	@echo ""
	@echo "=== DEVELOPMENT TARGETS ==="
	@echo "  start-validator                   Start a new solana-test-validator"
	@echo "  initialize                        Initialize programs"
	@echo ""
	@echo "=== TOKEN OPERATIONS ==="
	@echo "  create-spl-token                  Create a new SPL token"
	@echo "  update-token-mapping              Update token mapping"
	@echo "  remove-token-mapping              Update token mapping"
	@echo "  deposit-native-token              Deposit native tokens"
	@echo "  deposit-spl-token                 Deposit SPL tokens"
	@echo ""
	@echo "=== WITHDRAWAL OPERATIONS ==="
	@echo "  forced-native-withdrawal          Forced native token withdrawal"
	@echo "  forced-spl-withdrawal             Forced SPL token withdrawal"
	@echo "  execute-native-l2-withdrawal      Execute l2 initiated native token withdrawal"
	@echo "  execute-spl-l2-withdrawal         Execute l2 initiated spl token withdrawal"
	@echo "  process-native-forced-withdrawal  Process the forced withdrawal"
	@echo "  process-native-refund             Process the refund"
	@echo ""
	@echo "=== ROLE MANAGEMENT ==="
	@echo "  add-role-in-twine-chain           Add role in twine chain"
	@echo "  remove-role-in-twine-chain        Remove role in twine chain"
	@echo "  add-role-in-tokens-gateway        Add role in tokens gateway"
	@echo "  remove-role-in-tokens-gateway     Remove role in tokens gateway"
	@echo ""
	@echo "=== Copy MessageBuffer ==="
	@echo "  copy-messages-buffer           Copy the message buffer"
	@echo ""
	@echo "=== DATA RETRIEVAL TARGETS ==="
	@echo "  get-all-pdas                      Get all pdas"
	@echo "  get-batch-pda                     Get batch pda id"
	@echo "  get-messages-buffer-data          Get messages buffer data"
	@echo "  get-detailed-messages-buffer-data Get detailed messages buffer data"
	@echo "  get-tokens-mapping-data           Get tokens mapping data"
	@echo "  get-executed-payouts-buffer-data  Get executed payouts buffer data"
	@echo "  get-associated-token-account      Get associated token account of a wallet"
	@echo "  get-message-replicator-data       Get Message Replicator data"
	@echo "  get-twine-chain-storage-data      Get twine chain storage data"
	@echo "  get-twine-chain-role-manager-data Get twine chain role manager data"
	@echo "  get-tokens-gateway-role-manager-data Get tokens gateway role manager data"
	@echo ""
	@echo ""



# Detect proper -i for sed
# macos needs extra '-i' flag for sed
SED_INPLACE := -i
ifeq ($(shell uname -s),Darwin)
  SED_INPLACE := -i ''
endif

# ==============================
#       Update chain admins  
# ==============================
update-admin:
	@if [ -z "$(ADMIN)" ]; then \
		echo "Error: ADMIN variable not provided. Usage: make update-admin ADMIN=<new_address>"; \
		exit 1; \
	fi
	@echo "Updating admin address to: $(ADMIN)"
	@grep -rl 'pub const INITIAL_CHAIN_ADMIN: &str =' . --include '*.rs' | \
	while read -r f; do \
		sed $(SED_INPLACE) -E \
			's|^pub const INITIAL_CHAIN_ADMIN: &str = ".*";|pub const INITIAL_CHAIN_ADMIN: \&str = "$(ADMIN)";|' \
			"$$f"; \
	done
	@echo "Successfully updated admin address"

# ==============================
#       Update program pubkeys
# ==============================
sync-keys: sync-keys-twine-chain sync-keys-gateway sync-keys-oapp
	@echo "All keys synced successfully."

sync-keys-oapp:
	@PUBKEY=$$(solana-keygen pubkey $(OAPP_KEYPAIR)); \
	echo "Setting declare_id! to $$PUBKEY in $(OAPP_LIB)"; \
	sed $(SED_INPLACE) -E \
	  's|^solana_program::declare_id!\("[^"]*"\);$$|solana_program::declare_id!("'"$$PUBKEY"'");|' \
	  $(OAPP_LIB); \
	echo "Updated: $(OAPP_LIB)"

sync-keys-twine-chain:
	@PUBKEY=$$(solana-keygen pubkey $(TWINE_CHAIN_KEYPAIR)); \
	echo "Setting declare_id! to $$PUBKEY in $(TWINE_CHAIN_LIB)"; \
	sed $(SED_INPLACE) -E \
	  's|^solana_program::declare_id!\("[^"]*"\);$$|solana_program::declare_id!("'"$$PUBKEY"'");|' \
	  $(TWINE_CHAIN_LIB); \
	echo "Updated: $(TWINE_CHAIN_LIB)"

sync-keys-gateway:
	@PUBKEY=$$(solana-keygen pubkey $(TOKENS_GATEWAY_KEYPAIR)); \
	echo "Setting declare_id! to $$PUBKEY in $(TOKENS_GATEWAY_LIB)"; \
	sed $(SED_INPLACE) -E \
	  's|^solana_program::declare_id!\("[^"]*"\);$$|solana_program::declare_id!("'"$$PUBKEY"'");|' \
	  $(TOKENS_GATEWAY_LIB); \
	echo "Updated: $(TOKENS_GATEWAY_LIB)"

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

update-oapp:
	$(CARGO) run --bin update_oapp


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

# Usage: make remove-token-mapping l1_token=your_l1_token_here l2_token=your_l2_token_here
remove-token-mapping:
	@echo "Updating token mapping..."
	$(CARGO) run --bin interaction -- remove-token-mapping "$(l1_token)" "$(l2_token)"

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

# Usage: make process-spl-refund l1_token=l1_token_address l1_receiver=receiver_address message_nonce=nonce_value  public_values=values proof=proof_data
process-spl-refund:
	@echo "Processing spl refund..."
	$(CARGO) run --bin interaction -- process-spl-refund "$(l1_token)" "$(l1_receiver)" "$(message_nonce)" "$(public_values)" "$(proof)"

# Usage: make process-native-forced-withdrawal message_nonce=nonce_value receiver=receiver_address public_values=values proof=proof_data
process-native-forced-withdrawal:
	@echo "Processing native forced withdrawal..."
	$(CARGO) run --bin interaction -- process-native-forced-withdrawal "$(message_nonce)" "$(receiver)" "$(public_values)" "$(proof)"

# Usage: make process-spl-forced-withdrawal l1_token=l1_token_address l1_receiver=receiver_address message_nonce=nonce_value  public_values=values proof=proof_data
process-spl-forced-withdrawal:
	@echo "Processing spl forced withdrawal..."
	$(CARGO) run --bin interaction -- process-spl-forced-withdrawal "$(l1_token)" "$(l1_receiver)" "$(message_nonce)" "$(public_values)" "$(proof)"

# ==============================
#        Role Management
# ==============================
# Usage: make add-role-in-twine-chain role_type= message_appender/twine_operation_handler user_pubkey=your_pubkey
add-role-in-twine-chain:
	@echo "Add Role in twineChain..."
	$(CARGO) run --bin interaction -- add-role-in-twine-chain "$(role_type)" "$(user_pubkey)"

# Usage: make remove-role-in-twine-chain role_type= message_appender/twine_operation_handler user_pubkey=your_pubkey
remove-role-in-twine-chain:
	@echo "Add Role in twineChain..."
	$(CARGO) run --bin interaction -- remove-role-in-twine-chain "$(role_type)" "$(user_pubkey)"

# Usage: make add-role-in-tokens-gateway role_type=twine_operation_handler user_pubkey=your_pubkey
add-role-in-tokens-gateway:
	@echo "Add Role in twineChain..."
	$(CARGO) run --bin interaction -- add-role-in-tokens-gateway "$(role_type)" "$(user_pubkey)"

# Usage: make remove-role-in-tokens-gateway role_type=twine_operation_handler user_pubkey=your_pubkey
remove-role-in-tokens-gateway:
	@echo "Add Role in twineChain..."
	$(CARGO) run --bin interaction -- remove-role-in-tokens-gateway "$(role_type)" "$(user_pubkey)"
# ==============================
#        Data Retrieval Targets
# ==============================
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

get-detailed-messages-buffer-data:
	@echo "Getting messages buffer data..."
	$(CARGO) run --bin interaction -- get-detailed-messages-buffer-data

get-tokens-mapping-data:
	@echo "Getting messages buffer data..."
	$(CARGO) run --bin interaction -- get-tokens-mapping-data

# Usage: make get-associated-token-account wallet_address=your_wallet_address spl_token=your_spl_token
get-associated-token-account:
	@echo "Getting associated token account..."
	$(CARGO) run --bin interaction -- get-associated-token-account "$(wallet_address)" "$(spl_token)"

get-twine-chain-storage-data:
	@echo "Getting twine chain storage data..."
	$(CARGO) run --bin interaction -- get-twine-chain-storage-data

# Usage: make get-message-replicator-data start_nonce=the_start_nonce end_nonce=the_end_nocne
get-message-replicator-data:
	@echo "Getting message replicator data..."	
	$(CARGO) run --bin interaction -- get-message-replicator-data "$(start_nonce)" "$(end_nonce)"

get-twine-chain-role-manager-data:
	@echo "Getting twine chain role manager data..."
	$(CARGO) run --bin interaction -- get-twine-chain-role-manager-data

get-tokens-gateway-role-manager-data:
	@echo "Getting tokens gateway role manager data..."
	$(CARGO) run --bin interaction -- get-tokens-gateway-role-manager-data


# ==============================
#     Copy Message Buffer
# ==============================
# Usage: make copy-messages-buffer start_nonce=the_start_nonce end_nonce=the_end_nonce
copy-messages-buffer:
	@echo "Copy Message Buffer..."
	$(CARGO) run --bin interaction -- copy-messages-buffer "$(start_nonce)" "$(end_nonce)"



# ==============================
#     OApp Operations
# ==============================
init-store:
	@echo "Initializing OApp Store..."
	$(CARGO) run --bin interaction -- initialize-store

init-send-library:
	@echo "Initializing Send Library..."
	$(CARGO) run --bin interaction -- init-send-library

# Usage: make init-nonce remote_oapp=remote_oapp_address
init-nonce:
	@echo "Initializing Nonce.."
	$(CARGO) run --bin interaction -- init-nonce "$(remote_oapp)"

init-config:
	@echo "Initializing Config..."
	$(CARGO) run --bin interaction -- init-config

set-send-library:
	@echo "Setting Send Library..."
	$(CARGO) run --bin interaction -- set-send-library

set-config:
	@echo "Setting Config..."
	$(CARGO) run --bin interaction -- set-config
