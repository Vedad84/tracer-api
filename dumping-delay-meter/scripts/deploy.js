const hre = require("hardhat");
const fs = require('fs');
async function main() {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying test factory with the account:", deployer.address);

  const factory = await hre.ethers.getContractFactory("TestFactoryContract");
  const testFactory = await factory.deploy();

  await testFactory.deployed();
  console.log("Contract address is: ", testFactory.address);

  const content = `FACTORY_ADDRESS="${testFactory.address}"
FACTORY_ABI="./artifacts/contracts/TestFactoryContract.sol/TestFactoryContract.json"
TEST_CONTRACT_ABI="./artifacts/contracts/TestContract.sol/TestContract.json"`;

  fs.writeFile('./.env', content, err => {
    if (err) {
      console.error(err);
    }
    console.log("Environment file created")
  });
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
