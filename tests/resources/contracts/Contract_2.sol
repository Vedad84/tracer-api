// SPDX-License-Identifier: MIT

pragma solidity ^0.4.18;
contract ExistingWithoutABI  {

    address dc;

    function ExistingWithoutABI_func(address _t) public {
        dc = _t;
    }

    function setA_Signature(uint _val) public returns(bool success){
        require(dc.call(bytes4(keccak256("setA(uint256)")),_val));
        return true;
    }
}