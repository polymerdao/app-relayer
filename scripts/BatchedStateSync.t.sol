// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.15;

import "forge-std/Script.sol";
import {BatchedStateSync} from "../src/BatchedStateSync.sol";

contract BatchedStateSyncScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address polymerProver = vm.envAddress("POLYMER_PROVER_ADDRESS");
        uint256 batchThreshold = 2;

        vm.startBroadcast(deployerPrivateKey);
        
        BatchedStateSync batchedStateSync = new BatchedStateSync(
            polymerProver,
            batchThreshold
        );
        
        vm.stopBroadcast();

        console.log("BatchedStateSync deployed at:", address(batchedStateSync));
    }
}
