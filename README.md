# enviro-monitor

A minimal, complete IoT system. Motivation for the project was exposure to each layer of a typical IoT architecture, as well as the embedded rust ecosystem.

<p align="center">
  <img width="30%" alt="iot_diagram drawio-2" src="https://github.com/user-attachments/assets/4935b8a9-5c1f-4ba5-b318-0229fddedcc1" />
</p>
<p align="center">

</p>

## Setup

__Docker Compose__
```
export INFLUX_ADMIN_USERNAME="admin"
export INFLUX_ADMIN_PASSWORD="MyAdminPassword"
export INFLUX_ADMIN_TOKEN="$(openssl rand -hex 32)"

echo $INFLUX_ADMIN_USERNAME > docker/influxdb2-admin-username
echo $INFLUX_ADMIN_PASSWORD > docker/influxdb2-admin-password
echo $INFLUX_ADMIN_TOKEN > docker/influxdb2-admin-token
```
> Now change the `token` field in `docker/telegraf.conf` and `docker/grafana-provisioning/datasources/automatic.yaml` to the contents of `docker/influxdb2-admin-token`
```
cd docker && docker compose up
```
> for Grafana, go to `http://localhost:3000`

> for InfluxDB, go to `http://localhost:8086`

__Firmware__
```
export SSID="YourWifiSSID"
export PASSWORD="YourWifiPassword"

cd firmware && cargo run --release
```

## Resources
https://files.seeedstudio.com/wiki/XIAO_WiFi/Resources/XIAO_ESP32C3_v1.3_SCH_260116.pdf

https://files.seeedstudio.com/wiki/Seeed-Studio-XIAO-ESP32/esp32-c3_datasheet.pdf

https://files.seeedstudio.com/wiki/Seeed-Studio-XIAO-ESP32/esp32-c3_datasheet.pdf

https://docs.espressif.com/projects/rust/book/preface.html

https://github.com/Miceuz/docker-compose-mosquitto-influxdb-telegraf-grafana/tree/master



