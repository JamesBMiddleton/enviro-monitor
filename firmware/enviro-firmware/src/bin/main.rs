#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_net::{IpAddress, IpEndpoint, Runner, StackResources, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use embedded_io_async::{ErrorType, Read, Write};
use esp_hal::spi::{
    Mode,
    master::Spi,
};
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup, time::Rate};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_println as _;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, scan::ScanConfig,
    sta::StationConfig,
};
use minimq::{Buffers, ConfigBuilder, Publication, Session};
use bme68x::blocking::Bme68x;
use bme68x::interface::SpiInterface;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SERVER_IP: (u8, u8, u8, u8) = (192, 168, 1, 130);
const SERVER_PORT: u16 = 1883;

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 2.0.0
    // generator parameters: -o esp32c3 -o esp32c3-mini-1 -o esp-backtrace -o alloc -o log -o unstable-hal -o embassy -o wifi -o stable-x86_64-unknown-linux-gnu

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO2
    // - GPIO8
    // - GPIO9
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _gpio11 = peripherals.GPIO11;
    let _gpio12 = peripherals.GPIO12;
    let _gpio13 = peripherals.GPIO13;
    let _gpio14 = peripherals.GPIO14;
    let _gpio15 = peripherals.GPIO15;
    let _gpio16 = peripherals.GPIO16;
    let _gpio17 = peripherals.GPIO17;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    info!("Embassy initialized!");

    let spi_bus = Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    ).unwrap().with_sck(peripherals.GPIO8)
    .with_mosi(peripherals.GPIO9)
    .with_miso(peripherals.GPIO10);
    let cs = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    
    let spi_device = ExclusiveDevice::new(spi_bus, cs, esp_hal::delay::Delay::new()).unwrap();

    let spi_interface = SpiInterface::new(spi_device);

    let mut _bme688 = Bme68x::new(spi_interface, esp_hal::delay::Delay::new()); 

    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID.try_into().unwrap())
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                PASSWORD.try_into().unwrap(),
            )),
    );
    let wifi_interface = esp_radio::wifi::Interface::station();
    let mut wifi_controller = esp_radio::wifi::WifiController::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");

    info!("Wifi configured and started!");

    let config = embassy_net::Config::dhcpv4(Default::default());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    info!("Scanning...");

    let scan_config = ScanConfig::default().with_max(10);
    let result = wifi_controller.scan_async(&scan_config).await.unwrap();
    for ap in result {
        info!("{:?}", ap);
    }

    spawner.spawn(connection(wifi_controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("Got IP: {}", config.address);
    };

    let mqtt_rx_buffer = &mut [0u8; 1024];
    let mqtt_tx_buffer = &mut [0u8; 1024];
    let mut mqtt_session = Session::new(
        ConfigBuilder::new(Buffers::new(mqtt_rx_buffer, mqtt_tx_buffer))
            .client_id("sensor1")
            .unwrap(),
    );

    let mut tcp_rx_buffer = [0u8; 1024];
    let mut tcp_tx_buffer = [0u8; 1024];

    loop {
        let mut io = MqttIo {
            socket: TcpSocket::new(stack, &mut tcp_rx_buffer, &mut tcp_tx_buffer),
        };
        io.socket.set_timeout(Some(Duration::from_secs(10)));

        let remote = IpEndpoint::new(
            IpAddress::v4(SERVER_IP.0, SERVER_IP.1, SERVER_IP.2, SERVER_IP.3),
            SERVER_PORT,
        );

        info!("Attempting connection to {}...", remote);
        if let Err(e) = io.socket.connect(remote).await {
            info!("Connection error: {:?}", e);
            Timer::after(Duration::from_secs(3)).await;
            continue;
        };

        let mut conn = mqtt_session.connect(io.socket).await.unwrap();
        conn.publish(Publication::new("sensor1/temperature", "42.5"))
            .await
            .unwrap();
        conn.disconnect().await.unwrap();
        Timer::after(Duration::from_secs(5)).await;
    }
}

struct MqttIo<'a> {
    socket: TcpSocket<'a>,
}
impl<'a> ErrorType for MqttIo<'a> {
    type Error = embassy_net::tcp::Error;
}
impl<'a> Read for MqttIo<'a> {
    async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!() // I'm not reading anything
    }
}
impl<'a> Write for MqttIo<'a> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.socket.write(buf).await
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.socket.flush().await
    }
}

#[embassy_executor::task]
async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) {
    info!("start connection task");
    loop {
        info!("about to connect...");

        match controller.connect_async().await {
            Ok(info) => {
                info!("Wifi connected to {:?}", info);
                let info = controller.wait_for_disconnect_async().await.ok();
                info!("Wifi disconnected from {:?}", info);
            }
            Err(e) => {
                info!("Wifi failed to connect to {:?}", e);
            }
        }
        Timer::after(Duration::from_millis(5000)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await;
}
