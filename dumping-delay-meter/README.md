# Dumping delay meter

Tool to analyze time delay occured between transaction execution on a Validator-node connected to 
Neon proxy and corresponding account dumping events occured on a Validator-node connected to dumping
plugin. Analyzer doing this by generating new smart contracts using special TestFactory contract and
then, executing simple read-only methods of newly created contracts. Every new call to TestFactory
produces contract-creation event which then read by tool using eth_getLogs method. To better understand
mechanics of this tool refer to [Jira ticket NDEV-1409](https://neonlabs.atlassian.net/browse/NDEV-1409)

## Usage
cd to directory where this file is located and then

```bash
docker-compose build
docker-compose up -d
```

## Understanding results
Results of tests will be collected in the database running in docker. This database will be available 
at **localhost:5432**. There's single table **delay_stat** which contain three columns:
- **delay_slots** - (BIGINT) how much time (in solana slots) were left since event generation and TestContract method execution
- **success** - (BOOLEAN) was this call successful or not
- **err** - (VARCHAR(512)) description of the error (if any)

Each row in this table represents single call to TestContract method. By analyzing distribution of failed and successfull
calls related to delay_slots one can approximate delay of dumping pipeline.

**NOTE:** pgadmin GUI is also available on localhost:8080. 
- Login / password for GUI: il@neonlabs.org / qazwsx
- Login / password for DB: neon-proxy / neon-proxy-pass

## Tester settings

- **DEPLOYER_PRIVATE_KEY** - 32-bytes private key of the account which will be used to deploy FactoryContract
- **WEB3_URL** - main entrypoint to Neon infrastructure (neon-rpc)
- **CALLER** - 32-bytes private key of the account which will be used to deploy TestContracts using FactoryContract
- **NETWORK_ID** - Neon network ID (e. g. 245022926 for devnet)
- **GENERATION_INTERVAL** - interval in seconds between calling FactoryContract
- **MONITORING_INTERVAL** - interval in milliseconds between querying blockchain for log events
- **READ_DELAY_SPREAD_SLOTS** - interval in slots after TestContract creation event. Read-only function will be called somewhere within this interval (selected randomly)
- **PG_CONNECTION_STRING** - PostgreSQL connection string