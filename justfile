private_key := env("PRIVATE_KEY")
polymer_prover_address_dev := "0xfbfbfDdd6e35dA57b7B0F9a2C10E34Be70B3A4E9"
polymer_prover_address_sepolia := "0x4B723ee254aAbCf22b4D98a709F86C62A97D9957"
contract_addr_chain_a := "0x75364ec12D31Cc678dfCFDFc25FF264aC863211A"
contract_addr_chain_b := "0x75364ec12D31Cc678dfCFDFc25FF264aC863211A"

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
    --rpc-url https://sepolia.optimism.io

deploy-base-sepolia:
    POLYMER_PROVER_ADDRESS={{ polymer_prover_address_sepolia }} \
    forge script scripts/BatchedStateSync.t.sol:BatchedStateSyncScript \
    --broadcast \
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

call-crossChainChecker-base-sepolia:
    cast call {{ contract_addr_chain_b }} \
       "crossChainChecker(uint32)(bool,bytes,uint256)" \
       902 \
       --rpc-url https://sepolia.base.org

call-crossChainChecker-optimism-sepolia:
    cast call {{ contract_addr_chain_a }} \
       "crossChainChecker(uint32)(bool,bytes,uint256)" \
       902 \
       --rpc-url https://sepolia.optimism.io

update-batch:
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
