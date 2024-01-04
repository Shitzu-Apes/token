#!/bin/bash
set -e

near_key_path=~/.near-credentials/testnet/shrm.testnet.json
aurora_key=$(cat aurora-key.json | jq '.secret_key' | cut -c 2- | rev | cut -c 2- | rev)

cd solidity
forge build

# Deploy Codec library
code=$(cat out/Codec.sol/Codec.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
aurora-cli --network testnet --near-key-path $near_key_path deploy --code $code --aurora-secret-key $aurora_key
# Codec address: 0x89be868c648f772af7cbef1c781714abafacb292

# Deploy Utils library
code=$(cat out/Utils.sol/Utils.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
aurora-cli --network testnet --near-key-path $near_key_path deploy --code $code --aurora-secret-key $aurora_key
# Utils address: 0x5faf69b11ba25a072158da094694c0044a9b0808

# Deploy AuroraSdk library
forge build --libraries aurora-sdk/Codec.sol:Codec:0x89be868c648f772af7cbef1c781714abafacb292 --libraries aurora-sdk/Utils.sol:Utils:0x5faf69b11ba25a072158da094694c0044a9b0808
code=$(cat out/AuroraSdk.sol/AuroraSdk.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
aurora-cli --network testnet --near-key-path $near_key_path deploy --code $code --aurora-secret-key $aurora_key
# AuroraSdk address: 0x399985bfb386238f5e92bc6f34f69629c0b48111

# Deploy ShitzuMigrate
forge build --libraries aurora-sdk/AuroraSdk.sol:AuroraSdk:0x399985bfb386238f5e92bc6f34f69629c0b48111
code=$(cat out/ShitzuMigrate.sol/ShitzuMigrate.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
cat out/ShitzuMigrate.sol/ShitzuMigrate.json | jq '.abi' > out/ShitzuMigrate.sol/ShitzuMigrate.abi
aurora-cli --network testnet --near-key-path $near_key_path deploy --code $code --abi-path out/ShitzuMigrate.sol/ShitzuMigrate.abi --args '{"_shitzuAccountId": "shitzu-token.testnet", "_wNEAR": "4861825E75ab14553E5aF711EbbE6873d369d146"}' --aurora-secret-key $aurora_key
# ShitzuMigrate address: 0x00e44529ce3addac5019dc8e279eaca617932ce1
