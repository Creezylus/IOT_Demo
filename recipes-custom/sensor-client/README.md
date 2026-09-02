# Sensor Client Application

This repository contains the `sensor-client` application, designed to gather, filter, and transmit high-frequency sensor payload data to an upstream edge server over TCP.

---

## Features

* **Multi-Threaded Architecture:** Runs a dedicated network/processing thread (`net_thread_func`) alongside a data collection thread (`sim_thread_func`) using POSIX condition variables for safe bucket transfers.
* **Integrated Filtering:** Features a Biquad bandpass filter to process raw seismic data prior to transmission.
* **Simulated & Hardware Extensible:** Currently generates simulated accelerometer, humidity, and seismic data. Hardware sensor integration can be added directly inside `sim_thread_func` in `client.c`.
* **Yocto Integration Ready:** Simple build configuration targeting Embedded Linux deployments

---

## Hardware Integration

The data acquisition loop is located in `client.c` within `sim_thread_func()`. To swap out simulated data for actual physical sensors (e.g., I2C, SPI, UART, or GPIO-based hardware):

1. Locate `sim_thread_func()` in `client.c`.
2. Replace the random math calls (`random_float`, `sin`, `cos`) with your actual hardware driver read APIs.
3. Map your real readings directly to the `SensorData` fields (`accel_x`, `accel_y`, `accel_z`, `humidity`, `seismo`).

---

## Local Build & Execution

### Building Locally

To build the client executable locally on your machine, run `make`:

```bash
make
```

### YOCTO Integration
IMAGE_INSTALL:append = " sensor-client"