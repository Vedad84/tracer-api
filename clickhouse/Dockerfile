from yandex/clickhouse-server:21.3.20.1

COPY config.xml /etc/clickhouse-server/config.xml
COPY users.xml /etc/clickhouse-server/users.d/user.xml
COPY 202110061400_initial.up.sql /etc/clickhouse-server/schema.sql
COPY deploy.sh /docker-entrypoint-initdb.d/deploy.sh
