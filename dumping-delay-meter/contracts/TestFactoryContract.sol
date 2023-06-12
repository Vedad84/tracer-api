pragma solidity >=0.5.12;

import "./TestContract.sol";

contract TestFactoryContract {
    event contractCreated(address newContract);

    function createNewContract() external {
        TestContract new_contract = new TestContract();
        emit contractCreated(address(new_contract));
    }
}