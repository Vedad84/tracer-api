# Neon Tracer-API

## Terms

- **Neon-Dumper-Plugin** - special implementation of Solana-Geyser plugin suitable to save dumps of blockchain entities
  for further transaction replaying/tracing, historical queries and so on.
- **Dumper-DB** - database storing account/transaction/block/slot dumps produced by **Neon-Dumper-Plugin**
- **Transaction dump (historical transaction)** - Solana transaction stored inside **Dumper-DB**.
- **Account dump** - complete copy of some particular account at a given moment of time. Account dumped every time its
  state changed (lamports, data etc.) therefore account can have several dumps at a given slot. In that case, every
  particular dump is identified by write_version and transaction signature.

## Running full environment

Use **docker-compose-test.yml** as a template compose file. It contains next services:
- **postgres** - Dumper-DB. Crucial parameters:
  - POSTGRES_DB (env var) - name of database
  - POSTGRES_USER (env var) - configured user
  - POSTGRES_PASSWORD (env var) - password for configured user
- **pgadmin (not necessary)** - Web interface to postgres database suitable for debugging
- **validator** - Solana validator with Neon-Dumper-Plugin. Crucial parameters:
  - GEYSER_PLUGIN_CONFIG - path to JSON file inside container containing Dumper-Plugin configuration. Refer to 
  https://github.com/neonlabsorg/neon-dumper-plugin for details how to configure plugin. Default plugin configuration is
  stored at /opt/accountsdb-plugin-config.json. You can replace it by mapping external file as a volume.
- **dbcreation** - Database schema creation service for Neon Web3 Proxy 
(refer to https://github.com/neonlabsorg/proxy-model.py for details). Test environment in docker-compose-test.yml uses 
single postgres database for both dumper/tracer and proxy
- **indexer** - Neon Web3 Indexer service. (refer to https://github.com/neonlabsorg/proxy-model.py for details). 
Test environment in docker-compose-test.yml uses single postgres database for both dumper/tracer and indexer
- **proxy** - Neon Web3 Proxy service. (refer to https://github.com/neonlabsorg/proxy-model.py for details).
  Test environment in docker-compose-test.yml uses single postgres database for both dumper/tracer and proxy
- **deploy_contracts (not necessary)** - utility service deploying test smart-contracts to Neon-EVM.
- **neon-tracer** - Neon Tracer-API service. Environment variables:
  - LISTENER_ADDR - IP:PORT where to listen client connections
  - SOLANA_URL - URL of Solana Validator RPC entrypoint
  - EVM_LOADER - Address of Neon-EVM Loader smart-contract
  - NEON_API_URL - URL of Neon API (NeonCLI) (default: http://127.0.0.1:8080)
  - TRACER_DB_HOST - Hostname of Dumper-DB (same as for **postgres** service)
  - TRACER_DB_PORT - Port of Dumper-DB (same as for **postgres** service)
  - TRACER_DB_NAME - Name Dumper-DB database (same as POSTGRES_DB of **postgres** service)
  - TRACER_DB_USER - Username of Dumper-DB user (same as POSTGRES_USER of **postgres** service)
  - TRACER_DB_PASSWORD - Password of Dumper-DB user (same as POSTGRES_PASSWORD of **postgres** service)
  - WEB3_PROXY - URL of **proxy** service entrypoint
  - METRICS_IP - IP address where to provide Prometheus metrics data
  - METRICS_PORT - Port where to provide Prometheus metrics data (Finally, metrics will be provided at http://METRICS_IP:METRICS_PORT/metrics entrypoint)
  - NEON_TOKEN_MINT - address of Neon SPL token
  - NEON_CHAIN_ID - id of the network (e. g 111 for test environment)
  - MONITORING_INTERVAL_SEC - monitoring interval in seconds
- **faucet (not necessary)** - test faucet service
- **neon-rpc** - Router-like service providing single entrypoint to both **proxy** and **neon-tracer** services. 
Essentially just Nginx HTTP proxy server. Default test-configuration is stored inside image by path **/etc/nginx/nginx.conf**
You can find source of this configuration file in this repo at **./neon-rpc/nginx.conf** You can replace default in-image 
config by mapping external file inside **neon-rpc** container as volume
