// SPDX-License-Identifier: MIT

pragma solidity >=0.6.0 <0.9.0;


contract FundMe {

    mapping(address => uint256) public addressToAmountFunded;
    address[] public funders;
    address public owner;

     constructor() public {
         owner = msg.sender;
     }

    function fund() public payable returns (uint256){
        addressToAmountFunded[msg.sender] += msg.value;
        funders.push(msg.sender);
        return msg.value;
    }

     modifier onlyOwner {
         require(msg.sender == owner);
         _;
     }

     function withdraw() payable onlyOwner public {
         msg.sender.transfer(address(this).balance);

         for (uint256 funderIndex=0; funderIndex < funders.length; funderIndex++){
             address funder = funders[funderIndex];
             addressToAmountFunded[funder] = 0;
         }
         funders = new address[](0);
     }
}