#!/bin/bash
set -e

near_key_path=~/.near-credentials/mainnet/marior.near.json
aurora_key=$(cat aurora-key.json | jq '.secret_key' | cut -c 2- | rev | cut -c 2- | rev)

cd solidity
forge build

# Deploy Codec library
code=$(cat out/Codec.sol/Codec.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
aurora-cli --network mainnet --near-key-path $near_key_path deploy --code $code --aurora-secret-key $aurora_key
# Codec address: 0x346b24E661c1d6A46cF17300E63B7d56acEbB816

# Deploy Utils library
code=$(cat out/Utils.sol/Utils.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
aurora-cli --network mainnet --near-key-path $near_key_path deploy --code $code --aurora-secret-key $aurora_key
# Utils address: 0xf17961Be6CF6401e047CefC42721b2042b57CeCF

# Deploy AuroraSdk library
forge build --libraries aurora-sdk/Codec.sol:Codec:0x346b24E661c1d6A46cF17300E63B7d56acEbB816 --libraries aurora-sdk/Utils.sol:Utils:0xf17961Be6CF6401e047CefC42721b2042b57CeCF
code=$(cat out/AuroraSdk.sol/AuroraSdk.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
aurora-cli --network mainnet --near-key-path $near_key_path deploy --code $code --aurora-secret-key $aurora_key
# AuroraSdk address: 0xBBc81a1d9496DA65A256E14d7CD32E017Ff067e5

# Deploy ShitzuMigrate
forge build --libraries aurora-sdk/AuroraSdk.sol:AuroraSdk:0xBBc81a1d9496DA65A256E14d7CD32E017Ff067e5
code=$(cat out/ShitzuMigrate.sol/ShitzuMigrate.json | jq '.bytecode .object' | cut -c 4- | rev | cut -c 2- | rev)
cat out/ShitzuMigrate.sol/ShitzuMigrate.json | jq '.abi' > out/ShitzuMigrate.sol/ShitzuMigrate.abi
aurora-cli --network mainnet --near-key-path $near_key_path deploy --code $code --abi-path out/ShitzuMigrate.sol/ShitzuMigrate.abi --args '{"_shitzuNearId": "token.0xshitzu.near", "_wNEAR": "C42C30aC6Cc15faC9bD938618BcaA1a1FaE8501d", "_shitzuAurora": "68e401B61eA53889505cc1366710f733A60C2d41"}' --aurora-secret-key $aurora_key
# ShitzuMigrate address: 0xA6f40A8Ca2CE1A5D570A52BD34897aBDF75438FF
