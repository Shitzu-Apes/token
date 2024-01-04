// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

import "openzeppelin-contracts/access/AccessControl.sol";
import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/utils/Strings.sol";
import {AuroraSdk, Codec, NEAR, PromiseCreateArgs, PromiseResult, PromiseResultStatus, PromiseWithCallback} from "aurora-sdk/AuroraSdk.sol";

uint64 constant APPROVE_NEAR_GAS = 20_000_000_000_000;
uint64 constant MIGRATE_NEAR_GAS = 30_000_000_000_000;
uint64 constant MIGRATE_CALLBACK_NEAR_GAS = 125_000_000_000_000;
uint64 constant REFUND_NEAR_GAS = 30_000_000_000_000;

contract ShitzuMigrate is AccessControl {
    using AuroraSdk for NEAR;
    using AuroraSdk for PromiseCreateArgs;
    using AuroraSdk for PromiseWithCallback;
    using Codec for bytes;

    bytes32 public constant CALLBACK_ROLE = keccak256("CALLBACK_ROLE");

    IERC20 public wNEAR;
    IERC20 public shitzuAurora;
    string public shitzuNearId;
    NEAR public near;

    constructor(
        IERC20 _wNEAR,
        IERC20 _shitzuAurora,
        string memory _shitzuNearId
    ) {
        near = AuroraSdk.initNear(_wNEAR);
        wNEAR = _wNEAR;
        shitzuAurora = _shitzuAurora;
        shitzuNearId = _shitzuNearId;
        _grantRole(
            CALLBACK_ROLE,
            AuroraSdk.nearRepresentitiveImplicitAddress(address(this))
        );
    }

    function approveWNEAR() public {
        uint256 amount = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
        PromiseCreateArgs memory approveCall = near.auroraCall(
            address(this.wNEAR()),
            abi.encodeWithSelector(
                0x095ea7b3, // approve method selector
                address(this),
                amount
            ),
            0,
            APPROVE_NEAR_GAS
        );
        approveCall.transact();
    }

    function migrate(string memory accountId, uint128 amount) public {
        shitzuAurora.transferFrom(msg.sender, address(this), amount);

        bytes memory data = abi.encodePacked(
            '{"account_id": "',
            accountId,
            '", "amount": "',
            Strings.toString(amount),
            '"}'
        );
        PromiseCreateArgs memory callMigrate = near.call(
            shitzuNearId,
            "migrate",
            data,
            1,
            MIGRATE_NEAR_GAS
        );
        PromiseCreateArgs memory callback = near.auroraCall(
            address(this),
            abi.encodeWithSelector(
                this.migrateCallback.selector,
                msg.sender,
                amount
            ),
            0,
            MIGRATE_CALLBACK_NEAR_GAS
        );

        callMigrate.then(callback).transact();
    }

    function migrateCallback(
        address sender,
        uint128 amount
    ) public onlyRole(CALLBACK_ROLE) {
        PromiseResult memory promiseResult = AuroraSdk.promiseResult(0);

        if (promiseResult.status != PromiseResultStatus.Successful) {
            shitzuAurora.transfer(sender, amount);
        } else {
            shitzuAurora.transfer(
                0x0000000000000000000000000000000000000000,
                amount
            );
        }
    }
}
