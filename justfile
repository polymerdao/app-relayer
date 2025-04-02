private_key := env("PRIVATE_KEY")
polymer_api_token_testnet := env("POLYMER_API_TOKEN_TESTNET")

polymer_prover_address_dev := "0xfbfbfDdd6e35dA57b7B0F9a2C10E34Be70B3A4E9"
polymer_prover_address_sepolia := "0xcDa03d74DEc5B24071D1799899B2e0653C24e5Fa"

contract_addr_chain_a := "0x75364ec12D31Cc678dfCFDFc25FF264aC863211A"
contract_addr_chain_b := "0x75364ec12D31Cc678dfCFDFc25FF264aC863211A"

contract_addr_optimism_sepolia := "0x8af2F08959D5D389dEF029d5A6A7C876c1E329ca"
contract_addr_base_sepolia := "0x7c81028766F3283473c8840e30eB466a7809bf3E"

testnet_config := "./ts-relayer/config/config.testnet.yaml"
optimism_sepolia_rpc_url := env("OPTIMISM_SEPOLIA_RPC_URL", "https://sepolia.optimism.io")


deploy-dev-chain-a:
    POLYMER_PROVER_ADDRESS={{ polymer_prover_address_dev }} \
    forge script scripts/BatchedStateSync.t.sol:BatchedStateSyncScript \
    --broadcast \
    --rpc-url http://localhost:8553

deploy-dev-chain-b:
    POLYMER_PROVER_ADDRESS={{ polymer_prover_address_dev }} \
    forge script scripts/BatchedStateSync.t.sol:BatchedStateSyncScript \
    --broadcast \
    --rpc-url http://localhost:8554

deploy-optimism-sepolia:
    POLYMER_PROVER_ADDRESS={{ polymer_prover_address_sepolia }} \
    forge script scripts/BatchedStateSync.t.sol:BatchedStateSyncScript \
    --broadcast \
    --verifier blockscout \
    --rpc-url https://sepolia.optimism.io

deploy-base-sepolia:
    POLYMER_PROVER_ADDRESS={{ polymer_prover_address_sepolia }} \
    forge script scripts/BatchedStateSync.t.sol:BatchedStateSyncScript \
    --broadcast \
    --verifier blockscout \
    --rpc-url https://sepolia.base.org

run:
    cd ts-relayer && \
    CONFIG_PATH=./config/config.dev.yaml yarn dev 2>&1 | tee dev.log

call-crossChainChecker-chain-a:
    cast call {{ contract_addr_chain_a }} \
       "crossChainChecker(uint32)(bool,bytes,uint256)" \
       902 \
       --rpc-url http://localhost:8553

call-crossChainChecker-chain-b:
    cast call {{ contract_addr_chain_b }} \
       "crossChainChecker(uint32)(bool,bytes,uint256)" \
       902 \
       --rpc-url http://localhost:8554

call-crossChainChecker-optimism-sepolia:
    cast call {{ contract_addr_optimism_sepolia }} \
       "crossChainChecker(uint32)(bool,bytes,uint256)" \
       11155420 \
       --rpc-url https://sepolia.optimism.io

call-crossChainChecker-base-sepolia:
    cast call {{ contract_addr_base_sepolia }} \
       "crossChainChecker(uint32)(bool,bytes,uint256)" \
       84532 \
       --rpc-url https://sepolia.base.org

update-batch-dev:
    cast send "{{ contract_addr_chain_a }}" \
        "setBatchedValue(string,bytes)" \
        "key1" "0x1234" \
        --rpc-url http://localhost:8553 \
        --private-key {{private_key}}
    cast send "{{ contract_addr_chain_a }}" \
        "setBatchedValue(string,bytes)" \
        "key2" "0x5678" \
        --rpc-url http://localhost:8553 \
        --private-key {{private_key}}
    cast send "{{ contract_addr_chain_a }}" \
        "setBatchedValue(string,bytes)" \
        "key3" "0x9abc" \
        --rpc-url http://localhost:8553 \
        --private-key {{private_key}}

update-batch-testnet:
    cast send "{{ contract_addr_optimism_sepolia }}" \
        "setBatchedValue(string,bytes)" \
        "key1" "0x1234" \
        --rpc-url {{ optimism_sepolia_rpc_url }} \
        --private-key {{private_key}}
    cast send "{{ contract_addr_optimism_sepolia }}" \
        "setBatchedValue(string,bytes)" \
        "key2" "0x5678" \
        --rpc-url {{ optimism_sepolia_rpc_url }} \
        --private-key {{private_key}}
    cast send "{{ contract_addr_optimism_sepolia }}" \
        "setBatchedValue(string,bytes)" \
        "key3" "0x9abc" \
        --rpc-url {{ optimism_sepolia_rpc_url }} \
        --private-key {{private_key}}

build-docker:
    docker build -t ts-relayer -f ts-relayer/Dockerfile .

run-docker:
    docker run -it --rm \
        -e PRIVATE_KEY={{ private_key }} \
        -e POLYMER_API_TOKEN={{ polymer_api_token_testnet }} \
        -v {{ testnet_config }}:/app/config/config.yaml \
        ts-relayer
