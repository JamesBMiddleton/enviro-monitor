# enviro-monitor

A minimal, complete IoT system. Motivation for the project was exposure to each layer of a typical IoT architecture, as well as the embedded rust ecosystem.

<p align="center">
  <img width="310" height="615" alt="iot_diagram drawio-2" src="https://github.com/user-attachments/assets/4935b8a9-5c1f-4ba5-b318-0229fddedcc1" />
</p>

_To run docker compose_
```
export INFLUX_ADMIN_USERNAME="admin"
export INFLUX_ADMIN_PASSWORD="MyAdminPassword"
export INFLUX_ADMIN_TOKEN="$(openssl rand -hex 32)"

cd docker
echo $INFLUX_ADMIN_USERNAME > influxdb2-admin-username
echo $INFLUX_ADMIN_PASSWORD > influxdb2-admin-password
echo $INFLUX_ADMIN_TOKEN > influxdb2-admin-token
```
Change the `token` field in `docker/telegraf.conf` and `docker/grafana-provisioning/datasources/automatic.yaml` to the contents of `docker/influxdb2-admin-token`
```
docker compose up
```

_To build and flash the firmware_
```
export SSID="YourWifiSSID"
export PASSWORD="YourWifiPassword

cd firmware
cargo run --release
```

