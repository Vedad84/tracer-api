require("@nomiclabs/hardhat-waffle");
require('dotenv').config();

const proxy_url = process.env.WEB3_URL;
const network_id = parseInt(process.env.NETWORK_ID);

// Private keys for test accounts
const privateKeys = [
  process.env.DEPLOYER_PRIVATE_KEY
];

module.exports = {
  solidity: "0.8.4",
  defaultNetwork: 'neonlabs',
  networks: {
    neonlabs: {
      url: proxy_url,
      accounts: privateKeys,
      network_id: network_id,
      chainId: network_id,
      allowUnlimitedContractSize: false,
      timeout: 1000000,
      isFork: true
    }
  }
};
