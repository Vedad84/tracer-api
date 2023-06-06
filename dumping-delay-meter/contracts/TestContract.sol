pragma solidity >=0.5.12;

contract TestContract {
    uint256 creationBlock;

    constructor() {
        creationBlock = block.number;
    }

    function getCreationBlock() external view returns (uint256) {
        return creationBlock;
    }
}